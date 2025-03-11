//! The error module defines the error types used throughout the application.
//!
//! It provides a custom error enum with variants for different error conditions
//! and implementations for converting from standard error types.

use std::string::FromUtf8Error;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum NixDocError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Git operation failed: {0}")]
    GitOperation(String),

    #[error("Path error: {0}")]
    Path(#[from] std::path::StripPrefixError),

    #[error("Parsing error in file {0}: {1}")]
    Parse(String, String),

    #[error("No repository work directory found")]
    NoWorkDir,

    #[error("Not a valid local path or git repository: {0}")]
    InvalidPath(String),

    #[error("Failed to clone repository: {0}, {1}")]
    GitClone(String, String),

    #[error("Walkdir error: {0}")]
    WalkDir(#[from] walkdir::Error),

    #[error("Standard error: {0}")]
    StdError(String),

    #[error("CSV error: {0}")]
    Csv(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("UTF-8 conversion error: {0}")]
    Utf8(#[from] FromUtf8Error),
}

// Implement helper methods for creating errors
impl NixDocError {
    /// Helper for creating errors with a formatted message from any displayable error source.
    ///
    /// # Arguments
    /// - `err`: Any error that implements Display.
    /// - `error_type`: Function that constructs the specific error variant.
    ///
    /// # Returns
    /// A new NixDocError with the source error message.
    pub fn with_message<E: std::fmt::Display>(err: E, error_type: fn(String) -> Self) -> Self {
        error_type(err.to_string())
    }

    /// Creates a CSV-specific error with the given error message.
    ///
    /// # Arguments
    /// - `err`: Any error that implements Display.
    ///
    /// # Returns
    /// A NixDocError::Csv variant with the formatted error message.
    pub fn csv_error<E: std::fmt::Display>(err: E) -> Self {
        Self::with_message(err, NixDocError::Csv)
    }

    /// Creates a Git operation error with the given error message.
    ///
    /// # Arguments
    /// - `err`: Any error that implements Display.
    ///
    /// # Returns
    /// A NixDocError::GitOperation variant with the formatted error message.
    pub fn git_error<E: std::fmt::Display>(err: E) -> Self {
        Self::with_message(err, NixDocError::GitOperation)
    }

    /// Creates a serialization error with the given error message.
    ///
    /// # Arguments
    /// - `err`: Any error that implements Display.
    ///
    /// # Returns
    /// A NixDocError::Serialization variant with the formatted error message.
    pub fn serialization_error<E: std::fmt::Display>(err: E) -> Self {
        Self::with_message(err, NixDocError::Serialization)
    }
}

// Box<dyn Error> conversion
impl From<Box<dyn std::error::Error + Send + Sync>> for NixDocError {
    fn from(err: Box<dyn std::error::Error + Send + Sync>) -> Self {
        NixDocError::StdError(err.to_string())
    }
}

// CSV-specific error conversions
impl From<csv::Error> for NixDocError {
    fn from(err: csv::Error) -> Self {
        NixDocError::csv_error(err)
    }
}

impl<W> From<csv::IntoInnerError<W>> for NixDocError {
    fn from(err: csv::IntoInnerError<W>) -> Self {
        NixDocError::csv_error(err)
    }
}
