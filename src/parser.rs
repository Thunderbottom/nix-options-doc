//! The parser module contains functions for parsing Nix syntax trees
//! and extracting option documentation.
//!
//! It traverses the abstract syntax tree of Nix files to identify
//! module options and their metadata.

use crate::utils::{apply_replacements, clean_description, clean_literal_expr, custom_dedent};
use crate::OptionDoc;
use rnix::{SyntaxKind, SyntaxNode};
use std::collections::HashMap;

/// Recursively traverses the syntax tree of a Nix file to extract option definitions.
///
/// # Arguments
/// - `node`: The current syntax node being processed.
/// - `file_path`: The relative file path of the Nix file for documentation reference.
/// - `prefix`: The current option name prefix in the hierarchy.
/// - `replacements`: A map of variable replacements for dynamic segments.
/// - `source_text`: The full text of the source file for line number calculation.
///
/// # Returns
/// A vector of OptionDoc structs representing the found options or an error.
pub fn visit_node(
    node: &SyntaxNode,
    file_path: &str,
    prefix: &str,
    replacements: &HashMap<String, String>,
    source_text: &str,
) -> Result<Vec<OptionDoc>, Box<dyn std::error::Error + Send + Sync>> {
    let mut options = Vec::new();

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
                let mut nested_options = parse_attrset(
                    &value_node,
                    file_path,
                    &new_prefix,
                    replacements,
                    source_text,
                )?;
                options.append(&mut nested_options);
            }
        }
    } else {
        // Visit all children for other node types
        for child in node.children() {
            let mut child_options =
                visit_node(&child, file_path, prefix, replacements, source_text)?;
            options.append(&mut child_options);
        }
    }

    Ok(options)
}

/// Parses an attribute path node and returns a dot-separated string representing the option name.
///
/// # Arguments
/// - `node`: The syntax node representing the attribute path.
/// - `replacements`: A map of variable replacements to apply to any dynamic segments.
///
/// # Returns
/// A dot-separated string that represents the full option name with any variables replaced.
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
/// The 1-based line number where the node starts, calculated by counting newlines.
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

/// Clean and format a description string for documentation.
///
/// # Arguments
/// - `description`: The raw description string from the Nix file.
/// - `replacements`: A map of variable replacements to apply.
///
/// # Returns
/// A cleaned and formatted description string with proper indentation and variable substitution.
fn process_description(description: &str, replacements: &HashMap<String, String>) -> String {
    let replaced = apply_replacements(description, replacements);
    let dedented = custom_dedent(&replaced);
    clean_description(&dedented)
}

/// Parses an attribute set node to extract NixOS module option definitions.
///
/// # Arguments
/// - `node`: The syntax node representing the attribute set.
/// - `file_path`: The file path of the Nix file for reference.
/// - `current_prefix`: The current option name hierarchy as a dot-separated string.
/// - `replacements`: A map of variable replacements for dynamic values.
/// - `source_text`: The source text of the file for line number calculation.
///
/// # Returns
/// A vector of OptionDoc structs representing the options in the attribute set or an error.
fn parse_attrset(
    node: &SyntaxNode,
    file_path: &str,
    current_prefix: &str,
    replacements: &HashMap<String, String>,
    source_text: &str,
) -> Result<Vec<OptionDoc>, Box<dyn std::error::Error + Send + Sync>> {
    let mut options = Vec::new();

    match node.kind() {
        // Nested attributes
        SyntaxKind::NODE_ATTR_SET => {
            for child in node.children() {
                let mut child_options =
                    visit_node(&child, file_path, current_prefix, replacements, source_text)?;
                options.append(&mut child_options);
            }
        }
        // Child node, parse for mkOption or mkEnableOption
        SyntaxKind::NODE_APPLY => {
            // Try to get the function name from SELECT node (lib.mkOption style)
            let select_fn = node
                .children()
                .find(|n| n.kind() == SyntaxKind::NODE_SELECT)
                .and_then(|n| n.children().last())
                .map(|n| n.text().to_string());

            // If not found via SELECT, try IDENT node (direct mkOption style)
            let ident_fn = if select_fn.is_none() {
                node.children()
                    .find(|n| n.kind() == SyntaxKind::NODE_IDENT)
                    .map(|n| n.text().to_string())
            } else {
                None
            };

            // Use whichever function name we found
            let fn_name = select_fn.or(ident_fn);
            match fn_name.as_deref() {
                Some("mkEnableOption") => {
                    let description = node
                        .children()
                        .find(|n| n.kind() == SyntaxKind::NODE_STRING)
                        .map(|n| {
                            let desc_text =
                                n.text().to_string().trim_matches(['"', '\'']).to_string();
                            // Apply replacements and formatting to description
                            process_description(&desc_text, replacements)
                        });

                    options.push(OptionDoc {
                        name: current_prefix.to_string(),
                        description,
                        nix_type: "boolean".to_string(),
                        default_value: Some(String::from("false")),
                        example: Some(String::from("true")),
                        file_path: file_path.to_string(),
                        line_number: get_line_number(node, source_text),
                    });
                }
                Some("mkOption") => {
                    let mut nix_type = "any".to_string();
                    let mut description = None;
                    let mut default_value = None;
                    let mut example = None;

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
                                        nix_type = custom_dedent(&v.text().to_string());
                                    }
                                    (Some("description"), Some(v)) => {
                                        let desc_text = v
                                            .text()
                                            .to_string()
                                            .trim_matches(['"', '\''])
                                            .to_string();

                                        description =
                                            Some(process_description(&desc_text, replacements));
                                    }
                                    (Some("default"), Some(v)) => {
                                        // Clean and process default value
                                        let raw_value = v.text().to_string();
                                        let cleaned = clean_literal_expr(&raw_value);
                                        default_value = Some(custom_dedent(&cleaned));
                                    }
                                    (Some("example"), Some(v)) => {
                                        // Clean and process example
                                        let raw_value = v.text().to_string();
                                        let cleaned = clean_literal_expr(&raw_value);
                                        example = Some(custom_dedent(&cleaned));
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }

                    options.push(OptionDoc {
                        name: current_prefix.to_string(),
                        description,
                        nix_type,
                        default_value,
                        example,
                        file_path: file_path.to_string(),
                        line_number: get_line_number(node, source_text),
                    });
                }
                _ => {
                    log::debug!("Not a recognized option function: {:?}", fn_name);
                }
            }
        }
        // Handle `with <expr>;`
        SyntaxKind::NODE_WITH => {
            if let Some(body) = node.children().nth(1) {
                let mut nested_options =
                    visit_node(&body, file_path, current_prefix, replacements, source_text)?;
                options.append(&mut nested_options);
            }
        }
        _ => {
            log::debug!("Unhandled node kind: {:?}", node.kind());
        }
    }

    Ok(options)
}
