use chrono::{
    offset::{TimeZone, Utc},
    DateTime,
};
use chrono_humanize::HumanTime;
use colored::Colorize;
use log::debug;
use reqwest::Client;
use serde::Deserialize;
use std::{error::Error, fs::read_to_string, path::PathBuf, thread::sleep, time::Duration};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "ivy-dated", about = "how dated are your ivy dependecies?")]
struct Options {
    #[structopt(short = "f", long = "file", default_value = "ivy.xml")]
    ivy: PathBuf,
}

/// Ivy config shape
#[derive(Debug, Deserialize)]
struct IvyModule {
    pub dependencies: Dependencies,
}

#[derive(Debug, Deserialize)]
struct Dependencies {
    #[serde(rename = "$value")]
    pub data: Vec<Dependency>,
}

#[derive(Debug, Deserialize)]
struct Dependency {
    org: String,
    name: Option<String>,
    module: Option<String>,
    rev: String,
}

/// maven central search response shape
#[derive(Debug, Deserialize)]
struct Results {
    response: Response,
}

#[derive(Debug, Deserialize)]
struct Response {
    docs: Vec<Doc>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Doc {
    id: String,
    g: String,
    a: String,
    v: Option<String>,
    latest_version: Option<String>,
    timestamp: usize,
}

#[derive(Debug)]
struct Version {
    group: String,
    artifact: String,
    version: String,
    publish_time: DateTime<Utc>,
}

impl Into<Version> for Doc {
    fn into(self: Doc) -> Version {
        Version {
            group: self.g,
            artifact: self.a,
            version: self.v.or(self.latest_version).unwrap_or_default(),
            publish_time: Utc.timestamp(self.timestamp as i64 / 1000, 0),
        }
    }
}

trait Parser {
    fn parse(
        &self,
        path: PathBuf,
    ) -> Result<Vec<Dependency>, Box<dyn Error>>;
}

struct DefaultParser;

impl Parser for DefaultParser {
    fn parse(
        &self,
        path: PathBuf,
    ) -> Result<Vec<Dependency>, Box<dyn Error>> {
        let contents = read_to_string(path)?;
        Ok(serde_xml_rs::from_str::<IvyModule>(&contents)?
            .dependencies
            .data)
    }
}

trait Resolver {
    fn version_info(
        &self,
        group: String,
        artifact: String,
        version: String,
    ) -> Result<Option<Version>, Box<dyn Error>>;

    fn latest_version(
        &self,
        group: String,
        artifact: String,
    ) -> Result<Option<Version>, Box<dyn Error>>;
}

impl Resolver for Client {
    fn version_info(
        &self,
        group: String,
        artifact: String,
        version: String,
    ) -> Result<Option<Version>, Box<dyn Error>> {
        let url = format!(
            "http://search.maven.org/solrsearch/select?q=g:%22{group}%22+AND+a:%22{artifact}%22+AND+v:%22{version}%22&wt=json",
            group = group,
            artifact = artifact,
            version = version
        );
        debug!("pinned {}", url);
        let result: Results = self.get(&url).send()?.json()?;
        Ok(result.response.docs.first().cloned().map(Doc::into))
    }

    fn latest_version(
        &self,
        group: String,
        artifact: String,
    ) -> Result<Option<Version>, Box<dyn Error>> {
        let url = format!(
            "http://search.maven.org/solrsearch/select?q=g:%22{group}%22+AND+a:%22{artifact}%22&wt=json&core=gav",
            group = group,
            artifact = artifact
        );
        debug!("latest {}", url);
        let result: Results = self.get(&url).send()?.json()?;
        Ok(result.response.docs.first().cloned().map(Doc::into))
    }
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
    let client = reqwest::Client::new();
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
            match client.version_info(org.clone(), artifact.clone(), rev.clone())? {
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
