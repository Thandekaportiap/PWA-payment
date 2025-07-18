use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use validator::{Validate, ValidationError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub is_active: bool,
    pub email_verified: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateUserRequest {
    #[validate(length(min = 2, max = 100, message = "Name must be between 2 and 100 characters"))]
    pub name: String,
    
    #[validate(email(message = "Invalid email format"))]
    pub email: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateUserRequest {
    #[validate(length(min = 2, max = 100, message = "Name must be between 2 and 100 characters"))]
    pub name: Option<String>,
}

impl User {
    pub fn new(name: String, email: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            email: email.to_lowercase(),
            is_active: true,
            email_verified: false,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn update(&mut self, request: UpdateUserRequest) {
        if let Some(name) = request.name {
            self.name = name;
        }
        self.updated_at = Utc::now();
    }
}

fn validate_email_domain(email: &str) -> Result<(), ValidationError> {
    // Add custom email domain validation if needed
    let forbidden_domains = ["tempmail.com", "10minutemail.com"];
    
    if let Some(domain) = email.split('@').nth(1) {
        if forbidden_domains.contains(&domain) {
            return Err(ValidationError::new("forbidden_email_domain"));
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_creation() {
        let user = User::new("John Doe".to_string(), "john@example.com".to_string());
        assert_eq!(user.name, "John Doe");
        assert_eq!(user.email, "john@example.com");
        assert!(user.is_active);
        assert!(!user.email_verified);
    }

    #[test]
    fn test_email_normalization() {
        let user = User::new("Jane Doe".to_string(), "JANE@EXAMPLE.COM".to_string());
        assert_eq!(user.email, "jane@example.com");
    }
}