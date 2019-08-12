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

#[derive(Debug, Deserialize, Default)]
pub struct Dependency {
    pub org: String,
    pub name: Option<String>,
    pub module: Option<String>,
    pub rev: String,
}

impl Dependency {
    pub fn fullname(&self) -> String {
        format!(
            "{}/{}",
            self.org,
            self.name
                .as_ref()
                .or_else(|| self.module.as_ref())
                .unwrap_or(&"?".to_string())
        )
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_parser_parses() -> Result<(), Box<dyn Error>>{
        let deps = DefaultParser.parse("tests/data/ivy.xml".into())?;
        assert_eq!(deps.len(), 2);
        Ok(())
    }

    #[test]
    fn fullname_combines_components_for_display() {
        for (case, expect) in vec![
            (
                Dependency {
                    org: "foo".into(),
                    ..Dependency::default()
                },
                "foo/?",
            ),
            (
                Dependency {
                    org: "foo".into(),
                    module: Some("bar".into()),
                    ..Dependency::default()
                },
                "foo/bar",
            ),
            (
                Dependency {
                    org: "foo".into(),
                    name: Some("bar".into()),
                    ..Dependency::default()
                },
                "foo/bar",
            ),
        ] {
            assert_eq!(case.fullname(), expect);
        }
    }
}
