use csv;
use std::string::FromUtf8Error;
use thiserror::Error;
use walkdir;

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

// Implement From for Box<dyn std::error::Error>
impl From<Box<dyn std::error::Error>> for NixDocError {
    fn from(err: Box<dyn std::error::Error>) -> Self {
        NixDocError::StdError(err.to_string())
    }
}

// Implement From for csv::Error
impl From<csv::Error> for NixDocError {
    fn from(err: csv::Error) -> Self {
        NixDocError::Csv(err.to_string())
    }
}

// Implement From for csv::IntoInnerError
impl<W> From<csv::IntoInnerError<W>> for NixDocError {
    fn from(err: csv::IntoInnerError<W>) -> Self {
        NixDocError::Csv(err.to_string())
    }
}
