use chrono_humanize::HumanTime;
use colored::Colorize;

use reqwest::Client;
use std::{error::Error, path::PathBuf, thread::sleep, time::Duration};
use structopt::StructOpt;
mod parse;
mod resolve;

use crate::{
    parse::{DefaultParser, Dependency, Parser},
    resolve::Resolver,
};

#[derive(StructOpt, Debug)]
#[structopt(name = "ivy-dated", about = "how dated are your ivy dependecies?")]
struct Options {
    #[structopt(short = "f", long = "file", default_value = "ivy.xml")]
    ivy: PathBuf,
}

#[derive(Default, Debug)]
struct Stats {
    dated: usize,
    current: usize,
    unknown: usize,
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let Options { ivy } = Options::from_args();
    let client = Client::new();
    let stats: Result<Stats, Box<dyn Error>> = DefaultParser.parse(ivy)?.into_iter().try_fold(
        Stats::default(),
        |mut stats, dependency| {
            let Dependency {
                org,
                name,
                module,
                rev,
            } = dependency;
            let artifact = name.or(module).unwrap_or_default();
            match client.pinned_version(org.clone(), artifact.clone(), rev.clone())? {
                Some(pinned) => {
                    if let Some(latest) = client.latest_version(org.clone(), artifact.clone())? {
                        let current = latest.version == pinned.version;
                        if current {
                            stats.current += 1;
                            println!(
                                "{} {}/{}@{} 👌",
                                pinned.publish_time.to_string().bright_black(),
                                pinned.group.bold(),
                                pinned.artifact.bold(),
                                pinned.version.bold(),
                            )
                        } else {
                            stats.dated += 1;
                            let lag = pinned.publish_time - latest.publish_time;
                            println!(
                                "{} {}/{}@{} -> {} upgrade available {}",
                                pinned.publish_time.to_string().bright_black(),
                                pinned.group.bold(),
                                pinned.artifact.bold(),
                                pinned.version.bold().bright_yellow(),
                                latest.version.bold().bright_green(),
                                HumanTime::from(lag).to_string().bold()
                            )
                        };
                    }
                }
                _ => {
                    stats.unknown += 1;
                    println!(
                        "⚠️ no information found on {} {}@{}",
                        org,
                        artifact.bold(),
                        rev.bold()
                    )
                }
            }
            sleep(Duration::from_millis(200));
            Ok(stats)
        },
    );
    let Stats {
        dated,
        current,
        unknown,
    } = stats?;
    println!();
    println!("Dated: {} Current: {} Unknown: {}", dated, current, unknown);
    Ok(())
}
