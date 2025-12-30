//! Logging configuration types.

use serde::{Deserialize, Serialize};
use std::str::FromStr;

use crate::error::ShikiError;

/// Logging configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LoggingConfig {
    /// Log level.
    pub level: LogLevel,

    /// Log format.
    pub format: LogFormat,

    /// Log output destination.
    pub output: LogOutput,

    /// Log file path (when output = file).
    pub file_path: Option<String>,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            format: LogFormat::Json,
            output: LogOutput::Stdout,
            file_path: None,
        }
    }
}

/// Log level.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    /// Trace level.
    Trace,
    /// Debug level.
    Debug,
    /// Info level.
    #[default]
    Info,
    /// Warn level.
    Warn,
    /// Error level.
    Error,
}

impl FromStr for LogLevel {
    type Err = ShikiError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "trace" => Ok(LogLevel::Trace),
            "debug" => Ok(LogLevel::Debug),
            "info" => Ok(LogLevel::Info),
            "warn" | "warning" => Ok(LogLevel::Warn),
            "error" => Ok(LogLevel::Error),
            _ => Err(ShikiError::config(format!("Unknown log level: {}", s))),
        }
    }
}

impl From<LogLevel> for tracing::Level {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Trace => tracing::Level::TRACE,
            LogLevel::Debug => tracing::Level::DEBUG,
            LogLevel::Info => tracing::Level::INFO,
            LogLevel::Warn => tracing::Level::WARN,
            LogLevel::Error => tracing::Level::ERROR,
        }
    }
}

/// Log format.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    /// JSON format.
    #[default]
    Json,
    /// Text format.
    Text,
}

impl FromStr for LogFormat {
    type Err = ShikiError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "json" => Ok(LogFormat::Json),
            "text" => Ok(LogFormat::Text),
            _ => Err(ShikiError::config(format!("Unknown log format: {}", s))),
        }
    }
}

/// Log output destination.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogOutput {
    /// Standard output.
    #[default]
    Stdout,
    /// Standard error.
    Stderr,
    /// File output.
    File,
}

impl FromStr for LogOutput {
    type Err = ShikiError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "stdout" => Ok(LogOutput::Stdout),
            "stderr" => Ok(LogOutput::Stderr),
            "file" => Ok(LogOutput::File),
            _ => Err(ShikiError::config(format!("Unknown log output: {}", s))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logging_config_default() {
        let config = LoggingConfig::default();
        assert_eq!(config.level, LogLevel::Info);
        assert_eq!(config.format, LogFormat::Json);
        assert_eq!(config.output, LogOutput::Stdout);
    }

    #[test]
    fn test_log_level_parse() {
        assert_eq!("trace".parse::<LogLevel>().unwrap(), LogLevel::Trace);
        assert_eq!("DEBUG".parse::<LogLevel>().unwrap(), LogLevel::Debug);
        assert_eq!("Info".parse::<LogLevel>().unwrap(), LogLevel::Info);
        assert_eq!("warn".parse::<LogLevel>().unwrap(), LogLevel::Warn);
        assert_eq!("warning".parse::<LogLevel>().unwrap(), LogLevel::Warn);
        assert!("invalid".parse::<LogLevel>().is_err());
    }

    #[test]
    fn test_log_format_parse() {
        assert_eq!("json".parse::<LogFormat>().unwrap(), LogFormat::Json);
        assert_eq!("TEXT".parse::<LogFormat>().unwrap(), LogFormat::Text);
        assert!("invalid".parse::<LogFormat>().is_err());
    }

    #[test]
    fn test_log_output_parse() {
        assert_eq!("stdout".parse::<LogOutput>().unwrap(), LogOutput::Stdout);
        assert_eq!("STDERR".parse::<LogOutput>().unwrap(), LogOutput::Stderr);
        assert_eq!("file".parse::<LogOutput>().unwrap(), LogOutput::File);
        assert!("invalid".parse::<LogOutput>().is_err());
    }

    #[test]
    fn test_log_level_to_tracing() {
        assert_eq!(tracing::Level::from(LogLevel::Trace), tracing::Level::TRACE);
        assert_eq!(tracing::Level::from(LogLevel::Debug), tracing::Level::DEBUG);
        assert_eq!(tracing::Level::from(LogLevel::Info), tracing::Level::INFO);
        assert_eq!(tracing::Level::from(LogLevel::Warn), tracing::Level::WARN);
        assert_eq!(tracing::Level::from(LogLevel::Error), tracing::Level::ERROR);
    }
}
