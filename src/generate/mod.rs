//! The generate module contains functions for converting option documentation
//! into various output formats.
//!
//! Supported formats include Markdown, HTML, JSON, and CSV.

pub mod csv;
pub mod html;
pub mod json;
pub mod markdown;

// Re-export all generation functions
pub use csv::generate_csv;
pub use html::generate_html;
pub use json::generate_json;
pub use markdown::generate_markdown;
