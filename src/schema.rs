use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SchemaRegistry {
    #[serde(flatten)]
    pub functions: HashMap<String, FunctionSchema>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct FunctionSchema {
    #[serde(default)]
    pub positional: Option<PositionalSpec>,
    #[serde(default)]
    pub no_break_first_argument: bool,
    #[serde(default)]
    pub simple_keywords: Vec<String>,
    #[serde(default)]
    pub options: Vec<String>,
    #[serde(default)]
    pub one_value_keywords: Vec<String>,
    #[serde(default, alias = "keywords", alias = "list_keywords")] // backward compat
    pub multi_value_keywords: Vec<String>,
    #[serde(default, alias = "list_types")]
    pub list_keyword_types: HashMap<String, ListType>,
    #[serde(default)]
    pub default_list_type: ListType,
    #[serde(default)]
    pub compound_list_keywords: Vec<CompoundListKeyword>,
    #[serde(default)]
    pub modes: HashMap<String, FunctionSchema>,
    #[serde(default)]
    pub path_keywords: Vec<String>,
    #[serde(default)]
    pub subparsers: HashMap<String, FunctionSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PositionalSpec {
    #[serde(default)]
    pub min: usize,
    #[serde(default)]
    pub max: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CompoundListKeyword {
    pub name: String,
    #[serde(default)]
    pub headers: Vec<Vec<String>>,
    #[serde(default)]
    pub list_type: ListType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ListType {
    #[default]
    Packed,
    Path,
    CommandArgv,
    NPerLine {
        n: u8,
    },
    Condition,
}

impl ListType {
    /// Convenience constructor for n=1 (replaces former OnePerLine variant).
    pub const fn one_per_line() -> Self {
        ListType::NPerLine { n: 1 }
    }
}

// Custom deserialization for ListType so that the bare string
// `"n_per_line"` is accepted as `NPerLine { n: 1 }`, and a structured
// form `{ type = "n_per_line", n = 2 }` is accepted to set a different n.
impl<'de> Deserialize<'de> for ListType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Repr {
            Bare(String),
            Structured {
                #[serde(rename = "type")]
                ty: String,
                #[serde(default)]
                n: Option<u8>,
            },
        }
        let repr = Repr::deserialize(deserializer)?;
        let (ty, n) = match repr {
            Repr::Bare(s) => (s, None),
            Repr::Structured { ty, n } => (ty, n),
        };
        match ty.as_str() {
            "packed" => Ok(ListType::Packed),
            "path" => Ok(ListType::Path),
            "command_argv" => Ok(ListType::CommandArgv),
            "n_per_line" | "one_per_line" => Ok(ListType::NPerLine { n: n.unwrap_or(1) }),
            "condition" => Ok(ListType::Condition),
            other => Err(serde::de::Error::custom(format!(
                "unknown list_type: {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum ListKeywordInput {
    Name(String),
    Typed {
        name: String,
        list_type: ListType,
        #[serde(default)]
        n: Option<u8>,
    },
}

#[derive(Debug, Deserialize, Default)]
struct FunctionSchemaInput {
    #[serde(default)]
    positional: Option<PositionalSpec>,
    #[serde(default)]
    no_break_first_argument: bool,
    #[serde(default)]
    simple_keywords: Vec<String>,
    #[serde(default)]
    options: Vec<String>,
    #[serde(default)]
    one_value_keywords: Vec<String>,
    #[serde(default, alias = "keywords", alias = "list_keywords")]
    multi_value_keywords: Vec<ListKeywordInput>,
    #[serde(default, alias = "list_types")]
    list_keyword_types: HashMap<String, ListType>,
    #[serde(default)]
    default_list_type: ListType,
    #[serde(default)]
    compound_list_keywords: Vec<CompoundListKeyword>,
    #[serde(default)]
    modes: HashMap<String, FunctionSchema>,
    #[serde(default)]
    path_keywords: Vec<String>,
    #[serde(default)]
    subparsers: HashMap<String, FunctionSchema>,
}

impl<'de> Deserialize<'de> for FunctionSchema {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let input = FunctionSchemaInput::deserialize(deserializer)?;
        let mut multi_value_keywords = Vec::new();
        let mut list_keyword_types = input.list_keyword_types;

        for keyword in input.multi_value_keywords {
            match keyword {
                ListKeywordInput::Name(name) => multi_value_keywords.push(name),
                ListKeywordInput::Typed { name, list_type, n } => {
                    // If `n` is provided alongside an n_per_line list_type,
                    // override its `n` value.
                    let list_type = match (list_type, n) {
                        (ListType::NPerLine { .. }, Some(n)) => ListType::NPerLine { n },
                        (lt, _) => lt,
                    };
                    list_keyword_types.insert(name.clone(), list_type);
                    multi_value_keywords.push(name);
                }
            }
        }

        Ok(Self {
            positional: input.positional,
            no_break_first_argument: input.no_break_first_argument,
            simple_keywords: input.simple_keywords,
            options: input.options,
            one_value_keywords: input.one_value_keywords,
            multi_value_keywords,
            list_keyword_types,
            default_list_type: input.default_list_type,
            compound_list_keywords: input.compound_list_keywords,
            modes: input.modes,
            path_keywords: input.path_keywords,
            subparsers: input.subparsers,
        })
    }
}

impl FunctionSchema {
    // Keyword arguments are case-sensitive per cmake_parse_arguments semantics:
    // a lowercase token like `command` inside an argument list is a value, not
    // a new occurrence of the COMMAND keyword. (Command names themselves are
    // case-insensitive — that lookup lives in SchemaRegistry::get.)
    pub fn is_option(&self, keyword: &str) -> bool {
        self.options.iter().any(|k| k == keyword)
    }

    pub fn is_simple_keyword(&self, keyword: &str) -> bool {
        self.simple_keywords.iter().any(|k| k == keyword)
    }

    pub fn is_one_value_keyword(&self, keyword: &str) -> bool {
        self.one_value_keywords.iter().any(|k| k == keyword)
    }

    pub fn is_multi_value_keyword(&self, keyword: &str) -> bool {
        self.multi_value_keywords.iter().any(|k| k == keyword)
    }

    pub fn is_path_keyword(&self, keyword: &str) -> bool {
        self.path_keywords.iter().any(|k| k == keyword) || self.list_type(keyword) == ListType::Path
    }

    pub fn list_type(&self, keyword: &str) -> ListType {
        self.list_keyword_types
            .iter()
            .find(|(k, _)| k.as_str() == keyword)
            .map(|(_, v)| *v)
            .unwrap_or_else(|| {
                if self.path_keywords.iter().any(|k| k == keyword) {
                    ListType::Path
                } else {
                    ListType::Packed
                }
            })
    }

    pub fn compound_list_keyword(&self, keyword: &str) -> Option<&CompoundListKeyword> {
        self.compound_list_keywords
            .iter()
            .find(|compound| compound.name == keyword)
    }

    pub fn mode(&self, keyword: &str) -> Option<&FunctionSchema> {
        self.modes
            .iter()
            .find(|(k, _)| k.as_str() == keyword)
            .map(|(_, v)| v)
    }

    pub fn subparser(&self, keyword: &str) -> Option<&FunctionSchema> {
        self.subparsers
            .iter()
            .find(|(k, _)| k.as_str() == keyword)
            .map(|(_, v)| v)
    }

    pub fn is_any_keyword(&self, keyword: &str) -> bool {
        self.is_option(keyword)
            || self.is_one_value_keyword(keyword)
            || self.is_multi_value_keyword(keyword)
            || self.compound_list_keyword(keyword).is_some()
            || self.mode(keyword).is_some()
            || self.subparser(keyword).is_some()
    }
}

impl SchemaRegistry {
    pub fn get(&self, function_name: &str) -> Option<&FunctionSchema> {
        // CMake commands are case-insensitive, but typically lowercase in schemas.
        // We use a stack-allocated buffer to avoid heap allocations for common ASCII names.
        if function_name.len() <= 64 && function_name.is_ascii() {
            let mut buf = [0u8; 64];
            let bytes = function_name.as_bytes();
            for i in 0..bytes.len() {
                buf[i] = bytes[i].to_ascii_lowercase();
            }
            let lowercase_name = unsafe { std::str::from_utf8_unchecked(&buf[..bytes.len()]) };
            self.functions.get(lowercase_name)
        } else {
            self.functions.get(&function_name.to_lowercase())
        }
    }

    pub fn with_builtins() -> Self {
        crate::builtin_schemas::builtin_schemas()
    }

    pub fn merge(mut self, other: SchemaRegistry) -> SchemaRegistry {
        self.functions.extend(other.functions);
        self
    }
}
