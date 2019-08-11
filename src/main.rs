use chrono::NaiveDateTime;
use log::debug;
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

#[derive(Debug, Deserialize)]
struct Doc {
    id: String,
    g: String,
    a: String,
    v: String,
    timestamp: usize,
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let Options { ivy } = Options::from_args();
    let contents = read_to_string(ivy)?;
    let client = reqwest::Client::new();
    for dependency in serde_xml_rs::from_str::<IvyModule>(&contents)?
        .dependencies
        .data
    {
        let Dependency {
            org,
            name,
            module,
            rev,
        } = dependency;
        let url = format!(
            "http://search.maven.org/solrsearch/select?q=g:%22{group}%22+AND+a:%22{artifact}%22+AND+v:%22{version}%22&wt=json",
            group = org,
            artifact = name.or(module).unwrap_or_default(),
            version = rev
        );
        debug!("{}", url);
        let result: Results = client.get(&url).send()?.json()?;
        for Doc {
            g, a, v, timestamp, ..
        } in result.response.docs
        {
            let time = NaiveDateTime::from_timestamp(timestamp as i64 / 1000, 0);
            println!("{:?} {}/{}@{}", time, g, a, v);
        }
        sleep(Duration::from_millis(200));
    }
    Ok(())
}
