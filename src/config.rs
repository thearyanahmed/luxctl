use std::{collections::HashMap, fs, path::PathBuf};
use color_eyre::eyre::{self, Ok};
use secrecy::{ExposeSecret, SecretString};

// we'll always use this path.
static CFG_DIR : &str = ".lux";
static CFG_FILE : &str = "cfg";


pub struct Config {
    token: SecretString,
}

impl Config {
    pub fn new(token : &str) -> Config {
        Config {
            token: SecretString::from(token)
        }
    }

    pub fn token(&self) -> &SecretString {
        &self.token
    }

    pub fn expose_token(&self) -> &str {
        self.token.expose_secret()
    }
}

impl Config {

    fn config_path() -> Result<PathBuf, eyre::Error> {
        let home = dirs::home_dir()
            .ok_or_else(|| eyre::eyre!("could not determine home dir"))?;

        Ok(home.join(CFG_DIR).join(CFG_FILE))
    }

    pub fn load() -> Result<Config, eyre::Error> {
        let path = Self::config_path()?;
        let content = fs::read_to_string(&path)
            .map_err(|e| eyre::eyre!("failed to read config file: {}", e))?;

        let map: HashMap<&str, &str> = content
            .lines()
            .filter(|line| !line.trim().is_empty())
            .filter_map(|line| {
                let mut parts = line.splitn(2, "=");
                Some((parts.next()?.trim(), parts.next()?.trim()))
            })
            .collect();

        let token = map.get("token")
            .copied()
            .ok_or_else(|| eyre::eyre!("token not found in config"))?;

        Ok(Config::new(token))
    }

    pub fn exists() -> Result<bool, eyre::Error> {
        let path = Self::config_path()?;
        Ok(path.exists())
    }

    pub fn save(&self) -> Result<bool, eyre::Error> {
        let path = Self::config_path()?;

        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
            log::debug!("creating all dir {}", dir.display());
        }

        let content = format!("token={}\n", self.expose_token());
        fs::write(&path, content)?;
        log::debug!("config written successfully to path {}", path.display());

        Ok(true)
    }
}
