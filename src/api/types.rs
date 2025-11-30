use serde::Deserialize;

/// Generic API response wrapper
#[derive(Debug, Deserialize)]
pub struct ApiResponse<T> {
    pub data: T,
}

/// User information from the API
#[derive(Debug, Deserialize)]
pub struct ApiUser {
    pub id: i32,
    pub name: String,
    pub email: String,
}

impl ApiUser {
    pub fn id(&self) -> i32 {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_user_accessors() {
        let user = ApiUser {
            id: 42,
            name: "Test User".to_string(),
            email: "test@example.com".to_string(),
        };

        assert_eq!(user.id(), 42);
        assert_eq!(user.name(), "Test User");
    }

    #[test]
    fn test_api_response_deserialize() {
        let json = r#"{"data": {"id": 1, "name": "John", "email": "john@example.com"}}"#;
        let response: ApiResponse<ApiUser> = serde_json::from_str(json).unwrap();

        assert_eq!(response.data.id, 1);
        assert_eq!(response.data.name, "John");
    }
}
