use crate::error::{AppError, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub discord: DiscordConfig,
    pub database: DatabaseConfig,
    pub server: ServerConfig,
    pub validation: ValidationConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordConfig {
    pub token: String,
    pub channel_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub static_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    /// Maximum username length (-1 for no limit)
    pub max_username_length: i32,
    /// Whitelist of allowed usernames (if empty, uses blacklist)
    pub username_whitelist: Vec<String>,
    /// Blacklist of prohibited usernames (if empty and whitelist empty, allows any)
    pub username_blacklist: Vec<String>,
    /// Maximum duration in milliseconds
    pub max_duration_ms: i32,
    /// Minimum duration in milliseconds
    pub min_duration_ms: i32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            discord: DiscordConfig::default(),
            database: DatabaseConfig::default(),
            server: ServerConfig::default(),
            validation: ValidationConfig::default(),
        }
    }
}

impl Default for DiscordConfig {
    fn default() -> Self {
        Self {
            token: "YOUR_TOKEN_HERE".to_string(),
            channel_id: 1234567890123456789,
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "sqlite:splits.db".to_string(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 7758,
            static_dir: "static".to_string(),
        }
    }
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            max_username_length: -1, // No limit
            username_whitelist: vec![],
            username_blacklist: vec![],
            max_duration_ms: 24 * 60 * 60 * 1000, // 24 hours
            min_duration_ms: 100, // 100ms
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        // Start with default configuration
        let mut config = Config::default();

        // Try to load TOML configuration file
        if let Ok(config_str) = fs::read_to_string("config.toml") {
            match toml::from_str::<Config>(&config_str) {
                Ok(toml_config) => {
                    config = toml_config;
                    println!("Loaded configuration from config.toml");
                }
                Err(e) => {
                    eprintln!("Warning: Failed to parse config.toml: {}", e);
                }
            }
        }

        // Validate the configuration
        config.validate()?;

        Ok(config)
    }

    /// Validate the configuration
    fn validate(&self) -> Result<()> {
        if self.discord.token == "YOUR_TOKEN_HERE" {
            return Err(AppError::EnvVar(env::VarError::NotPresent));
        }
        
        if self.discord.channel_id == 1234567890123456789 {
            return Err(AppError::EnvVar(env::VarError::NotPresent));
        }

        if !Path::new(&self.server.static_dir).exists() {
            eprintln!("Warning: Static directory '{}' does not exist", self.server.static_dir);
        }

        Ok(())
    }

    /// Generate a sample configuration file
    pub fn generate_sample_config() -> Result<()> {
        let config = Config::default();
        let toml_string = toml::to_string_pretty(&config)
            .map_err(|e| AppError::Network(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
        
        fs::write("config.toml.example", toml_string)?;
        println!("Generated config.toml.example");
        Ok(())
    }

    /// Get the server address as a string
    pub fn server_address(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_sample_config() {
        // Clean up any existing example file
        let _ = fs::remove_file("config.toml.example");
        
        // Generate the sample config
        Config::generate_sample_config().expect("Failed to generate sample config");
        
        // Verify the file was created
        assert!(Path::new("config.toml.example").exists());
        
        // Verify the file contains valid TOML that can be parsed back
        let content = fs::read_to_string("config.toml.example").expect("Failed to read generated file");
        let parsed_config: Config = toml::from_str(&content).expect("Generated config is not valid TOML");
        
        // Verify it matches the default configuration
        let default_config = Config::default();
        assert_eq!(parsed_config.discord.token, default_config.discord.token);
        assert_eq!(parsed_config.discord.channel_id, default_config.discord.channel_id);
        assert_eq!(parsed_config.database.url, default_config.database.url);
        assert_eq!(parsed_config.server.host, default_config.server.host);
        assert_eq!(parsed_config.server.port, default_config.server.port);
        assert_eq!(parsed_config.server.static_dir, default_config.server.static_dir);
        assert_eq!(parsed_config.validation.max_username_length, default_config.validation.max_username_length);
        assert_eq!(parsed_config.validation.username_whitelist, default_config.validation.username_whitelist);
        assert_eq!(parsed_config.validation.username_blacklist, default_config.validation.username_blacklist);
        assert_eq!(parsed_config.validation.max_duration_ms, default_config.validation.max_duration_ms);
        assert_eq!(parsed_config.validation.min_duration_ms, default_config.validation.min_duration_ms);
    }
}