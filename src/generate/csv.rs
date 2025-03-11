use crate::error::NixDocError;
use crate::OptionDoc;

/// Generates a CSV formatted string documenting NixOS module options.
///
/// # Arguments
/// - `options`: A slice of option documentation entries containing module option details.
///
/// # Returns
/// A `Result` containing the formatted CSV string with headers and option records or a CSV error.
pub fn generate_csv(options: &[OptionDoc]) -> Result<String, NixDocError> {
    let mut wtr = csv::WriterBuilder::new()
        .has_headers(true)
        .from_writer(vec![]);

    // Write header - handle CSV errors directly
    if let Err(err) = wtr.write_record([
        "Option",
        "Type",
        "Default",
        "Example",
        "Description",
        "FilePath",
        "LineNumber",
    ]) {
        return Err(NixDocError::Csv(err.to_string()));
    }

    for option in options {
        let default = option.default_value.as_deref().unwrap_or("-");
        // For CSV, we need to flatten the description to a single line
        let description = option
            .description
            .as_deref()
            .map(|d| d.replace('\n', " ").replace('\r', ""))
            .unwrap_or_else(|| "-".to_string());

        // Handle CSV errors directly
        if let Err(err) = wtr.write_record([
            &option.name,
            &option.nix_type.to_string(),
            default,
            option.example.as_deref().unwrap_or("-"),
            &description,
            &option.file_path,
            &option.line_number.to_string(),
        ]) {
            return Err(NixDocError::Csv(err.to_string()));
        }
    }

    // Handle potential errors from into_inner
    let data = match wtr.into_inner() {
        Ok(data) => data,
        Err(e) => return Err(NixDocError::Csv(e.to_string())),
    };

    // Handle UTF-8 conversion errors
    String::from_utf8(data).map_err(|e| e.into())
}
