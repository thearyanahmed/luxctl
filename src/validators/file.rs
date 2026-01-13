use crate::tasks::TestCase;
use std::path::Path;
use tokio::fs;

/// Validator: check if file contents match expected value
pub struct FileContentsMatchValidator {
    pub path: String,
    pub expected_content: String,
}

impl FileContentsMatchValidator {
    pub fn new(path: &str, expected_content: &str) -> Self {
        Self {
            path: path.to_string(),
            expected_content: expected_content.to_string(),
        }
    }

    pub async fn validate(&self) -> Result<TestCase, String> {
        let path = Path::new(&self.path);

        if !path.exists() {
            return Ok(TestCase {
                name: format!("file {} exists", self.path),
                result: Err(format!("file '{}' does not exist", self.path)),
            });
        }

        let content = fs::read_to_string(path)
            .await
            .map_err(|e| format!("failed to read '{}': {}", self.path, e))?;

        let content_trimmed = content.trim();
        let expected_trimmed = self.expected_content.trim();

        let result = if content_trimmed == expected_trimmed {
            Ok(format!("file '{}' content matches expected", self.path))
        } else {
            // show preview of mismatch
            let content_preview: String = content_trimmed.chars().take(50).collect();
            let expected_preview: String = expected_trimmed.chars().take(50).collect();
            Err(format!(
                "content mismatch:\n  expected: '{}...'\n  got: '{}...'",
                expected_preview, content_preview
            ))
        };

        Ok(TestCase {
            name: format!("file '{}' content matches", self.path),
            result,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_file_contents_match_success() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "hello world").unwrap();

        let validator =
            FileContentsMatchValidator::new(file.path().to_str().unwrap(), "hello world");
        let result = validator.validate().await.unwrap();
        assert!(result.passed());
    }

    #[tokio::test]
    async fn test_file_contents_match_failure() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "hello world").unwrap();

        let validator =
            FileContentsMatchValidator::new(file.path().to_str().unwrap(), "goodbye world");
        let result = validator.validate().await.unwrap();
        assert!(!result.passed());
    }

    #[tokio::test]
    async fn test_file_not_exists() {
        let validator = FileContentsMatchValidator::new("/nonexistent/path.txt", "content");
        let result = validator.validate().await.unwrap();
        assert!(!result.passed());
        assert!(result.message().contains("does not exist"));
    }
}
