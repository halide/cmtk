use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;
use similar::{ChangeTag, TextDiff};
use std::fs;
use std::path::Path;
use std::process;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum DiscoverSource {
    Git,
}

#[derive(Subcommand)]
enum Commands {
    Parse {
        #[arg(value_name = "FILE")]
        file: String,
    },
    Format {
        #[arg(value_name = "FILE", num_args = 1..)]
        files: Vec<String>,
        #[arg(long = "scan-only", value_name = "FILE", num_args = 1..)]
        scan_only: Vec<String>,
        #[arg(short = 'i', long, conflicts_with = "check")]
        in_place: bool,
        #[arg(long, conflicts_with = "in_place")]
        check: bool,
        #[arg(long)]
        no_scan: bool,
        /// Auto-populate --scan-only from another source. `git` lists tracked CMake
        /// files in the current repository (respecting .gitignore via `git ls-files`).
        #[arg(long, value_enum, value_name = "SOURCE")]
        discover: Option<DiscoverSource>,
    },
    Scan {
        #[arg(value_name = "FILE", num_args = 1..)]
        files: Vec<String>,
        /// Write discovered schemas to .cmtkrc instead of printing to stdout
        #[arg(long)]
        write: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Parse { file } => {
            let content = fs::read_to_string(file).expect("Failed to read file");
            let parser = cmtk::parser::Parser::new(&content);
            let parse_result = parser.parse();
            println!("{:#?}", parse_result);
        }
        Commands::Format {
            files,
            scan_only,
            in_place,
            check,
            no_scan,
            discover,
        } => {
            let config = cmtk::config::Config::discover();
            let mut exit_code = 0;
            let mut all_results = Vec::new();

            let mut scan_only: Vec<String> = scan_only.clone();
            if let Some(DiscoverSource::Git) = discover {
                match discover_git_cmake_files() {
                    Ok(found) => scan_only.extend(found),
                    Err(e) => {
                        eprintln!("--discover=git: {}", e);
                        process::exit(1);
                    }
                }
            }

            for file in scan_only.iter().chain(files.iter()) {
                let content = match fs::read_to_string(file) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("Failed to read {}: {}", file, e);
                        exit_code = 1;
                        continue;
                    }
                };

                let parser = cmtk::parser::Parser::new(&content);
                let parse_result = parser.parse();
                let results = cmtk::analyzer::Analyzer::analyze_file(&parse_result);
                for r in &results {
                    if !r.is_definitive {
                        eprintln!(
                            "warning: schema for {} may be incomplete (unresolvable variable refs)",
                            r.name
                        );
                    }
                }
                all_results.extend(results);
            }

            if exit_code != 0 {
                process::exit(exit_code);
            }

            let discovered = cmtk::analyzer::results_to_registry(all_results);
            let merged = discovered.merge(config.function_schemas.clone());
            let auto_config = cmtk::config::Config {
                function_schemas: merged,
                ..config
            };

            for file in files {
                let content = match fs::read_to_string(file) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("Failed to read {}: {}", file, e);
                        exit_code = 1;
                        continue;
                    }
                };

                let parser = cmtk::parser::Parser::new(&content);
                let parse_result = parser.parse();

                let formatter = cmtk::formatter::Formatter::new(auto_config.clone());
                let formatted = formatter.format(&parse_result);

                if *no_scan && check_no_scan(&parse_result, &auto_config, file) {
                    exit_code = 1;
                }

                if *check {
                    if formatted != content {
                        print_diff(&content, &formatted, file);
                        exit_code = 1;
                    }
                } else if *in_place {
                    if formatted != content
                        && let Err(e) = fs::write(file, formatted)
                    {
                        eprintln!("Failed to write {}: {}", file, e);
                        exit_code = 1;
                    }
                } else {
                    print!("{}", formatted);
                }
            }

            if exit_code != 0 {
                process::exit(exit_code);
            }
        }
        Commands::Scan { files, write } => {
            let mut all_results = Vec::new();

            for file in files {
                let content = match fs::read_to_string(file) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("Failed to read {}: {}", file, e);
                        continue;
                    }
                };

                let parser = cmtk::parser::Parser::new(&content);
                let parse_result = parser.parse();
                let results = cmtk::analyzer::Analyzer::analyze_file(&parse_result);
                for r in &results {
                    if !r.is_definitive {
                        eprintln!(
                            "warning: schema for {} may be incomplete (unresolvable variable refs)",
                            r.name
                        );
                    }
                }
                all_results.extend(results);
            }

            let registry = cmtk::analyzer::results_to_registry(all_results);

            if *write {
                write_schemas_to_cmtkrc(&registry);
            } else {
                #[derive(Serialize)]
                struct ScanOutput {
                    functions: cmtk::schema::SchemaRegistry,
                }
                let output = ScanOutput {
                    functions: registry,
                };
                match toml::to_string(&output) {
                    Ok(s) => print!("{}", s),
                    Err(e) => {
                        eprintln!("Failed to serialize schemas: {}", e);
                        process::exit(1);
                    }
                }
            }
        }
    }
}

fn print_diff(original: &str, formatted: &str, filename: &str) {
    let diff = TextDiff::from_lines(original, formatted);
    let mut printed_header = false;
    for hunk in diff.unified_diff().context_radius(3).iter_hunks() {
        if !printed_header {
            eprintln!("--- {}", filename);
            eprintln!("+++ {}", filename);
            printed_header = true;
        }
        eprintln!("{}", hunk.header());
        for change in hunk.iter_changes() {
            let prefix = match change.tag() {
                ChangeTag::Delete => "-",
                ChangeTag::Insert => "+",
                ChangeTag::Equal => " ",
            };
            eprint!("{}{}", prefix, change);
        }
    }
}

/// Returns true if any command in root goes multi-line without a schema.
fn check_no_scan(root: &cmtk::cst::SyntaxNode, config: &cmtk::config::Config, file: &str) -> bool {
    use cmtk::cst::CommandNode;
    use cmtk::syntax::SyntaxKind;

    let mut found_issue = false;

    for node in root.children() {
        if node.kind() != SyntaxKind::COMMAND {
            continue;
        }
        let cmd = match CommandNode::cast(node.clone()) {
            Some(c) => c,
            None => continue,
        };
        let name_tok = match cmd.name() {
            Some(t) => t,
            None => continue,
        };
        let cmd_name = name_tok.text();
        if config.function_schemas.get(cmd_name).is_some() {
            continue;
        }

        if command_would_be_multiline(&node, config) {
            eprintln!(
                "{}: command `{}` formatted multi-line but has no schema (run `cmtk scan` to discover it)",
                file, cmd_name
            );
            found_issue = true;
        }
    }

    found_issue
}

fn command_would_be_multiline(node: &cmtk::cst::SyntaxNode, config: &cmtk::config::Config) -> bool {
    use cmtk::cst::CommandNode;
    use cmtk::syntax::SyntaxKind;

    let cmd = match CommandNode::cast(node.clone()) {
        Some(c) => c,
        None => return false,
    };

    let cmd_name = match cmd.name() {
        Some(t) => t.text().to_string(),
        None => return false,
    };

    let args = cmd.args();

    let has_comment = args.iter().any(|t| t.kind() == SyntaxKind::COMMENT);
    if has_comment {
        return true;
    }

    // Build single-line rendering: name(arg1 arg2 ...)
    let mut single = cmd_name;
    single.push('(');
    for (i, arg) in args.iter().enumerate() {
        if i > 0 {
            single.push(' ');
        }
        single.push_str(arg.text());
    }
    single.push(')');

    single.len() > config.line_width
}

fn write_schemas_to_cmtkrc(registry: &cmtk::schema::SchemaRegistry) {
    let path = ".cmtkrc";
    let mut toml_val: toml::Value = if Path::new(path).exists() {
        match fs::read_to_string(path) {
            Ok(s) => {
                toml::from_str(&s).unwrap_or_else(|_| toml::Value::Table(toml::map::Map::new()))
            }
            Err(_) => toml::Value::Table(toml::map::Map::new()),
        }
    } else {
        toml::Value::Table(toml::map::Map::new())
    };

    let root_table = match toml_val.as_table_mut() {
        Some(t) => t,
        None => {
            eprintln!("Failed to parse .cmtkrc as TOML table");
            process::exit(1);
        }
    };

    let fn_schemas = root_table
        .entry("function_schemas")
        .or_insert_with(|| toml::Value::Table(toml::map::Map::new()));

    let fn_table = match fn_schemas.as_table_mut() {
        Some(t) => t,
        None => {
            eprintln!("function_schemas is not a TOML table in .cmtkrc");
            process::exit(1);
        }
    };

    for (name, schema) in &registry.functions {
        let mut entry = toml::map::Map::new();

        let to_arr = |v: &[String]| {
            toml::Value::Array(v.iter().map(|s| toml::Value::String(s.clone())).collect())
        };

        entry.insert("options".into(), to_arr(&schema.options));
        entry.insert(
            "one_value_keywords".into(),
            to_arr(&schema.one_value_keywords),
        );
        entry.insert(
            "multi_value_keywords".into(),
            to_arr(&schema.multi_value_keywords),
        );

        fn_table.insert(name.clone(), toml::Value::Table(entry));
    }

    let serialized = match toml::to_string_pretty(&toml_val) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to serialize .cmtkrc: {}", e);
            process::exit(1);
        }
    };

    if let Err(e) = fs::write(path, serialized) {
        eprintln!("Failed to write .cmtkrc: {}", e);
        process::exit(1);
    }
}

fn discover_git_cmake_files() -> Result<Vec<String>, String> {
    let output = process::Command::new("git")
        .args(["ls-files", "-z"])
        .output()
        .map_err(|e| format!("failed to run git: {}", e))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git ls-files failed: {}", stderr.trim()));
    }
    let stdout = String::from_utf8(output.stdout)
        .map_err(|_| "git ls-files output was not UTF-8".to_string())?;
    Ok(stdout
        .split('\0')
        .filter(|s| !s.is_empty())
        .filter(|s| is_cmake_file(s))
        .map(|s| s.to_string())
        .collect())
}

fn is_cmake_file(path: &str) -> bool {
    let basename = path.rsplit('/').next().unwrap_or(path);
    basename == "CMakeLists.txt" || path.ends_with(".cmake") || path.ends_with(".cmake.in")
}
