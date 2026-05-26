use crate::schema::SchemaRegistry;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IndentStyle {
    #[default]
    Space,
    Tab,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub indent_style: IndentStyle,
    #[serde(default = "default_indent_width")]
    pub indent_width: usize,
    #[serde(default = "default_line_width")]
    pub line_width: usize,
    #[serde(default = "default_source_vertical_list_threshold")]
    pub source_vertical_list_threshold: isize,
    #[serde(default)]
    pub function_schemas: SchemaRegistry,
}

fn default_indent_width() -> usize {
    4
}

fn default_line_width() -> usize {
    100
}

fn default_source_vertical_list_threshold() -> isize {
    3
}

impl Default for Config {
    fn default() -> Self {
        Config {
            indent_style: IndentStyle::Space,
            indent_width: 4,
            line_width: 100,
            source_vertical_list_threshold: 3,
            function_schemas: SchemaRegistry::with_builtins(),
        }
    }
}

impl Config {
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let mut config: Config = toml::from_str(&content)?;
        config.function_schemas = SchemaRegistry::with_builtins().merge(config.function_schemas);
        Ok(config)
    }

    pub fn load_from_pyproject(path: impl AsRef<Path>) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let parsed: toml::Value = toml::from_str(&content)?;

        let cmtk_config = parsed
            .get("tool")
            .and_then(|t| t.get("cmtk"))
            .cloned()
            .unwrap_or_else(|| toml::Value::Table(toml::map::Map::new()));

        let mut config: Config = cmtk_config.try_into()?;
        config.function_schemas = SchemaRegistry::with_builtins().merge(config.function_schemas);
        Ok(config)
    }

    pub fn discover() -> Self {
        if Path::new(".cmtkrc").exists()
            && let Ok(config) = Self::load_from_file(".cmtkrc")
        {
            return config;
        }
        if Path::new("pyproject.toml").exists()
            && let Ok(config) = Self::load_from_pyproject("pyproject.toml")
        {
            return config;
        }
        Self::default()
    }
}
