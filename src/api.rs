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

