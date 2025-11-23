use std::env;


pub struct LighthouseAPI {
    pub base_url: String,
    pub api_version: String,
}

impl LighthouseAPI {
    pub fn new(base_url: &str, api_version: &str) -> LighthouseAPI {
        LighthouseAPI {
            base_url: base_url.to_string(),
            api_version: api_version.to_string(),
        }
    }
}


struct LighthouseAPIBaseURL(String);

impl LighthouseAPIBaseURL {
    pub fn from(base_url: &str) -> Result<Self, String> {
        // Regex pattern:
        // 1. localhost (http or https, any port)
        // 2. OR *.projectlighthouse.io with https only
        let pattern = r"^(https?://localhost(:\d+)?(/.*)?|https://([a-zA-Z0-9-]+\.)*projectlighthouse\.io(/.*)?)\s*$";

        let re = regex::Regex::new(pattern)
            .map_err(|e| format!("Invalid regex pattern: {}", e))?;

        if re.is_match(base_url) {
            Ok(LighthouseAPIBaseURL(base_url.to_string()))
        } else {
            Err("Invalid URL: must be localhost or https://*.projectlighthouse.io".to_string())
        }
    }
}


impl Default for LighthouseAPI {
    fn default() -> Self {

        let base_url = match env::var("LUX_API_BASE_URL") {
            Ok(val) => val,
            Err(_) => {
                "v1".to_string()
            },
        };

        log::info!("initiating lighthouse api with {}", base_url);

        LighthouseAPI::new("v1", "v1")
    }
}

impl LighthouseAPI {
    pub fn ping(&self) {
        log::info!("pong")
    }

    fn get(&self) {

    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lighthouse_api_base_url_validation() {
        // Valid localhost URLs
        assert!(LighthouseAPIBaseURL::from("http://localhost").is_ok());
        assert!(LighthouseAPIBaseURL::from("https://localhost").is_ok());
        assert!(LighthouseAPIBaseURL::from("http://localhost:8080").is_ok());
        assert!(LighthouseAPIBaseURL::from("https://localhost:3000/api").is_ok());

        // Valid projectlighthouse.io URLs (https only)
        assert!(LighthouseAPIBaseURL::from("https://projectlighthouse.io").is_ok());
        assert!(LighthouseAPIBaseURL::from("https://api.projectlighthouse.io").is_ok());
        assert!(LighthouseAPIBaseURL::from("https://api.projectlighthouse.io/v1").is_ok());

        // Invalid URLs
        assert!(LighthouseAPIBaseURL::from("http://projectlighthouse.io").is_err()); // http not allowed
        assert!(LighthouseAPIBaseURL::from("https://example.com").is_err()); // wrong domain
        assert!(LighthouseAPIBaseURL::from("ftp://localhost").is_err()); // wrong scheme
    }
}

