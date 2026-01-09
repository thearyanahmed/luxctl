use std::fmt;

/// Parsed parameter value with type information
#[derive(Debug, Clone, PartialEq)]
pub enum ParamValue {
    Bool(bool),
    Int(i64),
    String(String),
}

impl fmt::Display for ParamValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParamValue::Bool(b) => write!(f, "{}", b),
            ParamValue::Int(i) => write!(f, "{}", i),
            ParamValue::String(s) => write!(f, "{}", s),
        }
    }
}

impl ParamValue {
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            ParamValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            ParamValue::Int(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<&str> {
        match self {
            ParamValue::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn into_string(self) -> Option<String> {
        match self {
            ParamValue::String(s) => Some(s),
            _ => None,
        }
    }
}

/// A parsed validator definition
#[derive(Debug, Clone)]
pub struct ParsedValidator {
    pub name: String,
    pub params: Vec<ParamValue>,
}

impl ParsedValidator {
    /// Get parameter at index, returns None if out of bounds
    pub fn param(&self, index: usize) -> Option<&ParamValue> {
        self.params.get(index)
    }

    /// Get parameter as specific type with better error messages
    pub fn param_as_int(&self, index: usize) -> Result<i64, String> {
        self.param(index)
            .ok_or_else(|| format!("missing parameter at index {}", index))?
            .as_int()
            .ok_or_else(|| format!("parameter {} is not an integer", index))
    }

    pub fn param_as_string(&self, index: usize) -> Result<&str, String> {
        self.param(index)
            .ok_or_else(|| format!("missing parameter at index {}", index))?
            .as_string()
            .ok_or_else(|| format!("parameter {} is not a string", index))
    }

    pub fn param_as_bool(&self, index: usize) -> Result<bool, String> {
        self.param(index)
            .ok_or_else(|| format!("missing parameter at index {}", index))?
            .as_bool()
            .ok_or_else(|| format!("parameter {} is not a boolean", index))
    }
}

/// Parse a single typed parameter like "int(4221)" or "string(/path)"
fn parse_typed_param(input: &str) -> Result<ParamValue, String> {
    let input = input.trim();

    if let Some(inner) = input.strip_prefix("bool(").and_then(|s| s.strip_suffix(')')) {
        match inner.to_lowercase().as_str() {
            "true" => return Ok(ParamValue::Bool(true)),
            "false" => return Ok(ParamValue::Bool(false)),
            _ => return Err(format!("invalid boolean value: {}", inner)),
        }
    }

    if let Some(inner) = input.strip_prefix("int(").and_then(|s| s.strip_suffix(')')) {
        let value: i64 = inner
            .parse()
            .map_err(|_| format!("invalid integer value: {}", inner))?;
        return Ok(ParamValue::Int(value));
    }

    if let Some(inner) = input.strip_prefix("string(").and_then(|s| s.strip_suffix(')')) {
        return Ok(ParamValue::String(inner.to_string()));
    }

    Err(format!(
        "invalid parameter format: {}. expected bool(...), int(...), or string(...)",
        input
    ))
}

/// Parse a validator string like "tcp_listening:int(4221)"
/// Format: validator_name:param1,param2,...
pub fn parse_validator(input: &str) -> Result<ParsedValidator, String> {
    let input = input.trim();

    // split on first colon to get name and params
    let (name, params_str) = match input.split_once(':') {
        Some((n, p)) => (n.trim(), Some(p.trim())),
        None => (input, None),
    };

    if name.is_empty() {
        return Err("validator name cannot be empty".to_string());
    }

    let params = match params_str {
        Some(p) if !p.is_empty() => parse_params(p)?,
        _ => Vec::new(),
    };

    Ok(ParsedValidator {
        name: name.to_string(),
        params,
    })
}

/// Parse comma-separated parameters, handling nested parentheses
fn parse_params(input: &str) -> Result<Vec<ParamValue>, String> {
    let mut params = Vec::new();
    let mut current = String::new();
    let mut depth = 0;

    for ch in input.chars() {
        match ch {
            '(' => {
                depth += 1;
                current.push(ch);
            }
            ')' => {
                depth -= 1;
                current.push(ch);
            }
            ',' if depth == 0 => {
                if !current.trim().is_empty() {
                    params.push(parse_typed_param(&current)?);
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    // handle last parameter
    if !current.trim().is_empty() {
        params.push(parse_typed_param(&current)?);
    }

    Ok(params)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bool_true() {
        let result = parse_typed_param("bool(true)").unwrap();
        assert_eq!(result, ParamValue::Bool(true));
    }

    #[test]
    fn test_parse_bool_false() {
        let result = parse_typed_param("bool(false)").unwrap();
        assert_eq!(result, ParamValue::Bool(false));
    }

    #[test]
    fn test_parse_int() {
        let result = parse_typed_param("int(4221)").unwrap();
        assert_eq!(result, ParamValue::Int(4221));
    }

    #[test]
    fn test_parse_negative_int() {
        let result = parse_typed_param("int(-42)").unwrap();
        assert_eq!(result, ParamValue::Int(-42));
    }

    #[test]
    fn test_parse_string() {
        let result = parse_typed_param("string(/echo/hello)").unwrap();
        assert_eq!(result, ParamValue::String("/echo/hello".to_string()));
    }

    #[test]
    fn test_parse_string_with_spaces() {
        let result = parse_typed_param("string(hello world)").unwrap();
        assert_eq!(result, ParamValue::String("hello world".to_string()));
    }

    #[test]
    fn test_parse_validator_no_params() {
        let result = parse_validator("can_compile").unwrap();
        assert_eq!(result.name, "can_compile");
        assert!(result.params.is_empty());
    }

    #[test]
    fn test_parse_validator_single_param() {
        let result = parse_validator("tcp_listening:int(4221)").unwrap();
        assert_eq!(result.name, "tcp_listening");
        assert_eq!(result.params.len(), 1);
        assert_eq!(result.params[0], ParamValue::Int(4221));
    }

    #[test]
    fn test_parse_validator_multiple_params() {
        let result = parse_validator("http_get:string(/),int(200)").unwrap();
        assert_eq!(result.name, "http_get");
        assert_eq!(result.params.len(), 2);
        assert_eq!(result.params[0], ParamValue::String("/".to_string()));
        assert_eq!(result.params[1], ParamValue::Int(200));
    }

    #[test]
    fn test_parse_validator_three_params() {
        let result = parse_validator("http_get:string(/echo/hello),int(200),string(hello)").unwrap();
        assert_eq!(result.name, "http_get");
        assert_eq!(result.params.len(), 3);
        assert_eq!(
            result.params[0],
            ParamValue::String("/echo/hello".to_string())
        );
        assert_eq!(result.params[1], ParamValue::Int(200));
        assert_eq!(result.params[2], ParamValue::String("hello".to_string()));
    }

    #[test]
    fn test_parse_validator_bool_param() {
        let result = parse_validator("can_compile:bool(true)").unwrap();
        assert_eq!(result.name, "can_compile");
        assert_eq!(result.params.len(), 1);
        assert_eq!(result.params[0], ParamValue::Bool(true));
    }

    #[test]
    fn test_parse_validator_mixed_params() {
        let result =
            parse_validator("http_header_present:string(Content-Type),bool(true)").unwrap();
        assert_eq!(result.name, "http_header_present");
        assert_eq!(result.params.len(), 2);
        assert_eq!(
            result.params[0],
            ParamValue::String("Content-Type".to_string())
        );
        assert_eq!(result.params[1], ParamValue::Bool(true));
    }

    #[test]
    fn test_parse_validator_with_spaces() {
        let result = parse_validator("  http_get : string(/path) , int(200)  ").unwrap();
        assert_eq!(result.name, "http_get");
        assert_eq!(result.params.len(), 2);
    }

    #[test]
    fn test_invalid_param_format() {
        let result = parse_typed_param("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_bool_value() {
        let result = parse_typed_param("bool(maybe)");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_int_value() {
        let result = parse_typed_param("int(abc)");
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_validator_name() {
        let result = parse_validator(":int(123)");
        assert!(result.is_err());
    }

    #[test]
    fn test_param_accessors() {
        let parsed = parse_validator("test:int(42),string(hello),bool(true)").unwrap();

        assert_eq!(parsed.param_as_int(0).unwrap(), 42);
        assert_eq!(parsed.param_as_string(1).unwrap(), "hello");
        assert!(parsed.param_as_bool(2).unwrap());

        assert!(parsed.param_as_int(1).is_err()); // string is not int
        assert!(parsed.param_as_string(0).is_err()); // int is not string
        assert!(parsed.param_as_int(10).is_err()); // out of bounds
    }
}
