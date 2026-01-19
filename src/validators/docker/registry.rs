//! Docker image registry - hardcoded for security
//!
//! all allowed Docker images must be defined here at compile time.
//! this prevents arbitrary images from being executed on user machines.

use std::fmt;

/// represents a source for a Docker image
#[derive(Debug, Clone, Copy)]
pub enum ImageSource {
    /// remote registry URL (e.g., ghcr.io/org/repo:tag)
    Remote(&'static str),
    /// local Dockerfile path relative to luxctl repo
    Local(&'static str),
}

impl ImageSource {
    pub fn path(&self) -> &'static str {
        match self {
            ImageSource::Remote(url) => url,
            ImageSource::Local(path) => path,
        }
    }

    pub fn is_remote(&self) -> bool {
        matches!(self, ImageSource::Remote(_))
    }
}

/// registered Docker image with metadata
#[derive(Debug, Clone, Copy)]
pub struct RegisteredImage {
    pub key: &'static str,
    pub description: &'static str,
    pub source: ImageSource,
}

impl fmt::Display for RegisteredImage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.key)
    }
}

/// all allowed Docker images - add new images here
const REGISTERED_IMAGES: &[RegisteredImage] = &[
    RegisteredImage {
        key: "go1.22",
        description: "Go 1.22 build and test environment",
        source: ImageSource::Local("docker/Go1.22"),
    },
    RegisteredImage {
        key: "go1.22-race",
        description: "Go 1.22 with race detector enabled",
        source: ImageSource::Local("docker/Go1.22-race"),
    },
    RegisteredImage {
        key: "api-client-test",
        description: "Salvo.rs test server for API client validation",
        source: ImageSource::Remote("ghcr.io/projectlighthouse/api-client-test:latest"),
    },
];

/// lookup a registered image by key
/// returns None if the image is not in the registry (security measure)
pub fn lookup(key: &str) -> Option<&'static RegisteredImage> {
    let key_lower = key.to_lowercase();
    REGISTERED_IMAGES.iter().find(|img| img.key == key_lower)
}

/// check if an image key is registered
pub fn is_registered(key: &str) -> bool {
    lookup(key).is_some()
}

/// list all registered image keys
pub fn list_keys() -> Vec<&'static str> {
    REGISTERED_IMAGES.iter().map(|img| img.key).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lookup_existing() {
        let img = lookup("go1.22");
        assert!(img.is_some());
        assert_eq!(img.unwrap().key, "go1.22");
    }

    #[test]
    fn test_lookup_case_insensitive() {
        let img = lookup("Go1.22");
        assert!(img.is_some());
        assert_eq!(img.unwrap().key, "go1.22");
    }

    #[test]
    fn test_lookup_nonexistent() {
        let img = lookup("malicious-image");
        assert!(img.is_none());
    }

    #[test]
    fn test_is_registered() {
        assert!(is_registered("go1.22"));
        assert!(is_registered("api-client-test"));
        assert!(!is_registered("unknown"));
    }

    #[test]
    fn test_list_keys() {
        let keys = list_keys();
        assert!(keys.contains(&"go1.22"));
        assert!(keys.contains(&"go1.22-race"));
        assert!(keys.contains(&"api-client-test"));
    }
}
