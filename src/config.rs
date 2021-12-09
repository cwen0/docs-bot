use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::time::Instant;

static CONFIG_FILE_NAME: &str = "docsbot.toml";
// const REFRESH_EVERY: Duration = Duration::from_secs(2 * 60); // Every two minutes

lazy_static::lazy_static! {
    static ref CONFIG_CACHE:
        RwLock<HashMap<String, (Result<Arc<Config>, ConfigurationError>, Instant)>> =
        RwLock::new(HashMap::new());
}

#[derive(PartialEq, Eq, Debug, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    pub repos: Vec<RepoConfig>,
}

#[derive(PartialEq, Eq, Clone, Debug, serde::Deserialize)]
pub struct RepoConfig {
    pub name: String,
    pub labels: Vec<LabelConfig>,
}

#[derive(PartialEq, Eq, Clone, Debug, serde::Deserialize)]
pub struct LabelConfig {
    pub label: String,
    pub base_branch: String,
    pub sync_paths: Vec<SyncPath>,
}

#[derive(PartialEq, Eq, Clone, Debug, serde::Deserialize)]
pub struct SyncPath {
    pub source_directory: String,
    pub source_sidebars: String,
    pub target_directory: String,
    pub target_sidebars: String,
}

pub async fn get_repo_config(repo: &str) -> Result<Arc<RepoConfig>, ConfigurationError> {
    let config = parse_config_file()?;

    for repo_config in config.repos.iter() {
        let real_repo = repo_config.clone();
        if real_repo.name.eq(repo) {
            return Ok(Arc::new(real_repo));
        }
    }

    Err(ConfigurationError::Missing)
}

fn parse_config_file() -> Result<Arc<Config>, ConfigurationError> {
    let contents = fs::read_to_string(CONFIG_FILE_NAME).unwrap();

    let config = Arc::new(toml::from_str::<Config>(contents.as_str()).map_err(ConfigurationError::Toml)?);
    log::debug!("parse config {:?}", config);
    Ok(config)
}

// fn get_cached_config(repo: &str) -> Option<Result<Arc<Config>, ConfigurationError>> {
//     let cache = CONFIG_CACHE.read().unwrap();
//     cache.get(repo).and_then(|(config, fetch_time)| {
//         if fetch_time.elapsed() < REFRESH_EVERY {
//             Some(config.clone())
//         } else {
//             None
//         }
//     })
// }

// async fn get_fresh_config(
//     gh: &GithubClient,
//     repo: &str,
// ) -> Result<Arc<Config>, ConfigurationError> {
//     let contents = gh
//         .raw_file(repo, "master", CONFIG_FILE_NAME)
//         .await
//         .map_err(|e| ConfigurationError::Http(Arc::new(e)))?
//         .ok_or(ConfigurationError::Missing)?;
//     let config = Arc::new(toml::from_slice::<Config>(&contents).map_err(ConfigurationError::Toml)?);
//     log::debug!("fresh configuration for {}: {:?}", repo, config);
//     Ok(config)
// }

#[derive(Clone, Debug)]
pub enum ConfigurationError {
    Missing,
    NotFound,
    Toml(toml::de::Error),
    Http(Arc<anyhow::Error>),
}

impl std::error::Error for ConfigurationError {}

impl fmt::Display for ConfigurationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ConfigurationError::Missing => write!(
                f,
                "Repo config is not in docsbot.toml"
            ),
            ConfigurationError::NotFound => write!(
                f,
                "docsbot.toml not found"
            ),
            ConfigurationError::Toml(e) => {
                write!(f, "Malformed `docsbot.toml` in master branch.\n{}", e)
            }
            ConfigurationError::Http(_) => {
                write!(f, "Failed to query configuration for this repository.")
            }
        }
    }
}