use assert_cmd::prelude::*;
use cmtk::analyzer::Analyzer;
use std::io::Write;
use std::process::Command;
use tempfile::NamedTempFile;

fn parse_and_analyze(src: &str) -> Vec<cmtk::analyzer::AnalysisResult> {
    let parser = cmtk::parser::Parser::new(src);
    let root = parser.parse();
    Analyzer::analyze_file(&root)
}

// T1: Standard form
#[test]
fn test_standard_form() {
    let src = r#"
function(my_func)
  cmake_parse_arguments(ARG "A;B" "C" "D;E" ${ARGN})
endfunction()
"#;
    let results = parse_and_analyze(src);
    assert_eq!(results.len(), 1);
    let r = &results[0];
    assert_eq!(r.name, "my_func");
    assert!(r.is_definitive);
    let mut opts = r.schema.options.clone();
    opts.sort();
    assert_eq!(opts, vec!["A", "B"]);
    assert_eq!(r.schema.one_value_keywords, vec!["C"]);
    let mut mv = r.schema.multi_value_keywords.clone();
    mv.sort();
    assert_eq!(mv, vec!["D", "E"]);
}

// T2: PARSE_ARGV form
#[test]
fn test_parse_argv_form() {
    let src = r#"
function(my_func)
  cmake_parse_arguments(PARSE_ARGV 0 ARG "A" "B" "C")
endfunction()
"#;
    let results = parse_and_analyze(src);
    assert_eq!(results.len(), 1);
    let r = &results[0];
    assert_eq!(r.name, "my_func");
    assert!(r.is_definitive);
    assert_eq!(r.schema.options, vec!["A"]);
    assert_eq!(r.schema.one_value_keywords, vec!["B"]);
    assert_eq!(r.schema.multi_value_keywords, vec!["C"]);
}

// T3: Variable resolution via set()
#[test]
fn test_variable_resolution() {
    let src = r#"
function(my_func)
  set(MY_OPTS "X;Y")
  cmake_parse_arguments(ARG "${MY_OPTS}" "" "")
endfunction()
"#;
    let results = parse_and_analyze(src);
    assert_eq!(results.len(), 1);
    let r = &results[0];
    assert!(r.is_definitive);
    let mut opts = r.schema.options.clone();
    opts.sort();
    assert_eq!(opts, vec!["X", "Y"]);
    assert!(r.schema.one_value_keywords.is_empty());
    assert!(r.schema.multi_value_keywords.is_empty());
}

// T4: Unresolvable variable ref → is_definitive: false
#[test]
fn test_unresolvable_ref() {
    let src = r#"
function(my_func)
  cmake_parse_arguments(ARG ${EXTERNAL_OPTS} "" "")
endfunction()
"#;
    let results = parse_and_analyze(src);
    assert_eq!(results.len(), 1);
    let r = &results[0];
    assert!(!r.is_definitive);
}

// T5: Empty string args → empty lists
#[test]
fn test_empty_args() {
    let src = r#"
function(my_func)
  cmake_parse_arguments(ARG "" "" "SRCS;DEPS")
endfunction()
"#;
    let results = parse_and_analyze(src);
    assert_eq!(results.len(), 1);
    let r = &results[0];
    assert!(r.is_definitive);
    assert!(r.schema.options.is_empty());
    assert!(r.schema.one_value_keywords.is_empty());
    let mut mv = r.schema.multi_value_keywords.clone();
    mv.sort();
    assert_eq!(mv, vec!["DEPS", "SRCS"]);
}

// T6: Macro support
#[test]
fn test_macro_support() {
    let src = r#"
macro(my_macro)
  cmake_parse_arguments(ARG "OPT" "KEY" "LIST" ${ARGN})
endmacro()
"#;
    let results = parse_and_analyze(src);
    assert_eq!(results.len(), 1);
    let r = &results[0];
    assert_eq!(r.name, "my_macro");
    assert_eq!(r.schema.options, vec!["OPT"]);
    assert_eq!(r.schema.one_value_keywords, vec!["KEY"]);
    assert_eq!(r.schema.multi_value_keywords, vec!["LIST"]);
}

// T7: Nested macro in function body — both captured separately
#[test]
fn test_nested_macro_in_function() {
    let src = r#"
function(outer_func)
  cmake_parse_arguments(ARG "OPT" "" "" ${ARGN})
  macro(inner_macro)
    cmake_parse_arguments(M "X" "Y" "" ${ARGN})
  endmacro()
endfunction()
"#;
    let results = parse_and_analyze(src);
    assert_eq!(results.len(), 2);
    let names: Vec<&str> = results.iter().map(|r| r.name.as_str()).collect();
    assert!(names.contains(&"outer_func"));
    assert!(names.contains(&"inner_macro"));
}

// T8: No function/macro → empty results
#[test]
fn test_no_functions() {
    let src = r#"
cmake_minimum_required(VERSION 3.20)
project(MyProject)
"#;
    let results = parse_and_analyze(src);
    assert!(results.is_empty());
}

// T9: File-scope cmake_parse_arguments (not inside function) → ignored
#[test]
fn test_file_scope_ignored() {
    let src = r#"
cmake_parse_arguments(ARG "A" "B" "C" ${ARGN})
"#;
    let results = parse_and_analyze(src);
    assert!(results.is_empty());
}

// T10: Multiple functions in one file → all captured
#[test]
fn test_multiple_functions() {
    let src = r#"
function(func_one)
  cmake_parse_arguments(ARG "OPT1" "" "" ${ARGN})
endfunction()

function(func_two)
  cmake_parse_arguments(ARG "" "KEY1" "" ${ARGN})
endfunction()
"#;
    let results = parse_and_analyze(src);
    assert_eq!(results.len(), 2);
    let names: Vec<&str> = results.iter().map(|r| r.name.as_str()).collect();
    assert!(names.contains(&"func_one"));
    assert!(names.contains(&"func_two"));
}

// T11: scan subcommand prints valid TOML with [functions.my_func]
#[test]
fn test_scan_stdout() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        r#"
function(my_func)
  cmake_parse_arguments(ARG "OPT" "KEY" "LIST" ${{ARGN}})
endfunction()
"#
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("cmtk").unwrap();
    cmd.arg("scan").arg(file.path());
    let output = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8(output).unwrap();

    // Should contain [functions.my_func]
    assert!(
        stdout.contains("[functions.my_func]"),
        "stdout was: {}",
        stdout
    );
    // Should be valid TOML
    let parsed: toml::Value = toml::from_str(&stdout).expect("scan output should be valid TOML");
    assert!(
        parsed.get("functions").is_some(),
        "should have functions key"
    );
}

// T12: scan --write creates/updates .cmtkrc; existing indent_style preserved
#[test]
fn test_scan_write_preserves_existing_config() {
    let dir = tempfile::tempdir().unwrap();
    let cmtkrc_path = dir.path().join(".cmtkrc");
    let cmake_path = dir.path().join("CMakeLists.txt");

    // Write existing config with indent_style
    std::fs::write(
        &cmtkrc_path,
        r#"indent_style = "tab"
indent_width = 2
"#,
    )
    .unwrap();

    // Write CMake file
    std::fs::write(
        &cmake_path,
        r#"
function(my_func)
  cmake_parse_arguments(ARG "OPT" "KEY" "LIST" ${ARGN})
endfunction()
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("cmtk").unwrap();
    cmd.current_dir(dir.path())
        .arg("scan")
        .arg("--write")
        .arg(&cmake_path);
    cmd.assert().success();

    let content = std::fs::read_to_string(&cmtkrc_path).unwrap();
    // Existing keys preserved
    assert!(
        content.contains("indent_style"),
        "should preserve indent_style; got: {}",
        content
    );
    // New schema written under function_schemas
    assert!(
        content.contains("my_func") || content.contains("my_func"),
        "should contain MY_FUNC; got: {}",
        content
    );
}

// T13: format --no-scan exits 1 when unknown function formatted multi-line
#[test]
fn test_format_no_scan_exits_1_for_unknown_multiline() {
    let mut file = NamedTempFile::new().unwrap();
    // A long command with no schema that will exceed the default line width.
    writeln!(
        file,
        "unknown_custom_command(VERY_LONG_ARG_ONE VERY_LONG_ARG_TWO VERY_LONG_ARG_THREE VERY_LONG_ARG_FOUR VERY_LONG_ARG_FIVE)"
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("cmtk").unwrap();
    cmd.arg("format").arg("--no-scan").arg(file.path());
    cmd.assert().failure().code(1);
}

// T14: Multi-line cmake_parse_arguments call is parsed correctly
#[test]
fn test_multiline_cmake_parse_arguments() {
    let src = r#"
function(my_func)
  cmake_parse_arguments(
    ARG
    "A;B"
    "C"
    "D;E"
    ${ARGN}
  )
endfunction()
"#;
    let results = parse_and_analyze(src);
    assert_eq!(results.len(), 1, "should find one schema");
    let r = &results[0];
    assert_eq!(r.name, "my_func");
    assert!(r.is_definitive);
    let mut opts = r.schema.options.clone();
    opts.sort();
    assert_eq!(opts, vec!["A", "B"]);
    assert_eq!(r.schema.one_value_keywords, vec!["C"]);
    let mut mv = r.schema.multi_value_keywords.clone();
    mv.sort();
    assert_eq!(mv, vec!["D", "E"]);
}
