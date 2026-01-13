use super::compile::CanCompileValidator;
use super::http::{
    ConcurrentRequestsValidator, HttpGetFileValidator, HttpGetValidator,
    HttpGetWithHeaderValidator, HttpHeaderPresentValidator, HttpHeaderValueValidator,
    HttpPostFileValidator, HttpStatusValidator,
};
use super::parser::{parse_validator, ParsedValidator};
use super::port::PortValidator;
use crate::tasks::TestCase;

/// Runtime validator that can execute any parsed validator type
pub enum RuntimeValidator {
    TcpListening(PortValidator),
    HttpResponseStatus(HttpStatusValidator),
    HttpGet(HttpGetValidator),
    HttpHeaderPresent(HttpHeaderPresentValidator),
    HttpHeaderValue(HttpHeaderValueValidator),
    HttpGetWithHeader(HttpGetWithHeaderValidator),
    ConcurrentRequests(ConcurrentRequestsValidator),
    HttpPostFile(HttpPostFileValidator),
    HttpGetFile(HttpGetFileValidator),
    CanCompile(CanCompileValidator),
    // placeholder for validators not yet implemented
    NotImplemented(String),
}

impl RuntimeValidator {
    pub async fn validate(&self) -> Result<TestCase, String> {
        match self {
            RuntimeValidator::TcpListening(v) => v.validate().await,
            RuntimeValidator::HttpResponseStatus(v) => v.validate().await,
            RuntimeValidator::HttpGet(v) => v.validate().await,
            RuntimeValidator::HttpHeaderPresent(v) => v.validate().await,
            RuntimeValidator::HttpHeaderValue(v) => v.validate().await,
            RuntimeValidator::HttpGetWithHeader(v) => v.validate().await,
            RuntimeValidator::ConcurrentRequests(v) => v.validate().await,
            RuntimeValidator::HttpPostFile(v) => v.validate().await,
            RuntimeValidator::HttpGetFile(v) => v.validate().await,
            RuntimeValidator::CanCompile(v) => v.validate().await,
            RuntimeValidator::NotImplemented(name) => Ok(TestCase {
                name: format!("validator '{}'", name),
                result: Err(format!("validator '{}' not implemented yet", name)),
            }),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            RuntimeValidator::TcpListening(_) => "tcp_listening",
            RuntimeValidator::HttpResponseStatus(_) => "http_response_status",
            RuntimeValidator::HttpGet(_) => "http_get",
            RuntimeValidator::HttpHeaderPresent(_) => "http_header_present",
            RuntimeValidator::HttpHeaderValue(_) => "http_header_value",
            RuntimeValidator::HttpGetWithHeader(_) => "http_get_with_header",
            RuntimeValidator::ConcurrentRequests(_) => "concurrent_requests",
            RuntimeValidator::HttpPostFile(_) => "http_post_file",
            RuntimeValidator::HttpGetFile(_) => "http_get_file",
            RuntimeValidator::CanCompile(_) => "can_compile",
            RuntimeValidator::NotImplemented(name) => name,
        }
    }
}

/// Create a RuntimeValidator from a validator DSL string
pub fn create_validator(validator_str: &str) -> Result<RuntimeValidator, String> {
    let parsed = parse_validator(validator_str)?;
    create_from_parsed(&parsed)
}

/// Create a RuntimeValidator from a parsed validator definition
fn create_from_parsed(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    match parsed.name.as_str() {
        "tcp_listening" => create_tcp_listening(parsed),
        "http_response_status" => create_http_response_status(parsed),
        "http_get" => create_http_get(parsed),
        "http_header_present" => create_http_header_present(parsed),
        "http_header_value" => create_http_header_value(parsed),
        "http_get_with_header" => create_http_get_with_header(parsed),
        "concurrent_requests" => create_concurrent_requests(parsed),
        "http_post_file" => create_http_post_file(parsed),
        "can_compile" => create_can_compile(parsed),
        "http_get_file" => create_http_get_file(parsed),
        // validators we know about but haven't implemented yet
        "file_contents_match" | "http_get_compressed" => {
            Ok(RuntimeValidator::NotImplemented(parsed.name.clone()))
        }
        _ => Ok(RuntimeValidator::NotImplemented(parsed.name.clone())),
    }
}

// tcp_listening:int(4221)
fn create_tcp_listening(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let port = parsed.param_as_int(0)? as u16;
    Ok(RuntimeValidator::TcpListening(PortValidator::new(port)))
}

// http_response_status:int(200)
fn create_http_response_status(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let status = parsed.param_as_int(0)? as u16;
    Ok(RuntimeValidator::HttpResponseStatus(
        HttpStatusValidator::new(status),
    ))
}

// http_get:string(/path),int(200) OR http_get:string(/path),int(200),string(expected_body)
fn create_http_get(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let path = parsed.param_as_string(0)?;
    let status = parsed.param_as_int(1)? as u16;
    let expected_body = parsed
        .param(2)
        .and_then(|p| p.as_string())
        .map(|s| s.to_string());

    Ok(RuntimeValidator::HttpGet(HttpGetValidator::new(
        path,
        status,
        expected_body,
    )))
}

// http_header_present:string(Content-Type),bool(true)
fn create_http_header_present(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let header_name = parsed.param_as_string(0)?;
    let should_exist = parsed.param_as_bool(1)?;

    Ok(RuntimeValidator::HttpHeaderPresent(
        HttpHeaderPresentValidator::new(header_name, should_exist),
    ))
}

// http_header_value:string(Content-Encoding),string(gzip),bool(true)
fn create_http_header_value(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let header_name = parsed.param_as_string(0)?;
    let expected_value = parsed.param_as_string(1)?;
    // third param (bool) indicates if it should match, we ignore false case for now

    Ok(RuntimeValidator::HttpHeaderValue(
        HttpHeaderValueValidator::new(header_name, expected_value),
    ))
}

// http_get_with_header:string(/user-agent),string(User-Agent),string(test-agent),int(200),string(test-agent)
fn create_http_get_with_header(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let path = parsed.param_as_string(0)?;
    let header_name = parsed.param_as_string(1)?;
    let header_value = parsed.param_as_string(2)?;
    let expected_status = parsed.param_as_int(3)? as u16;
    let expected_body = parsed
        .param(4)
        .and_then(|p| p.as_string())
        .map(|s| s.to_string());

    Ok(RuntimeValidator::HttpGetWithHeader(
        HttpGetWithHeaderValidator::new(
            path,
            header_name,
            header_value,
            expected_status,
            expected_body,
        ),
    ))
}

// concurrent_requests:int(3),string(/echo/test),int(200)
fn create_concurrent_requests(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let num_connections = parsed.param_as_int(0)? as u32;
    let path = parsed.param_as_string(1)?;
    let expected_status = parsed.param_as_int(2)? as u16;

    Ok(RuntimeValidator::ConcurrentRequests(
        ConcurrentRequestsValidator::new(num_connections, path, expected_status),
    ))
}

// http_post_file:string(/files/upload.txt),string(hello world),int(201)
fn create_http_post_file(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let path = parsed.param_as_string(0)?;
    let body = parsed.param_as_string(1)?;
    let expected_status = parsed.param_as_int(2)? as u16;

    Ok(RuntimeValidator::HttpPostFile(HttpPostFileValidator::new(
        path,
        body,
        expected_status,
    )))
}

// can_compile:bool(true)
fn create_can_compile(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let expected_success = parsed.param_as_bool(0)?;
    Ok(RuntimeValidator::CanCompile(CanCompileValidator::new(
        expected_success,
    )))
}

// http_get_file:string(/files/test.txt),int(200)
fn create_http_get_file(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let path = parsed.param_as_string(0)?;
    let expected_status = parsed.param_as_int(1)? as u16;
    Ok(RuntimeValidator::HttpGetFile(HttpGetFileValidator::new(
        path,
        expected_status,
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_tcp_listening() {
        let validator = create_validator("tcp_listening:int(4221)").unwrap();
        assert_eq!(validator.name(), "tcp_listening");
    }

    #[test]
    fn test_create_http_response_status() {
        let validator = create_validator("http_response_status:int(200)").unwrap();
        assert_eq!(validator.name(), "http_response_status");
    }

    #[test]
    fn test_create_http_get() {
        let validator = create_validator("http_get:string(/),int(200)").unwrap();
        assert_eq!(validator.name(), "http_get");
    }

    #[test]
    fn test_create_http_get_with_body() {
        let validator = create_validator("http_get:string(/echo/hello),int(200),string(hello)").unwrap();
        assert_eq!(validator.name(), "http_get");
    }

    #[test]
    fn test_create_http_header_present() {
        let validator =
            create_validator("http_header_present:string(Content-Type),bool(true)").unwrap();
        assert_eq!(validator.name(), "http_header_present");
    }

    #[test]
    fn test_create_http_get_with_header() {
        let validator = create_validator(
            "http_get_with_header:string(/user-agent),string(User-Agent),string(test-agent),int(200),string(test-agent)"
        ).unwrap();
        assert_eq!(validator.name(), "http_get_with_header");
    }

    #[test]
    fn test_create_concurrent_requests() {
        let validator =
            create_validator("concurrent_requests:int(3),string(/echo/test),int(200)").unwrap();
        assert_eq!(validator.name(), "concurrent_requests");
    }

    #[test]
    fn test_create_http_post_file() {
        let validator =
            create_validator("http_post_file:string(/files/upload.txt),string(hello world),int(201)")
                .unwrap();
        assert_eq!(validator.name(), "http_post_file");
    }

    #[test]
    fn test_unknown_validator() {
        let validator = create_validator("unknown_validator:int(1)").unwrap();
        assert_eq!(validator.name(), "unknown_validator");
        // should be NotImplemented variant
        matches!(validator, RuntimeValidator::NotImplemented(_));
    }

    #[test]
    fn test_not_implemented_validators() {
        let names = ["file_contents_match:string(/tmp/test.txt),string(expected)"];

        for name in names {
            let validator = create_validator(name).unwrap();
            matches!(validator, RuntimeValidator::NotImplemented(_));
        }
    }

    #[test]
    fn test_create_can_compile() {
        let validator = create_validator("can_compile:bool(true)").unwrap();
        assert_eq!(validator.name(), "can_compile");
    }

    #[test]
    fn test_create_http_get_file() {
        let validator = create_validator("http_get_file:string(/files/test.txt),int(200)").unwrap();
        assert_eq!(validator.name(), "http_get_file");
    }
}
