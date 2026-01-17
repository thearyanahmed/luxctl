use crate::{
    api::{ApiUser, LighthouseAPIClient},
    config::Config,
};
use color_eyre::eyre::{eyre, Ok, Result};

pub struct TokenAuthenticator {
    pub token: String,
}

impl TokenAuthenticator {
    pub fn new(token: &str) -> Self {
        TokenAuthenticator {
            token: token.to_string(),
        }
    }

    pub async fn authenticate(&self) -> Result<ApiUser> {
        if self.token.is_empty() {
            return Err(eyre!("token must not be empty."));
        }

        // Create a temporary config to build the client with token
        let cfg = Config::new(&self.token);
        let client = LighthouseAPIClient::from_config(&cfg);

        let user = client.me().await?;

        // Save config only after successful authentication
        cfg.save()?;

        Ok(user)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_stores_token() {
        let auth = TokenAuthenticator::new("my-secret-token");
        assert_eq!(auth.token, "my-secret-token");
    }

    #[test]
    fn test_new_trims_nothing() {
        // token is stored as-is, no trimming
        let auth = TokenAuthenticator::new("  token-with-spaces  ");
        assert_eq!(auth.token, "  token-with-spaces  ");
    }

    #[tokio::test]
    async fn test_empty_token_returns_error() {
        let auth = TokenAuthenticator::new("");
        let result = auth.authenticate().await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "token must not be empty.");
    }

    #[tokio::test]
    async fn test_whitespace_only_token_is_not_empty() {
        // whitespace-only token passes empty check but will fail API call
        let auth = TokenAuthenticator::new("   ");
        let result = auth.authenticate().await;

        // should not fail with "empty" error
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(!err.contains("must not be empty"));
    }
}
