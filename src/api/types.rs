use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ApiResponse<T> {
    pub data: T,
}

#[derive(Debug, Deserialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub links: PaginationLinks,
    pub meta: PaginationMeta,
}

#[derive(Debug, Deserialize)]
pub struct PaginationLinks {
    pub first: Option<String>,
    pub last: Option<String>,
    pub prev: Option<String>,
    pub next: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PaginationMeta {
    pub current_page: i32,
    pub from: Option<i32>,
    pub last_page: i32,
    pub path: String,
    pub per_page: i32,
    pub to: Option<i32>,
    pub total: i32,
}

#[derive(Debug, Deserialize)]
pub struct ApiUser {
    pub id: i32,
    pub name: String,
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct ProjectStats {
    pub attempted_count: i32,
    pub succeed_count: i32,
    pub failed_count: i32,
}

#[derive(Debug, Deserialize)]
pub struct Project {
    pub id: i32,
    pub slug: String,
    pub name: String,
    pub short_description: String,
    pub is_published: bool,
    pub is_featured: bool,
    pub show_tasks: bool,
    pub stats: ProjectStats,
    pub published_at: Option<String>,
    pub tasks_count: i32,
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
