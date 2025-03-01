use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NixType {
    Bool,
    Int,
    Float,
    Str,
    Path,
    Enum(Vec<String>),
    Attrs,
    List,
    Set,
    Option(Box<NixType>),
    Either(Vec<Box<NixType>>),
    Unknown(String),
}

// Definition for types that are presentt in nix
// Handle missing types as "Unknown"
impl NixType {
    pub fn from_nix_str(type_str: &str) -> Self {
        // Basic types
        match type_str {
            "types.bool" => NixType::Bool,
            "types.int" | "types.integer" => NixType::Int,
            "types.float" => NixType::Float,
            "types.str" | "types.string" => NixType::Str,
            "types.path" => NixType::Path,
            "types.attrs" => NixType::Attrs,
            "types.listOf" => NixType::List,
            _ => {
                // Try to parse more complex types
                if type_str.contains("types.enum") {
                    // Very basic parse for enum values
                    NixType::Enum(vec!["...".to_string()])
                } else if type_str.contains("types.option") {
                    // Extract inner type if possible
                    NixType::Option(Box::new(NixType::Unknown("".to_string())))
                } else if type_str.contains("types.either") {
                    NixType::Either(vec![])
                } else {
                    NixType::Unknown(type_str.to_string())
                }
            }
        }
    }
}

impl fmt::Display for NixType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NixType::Bool => write!(f, "boolean"),
            NixType::Int => write!(f, "integer"),
            NixType::Float => write!(f, "float"),
            NixType::Str => write!(f, "string"),
            NixType::Path => write!(f, "path"),
            NixType::Enum(values) => {
                if values.is_empty() {
                    write!(f, "enum")
                } else {
                    write!(f, "enum: [{}]", values.join(", "))
                }
            }
            NixType::Attrs => write!(f, "attribute set"),
            NixType::List => write!(f, "list"),
            NixType::Set => write!(f, "set"),
            NixType::Option(inner) => write!(f, "optional {}", inner),
            NixType::Either(types) => {
                if types.is_empty() {
                    write!(f, "either")
                } else {
                    let type_strings: Vec<String> = types.iter().map(|t| t.to_string()).collect();
                    write!(f, "either: [{}]", type_strings.join(", "))
                }
            }
            NixType::Unknown(s) => write!(f, "{}", s),
        }
    }
}
