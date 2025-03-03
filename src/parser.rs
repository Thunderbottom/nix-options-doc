use crate::types::NixType;
use crate::utils::apply_replacements;
use crate::OptionDoc;
use rnix::{SyntaxKind, SyntaxNode};
use std::collections::HashMap;

/// Recursively traverses the syntax tree of a Nix file to extract option definitions.
///
/// # Arguments
/// - `node`: The current syntax node being processed.
/// - `file_path`: The relative file path of the Nix file.
/// - `options`: A mutable vector to collect found option documentation entries.
/// - `prefix`: The current option name prefix.
/// - `replacements`: A map of variable replacements for dynamic segments in option names or descriptions.
/// - `source_text`: The full text of the source file (for line number calculation).
///
/// # Returns
/// Returns `Ok(())` if the traversal is successful; otherwise returns an error.
pub fn visit_node(
    node: &SyntaxNode,
    file_path: &str,
    options: &mut Vec<OptionDoc>,
    prefix: &str,
    replacements: &HashMap<String, String>,
    source_text: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if node.kind() == SyntaxKind::NODE_ATTRPATH_VALUE {
        let key = node
            .children()
            .find(|n| n.kind() == SyntaxKind::NODE_ATTRPATH)
            .as_ref()
            .map(|n| parse_attrpath(n, replacements));

        if let Some(value_node) = node.children().nth(1) {
            if let Some(key) = key {
                let new_prefix = if prefix.is_empty() {
                    key
                } else {
                    format!("{}.{}", prefix, key)
                };
                parse_attrset(
                    &value_node,
                    file_path,
                    options,
                    &new_prefix,
                    replacements,
                    source_text,
                )?;
            }
        }
    } else {
        // Visit all children for other node types
        for child in node.children() {
            visit_node(
                &child,
                file_path,
                options,
                prefix,
                replacements,
                source_text,
            )?;
        }
    }

    Ok(())
}

/// Parses an attribute path node and returns a dot-separated string representing the option name,
/// applying any variable replacements as needed.
///
/// # Arguments
/// - `node`: The syntax node representing the attribute path.
/// - `replacements`: A map of variable replacements to apply.
///
/// # Returns
/// A dot-separated string that represents the full option name.
fn parse_attrpath(node: &SyntaxNode, replacements: &HashMap<String, String>) -> String {
    node.children()
        .map(|child| apply_replacements(&child.text().to_string(), replacements))
        .collect::<Vec<_>>()
        .join(".")
}

/// Determines the 1-based line number where a syntax node starts in the source file.
///
/// # Arguments
/// - `node`: The syntax node for which to determine the line number.
/// - `source_text`: The complete text of the source file.
///
/// # Returns
/// The line number where the node starts.
fn get_line_number(node: &SyntaxNode, source_text: &str) -> usize {
    // Get the text range of this node
    let text_range = node.text_range();
    let start_offset: usize = text_range.start().into();

    // Count newlines up to this position
    let line_count = source_text[..start_offset]
        .chars()
        .filter(|&c| c == '\n')
        .count();

    // Line numbers are 1-based
    line_count + 1
}

/// Parses an attribute set node to extract NixOS module option definitions.
///
/// # Arguments
/// - `node`: The syntax node representing the attribute set.
/// - `file_path`: The file path of the Nix file.
/// - `options`: A mutable vector to accumulate option documentation entries.
/// - `current_prefix`: The current option name hierarchy.
/// - `replacements`: A map of variable replacements for dynamic values.
/// - `source_text`: The source text of the file (used for computing line numbers).
///
/// # Returns
/// Returns `Ok(())` if parsing is successful; otherwise returns an error.
fn parse_attrset(
    node: &SyntaxNode,
    file_path: &str,
    options: &mut Vec<OptionDoc>,
    current_prefix: &str,
    replacements: &HashMap<String, String>,
    source_text: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    match node.kind() {
        // Nested attributes
        SyntaxKind::NODE_ATTR_SET => {
            for child in node.children() {
                visit_node(
                    &child,
                    file_path,
                    options,
                    current_prefix,
                    replacements,
                    source_text,
                )?;
            }
        }
        // Child node, parse for mkOption or mkEnableOption
        SyntaxKind::NODE_APPLY => {
            let fn_name = node
                .children()
                .find(|n| n.kind() == SyntaxKind::NODE_SELECT)
                .and_then(|n| n.children().last())
                .map(|n| n.text().to_string());

            match fn_name.as_deref() {
                Some("mkEnableOption") => {
                    let description = node
                        .children()
                        .find(|n| n.kind() == SyntaxKind::NODE_STRING)
                        .map(|n| {
                            let desc_text = n
                                .text()
                                .to_string()
                                .trim_matches(['"', '\''])
                                .lines()
                                .map(str::trim)
                                .collect::<Vec<_>>()
                                .join("\n")
                                .trim()
                                .to_string();
                            // Apply replacements to description
                            apply_replacements(&desc_text, replacements)
                        });

                    options.retain(|opt| opt.name != current_prefix);
                    options.push(OptionDoc {
                        name: current_prefix.to_string(),
                        description,
                        nix_type: NixType::Bool,
                        default_value: Some(String::from("false")),
                        file_path: file_path.to_string(),
                        line_number: get_line_number(node, source_text),
                    });
                }
                Some("mkOption") => {
                    let mut nix_type = NixType::Unknown("any".to_string());
                    let mut description = None;
                    let mut default_value = None;

                    if let Some(attr_set) = node
                        .children()
                        .find(|n| n.kind() == SyntaxKind::NODE_ATTR_SET)
                    {
                        for attr in attr_set.children() {
                            if attr.kind() == SyntaxKind::NODE_ATTRPATH_VALUE {
                                let attr_key = attr
                                    .children()
                                    .find(|n| n.kind() == SyntaxKind::NODE_ATTRPATH)
                                    .and_then(|n| n.children().next())
                                    .map(|n| n.text().to_string());

                                let attr_value = attr.children().nth(1);

                                match (attr_key.as_deref(), attr_value) {
                                    (Some("type"), Some(v)) => {
                                        let type_str = v.text().to_string();
                                        nix_type = NixType::from_nix_str(&type_str);
                                    }
                                    (Some("description"), Some(v)) => {
                                        let desc_text = v
                                            .text()
                                            .to_string()
                                            .trim_matches(['"', '\''])
                                            .lines()
                                            .map(str::trim)
                                            .collect::<Vec<_>>()
                                            .join("\n")
                                            .trim()
                                            .to_string();

                                        description =
                                            Some(apply_replacements(&desc_text, replacements));
                                    }
                                    (Some("default"), Some(v)) => {
                                        default_value = Some(v.text().to_string());
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }

                    options.retain(|opt| opt.name != current_prefix);
                    options.push(OptionDoc {
                        name: current_prefix.to_string(),
                        description,
                        nix_type,
                        default_value,
                        file_path: file_path.to_string(),
                        line_number: get_line_number(node, source_text),
                    });
                }
                _ => {}
            }
        }
        _ => {}
    }

    Ok(())
}
