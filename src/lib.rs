//! Splits application library
//! 
//! This application tracks split times and integrates with Discord.

pub mod error;
pub mod models;
pub mod config;
pub mod database;
pub mod discord;
pub mod handlers;
pub mod validation;

pub use error::{AppError, Result};
pub use models::{Split, SplitData, AppContext, AppState};
pub use config::Config;
