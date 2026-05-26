use cmtk::analyzer::{Analyzer, results_to_registry};
use cmtk::config::{Config, IndentStyle};
use cmtk::formatter::Formatter;
use cmtk::parser::Parser;
use cmtk::schema::SchemaRegistry;
use serde::Deserialize;
use similar::TextDiff;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
struct GoldenSpec {
    #[serde(default)]
    scan_sets: HashMap<String, ScanSet>,
    cases: Vec<GoldenCase>,
}

#[derive(Debug, Deserialize)]
struct ScanSet {
    files: Vec<PathBuf>,
}

#[derive(Debug, Deserialize)]
struct GoldenCase {
    name: String,
    inputs: Vec<PathBuf>,
    golden: PathBuf,
    #[serde(default)]
    scan_sets: Vec<String>,
    #[serde(default)]
    scan_only: Vec<PathBuf>,
    #[serde(default)]
    config: ConfigOverrides,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfigOverrides {
    indent_style: Option<IndentStyle>,
    indent_width: Option<usize>,
    line_width: Option<usize>,
    source_vertical_list_threshold: Option<isize>,
    function_schemas: Option<SchemaRegistry>,
}

impl ConfigOverrides {
    fn apply_to(&self, mut config: Config) -> Config {
        if let Some(indent_style) = self.indent_style.clone() {
            config.indent_style = indent_style;
        }
        if let Some(indent_width) = self.indent_width {
            config.indent_width = indent_width;
        }
        if let Some(line_width) = self.line_width {
            config.line_width = line_width;
        }
        if let Some(source_vertical_list_threshold) = self.source_vertical_list_threshold {
            config.source_vertical_list_threshold = source_vertical_list_threshold;
        }
        if let Some(function_schemas) = self.function_schemas.clone() {
            config.function_schemas = config.function_schemas.merge(function_schemas);
        }
        config
    }
}

#[test]
fn golden_regressions() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let spec = load_spec(&root);
    let update = env::var_os("UPDATE_GOLDENS").is_some();

    assert!(!spec.cases.is_empty(), "golden spec must include cases");

    for case in spec.cases {
        let formatted = run_case(&root, &spec.scan_sets, &case);
        let golden_path = root.join(&case.golden);

        if update {
            if let Some(parent) = golden_path.parent() {
                fs::create_dir_all(parent).expect("failed to create golden directory");
            }
            fs::write(&golden_path, &formatted).expect("failed to update golden output");
            continue;
        }

        let expected = fs::read_to_string(&golden_path).unwrap_or_else(|err| {
            panic!(
                "failed to read golden output for `{}` at {}: {err}\n\
                 rerun with UPDATE_GOLDENS=1 to create it",
                case.name,
                case.golden.display()
            )
        });

        if formatted != expected {
            panic!(
                "golden output changed for `{}`\n{}",
                case.name,
                diff(&expected, &formatted, &case.golden)
            );
        }
    }
}

#[test]
fn golden_outputs_are_idempotent() {
    if env::var_os("UPDATE_GOLDENS").is_some() {
        return;
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let spec = load_spec(&root);

    assert!(!spec.cases.is_empty(), "golden spec must include cases");

    for case in spec.cases {
        let golden_path = root.join(&case.golden);
        let golden = fs::read_to_string(&golden_path).unwrap_or_else(|err| {
            panic!(
                "failed to read golden output for `{}` at {}: {err}",
                case.name,
                case.golden.display()
            )
        });

        let config = config_for_case(&root, &spec.scan_sets, &case);
        let tree = Parser::new(&golden).parse();
        let reformatted = Formatter::new(config).format(&tree);

        if reformatted != golden {
            panic!(
                "golden output is not idempotent for `{}`\n{}",
                case.name,
                diff(&golden, &reformatted, &case.golden)
            );
        }
    }
}

fn load_spec(root: &Path) -> GoldenSpec {
    let spec_path = root.join("tests/golden/spec.toml");
    let spec_text = fs::read_to_string(&spec_path).expect("failed to read golden spec");
    toml::from_str(&spec_text).expect("failed to parse golden spec")
}

fn run_case(root: &Path, scan_sets: &HashMap<String, ScanSet>, case: &GoldenCase) -> String {
    assert!(
        !case.inputs.is_empty(),
        "golden case `{}` must list at least one input",
        case.name
    );

    let config = config_for_case(root, scan_sets, case);

    let mut formatted = String::new();
    for input in &case.inputs {
        let input_path = root.join(input);
        let content = fs::read_to_string(&input_path).unwrap_or_else(|err| {
            panic!(
                "failed to read input for `{}` at {}: {err}",
                case.name,
                input.display()
            )
        });
        let tree = Parser::new(&content).parse();
        formatted.push_str(&Formatter::new(config.clone()).format(&tree));
    }
    formatted
}

fn config_for_case(root: &Path, scan_sets: &HashMap<String, ScanSet>, case: &GoldenCase) -> Config {
    let mut config = case.config.apply_to(Config::default());
    let scan_paths = scan_paths(scan_sets, case);
    if !scan_paths.is_empty() {
        let mut discovered = Vec::new();
        for scan_path in scan_paths {
            let path = root.join(&scan_path);
            let content = fs::read_to_string(&path).unwrap_or_else(|err| {
                panic!(
                    "failed to read scan-only file for `{}` at {}: {err}",
                    case.name,
                    scan_path.display()
                )
            });
            let tree = Parser::new(&content).parse();
            discovered.extend(Analyzer::analyze_file(&tree));
        }
        config.function_schemas = results_to_registry(discovered).merge(config.function_schemas);
    }
    config
}

fn scan_paths(scan_sets: &HashMap<String, ScanSet>, case: &GoldenCase) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for scan_set in &case.scan_sets {
        let set = scan_sets.get(scan_set).unwrap_or_else(|| {
            panic!(
                "golden case `{}` references unknown scan set `{}`",
                case.name, scan_set
            )
        });
        paths.extend(set.files.iter().cloned());
    }
    paths.extend(case.scan_only.iter().cloned());
    paths
}

fn diff(expected: &str, actual: &str, golden_path: &Path) -> String {
    TextDiff::from_lines(expected, actual)
        .unified_diff()
        .header(
            &format!("expected/{}", golden_path.display()),
            &format!("actual/{}", golden_path.display()),
        )
        .to_string()
}
