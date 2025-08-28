use crate::error::AppError;
use crate::config::ValidationConfig;

pub type ValidationResult<T> = Result<T, ValidationError>;

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Invalid username: {0}")]
    InvalidUsername(String),
    #[error("Invalid duration: {0}")]
    InvalidDuration(String),
    #[error("Field validation failed: {field} - {message}")]
    FieldValidation { field: String, message: String },
}

impl From<ValidationError> for AppError {
    fn from(err: ValidationError) -> Self {
        AppError::Network(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            err.to_string(),
        ))
    }
}

/// Username validation rules
pub struct UsernameValidator;

impl UsernameValidator {
    /// Validate username with configuration
    pub fn validate(username: &str, config: &ValidationConfig) -> ValidationResult<()> {
        // Check if empty
        if username.trim().is_empty() {
            return Err(ValidationError::InvalidUsername(
                "Username cannot be empty".to_string(),
            ));
        }

        // Check length if max_username_length is not -1
        if config.max_username_length > 0 && username.len() > config.max_username_length as usize {
            return Err(ValidationError::InvalidUsername(
                format!("Username must be {} characters or less", config.max_username_length),
            ));
        }

        // Check whitelist first (if not empty)
        if !config.username_whitelist.is_empty() {
            let lower_username = username.to_lowercase();
            let is_whitelisted = config.username_whitelist.iter()
                .any(|allowed| lower_username == allowed.to_lowercase());
            
            if !is_whitelisted {
                return Err(ValidationError::InvalidUsername(
                    "Username not in whitelist".to_string(),
                ));
            }
        } else if !config.username_blacklist.is_empty() {
            // Use blacklist if whitelist is empty
            let lower_username = username.to_lowercase();
            let is_blacklisted = config.username_blacklist.iter()
                .any(|prohibited| lower_username.contains(&prohibited.to_lowercase()));
            
            if is_blacklisted {
                return Err(ValidationError::InvalidUsername(
                    "Username on blacklist".to_string(),
                ));
            }
        }

        Ok(())
    }
}

/// Duration validation rules  
pub struct DurationValidator;

impl DurationValidator {
    /// Validate duration with configuration
    pub fn validate(duration_ms: i32, config: &ValidationConfig) -> ValidationResult<()> {
        // Duration must be positive
        if duration_ms <= 0 {
            return Err(ValidationError::InvalidDuration(
                "Duration must be positive".to_string(),
            ));
        }

        // Duration must not exceed maximum
        if duration_ms > config.max_duration_ms {
            return Err(ValidationError::InvalidDuration(
                format!("Duration cannot exceed {}ms", config.max_duration_ms),
            ));
        }

        // Duration must meet minimum requirement
        if duration_ms < config.min_duration_ms {
            return Err(ValidationError::InvalidDuration(
                format!("Duration must be at least {}ms", config.min_duration_ms),
            ));
        }

        Ok(())
    }

    /// Format duration for display
    pub fn format_duration(duration_ms: i32) -> String {
        let total_seconds = duration_ms / 1000;
        let minutes = total_seconds / 60;
        let seconds = total_seconds % 60;
        let milliseconds = duration_ms % 1000;

        if minutes > 0 {
            format!("{}m{:02}.{:03}s", minutes, seconds, milliseconds)
        } else if seconds > 0 {
            format!("{}.{:03}s", seconds, milliseconds)
        } else {
            format!("{}ms", milliseconds)
        }
    }
}

/// General field validator
pub struct FieldValidator;

impl FieldValidator {
    pub fn validate_boolean(_value: bool, field_name: &str) -> ValidationResult<()> {
        // Booleans are always valid, but we can add specific business logic here
        match field_name {
            "is_down" | "is_elevator" | "is_encumbered" => Ok(()),
            _ => Err(ValidationError::FieldValidation {
                field: field_name.to_string(),
                message: "Unknown boolean field".to_string(),
            }),
        }
    }

    /// Validate string fields
    pub fn validate_string(value: &str, field_name: &str, max_length: Option<usize>) -> ValidationResult<()> {
        if value.trim().is_empty() {
            return Err(ValidationError::FieldValidation {
                field: field_name.to_string(),
                message: "Field cannot be empty".to_string(),
            });
        }

        if let Some(max_len) = max_length {
            if value.len() > max_len {
                return Err(ValidationError::FieldValidation {
                    field: field_name.to_string(),
                    message: format!("Field exceeds maximum length of {}", max_len),
                });
            }
        }

        Ok(())
    }
}
