use crate::tasks::TestCase;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);
const DEFAULT_PORT: u16 = 4221;

/// HTTP response parsed into parts
#[derive(Debug)]
pub struct HttpResponse {
    pub status_code: u16,
    pub status_text: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

impl HttpResponse {
    pub fn parse(raw: &str) -> Result<Self, String> {
        let mut lines = raw.lines();

        // parse status line: HTTP/1.1 200 OK
        let status_line = lines.next().ok_or("empty response")?;
        let parts: Vec<&str> = status_line.splitn(3, ' ').collect();
        if parts.len() < 2 {
            return Err(format!("invalid status line: {}", status_line));
        }

        let status_code: u16 = parts[1]
            .parse()
            .map_err(|_| format!("invalid status code: {}", parts[1]))?;
        let status_text = parts.get(2).unwrap_or(&"").to_string();

        // parse headers until empty line
        let mut headers = Vec::new();
        for line in lines.by_ref() {
            if line.is_empty() || line == "\r" {
                break;
            }
            if let Some((key, value)) = line.split_once(':') {
                headers.push((key.trim().to_lowercase(), value.trim().to_string()));
            }
        }

        // rest is body
        let body: String = lines.collect::<Vec<_>>().join("\n");

        Ok(HttpResponse {
            status_code,
            status_text,
            headers,
            body,
        })
    }

    pub fn get_header(&self, name: &str) -> Option<&str> {
        let name_lower = name.to_lowercase();
        self.headers
            .iter()
            .find(|(k, _)| k == &name_lower)
            .map(|(_, v)| v.as_str())
    }

    pub fn has_header(&self, name: &str) -> bool {
        self.get_header(name).is_some()
    }
}

/// Send an HTTP request and get the response
pub async fn http_request(
    port: u16,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: Option<&str>,
) -> Result<HttpResponse, String> {
    let addr = format!("127.0.0.1:{}", port);

    let connect_result = timeout(DEFAULT_TIMEOUT, TcpStream::connect(&addr)).await;
    let mut stream = match connect_result {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => return Err(format!("connection failed: {}", e)),
        Err(_) => return Err("connection timeout".to_string()),
    };

    // build request
    let mut request = format!("{} {} HTTP/1.1\r\n", method, path);
    request.push_str("Host: 127.0.0.1\r\n");
    request.push_str("Connection: close\r\n");

    for (key, value) in headers {
        request.push_str(&format!("{}: {}\r\n", key, value));
    }

    if let Some(body_content) = body {
        request.push_str(&format!("Content-Length: {}\r\n", body_content.len()));
    }

    request.push_str("\r\n");

    if let Some(body_content) = body {
        request.push_str(body_content);
    }

    stream
        .write_all(request.as_bytes())
        .await
        .map_err(|e| format!("failed to send request: {}", e))?;

    // read response with timeout
    let mut response = Vec::new();
    let read_result = timeout(DEFAULT_TIMEOUT, stream.read_to_end(&mut response)).await;

    match read_result {
        Ok(Ok(_)) => {}
        Ok(Err(e)) => return Err(format!("failed to read response: {}", e)),
        Err(_) => return Err("read timeout".to_string()),
    }

    let response_str = String::from_utf8_lossy(&response);
    HttpResponse::parse(&response_str)
}

/// Validator: check if server responds with expected status code
pub struct HttpStatusValidator {
    pub port: u16,
    pub expected_status: u16,
}

impl HttpStatusValidator {
    pub fn new(expected_status: u16) -> Self {
        Self {
            port: DEFAULT_PORT,
            expected_status,
        }
    }

    pub async fn validate(&self) -> Result<TestCase, String> {
        let response = http_request(self.port, "GET", "/", &[], None).await?;

        let result = if response.status_code == self.expected_status {
            Ok(format!("server returned {} as expected", self.expected_status))
        } else {
            Err(format!(
                "expected status {}, got {}",
                self.expected_status, response.status_code
            ))
        };

        Ok(TestCase {
            name: format!("http response status {}", self.expected_status),
            result,
        })
    }
}

/// Validator: GET request with path, expected status, and optional body check
pub struct HttpGetValidator {
    pub port: u16,
    pub path: String,
    pub expected_status: u16,
    pub expected_body: Option<String>,
}

impl HttpGetValidator {
    pub fn new(path: &str, expected_status: u16, expected_body: Option<String>) -> Self {
        Self {
            port: DEFAULT_PORT,
            path: path.to_string(),
            expected_status,
            expected_body,
        }
    }

    pub async fn validate(&self) -> Result<TestCase, String> {
        let response = http_request(self.port, "GET", &self.path, &[], None).await?;

        let mut errors = Vec::new();

        if response.status_code != self.expected_status {
            errors.push(format!(
                "expected status {}, got {}",
                self.expected_status, response.status_code
            ));
        }

        if let Some(ref expected) = self.expected_body {
            let body_trimmed = response.body.trim();
            if body_trimmed != expected {
                errors.push(format!(
                    "expected body '{}', got '{}'",
                    expected, body_trimmed
                ));
            }
        }

        let result = if errors.is_empty() {
            Ok(format!(
                "GET {} returned {} OK",
                self.path, self.expected_status
            ))
        } else {
            Err(errors.join("; "))
        };

        Ok(TestCase {
            name: format!("GET {} returns {}", self.path, self.expected_status),
            result,
        })
    }
}

/// Validator: check if a header is present in the response
pub struct HttpHeaderPresentValidator {
    pub port: u16,
    pub path: String,
    pub header_name: String,
    pub should_exist: bool,
}

impl HttpHeaderPresentValidator {
    pub fn new(header_name: &str, should_exist: bool) -> Self {
        Self {
            port: DEFAULT_PORT,
            path: "/".to_string(),
            header_name: header_name.to_string(),
            should_exist,
        }
    }

    pub fn with_path(mut self, path: &str) -> Self {
        self.path = path.to_string();
        self
    }

    pub async fn validate(&self) -> Result<TestCase, String> {
        let response = http_request(self.port, "GET", &self.path, &[], None).await?;

        let has_header = response.has_header(&self.header_name);
        let result = if has_header == self.should_exist {
            if self.should_exist {
                Ok(format!("header '{}' is present", self.header_name))
            } else {
                Ok(format!("header '{}' is absent as expected", self.header_name))
            }
        } else if self.should_exist {
            Err(format!("header '{}' not found in response", self.header_name))
        } else {
            Err(format!(
                "header '{}' should not be present",
                self.header_name
            ))
        };

        Ok(TestCase {
            name: format!(
                "header '{}' {}",
                self.header_name,
                if self.should_exist {
                    "present"
                } else {
                    "absent"
                }
            ),
            result,
        })
    }
}

/// Validator: check header has specific value
pub struct HttpHeaderValueValidator {
    pub port: u16,
    pub path: String,
    pub header_name: String,
    pub expected_value: String,
}

impl HttpHeaderValueValidator {
    pub fn new(header_name: &str, expected_value: &str) -> Self {
        Self {
            port: DEFAULT_PORT,
            path: "/".to_string(),
            header_name: header_name.to_string(),
            expected_value: expected_value.to_string(),
        }
    }

    pub async fn validate(&self) -> Result<TestCase, String> {
        let response = http_request(self.port, "GET", &self.path, &[], None).await?;

        let result = match response.get_header(&self.header_name) {
            Some(value) if value == self.expected_value => {
                Ok(format!(
                    "header '{}' has value '{}'",
                    self.header_name, self.expected_value
                ))
            }
            Some(value) => Err(format!(
                "header '{}' expected '{}', got '{}'",
                self.header_name, self.expected_value, value
            )),
            None => Err(format!("header '{}' not found", self.header_name)),
        };

        Ok(TestCase {
            name: format!("header '{}' = '{}'", self.header_name, self.expected_value),
            result,
        })
    }
}

/// Validator: GET with custom request header
pub struct HttpGetWithHeaderValidator {
    pub port: u16,
    pub path: String,
    pub request_header: (String, String),
    pub expected_status: u16,
    pub expected_body: Option<String>,
}

impl HttpGetWithHeaderValidator {
    pub fn new(
        path: &str,
        header_name: &str,
        header_value: &str,
        expected_status: u16,
        expected_body: Option<String>,
    ) -> Self {
        Self {
            port: DEFAULT_PORT,
            path: path.to_string(),
            request_header: (header_name.to_string(), header_value.to_string()),
            expected_status,
            expected_body,
        }
    }

    pub async fn validate(&self) -> Result<TestCase, String> {
        let headers = [(
            self.request_header.0.as_str(),
            self.request_header.1.as_str(),
        )];
        let response = http_request(self.port, "GET", &self.path, &headers, None).await?;

        let mut errors = Vec::new();

        if response.status_code != self.expected_status {
            errors.push(format!(
                "expected status {}, got {}",
                self.expected_status, response.status_code
            ));
        }

        if let Some(ref expected) = self.expected_body {
            let body_trimmed = response.body.trim();
            if body_trimmed != expected {
                errors.push(format!(
                    "expected body '{}', got '{}'",
                    expected, body_trimmed
                ));
            }
        }

        let result = if errors.is_empty() {
            Ok(format!(
                "GET {} with header {}={} returned {} OK",
                self.path, self.request_header.0, self.request_header.1, self.expected_status
            ))
        } else {
            Err(errors.join("; "))
        };

        Ok(TestCase {
            name: format!(
                "GET {} with {}: {}",
                self.path, self.request_header.0, self.request_header.1
            ),
            result,
        })
    }
}

/// Validator: test concurrent connections
pub struct ConcurrentRequestsValidator {
    pub port: u16,
    pub num_connections: u32,
    pub path: String,
    pub expected_status: u16,
}

impl ConcurrentRequestsValidator {
    pub fn new(num_connections: u32, path: &str, expected_status: u16) -> Self {
        Self {
            port: DEFAULT_PORT,
            num_connections,
            path: path.to_string(),
            expected_status,
        }
    }

    pub async fn validate(&self) -> Result<TestCase, String> {
        let mut handles = Vec::new();

        for i in 0..self.num_connections {
            let port = self.port;
            let path = self.path.clone();
            let expected = self.expected_status;

            let handle = tokio::spawn(async move {
                let response = http_request(port, "GET", &path, &[], None).await?;
                if response.status_code == expected {
                    Ok(i)
                } else {
                    Err(format!(
                        "connection {} got status {} instead of {}",
                        i, response.status_code, expected
                    ))
                }
            });
            handles.push(handle);
        }

        let mut successes = 0;
        let mut errors = Vec::new();

        for handle in handles {
            match handle.await {
                Ok(Ok(_)) => successes += 1,
                Ok(Err(e)) => errors.push(e),
                Err(e) => errors.push(format!("task failed: {}", e)),
            }
        }

        let result = if successes == self.num_connections {
            Ok(format!(
                "all {} concurrent requests succeeded",
                self.num_connections
            ))
        } else {
            Err(format!(
                "{}/{} requests succeeded. errors: {}",
                successes,
                self.num_connections,
                errors.join("; ")
            ))
        };

        Ok(TestCase {
            name: format!("{} concurrent requests", self.num_connections),
            result,
        })
    }
}

/// Validator: POST request with file content
pub struct HttpPostFileValidator {
    pub port: u16,
    pub path: String,
    pub body: String,
    pub expected_status: u16,
}

impl HttpPostFileValidator {
    pub fn new(path: &str, body: &str, expected_status: u16) -> Self {
        Self {
            port: DEFAULT_PORT,
            path: path.to_string(),
            body: body.to_string(),
            expected_status,
        }
    }

    pub async fn validate(&self) -> Result<TestCase, String> {
        let response = http_request(self.port, "POST", &self.path, &[], Some(&self.body)).await?;

        let result = if response.status_code == self.expected_status {
            Ok(format!(
                "POST {} returned {} as expected",
                self.path, self.expected_status
            ))
        } else {
            Err(format!(
                "expected status {}, got {}",
                self.expected_status, response.status_code
            ))
        };

        Ok(TestCase {
            name: format!("POST {} returns {}", self.path, self.expected_status),
            result,
        })
    }
}

/// Validator: GET file from server and validate status
pub struct HttpGetFileValidator {
    pub port: u16,
    pub path: String,
    pub expected_status: u16,
}

impl HttpGetFileValidator {
    pub fn new(path: &str, expected_status: u16) -> Self {
        Self {
            port: DEFAULT_PORT,
            path: path.to_string(),
            expected_status,
        }
    }

    pub async fn validate(&self) -> Result<TestCase, String> {
        let response = http_request(self.port, "GET", &self.path, &[], None).await?;

        let result = if response.status_code == self.expected_status {
            let content_info = response
                .get_header("content-length")
                .map(|len| format!(" ({} bytes)", len))
                .unwrap_or_default();
            Ok(format!(
                "GET {} returned {}{} OK",
                self.path, self.expected_status, content_info
            ))
        } else {
            Err(format!(
                "expected status {}, got {}",
                self.expected_status, response.status_code
            ))
        };

        Ok(TestCase {
            name: format!("GET file {} returns {}", self.path, self.expected_status),
            result,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_http_response() {
        let raw = "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 5\r\n\r\nhello";
        let response = HttpResponse::parse(raw).unwrap();

        assert_eq!(response.status_code, 200);
        assert_eq!(response.status_text, "OK");
        assert_eq!(response.get_header("content-type"), Some("text/plain"));
        assert_eq!(response.get_header("Content-Type"), Some("text/plain")); // case insensitive
        assert_eq!(response.body, "hello");
    }

    #[test]
    fn test_parse_http_response_no_body() {
        let raw = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n";
        let response = HttpResponse::parse(raw).unwrap();

        assert_eq!(response.status_code, 404);
        assert_eq!(response.status_text, "Not Found");
        assert!(response.body.is_empty());
    }

    #[test]
    fn test_has_header() {
        let raw = "HTTP/1.1 200 OK\r\nX-Custom: value\r\n\r\n";
        let response = HttpResponse::parse(raw).unwrap();

        assert!(response.has_header("X-Custom"));
        assert!(response.has_header("x-custom")); // case insensitive
        assert!(!response.has_header("X-Missing"));
    }
}
