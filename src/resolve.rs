use chrono::{
    offset::{TimeZone, Utc},
    DateTime,
};
use log::debug;
use reqwest::Client;
use serde::Deserialize;
use std::error::Error;

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
pub struct Version {
    pub group: String,
    pub artifact: String,
    pub version: String,
    pub publish_time: DateTime<Utc>,
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

pub trait Resolver {
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
