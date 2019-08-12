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

const REQUEST_DELAY: Duration = Duration::from_millis(200);

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
    let mut dependencies = DefaultParser.parse(ivy)?;
    dependencies.sort_by(|a, b| a.fullname().cmp(&b.fullname()));
    let stats: Result<Stats, Box<dyn Error>> =
        dependencies
            .into_iter()
            .try_fold(Stats::default(), |stats, dependency| {
                sleep(REQUEST_DELAY);
                let fullname = dependency.fullname();
                let Dependency {
                    org,
                    name,
                    module,
                    rev,
                } = dependency;
                let artifact = name.or(module).unwrap_or_default();
                if let Some(pinned) = client.pinned_version(&org, &artifact, &rev)? {
                    if let Some(latest) = client.latest_version(&org, &artifact)? {
                        if latest.version == pinned.version {
                            println!(
                                "{} {}@{} 👌",
                                pinned.publish_time.to_string().bright_black(),
                                fullname,
                                pinned.version.bold(),
                            );
                            return Ok(Stats {
                                current: stats.current + 1,
                                ..stats
                            });
                        } else {
                            let lag = pinned.publish_time - latest.publish_time;
                            println!(
                                "{} {}@{} -> {} upgrade available {}",
                                pinned.publish_time.to_string().bright_black(),
                                fullname.bold(),
                                pinned.version.bold().bright_yellow(),
                                latest.version.bold().bright_green(),
                                HumanTime::from(lag).to_string().bold()
                            );
                            return Ok(Stats {
                                dated: stats.dated + 1,
                                ..stats
                            });
                        }
                    }
                }
                println!(
                    "⚠️ no information found on {}@{}",
                    fullname.bold(),
                    rev.bold()
                );
                Ok(Stats {
                    unknown: stats.unknown + 1,
                    ..stats
                })
            });
    let Stats {
        dated,
        current,
        unknown,
    } = stats?;
    println!();
    println!("Dated: {} Current: {} Unknown: {}", dated, current, unknown);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_profiles_file_param() -> Result<(), Box<dyn Error>> {
        let Options { ivy } = Options::from_iter_safe(&["ivy-dated", "-f", "path/to/test.xml"])?;
        assert_eq!(ivy, PathBuf::from("path/to/test.xml"));
        Ok(())
    }

    #[test]
    fn options_defines_a_default_file() -> Result<(), Box<dyn Error>> {
        let Options { ivy } = Options::from_iter_safe(&["ivy-dated"])?;
        assert_eq!(ivy, PathBuf::from("ivy.xml"));
        Ok(())
    }
}
