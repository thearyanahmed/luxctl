use secrecy::{ExposeSecret, SecretString};
pub struct Config {
    token: SecretString,
}

impl Config {
    pub fn new(token : impl Into<String>) -> Config {
        Config {
            token: SecretString::from(token.into())
        }
    }

    pub fn token(&self) -> &SecretString {
        &self.token
    }

    pub fn expose_token(&self) -> &str {
        self.token.expose_secret()
    }
}


