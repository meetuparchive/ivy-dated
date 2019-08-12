use serde::Deserialize;
use std::{error::Error, fs::read_to_string, path::PathBuf};

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
pub struct Dependency {
    pub org: String,
    pub name: Option<String>,
    pub module: Option<String>,
    pub rev: String,
}

pub trait Parser {
    fn parse(
        &self,
        path: PathBuf,
    ) -> Result<Vec<Dependency>, Box<dyn Error>>;
}

pub struct DefaultParser;

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
