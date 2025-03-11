use crate::error::NixDocError;
use crate::OptionDoc;

/// Generates a pretty-printed JSON string documenting NixOS module options.
///
/// # Arguments
/// - `options`: A slice of option documentation entries to be serialized to JSON.
///
/// # Returns
/// A `Result` containing the formatted JSON string or a serialization error.
pub fn generate_json(options: &[OptionDoc]) -> Result<String, NixDocError> {
    serde_json::to_string_pretty(options).map_err(|e| NixDocError::Serialization(e.to_string()))
}
