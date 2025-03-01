use std::collections::HashMap;

/// Replaces dynamic variables in the given text using the provided replacements.
/// For example, given text `"options.${namespace}.attribute"` and a replacement
/// mapping `{"namespace": "flake"}`, this returns `"options.flake.attribute"`.
pub fn apply_replacements(text: &str, replacements: &HashMap<String, String>) -> String {
    replacements
        .iter()
        .fold(text.to_string(), |acc, (key, value)| {
            acc.replace(&format!("${{{}}}", key), value)
        })
}
