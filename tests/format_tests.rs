use cmtk::config::{Config, IndentStyle};
use cmtk::formatter::Formatter;
use cmtk::parser::Parser;
use cmtk::schema::{FunctionSchema, ListType, PositionalSpec, SchemaRegistry};
use std::collections::HashMap;

fn format(src: &str, config: Config) -> String {
    let tree = Parser::new(src).parse();
    Formatter::new(config).format(&tree)
}

fn default_config() -> Config {
    Config::default()
}

fn config_with_width(line_width: usize) -> Config {
    Config {
        line_width,
        ..Config::default()
    }
}

fn config_with_schema(function_name: &str, keywords: Vec<&str>) -> Config {
    let schema = FunctionSchema {
        multi_value_keywords: keywords.into_iter().map(String::from).collect(),
        ..FunctionSchema::default()
    };
    let mut functions = HashMap::new();
    functions.insert(function_name.to_lowercase(), schema);
    Config {
        function_schemas: SchemaRegistry { functions },
        ..Config::default()
    }
}

// --- Basic formatting ---

#[test]
fn test_simple_command_no_args() {
    let src = "enable_testing()\n";
    assert_eq!(format(src, default_config()), "enable_testing()\n");
}

#[test]
fn test_simple_command_single_arg() {
    let src = "message(hello)\n";
    assert_eq!(format(src, default_config()), "message(hello)\n");
}

#[test]
fn test_extra_whitespace_between_args_normalized() {
    // Formatter strips whitespace tokens and reconstructs with single spaces
    let src = "set(  FOO   bar   baz  )\n";
    assert_eq!(format(src, default_config()), "set(FOO bar baz)\n");
}

#[test]
fn test_nested_parentheses_in_condition_are_preserved() {
    let src = "if (HANNK_BUILD_TFLITE AND (Halide_TARGET MATCHES \"wasm\"))\n    message(FATAL_ERROR \"HANNK_BUILD_TFLITE must be OFF when targeting wasm\")\nendif ()\n";
    let out = format(src, default_config());
    assert_eq!(out, src);
}

// --- Indentation ---

#[test]
fn test_indent_style_spaces() {
    let config = Config {
        indent_style: IndentStyle::Space,
        indent_width: 2,
        line_width: 10, // Force wrapping
        ..Config::default()
    };
    let src = "my_command(mytarget PUBLIC libfoo libbar)\n";
    let out = format(src, config);
    // Wrapped arguments use hanging indentation with spaces.
    assert!(out.contains("\n  "), "Expected 2-space indent in:\n{out}");
}

#[test]
fn test_indent_style_tabs() {
    let config = Config {
        indent_style: IndentStyle::Tab,
        indent_width: 1,
        line_width: 10, // Force wrapping
        ..Config::default()
    };
    let src = "target_link_libraries(mytarget PUBLIC libfoo)\n";
    let out = format(src, config);
    assert!(out.contains('\t'), "Expected tab indent in:\n{out}");
}

// --- Line width wrapping ---

#[test]
fn test_short_command_stays_on_one_line() {
    let config = config_with_width(80);
    let src = "set(FOO bar)\n";
    assert_eq!(format(src, config), "set(FOO bar)\n");
}

#[test]
fn test_long_command_wraps() {
    let config = config_with_width(20);
    let src = "target_link_libraries(mytarget PUBLIC libfoo libbar libbaz)\n";
    let out = format(src, config);
    assert!(out.contains('\n'), "Expected line break in:\n{out}");
    let lines: Vec<&str> = out.lines().collect();
    assert!(lines.len() > 1, "Expected multiple lines:\n{out}");
}

#[test]
fn test_wrapping_indentation_is_one_level_in() {
    let config = Config {
        indent_style: IndentStyle::Space,
        indent_width: 4,
        line_width: 20,
        ..Config::default()
    };
    let src = "my_command(mytarget PUBLIC libfoo libbar)\n";
    let out = format(src, config);
    // Each argument should be on its own line with 4-space indent
    for line in out.lines().skip(1) {
        if line != ")" && !line.is_empty() {
            assert!(
                line.starts_with("    "),
                "Expected 4-space indent on arg line: {line:?}\nFull output:\n{out}"
            );
        }
    }
}

// --- Schema-aware formatting ---

#[test]
fn test_schema_keyword_stays_inline_when_it_fits() {
    let config = config_with_schema(
        "target_link_libraries",
        vec!["PUBLIC", "PRIVATE", "INTERFACE"],
    );
    let src = "target_link_libraries(mytarget PUBLIC libfoo)\n";
    let out = format(src, config);
    assert_eq!(out, "target_link_libraries(mytarget PUBLIC libfoo)\n");
}

#[test]
fn test_schema_keyword_starts_wrapped_group() {
    let mut config = config_with_schema("target_link_libraries", vec!["PUBLIC", "PRIVATE"]);
    config.line_width = 40;
    let src = "target_link_libraries(mytarget PUBLIC libfoo PRIVATE libbar)\n";
    let out = format(src, config);
    assert!(
        out.contains("\n    PUBLIC libfoo"),
        "Expected PUBLIC to start a wrapped group in:\n{out}"
    );
    assert!(
        out.contains("\n    PRIVATE libbar"),
        "Expected PRIVATE to start a wrapped group in:\n{out}"
    );
}

#[test]
fn test_schema_values_after_keyword_indented_deeper() {
    let config = Config {
        indent_style: IndentStyle::Space,
        indent_width: 4,
        line_width: 200,
        function_schemas: {
            let schema = FunctionSchema {
                multi_value_keywords: vec!["SOURCES".to_string(), "HEADERS".to_string()],
                ..FunctionSchema::default()
            };
            let mut functions = HashMap::new();
            functions.insert("my_lib".to_string(), schema);
            SchemaRegistry { functions }
        },
        ..Default::default()
    };
    let src = "MY_LIB(mylib SOURCES foo.cpp bar.cpp HEADERS foo.h)\n";
    let out = format(src, config);
    assert_eq!(out, "MY_LIB(mylib SOURCES foo.cpp bar.cpp HEADERS foo.h)\n");
}

#[test]
fn test_schema_case_insensitive_lookup() {
    let config = config_with_schema("add_library", vec!["STATIC", "SHARED", "MODULE"]);
    // Command name in lowercase should still find the schema
    let src = "add_library(mylib STATIC foo.cpp)\n";
    let out = format(src, config);
    assert!(
        out.contains('\n'),
        "Expected multiline output for schema match:\n{out}"
    );
}

// --- Comments are preserved ---

#[test]
fn test_comment_in_file_preserved() {
    let src = "# Top-level comment\nset(FOO bar)\n";
    let out = format(src, default_config());
    assert!(
        out.contains("# Top-level comment"),
        "Comment should be preserved:\n{out}"
    );
}

// --- Config discovery ---

#[test]
fn test_config_default_values() {
    let config = Config::default();
    assert_eq!(config.indent_width, 4);
    assert_eq!(config.line_width, 100);
    assert_eq!(config.source_vertical_list_threshold, 3);
    assert!(matches!(config.indent_style, IndentStyle::Space));
}

#[test]
fn test_config_load_from_toml_string() {
    let toml_str = r#"
indent_style = "space"
indent_width = 2
line_width = 100
source_vertical_list_threshold = -1
"#;
    let config: Config = toml::from_str(toml_str).expect("should parse");
    assert_eq!(config.indent_width, 2);
    assert_eq!(config.line_width, 100);
    assert_eq!(config.source_vertical_list_threshold, -1);
}

#[test]
fn test_config_load_schema_from_toml() {
    let toml_str = r#"
[my_func]
multi_value_keywords = ["OPTION_A", "OPTION_B"]
"#;
    let registry: SchemaRegistry = toml::from_str(toml_str).expect("should parse schema");
    let schema = registry.get("MY_FUNC").expect("should find schema");
    assert_eq!(schema.multi_value_keywords, vec!["OPTION_A", "OPTION_B"]);
}

#[test]
fn test_config_load_modes_from_toml() {
    let toml_str = r#"
[my_func]
no_break_first_argument = true

[my_func.modes.CONFIGURE]
one_value_keywords = ["OUTPUT", "CONTENT"]
"#;
    let registry: SchemaRegistry = toml::from_str(toml_str).expect("should parse schema");
    let schema = registry.get("MY_FUNC").expect("should find schema");
    // Mode selectors (and keyword arguments generally) are case-sensitive per
    // CMake semantics — the registered name is the only one that matches.
    assert!(schema.mode("configure").is_none());
    let mode = schema.mode("CONFIGURE").expect("should find mode");
    assert_eq!(mode.one_value_keywords, vec!["OUTPUT", "CONTENT"]);
}

#[test]
fn test_config_schema_lookup_case_insensitive() {
    let toml_str = r#"
[my_func]
multi_value_keywords = ["SOURCES"]
"#;
    let registry: SchemaRegistry = toml::from_str(toml_str).expect("should parse");
    assert!(registry.get("my_func").is_some());
    assert!(registry.get("MY_FUNC").is_some());
    assert!(registry.get("My_Func").is_some());
}

#[test]
fn test_option_flag_on_its_own_line() {
    let mut functions = HashMap::new();
    functions.insert(
        "my_command".to_string(),
        FunctionSchema {
            options: vec!["FORCE".to_string()],
            ..Default::default()
        },
    );
    let config = Config {
        function_schemas: SchemaRegistry { functions },
        ..Default::default()
    };
    let src = "my_command(FORCE arg1)\n";
    let out = format(src, config);
    assert_eq!(out, "my_command(FORCE arg1)\n");
}

#[test]
fn test_one_value_keyword_and_value_on_same_line() {
    let mut functions = HashMap::new();
    functions.insert(
        "my_command".to_string(),
        FunctionSchema {
            one_value_keywords: vec!["DESTINATION".to_string()],
            ..Default::default()
        },
    );
    let config = Config {
        function_schemas: SchemaRegistry { functions },
        ..Default::default()
    };
    let src = "my_command(DESTINATION /usr/bin arg1)\n";
    let out = format(src, config);
    assert_eq!(out, "my_command(DESTINATION /usr/bin arg1)\n");
}

#[test]
fn test_one_value_keyword_at_end_no_panic() {
    let mut functions = HashMap::new();
    functions.insert(
        "my_command".to_string(),
        FunctionSchema {
            one_value_keywords: vec!["DESTINATION".to_string()],
            ..Default::default()
        },
    );
    let config = Config {
        function_schemas: SchemaRegistry { functions },
        ..Default::default()
    };
    let src = "my_command(DESTINATION)\n";
    let out = format(src, config);
    assert_eq!(out, "my_command(DESTINATION)\n");
}

#[test]
fn test_inline_comment_in_command_preserved() {
    let src = "set(FOO bar # some comment\n)\n";
    let out = format(src, default_config());
    assert!(
        out.contains("# some comment"),
        "Comment should be preserved"
    );
}

#[test]
fn test_standalone_comment_forces_multiline() {
    let src = "set(FOO bar\n# comment\n)\n";
    let out = format(src, default_config());
    assert!(out.contains('\n'), "Comment should force multiline");
    assert!(out.contains("    # comment"), "Comment should be indented");
}

#[test]
fn test_comment_ends_schema_value_group_before_next_keyword() {
    let mut functions = HashMap::new();
    functions.insert(
        "my_command".to_string(),
        FunctionSchema {
            multi_value_keywords: vec!["LIST".to_string()],
            one_value_keywords: vec!["KEY".to_string()],
            ..Default::default()
        },
    );
    let config = Config {
        function_schemas: SchemaRegistry { functions },
        line_width: 40,
        ..Default::default()
    };
    let src = "my_command(LIST a # explains key\nKEY b)\n";
    let out = format(src, config);
    assert_eq!(
        out,
        "my_command(\n    LIST a  # explains key\n    KEY b\n)\n"
    );
}

#[test]
fn test_comment_before_one_value_keyword_does_not_swallow_keyword() {
    let mut functions = HashMap::new();
    functions.insert(
        "add_halide_library".to_string(),
        FunctionSchema {
            positional: Some(PositionalSpec { min: 1, max: 1 }),
            one_value_keywords: vec!["FROM".to_string(), "ASSEMBLY".to_string()],
            multi_value_keywords: vec!["PARAMS".to_string()],
            ..Default::default()
        },
    );
    let config = Config {
        function_schemas: SchemaRegistry { functions },
        line_width: 50,
        ..Default::default()
    };
    let src = "add_halide_library(my_target FROM generator PARAMS a b # describes assembly\nASSEMBLY out.s)\n";
    let out = format(src, config);
    assert_eq!(
        out,
        "add_halide_library(\n    my_target\n    FROM generator\n    PARAMS a b  # describes assembly\n    ASSEMBLY out.s\n)\n"
    );
}

#[test]
fn test_short_trailing_comment_stays_on_formatted_line() {
    let config = config_with_width(80);
    let src = "target_link_libraries(benchmark PRIVATE Halide::Tools # for halide_benchmark.h\nHalide::Runtime)\n";
    let out = format(src, config);
    assert_eq!(
        out,
        "target_link_libraries(\n    benchmark\n    PRIVATE\n    Halide::Tools  # for halide_benchmark.h\n    Halide::Runtime\n)\n"
    );
}

#[test]
fn test_short_trailing_comment_stays_on_continuation_line() {
    let config = config_with_width(80);
    let src = "target_link_libraries(benchmark PRIVATE tflite_parser interpreter error_util file_util hannk_log_stderr Halide::Tools # for halide_benchmark.h\nHalide::Runtime)\n";
    let out = format(src, config);
    assert_eq!(
        out,
        "target_link_libraries(\n    benchmark\n    PRIVATE\n    tflite_parser interpreter error_util file_util hannk_log_stderr\n    Halide::Tools  # for halide_benchmark.h\n    Halide::Runtime\n)\n"
    );
}

#[test]
fn test_long_trailing_comment_moves_before_commented_argument() {
    let config = config_with_width(68);
    let src = "target_compile_options(${TARGET} PRIVATE $<$<CXX_COMPILER_ID:GNU,Clang,AppleClang>:-Wdeprecated-declarations> $<$<CXX_COMPILER_ID:MSVC>:/w14996> # 4996: compiler encountered deprecated declaration\n)\n";
    let out = format(src, config);
    assert_eq!(
        out,
        "target_compile_options(\n    ${TARGET}\n    PRIVATE\n    $<$<CXX_COMPILER_ID:GNU,Clang,AppleClang>:-Wdeprecated-declarations>\n    # 4996: compiler encountered deprecated declaration\n    $<$<CXX_COMPILER_ID:MSVC>:/w14996>\n)\n"
    );
}

#[test]
fn test_standalone_comment_before_close_keeps_close_on_next_line() {
    let config = config_with_width(80);
    let src =
        "target_link_libraries(benchmark PRIVATE Halide::Tools\n# for halide_benchmark.h\n)\n";
    let out = format(src, config);
    assert_eq!(
        out,
        "target_link_libraries(\n    benchmark\n    PRIVATE Halide::Tools\n    # for halide_benchmark.h\n)\n"
    );
}

#[test]
fn test_standalone_comments_inside_value_group_preserve_continuation_indent() {
    let config = config_with_width(80);
    let src = "target_compile_definitions(t PRIVATE\n# Skip default module definition\nHALIDE_PYTHON_EXTENSION_OMIT_MODULE_DEFINITION\n# Explicit module name\nHALIDE_PYTHON_EXTENSION_MODULE_NAME=module)\n";
    let out = format(src, config);
    assert_eq!(
        out,
        "target_compile_definitions(\n    t\n    PRIVATE\n    # Skip default module definition\n    HALIDE_PYTHON_EXTENSION_OMIT_MODULE_DEFINITION\n    # Explicit module name\n    HALIDE_PYTHON_EXTENSION_MODULE_NAME=module\n)\n"
    );
}

#[test]
fn test_builtin_target_link_libraries_schema_active() {
    let src = "target_link_libraries(mytarget PUBLIC libfoo PRIVATE libbar)\n";
    let out = format(src, default_config());
    assert_eq!(
        out,
        "target_link_libraries(mytarget PUBLIC libfoo PRIVATE libbar)\n"
    );
}

#[test]
fn test_target_link_options_builtin_schema_groups_visibility_options() {
    let config = config_with_width(80);
    let src = "target_link_options(t PRIVATE \"SHELL:-s ALLOW_MEMORY_GROWTH=1\" \"SHELL:-s ENVIRONMENT=node\" \"SHELL:-s NODERAWFS\" \"SHELL:$<$<CONFIG:Debug>:-s ASSERTIONS=1>\")\n";
    let out = format(src, config);
    assert_eq!(
        out,
        "target_link_options(\n    t\n    PRIVATE\n    \"SHELL:-s ALLOW_MEMORY_GROWTH=1\" \"SHELL:-s ENVIRONMENT=node\"\n    \"SHELL:-s NODERAWFS\" \"SHELL:$<$<CONFIG:Debug>:-s ASSERTIONS=1>\"\n)\n"
    );
}

#[test]
fn test_target_include_directories_builtin_schema_uses_path_lists() {
    let config = config_with_width(80);
    let src =
        "target_include_directories(t PUBLIC include src/include PRIVATE generated/include)\n";
    let out = format(src, config);
    assert_eq!(
        out,
        "target_include_directories(\n    t\n    PUBLIC\n        include\n        src/include\n    PRIVATE generated/include\n)\n"
    );
}

#[test]
fn test_user_schema_overrides_builtin() {
    let mut functions = HashMap::new();
    // Override TARGET_LINK_LIBRARIES to have no keywords
    functions.insert(
        "target_link_libraries".to_string(),
        FunctionSchema::default(),
    );
    let config = Config {
        function_schemas: SchemaRegistry { functions },
        ..Default::default()
    };
    let src = "target_link_libraries(mytarget PUBLIC libfoo)\n";
    let out = format(src, config);
    // Since it fits on one line and has no keywords (overridden), it should stay on one line
    assert_eq!(out, "target_link_libraries(mytarget PUBLIC libfoo)\n");
}

#[test]
fn test_backward_compat_keywords_alias_in_toml() {
    let toml_str = r#"
[my_func]
keywords = ["K1", "K2"]
"#;
    let registry: SchemaRegistry = toml::from_str(toml_str).expect("should parse");
    let schema = registry.get("MY_FUNC").expect("should find schema");
    assert_eq!(schema.multi_value_keywords, vec!["K1", "K2"]);
}

#[test]
fn test_typed_list_keywords_in_toml() {
    let toml_str = r#"
[my_func]
list_keywords = [
    { name = "FILES", list_type = "path" },
    { name = "COMMAND", list_type = "command_argv" },
]
"#;
    let registry: SchemaRegistry = toml::from_str(toml_str).expect("should parse");
    let schema = registry.get("MY_FUNC").expect("should find schema");
    assert_eq!(schema.multi_value_keywords, vec!["FILES", "COMMAND"]);
    assert_eq!(schema.list_type("FILES"), ListType::Path);
    assert_eq!(schema.list_type("COMMAND"), ListType::CommandArgv);
}

#[test]
fn test_typed_path_list_formats_one_per_line() {
    let toml_str = r#"
[my_func]
list_keywords = [{ name = "FILES", list_type = "path" }]
"#;
    let registry: SchemaRegistry = toml::from_str(toml_str).expect("should parse");
    let config = Config {
        function_schemas: registry,
        line_width: 20,
        ..Default::default()
    };
    let src = "my_func(FILES a.cpp b.cpp)\n";
    let out = format(src, config);
    assert_eq!(out, "my_func(\n    FILES\n    a.cpp\n    b.cpp\n)\n");
}

#[test]
fn test_typed_path_list_keeps_singleton_inline_when_it_fits() {
    let toml_str = r#"
[my_func]
list_keywords = [{ name = "FILES", list_type = "path" }]
"#;
    let registry: SchemaRegistry = toml::from_str(toml_str).expect("should parse");
    let config = Config {
        function_schemas: registry,
        line_width: 80,
        ..Default::default()
    };
    let src = "my_func(FILES a.cpp)\n";
    let out = format(src, config);
    assert_eq!(out, "my_func(FILES a.cpp)\n");
}

#[test]
fn test_source_vertical_list_threshold_preserves_vertical_packed_list() {
    let config = default_config();
    let src = "set(extra_output_names\n    # keep-sorted start\n    ASSEMBLY\n    BITCODE\n    COMPILER_LOG\n    C_SOURCE\n    # keep-sorted end\n)\n";
    let out = format(src, config);
    assert_eq!(
        out,
        "set(extra_output_names\n    # keep-sorted start\n    ASSEMBLY\n    BITCODE\n    COMPILER_LOG\n    C_SOURCE\n    # keep-sorted end\n)\n"
    );
}

#[test]
fn test_source_vertical_list_threshold_can_disable_vertical_preservation() {
    let config = Config {
        source_vertical_list_threshold: -1,
        ..default_config()
    };
    let src = "set(extra_output_names\n    # keep-sorted start\n    ASSEMBLY\n    BITCODE\n    COMPILER_LOG\n    C_SOURCE\n    # keep-sorted end\n)\n";
    let out = format(src, config);
    assert_eq!(
        out,
        "set(extra_output_names\n    # keep-sorted start\n    ASSEMBLY BITCODE COMPILER_LOG C_SOURCE\n    # keep-sorted end\n)\n"
    );
}

#[test]
fn test_source_vertical_list_threshold_zero_chops_packed_list() {
    let config = Config {
        line_width: 30,
        source_vertical_list_threshold: 0,
        ..default_config()
    };
    let src = "set(extra_output_names ASSEMBLY BITCODE COMPILER_LOG C_SOURCE)\n";
    let out = format(src, config);
    assert_eq!(
        out,
        "set(extra_output_names\n    ASSEMBLY\n    BITCODE\n    COMPILER_LOG\n    C_SOURCE\n)\n"
    );
}

#[test]
fn test_list_append_keeps_mode_and_list_variable_together() {
    let config = config_with_width(60);
    let src = "list(APPEND xcfw_commands COMMAND ${CMAKE_COMMAND} -E make_directory \"${staging}/${platform}/Headers\")\n";
    let out = format(src, config);
    assert_eq!(
        out,
        "list(APPEND xcfw_commands\n    COMMAND ${CMAKE_COMMAND} -E make_directory\n    \"${staging}/${platform}/Headers\"\n)\n"
    );
}

#[test]
fn test_list_prepend_keeps_mode_and_list_variable_together() {
    let config = config_with_width(30);
    let src = "list(PREPEND plugins_args -p --verbose --dry-run)\n";
    let out = format(src, config);
    assert_eq!(
        out,
        "list(PREPEND plugins_args\n    -p --verbose --dry-run\n)\n"
    );
}

#[test]
fn test_list_transform_append_groups_list_and_action() {
    let config = config_with_width(40);
    let src = "list(TRANSFORM ARG_TARGETS APPEND \"-no_runtime\")\n";
    let out = format(src, config);
    assert_eq!(
        out,
        "list(TRANSFORM ARG_TARGETS\n    APPEND \"-no_runtime\"\n)\n"
    );
}

#[test]
fn test_list_transform_replace_groups_action_and_output_variable() {
    let config = config_with_width(72);
    let src = "list(TRANSFORM ARG_PLUGINS REPLACE \"(.+)\" \"$<TARGET_FILE:\\\\1>\" OUTPUT_VARIABLE plugins_args)\n";
    let out = format(src, config);
    assert_eq!(
        out,
        "list(TRANSFORM ARG_PLUGINS\n    REPLACE \"(.+)\" \"$<TARGET_FILE:\\\\1>\"\n    OUTPUT_VARIABLE plugins_args\n)\n"
    );
}

#[test]
fn test_typed_command_argv_list_wraps_continuations_deeper() {
    let mut functions = HashMap::new();
    functions.insert(
        "my_command".to_string(),
        FunctionSchema {
            multi_value_keywords: vec!["COMMAND".to_string(), "DEPENDS".to_string()],
            list_keyword_types: [("COMMAND".to_string(), ListType::CommandArgv)]
                .into_iter()
                .collect(),
            ..Default::default()
        },
    );
    let config = Config {
        function_schemas: SchemaRegistry { functions },
        line_width: 55,
        ..Default::default()
    };
    let src = "my_command(COMMAND generator --emit generated.cpp --target host --output \"${OUT}\" DEPENDS generator)\n";
    let out = format(src, config);
    assert_eq!(
        out,
        "my_command(\n    COMMAND\n        generator --emit generated.cpp --target host\n        --output \"${OUT}\"\n    DEPENDS generator\n)\n"
    );
}

#[test]
fn test_typed_command_argv_preserves_source_vertical_command_lines() {
    let mut functions = HashMap::new();
    functions.insert(
        "my_command".to_string(),
        FunctionSchema {
            multi_value_keywords: vec!["COMMAND".to_string(), "DEPENDS".to_string()],
            list_keyword_types: [("COMMAND".to_string(), ListType::CommandArgv)]
                .into_iter()
                .collect(),
            ..Default::default()
        },
    );
    let config = Config {
        function_schemas: SchemaRegistry { functions },
        line_width: 70,
        ..Default::default()
    };
    let src = "my_command(\n    COMMAND generator\n        --emit generated.cpp\n        --target host\n        --output \"${OUT}\"\n    DEPENDS generator\n)\n";
    let out = format(src, config);
    assert_eq!(
        out,
        "my_command(\n    COMMAND\n        generator\n        --emit generated.cpp\n        --target host\n        --output \"${OUT}\"\n    DEPENDS generator\n)\n"
    );
}

#[test]
fn test_typed_command_argv_keeps_negative_number_values_with_flags() {
    let mut functions = HashMap::new();
    functions.insert(
        "my_command".to_string(),
        FunctionSchema {
            multi_value_keywords: vec!["COMMAND".to_string()],
            list_keyword_types: [("COMMAND".to_string(), ListType::CommandArgv)]
                .into_iter()
                .collect(),
            ..Default::default()
        },
    );
    let config = Config {
        function_schemas: SchemaRegistry { functions },
        line_width: 42,
        ..Default::default()
    };
    let src = "my_command(COMMAND tool --threshold -1 --scale -2.5 --name output)\n";
    let out = format(src, config);
    assert_eq!(
        out,
        "my_command(\n    COMMAND\n        tool --threshold -1 --scale -2.5\n        --name output\n)\n"
    );
}

#[test]
fn test_typed_command_argv_wraps_before_redirect_operator_not_after() {
    // At this width, the line breaks between `--extra-flag` and `>`. Without
    // the redirect/target pairing, the break would instead land between `>`
    // and `foo.txt` (the operator riding with `--extra-flag` as if it were
    // that flag's value), splitting the operator from its target.
    let mut functions = HashMap::new();
    functions.insert(
        "my_command".to_string(),
        FunctionSchema {
            multi_value_keywords: vec!["COMMAND".to_string()],
            list_keyword_types: [("COMMAND".to_string(), ListType::CommandArgv)]
                .into_iter()
                .collect(),
            ..Default::default()
        },
    );
    let config = Config {
        function_schemas: SchemaRegistry { functions },
        line_width: 33,
        ..Default::default()
    };
    let src = "my_command(COMMAND tool --verbose --extra-flag > foo.txt)\n";
    let out = format(src, config);
    assert_eq!(
        out,
        "my_command(\n    COMMAND\n        tool --verbose --extra-flag\n        > foo.txt\n)\n"
    );
}

#[test]
fn test_typed_command_argv_does_not_pair_squished_redirect_with_unrelated_argument() {
    let mut functions = HashMap::new();
    functions.insert(
        "my_command".to_string(),
        FunctionSchema {
            multi_value_keywords: vec!["COMMAND".to_string()],
            list_keyword_types: [("COMMAND".to_string(), ListType::CommandArgv)]
                .into_iter()
                .collect(),
            ..Default::default()
        },
    );
    let config = Config {
        function_schemas: SchemaRegistry { functions },
        line_width: 22,
        ..Default::default()
    };
    let src = "my_command(COMMAND tool >foo.txt bar.txt)\n";
    let out = format(src, config);
    assert_eq!(
        out,
        "my_command(\n    COMMAND\n        tool >foo.txt\n        bar.txt\n)\n"
    );
}

#[test]
fn test_typed_command_argv_does_not_pair_dash_separator_or_equals_flags() {
    let mut functions = HashMap::new();
    functions.insert(
        "my_command".to_string(),
        FunctionSchema {
            multi_value_keywords: vec!["COMMAND".to_string()],
            list_keyword_types: [("COMMAND".to_string(), ListType::CommandArgv)]
                .into_iter()
                .collect(),
            ..Default::default()
        },
    );
    let config = Config {
        function_schemas: SchemaRegistry { functions },
        line_width: 29,
        ..Default::default()
    };
    let src = "my_command(COMMAND tool -- --literal value)\n";
    let out = format(src, config);
    assert_eq!(
        out,
        "my_command(\n    COMMAND\n        tool -- --literal value\n)\n"
    );
}

#[test]
fn test_find_package_uses_block_groups_from_builtin_schema() {
    let config = config_with_width(80);
    let src = "find_package(Halide_LLVM 21...99 REQUIRED COMPONENTS WebAssembly X86 OPTIONAL_COMPONENTS AArch64 ARM Hexagon NVPTX PowerPC RISCV)\n";
    let out = format(src, config);
    assert_eq!(
        out,
        "find_package(\n    Halide_LLVM 21...99 REQUIRED\n    COMPONENTS WebAssembly X86\n    OPTIONAL_COMPONENTS AArch64 ARM Hexagon NVPTX PowerPC RISCV\n)\n"
    );
}

#[test]
fn test_set_keeps_variable_name_after_open_paren() {
    let config = config_with_width(80);
    let src = "set(Halide_INSTALL_TOOLSDIR \"${CMAKE_INSTALL_DATADIR}/tools\" CACHE STRING \"Path to Halide build-time tools and sources\")\n";
    let out = format(src, config);
    assert_eq!(
        out,
        "set(Halide_INSTALL_TOOLSDIR \"${CMAKE_INSTALL_DATADIR}/tools\"\n    CACHE STRING \"Path to Halide build-time tools and sources\"\n)\n"
    );
}

#[test]
fn test_set_lowercase_cache_value_is_not_cache_signature() {
    let config = config_with_width(40);
    let src = "set(RUNTIME_CPP\n    allocation_cache\n    cache\n    can_use_target\n    cuda\n)\n";
    let out = format(src, config);
    assert!(
        out.contains("\n    cache\n    can_use_target\n    cuda\n"),
        "lowercase cache should remain a list value:\n{out}"
    );
    assert!(
        !out.contains("cache can_use_target cuda"),
        "lowercase cache should not start the CACHE signature:\n{out}"
    );
}

#[test]
fn test_target_sources_path_lists_are_one_per_line() {
    let config = config_with_width(80);
    let src = "target_sources(Halide PRIVATE src/Argument.cpp src/Bounds.cpp PUBLIC include/Halide.h include/HalideBuffer.h)\n";
    let out = format(src, config);
    assert_eq!(
        out,
        "target_sources(\n    Halide\n    PRIVATE\n        src/Argument.cpp\n        src/Bounds.cpp\n    PUBLIC\n        include/Halide.h\n        include/HalideBuffer.h\n)\n"
    );
}

#[test]
fn test_find_package_trailing_option_comment_stays_trailing_when_it_fits() {
    let config = config_with_width(80);
    let src = "find_package(Halide_LLVM 21...99 REQUIRED  # Use 99 to fake a minimum-only constraint\nCOMPONENTS WebAssembly X86)\n";
    let out = format(src, config);
    assert_eq!(
        out,
        "find_package(\n    Halide_LLVM 21...99 REQUIRED  # Use 99 to fake a minimum-only constraint\n    COMPONENTS WebAssembly X86\n)\n"
    );
}

#[test]
fn test_find_package_trailing_option_comment_promotes_to_leading_on_overflow() {
    let config = config_with_width(60);
    let src = "find_package(Halide_LLVM 21...99 REQUIRED  # Use 99 to fake a minimum-only constraint\nCOMPONENTS WebAssembly X86)\n";
    let out = format(src, config);
    assert_eq!(
        out,
        "find_package(\n    # Use 99 to fake a minimum-only constraint\n    Halide_LLVM 21...99 REQUIRED\n    COMPONENTS WebAssembly X86\n)\n"
    );
}

#[test]
fn test_comment_prevents_option_gluing() {
    let config = config_with_width(80);
    let src = "find_package(MyPkg # comment\nREQUIRED)\n";
    let out = format(src, config);
    assert_eq!(
        out,
        "find_package(\n    MyPkg  # comment\n    REQUIRED\n)\n"
    );
}

#[test]
fn test_comment_prevents_closing_paren_gluing_head_only() {
    let config = default_config();
    let src = "set(# line comment\n)\n";
    let out = format(src, config);
    assert_eq!(out, "set(  # line comment\n)\n");
}

#[test]
fn test_comment_prevents_closing_paren_gluing_with_args() {
    let config = default_config();
    let src = "foreach(FOO# line comment\n)\n";
    let out = format(src, config);
    assert_eq!(out, "foreach (FOO  # line comment\n)\n");
}

#[test]
fn test_string_join_keeps_mode_glue_and_output_together() {
    let config = config_with_width(80);
    let src = "string(JOIN \"\" error_message \"Halide cannot be built when CMAKE_BUILD_TYPE is empty. \" \"Please set CMAKE_BUILD_TYPE to one of the standard types: \" \"Debug, Release, RelWithDebInfo, or MinSizeRel.\")\n";
    let out = format(src, config);
    assert_eq!(
        out,
        "string(JOIN \"\" error_message\n    \"Halide cannot be built when CMAKE_BUILD_TYPE is empty. \"\n    \"Please set CMAKE_BUILD_TYPE to one of the standard types: \"\n    \"Debug, Release, RelWithDebInfo, or MinSizeRel.\"\n)\n"
    );
}

#[test]
fn test_string_concat_keeps_mode_and_output_together() {
    let config = config_with_width(20);
    let src = "string(CONCAT stub_text \"#include <Halide.h>\\n\" \"int main(int argc, char **argv) {\\n\" \"    return 0;\\n\" \"}\\n\")\n";
    let out = format(src, config);
    // Modes with a single positional keep mode + positional glued as the
    // command header even when the line overflows the budget — the pair is
    // semantically a unit (CONCAT output_var, FATAL_ERROR text, etc.) and the
    // overflow is bounded by one positional.
    assert_eq!(
        out,
        "string(CONCAT stub_text\n    \"#include <Halide.h>\\n\"\n    \"int main(int argc, char **argv) {\\n\"\n    \"    return 0;\\n\"\n    \"}\\n\"\n)\n"
    );
}

#[test]
fn test_execute_process_builtin_schema_groups_command_and_outputs() {
    let config = config_with_width(80);
    let src = "execute_process(COMMAND \"${NODE_JS_EXECUTABLE}\" --version OUTPUT_VARIABLE NODE_JS_VERSION_RAW OUTPUT_STRIP_TRAILING_WHITESPACE)\n";
    let out = format(src, config);
    assert_eq!(
        out,
        "execute_process(\n    COMMAND \"${NODE_JS_EXECUTABLE}\" --version\n    OUTPUT_VARIABLE NODE_JS_VERSION_RAW\n    OUTPUT_STRIP_TRAILING_WHITESPACE\n)\n"
    );
}

#[test]
fn test_add_test_builtin_schema_groups_name_and_command() {
    let config = config_with_width(55);
    let src = "add_test(NAME ${test_name} COMMAND compare_vs_tflite ${t} --benchmark 0)\n";
    let out = format(src, config);
    assert_eq!(
        out,
        "add_test(\n    NAME ${test_name}\n    COMMAND compare_vs_tflite ${t} --benchmark 0\n)\n"
    );
}

#[test]
fn test_set_tests_properties_builtin_schema_groups_properties() {
    let config = config_with_width(55);
    let src = "set_tests_properties(${test_name} PROPERTIES LABELS hannk_tests TIMEOUT 60)\n";
    let out = format(src, config);
    // PROPERTIES is the trailing keyword, so the flat rule pulls the pairs flush
    // with it. A wrapping pair value would still indent one level deeper.
    assert_eq!(
        out,
        "set_tests_properties(\n    ${test_name}\n    PROPERTIES\n    LABELS hannk_tests\n    TIMEOUT 60\n)\n"
    );
}

#[test]
fn test_define_property_builtin_schema_groups_property_and_docs() {
    let config = config_with_width(80);
    let src = "define_property(TARGET PROPERTY Halide_RT_TARGETS # nolint\nBRIEF_DOCS \"On a Halide runtime target, lists the targets the runtime backs\" FULL_DOCS \"On a Halide runtime target, lists the targets the runtime backs\")\n";
    let out = format(src, config);
    assert_eq!(
        out,
        "define_property(\n    TARGET\n    PROPERTY Halide_RT_TARGETS  # nolint\n    BRIEF_DOCS \"On a Halide runtime target, lists the targets the runtime backs\"\n    FULL_DOCS \"On a Halide runtime target, lists the targets the runtime backs\"\n)\n"
    );
}

#[test]
fn test_file_configure_builtin_schema_keeps_output_and_content_groups() {
    let config = config_with_width(80);
    let src = "file(CONFIGURE OUTPUT \"${CMAKE_CURRENT_BINARY_DIR}/very/long/generated/path/output.txt\" CONTENT \"Generated content that is long enough to force this command to wrap\")\n";
    let out = format(src, config);
    // One-value keywords keep their value on the same line regardless of width
    assert_eq!(
        out,
        "file(CONFIGURE\n    OUTPUT \"${CMAKE_CURRENT_BINARY_DIR}/very/long/generated/path/output.txt\"\n    CONTENT \"Generated content that is long enough to force this command to wrap\"\n)\n"
    );
}

#[test]
fn test_file_configure_keeps_mode_separate_from_output_keyword() {
    let config = config_with_width(100);
    let src = "file(CONFIGURE OUTPUT \"${CMAKE_CURRENT_BINARY_DIR}/${TARGET}.${SYMBOL}.ldscript\" CONTENT \"{ global: ${SYMBOL}; local: *; };\\n\")\n";
    let out = format(src, config);
    assert_eq!(
        out,
        "file(CONFIGURE\n    OUTPUT \"${CMAKE_CURRENT_BINARY_DIR}/${TARGET}.${SYMBOL}.ldscript\"\n    CONTENT \"{ global: ${SYMBOL}; local: *; };\\n\"\n)\n"
    );
}

#[test]
fn test_cmake_language_call_treats_function_name_as_positional() {
    let config = config_with_width(40);
    let src = "cmake_language(CALL my_callback_function arg1 arg2 arg3 arg4)\n";
    let out = format(src, config);
    // CALL is the mode, function name is the sole positional, remaining args
    // pack as a generic value list under the CALL header.
    assert_eq!(
        out,
        "cmake_language(CALL my_callback_function\n    arg1 arg2 arg3 arg4\n)\n"
    );
}

#[test]
fn test_cmake_language_eval_code_groups_strings_under_code_keyword() {
    let config = config_with_width(60);
    let src = "cmake_language(EVAL CODE \"if (NOT TARGET foo)\" \"  add_library(foo INTERFACE)\" \"endif ()\")\n";
    let out = format(src, config);
    // CODE collects all string fragments under it; trailing-block flat applies
    // because CODE is the sole block in the EVAL mode body, so its values
    // emit at the same indent as CODE rather than one level deeper.
    assert_eq!(
        out,
        "cmake_language(EVAL\n    CODE\n    \"if (NOT TARGET foo)\" \"  add_library(foo INTERFACE)\"\n    \"endif ()\"\n)\n"
    );
}

#[test]
fn test_cmake_language_defer_call_form() {
    let config = config_with_width(60);
    let src = "cmake_language(DEFER DIRECTORY \"${CMAKE_SOURCE_DIR}/sub\" ID my_id CALL my_callback some_arg another_arg)\n";
    let out = format(src, config);
    assert_eq!(
        out,
        "cmake_language(DEFER\n    DIRECTORY \"${CMAKE_SOURCE_DIR}/sub\"\n    ID my_id\n    CALL my_callback some_arg another_arg\n)\n"
    );
}

#[test]
fn test_cmake_language_defer_cancel_call_packs_id_list() {
    let config = config_with_width(60);
    let src = "cmake_language(DEFER CANCEL_CALL id_one id_two id_three id_four)\n";
    let out = format(src, config);
    // The full single-line form is 64 chars and exceeds width 60, so DEFER
    // wraps with CANCEL_CALL on a continuation line at indent 1.
    assert_eq!(
        out,
        "cmake_language(DEFER\n    CANCEL_CALL id_one id_two id_three id_four\n)\n"
    );
}

#[test]
fn test_cmake_language_set_dependency_provider_groups_supported_methods() {
    let config = config_with_width(60);
    let src = "cmake_language(SET_DEPENDENCY_PROVIDER my_provider SUPPORTED_METHODS FIND_PACKAGE FETCHCONTENT_MAKEAVAILABLE_SERIAL)\n";
    let out = format(src, config);
    // The mode keeps the provider name on the header line; SUPPORTED_METHODS
    // chops because its values don't fit on the keyword line at width 60.
    // Trailing-block flat applies (SUPPORTED_METHODS is the sole block in the
    // mode body), so the values land at SUPPORTED_METHODS's own indent.
    assert_eq!(
        out,
        "cmake_language(SET_DEPENDENCY_PROVIDER my_provider\n    SUPPORTED_METHODS\n    FIND_PACKAGE FETCHCONTENT_MAKEAVAILABLE_SERIAL\n)\n"
    );
}

#[test]
fn test_cmake_language_get_message_log_level_one_positional() {
    let config = config_with_width(80);
    let src = "cmake_language(GET_MESSAGE_LOG_LEVEL current_log_level)\n";
    let out = format(src, config);
    assert_eq!(
        out,
        "cmake_language(GET_MESSAGE_LOG_LEVEL current_log_level)\n"
    );
}

#[test]
fn test_cmake_language_get_experimental_feature_enabled_two_positionals() {
    let config = config_with_width(80);
    let src = "cmake_language(GET_EXPERIMENTAL_FEATURE_ENABLED CXX_MODULES is_enabled)\n";
    let out = format(src, config);
    // Both positionals stay grouped under the mode header — the schema's
    // positional max of 2 keeps the feature name and result variable together.
    assert_eq!(
        out,
        "cmake_language(GET_EXPERIMENTAL_FEATURE_ENABLED CXX_MODULES is_enabled)\n"
    );
}

#[test]
fn test_cmake_language_mode_header_chops_positionals_when_overflowing() {
    let config = config_with_width(40);
    let src = "cmake_language(GET_EXPERIMENTAL_FEATURE_ENABLED CXX_MODULES is_enabled_long_var)\n";
    let out = format(src, config);
    // The single-line render overflows width 40 by a wide margin. The mode
    // keyword stays glued to the open paren (best-effort no_break_first), and
    // both positionals chop onto their own lines so they fit at arg_indent.
    assert_eq!(
        out,
        "cmake_language(GET_EXPERIMENTAL_FEATURE_ENABLED\n    CXX_MODULES\n    is_enabled_long_var\n)\n"
    );
}

#[test]
fn test_set_property_keeps_scope_on_opening_line() {
    let config = config_with_width(72);
    let src = "set_property(TARGET \"${TARGET}\" PROPERTY Halide_PYTHON_GENERATOR_SOURCE \"${CMAKE_CURRENT_SOURCE_DIR}/${ARG_SOURCES}\")\n";
    let out = format(src, config);
    // PROPERTY is the last keyword with a prior keyword (TARGET) seen: flat applies.
    assert_eq!(
        out,
        "set_property(TARGET \"${TARGET}\"\n    PROPERTY\n    Halide_PYTHON_GENERATOR_SOURCE\n    \"${CMAKE_CURRENT_SOURCE_DIR}/${ARG_SOURCES}\"\n)\n"
    );
}

#[test]
fn test_set_target_properties_keeps_target_on_opening_line() {
    let config = config_with_width(76);
    let src = "set_target_properties(${TARGET}_pystub PROPERTIES CXX_VISIBILITY_PRESET hidden VISIBILITY_INLINES_HIDDEN ON POSITION_INDEPENDENT_CODE ON)\n";
    let out = format(src, config);
    // PROPERTIES is the trailing keyword, so the flat rule pulls the pairs flush
    // with it. A wrapping pair value would still indent one level deeper.
    assert_eq!(
        out,
        "set_target_properties(${TARGET}_pystub\n    PROPERTIES\n    CXX_VISIBILITY_PRESET hidden\n    VISIBILITY_INLINES_HIDDEN ON\n    POSITION_INDEPENDENT_CODE ON\n)\n"
    );
}

#[test]
fn test_export_builtin_schema_groups_targets_and_file() {
    let config = config_with_width(80);
    let src = "export(TARGETS ${TARGET} NAMESPACE ${ARG_PACKAGE_NAMESPACE} APPEND FILE \"${ARG_EXPORT_FILE}\")\n";
    let out = format(src, config);
    assert_eq!(
        out,
        "export(\n    TARGETS ${TARGET}\n    NAMESPACE ${ARG_PACKAGE_NAMESPACE}\n    APPEND\n    FILE \"${ARG_EXPORT_FILE}\"\n)\n"
    );
}

#[test]
fn test_cmake_path_builtin_schema_groups_mode_keywords() {
    let config = config_with_width(80);
    let src = "cmake_path(ABSOLUTE_PATH ARG_OUTPUT_DIR BASE_DIRECTORY \"${CMAKE_CURRENT_BINARY_DIR}\" NORMALIZE)\n";
    let out = format(src, config);
    assert_eq!(
        out,
        "cmake_path(ABSOLUTE_PATH ARG_OUTPUT_DIR\n    BASE_DIRECTORY \"${CMAKE_CURRENT_BINARY_DIR}\"\n    NORMALIZE\n)\n"
    );
}

#[test]
fn test_target_sources_file_set_builtin_schema_formats_nested_path_lists() {
    let config = config_with_width(80);
    let src = "target_sources(my_lib INTERFACE FILE_SET HEADERS BASE_DIRS include FILES include/a.h include/b.h)\n";
    let out = format(src, config);
    // FILE_SET content fits on one line, so it's inlined. INTERFACE has no values (FILE_SET follows).
    assert_eq!(
        out,
        "target_sources(\n    my_lib\n    INTERFACE\n    FILE_SET HEADERS BASE_DIRS include FILES include/a.h include/b.h\n)\n"
    );
}

#[test]
fn test_add_custom_command_builtin_schema_formats_output_and_command_groups() {
    let config = config_with_width(80);
    let src = "add_custom_command(OUTPUT \"${CMAKE_CURRENT_BINARY_DIR}/generated.cpp\" COMMAND generator --emit generated.cpp DEPENDS generator input.txt VERBATIM)\n";
    let out = format(src, config);
    assert_eq!(
        out,
        "add_custom_command(\n    OUTPUT \"${CMAKE_CURRENT_BINARY_DIR}/generated.cpp\"\n    COMMAND generator --emit generated.cpp\n    DEPENDS generator input.txt\n    VERBATIM\n)\n"
    );
}

#[test]
fn test_add_custom_command_keeps_short_singleton_output_inline() {
    let config = config_with_width(80);
    let src = "add_custom_command(OUTPUT generated.cpp COMMAND generator --emit generated.cpp --target host --output \"${OUT}\" DEPENDS generator VERBATIM)\n";
    let out = format(src, config);
    assert_eq!(
        out,
        "add_custom_command(\n    OUTPUT generated.cpp\n    COMMAND generator --emit generated.cpp --target host --output \"${OUT}\"\n    DEPENDS generator\n    VERBATIM\n)\n"
    );
}

#[test]
fn test_add_custom_target_builtin_schema_keeps_target_name_first() {
    let config = config_with_width(72);
    let src = "add_custom_target(\"${TARGET}.update\" DEPENDS \"${stub_file}\" BYPRODUCTS \"${stamp}\" VERBATIM)\n";
    let out = format(src, config);
    assert_eq!(
        out,
        "add_custom_target(\"${TARGET}.update\"\n    DEPENDS \"${stub_file}\"\n    BYPRODUCTS \"${stamp}\"\n    VERBATIM\n)\n"
    );
}

#[test]
fn test_add_custom_target_builtin_schema_wraps_command_argv() {
    let config = config_with_width(72);
    let src = "add_custom_target(run_codegen COMMAND generator --emit generated.cpp --target host --output \"${OUT}\" COMMAND_EXPAND_LISTS DEPENDS generator input.txt)\n";
    let out = format(src, config);
    assert_eq!(
        out,
        "add_custom_target(run_codegen\n    COMMAND\n        generator --emit generated.cpp --target host --output \"${OUT}\"\n    COMMAND_EXPAND_LISTS\n    DEPENDS\n        generator\n        input.txt\n)\n"
    );
}

#[test]
fn test_foreach_in_items_keeps_selector_with_in_keyword() {
    let config = config_with_width(48);
    let src = "foreach (hdr_name IN ITEMS HalideBuffer.h HalideRuntime.h HalideRuntimeCuda.h)\n";
    let out = format(src, config);
    assert_eq!(
        out,
        "foreach (hdr_name IN ITEMS\n    HalideBuffer.h\n    HalideRuntime.h\n    HalideRuntimeCuda.h\n)\n"
    );
}

#[test]
fn test_foreach_in_zip_lists_keeps_loop_variables_together() {
    let config = config_with_width(42);
    let src = "foreach (out file IN ZIP_LISTS outputs output_files)\n";
    let out = format(src, config);
    assert_eq!(
        out,
        "foreach (out file IN ZIP_LISTS\n    outputs\n    output_files\n)\n"
    );
}

#[test]
fn test_foreach_in_items_compound_header_stays_on_opening_line_when_it_fits() {
    let config = config_with_width(80);
    let src = "foreach (hdr_name IN ITEMS HalideBuffer.h HalideRuntime.h HalideRuntimeCuda.h)\n";
    let out = format(src, config);
    assert_eq!(
        out,
        "foreach (hdr_name IN ITEMS HalideBuffer.h HalideRuntime.h HalideRuntimeCuda.h)\n"
    );
}

#[test]
fn test_install_targets_subparsers_stay_inline_when_they_fit() {
    let config = config_with_width(100);
    let src = "install(TARGETS Halide Halide_Generator Halide_GenGen EXPORT Halide_Targets RUNTIME COMPONENT Halide_Runtime LIBRARY COMPONENT Halide_Runtime NAMELINK_COMPONENT Halide_Development ARCHIVE COMPONENT Halide_Development FILE_SET HEADERS COMPONENT Halide_Development)\n";
    let out = format(src, config);
    assert_eq!(
        out,
        "install(\n    TARGETS Halide Halide_Generator Halide_GenGen\n    EXPORT Halide_Targets\n    RUNTIME COMPONENT Halide_Runtime\n    LIBRARY COMPONENT Halide_Runtime NAMELINK_COMPONENT Halide_Development\n    ARCHIVE COMPONENT Halide_Development\n    FILE_SET HEADERS COMPONENT Halide_Development\n)\n"
    );
}

#[test]
fn test_subparser_trailing_comment_no_inline() {
    let config = config_with_width(120);
    let src = "install(\n    TARGETS Halide\n    LIBRARY\n        COMPONENT Halide_Runtime # comment\n        NAMELINK_COMPONENT Halide_Development\n)\n";
    let out = format(src, config);
    assert_eq!(
        out,
        "install(\n    TARGETS Halide\n    LIBRARY\n    COMPONENT Halide_Runtime  # comment\n    NAMELINK_COMPONENT Halide_Development\n)\n"
    );
}

// Empty-trailing-`#` as a manual line-break hint.
//
// The formatter has no notion of "this set() call holds keyword args for some
// other function" — it just sees positional values after the variable name.
// The default behavior packs by width, breaking apart KEYWORD value pairs.
//
// Rather than add global inference, we lean on a clang-format-style escape
// hatch: a bare `#` at end of a source line is a no-op comment that the
// formatter treats as a break point, preserving the line below.
//
// These tests pin both the "no hint" baseline (so we notice if a smarter
// pass starts breaking these) and the "with hints" workaround output, plus
// idempotence on the hinted form.

fn format_twice(src: &str, config: &Config) -> (String, String) {
    let once = format(src, config.clone());
    let twice = format(&once, config.clone());
    (once, twice)
}

#[test]
fn test_set_with_keyword_args_packs_densely_without_break_hints() {
    let src = "\
set(generator_args
    COMMAND ${generator_cmd}
    DEPENDS ${generator_cmd_deps} ${ARG_DEPENDS}
    EXTRA_OUTPUTS ${extra_outputs}
    OUTPUT_DIR \"${ARG_OUTPUT_DIR}\"
    PARAMS ${ARG_PARAMS}
)
";
    let out = format(src, default_config());
    // Baseline: set() has no kwarg schema (CMake itself doesn't either —
    // generator_args here is just a list value), so the formatter packs
    // tokens by width and the KEYWORD value pairs run together.
    assert_eq!(
        out,
        "\
set(generator_args
    COMMAND ${generator_cmd} DEPENDS ${generator_cmd_deps} ${ARG_DEPENDS} EXTRA_OUTPUTS
    ${extra_outputs} OUTPUT_DIR \"${ARG_OUTPUT_DIR}\" PARAMS ${ARG_PARAMS}
)
"
    );
}

#[test]
fn test_set_with_trailing_hash_break_hints_preserves_line_per_kwarg() {
    let src = "\
set(generator_args
    COMMAND ${generator_cmd}                              #
    DEPENDS ${generator_cmd_deps} ${ARG_DEPENDS}          #
    EXTRA_OUTPUTS ${extra_outputs}                        #
    OUTPUT_DIR \"${ARG_OUTPUT_DIR}\"                        #
    PARAMS ${ARG_PARAMS}                                  #
)
";
    let (once, twice) = format_twice(src, &default_config());
    // Each trailing `#` flushes the current value group and forces a line
    // break. The `#` itself renders as `  #` after the value.
    assert_eq!(
        once,
        "\
set(generator_args
    COMMAND ${generator_cmd}  #
    DEPENDS ${generator_cmd_deps} ${ARG_DEPENDS}  #
    EXTRA_OUTPUTS ${extra_outputs}  #
    OUTPUT_DIR \"${ARG_OUTPUT_DIR}\"  #
    PARAMS ${ARG_PARAMS}  #
)
"
    );
    assert_eq!(once, twice, "trailing-# hinted output must be idempotent");
}

#[test]
fn test_string_concat_preserves_one_fragment_per_source_line() {
    let src = "\
string(CONCAT stub_text
    \"#include <Python.h>\\n\"
    \"#include \\\"Halide.h\\\"\\n\"
    \"HALIDE_GENERATOR_PYSTUB(${GEN_NAME}, ${MODULE_NAME})\\n\"
)
";
    let out = format(src, default_config());
    // The three fragments each start on their own source line and there are
    // at least 3 of them, so the source-vertical heuristic preserves
    // one-per-line layout — no trailing-# hint needed.
    assert_eq!(
        out,
        "\
string(CONCAT stub_text
    \"#include <Python.h>\\n\"
    \"#include \\\"Halide.h\\\"\\n\"
    \"HALIDE_GENERATOR_PYSTUB(${GEN_NAME}, ${MODULE_NAME})\\n\"
)
"
    );
}

#[test]
fn test_string_concat_with_trailing_hash_keeps_one_fragment_per_line() {
    let src = "\
string(CONCAT stub_text
    \"#include <Python.h>\\n\"                                                       #
    \"#include \\\"Halide.h\\\"\\n\"                                                     #
    \"HALIDE_GENERATOR_PYSTUB(${GEN_NAME}, ${MODULE_NAME})\\n\"                       #
)
";
    let (once, twice) = format_twice(src, &default_config());
    assert_eq!(
        once,
        "\
string(CONCAT stub_text
    \"#include <Python.h>\\n\"  #
    \"#include \\\"Halide.h\\\"\\n\"  #
    \"HALIDE_GENERATOR_PYSTUB(${GEN_NAME}, ${MODULE_NAME})\\n\"  #
)
"
    );
    assert_eq!(once, twice, "trailing-# hinted output must be idempotent");
}

#[test]
fn test_cmake_parse_arguments_parse_argv_keeps_prefix_and_options_inline() {
    let src = "\
cmake_parse_arguments(PARSE_ARGV 1 ARG \"\"
    \"FILE_BASE_NAME;FUNCTION_NAME;GENERATOR;GRADIENT_DESCENT;OUTPUT_DIR;TYPE;USE_RUNTIME\"
    \"COMMAND;DEPENDS;EXTRA_OUTPUTS;PARAMS;PLUGINS;TARGETS\")
";
    let (once, twice) = format_twice(src, &default_config());
    // PARSE_ARGV mode glues N, prefix, options onto the mode line; the two
    // long keyword-list strings pack onto continuation lines below. Without
    // a schema the generic ALL-CAPS heuristic would treat `ARG` as a keyword
    // and push the lists to an extra-indented sub-block.
    assert_eq!(
        once,
        "\
cmake_parse_arguments(
    PARSE_ARGV 1 ARG \"\"
    \"FILE_BASE_NAME;FUNCTION_NAME;GENERATOR;GRADIENT_DESCENT;OUTPUT_DIR;TYPE;USE_RUNTIME\"
    \"COMMAND;DEPENDS;EXTRA_OUTPUTS;PARAMS;PLUGINS;TARGETS\"
)
"
    );
    assert_eq!(once, twice);
}

#[test]
fn test_cmake_parse_arguments_short_call_stays_on_one_line() {
    let src = "cmake_parse_arguments(PARSE_ARGV 1 ARG \"\" \"OUT\" \"ARGS\")\n";
    let out = format(src, default_config());
    assert_eq!(
        out,
        "cmake_parse_arguments(PARSE_ARGV 1 ARG \"\" \"OUT\" \"ARGS\")\n"
    );
}

#[test]
fn test_cmake_parse_arguments_form_one_no_mode_keyword() {
    let src = "cmake_parse_arguments(ARG \"\" \"FILE_BASE_NAME\" \"COMMAND;DEPENDS\" ${ARGN})\n";
    let out = format(src, default_config());
    // Form 1 (no PARSE_ARGV): prefix + 3 lists + ${ARGN} are all positional.
    assert_eq!(
        out,
        "cmake_parse_arguments(ARG \"\" \"FILE_BASE_NAME\" \"COMMAND;DEPENDS\" ${ARGN})\n"
    );
}

#[test]
fn test_generic_keyword_heuristic_skips_boolean_literals() {
    // unknown_macro has no schema, so its arguments go through the generic
    // keyword heuristic. ALL-CAPS boolean literals (YES, OFF, TRUE, FALSE)
    // are values, not logical-group breakpoints; they must not split the
    // argument list. NO and ON are excluded by the >= 3 char length rule.
    let src = "unknown_macro(target ENABLE_FOO TRUE ENABLE_BAR FALSE FLAG YES OTHER OFF)\n";
    let out = format(src, default_config());
    assert_eq!(
        out,
        "unknown_macro(target ENABLE_FOO TRUE ENABLE_BAR FALSE FLAG YES OTHER OFF)\n"
    );
}

#[test]
fn test_generic_keyword_heuristic_still_breaks_on_long_uppercase_keywords() {
    // URL, CXX_STANDARD, etc. should still be recognized as keyword breaks
    // for unknown calls — the boolean-literal exception is a closed set.
    let src = "unknown_call(target URL https://example.com/foo.tar.gz HASH_SHA256 abc CXX_STANDARD 17 other_value second_value third_value)\n";
    let out = format(src, default_config());
    assert_eq!(
        out,
        "unknown_call(\n    target\n    URL https://example.com/foo.tar.gz\n    HASH_SHA256 abc\n    CXX_STANDARD 17 other_value second_value third_value\n)\n"
    );
}

#[test]
fn test_head_trailing_comment_stays_on_head_line() {
    // A comment on the same source line as `(` and before any arg must be
    // emitted as a trailing comment on the `name(` line so line-based lint
    // suppressions (e.g. `# nolint`) survive multi-lining.
    let src = "define_property(  # nolint\n    TARGET PROPERTY Halide_RT_TARGETS\n    BRIEF_DOCS \"Lists the targets the runtime backs\"\n    FULL_DOCS \"Lists the targets the runtime backs\"\n)\n";
    let out = format(src, default_config());
    assert!(
        out.lines().next().unwrap().ends_with("# nolint"),
        "head line should retain trailing comment; got:\n{}",
        out
    );
    assert!(
        out.starts_with("define_property(  # nolint\n"),
        "expected head line `define_property(  # nolint`; got:\n{}",
        out
    );
}

#[test]
fn test_leading_comment_between_paren_and_first_arg_stays_on_its_own_line() {
    // A comment on a line BY ITSELF (newline between `(` and `#`) is not a
    // head-trailing comment; it stays as a standalone line before arg 0.
    let src = "define_property(\n    # leading note\n    TARGET PROPERTY X\n    BRIEF_DOCS \"a\"\n    FULL_DOCS \"a\"\n)\n";
    let out = format(src, default_config());
    assert!(
        out.starts_with("define_property(\n    # leading note\n"),
        "expected standalone leading comment; got:\n{}",
        out
    );
}

#[test]
fn test_unquoted_quoted_concatenation_stays_one_argument() {
    // In CMake, an unquoted token directly followed by a quoted token with no
    // whitespace between them is ONE argument (e.g. -DKEY="value"). Splitting
    // it across two tokens turns one `-DKEY="value"` into `-DKEY= "value"`,
    // which downstream tools see as `-DKEY=` plus a stray positional.
    let config = config_with_schema("vcpkg_cmake_configure", vec!["SOURCE_PATH", "OPTIONS"]);
    let src = "vcpkg_cmake_configure(\n    SOURCE_PATH \"${SOURCE_PATH}/tensorflow/lite\"\n    OPTIONS\n    -DTENSORFLOW_SOURCE_DIR=\"${SOURCE_PATH}\"\n    -DBUILD_SHARED_LIBS=OFF\n)\n";
    let out = format(src, config);
    assert!(
        out.contains("-DTENSORFLOW_SOURCE_DIR=\"${SOURCE_PATH}\""),
        "expected `-DKEY=\"${{VAL}}\"` to remain one token; got:\n{}",
        out
    );
    assert!(
        !out.contains("-DTENSORFLOW_SOURCE_DIR= "),
        "must not insert whitespace at `=` between unquoted and quoted segments; got:\n{}",
        out
    );
}

#[test]
fn test_bracketed_keyword_token_stays_one_argument() {
    // Halide's add_halide_library registers literal `FEATURES[<triple>]`
    // strings as keywords for cmake_parse_arguments — splitting them on
    // whitespace silently breaks per-architecture override dispatch.
    let config = config_with_schema("add_halide_library", vec!["FROM"]);
    let src = "add_halide_library(\n    foo\n    FROM foo.generator\n    FEATURES[x86-64-osx] avx2 sse41\n    FEATURES[arm-64-osx] arm_dot_prod\n)\n";
    let out = format(src, config);
    assert!(
        out.contains("FEATURES[x86-64-osx]"),
        "expected `FEATURES[x86-64-osx]` to remain one token; got:\n{}",
        out
    );
    assert!(
        out.contains("FEATURES[arm-64-osx]"),
        "expected `FEATURES[arm-64-osx]` to remain one token; got:\n{}",
        out
    );
    assert!(
        !out.contains("FEATURES ["),
        "must not insert whitespace before `[`; got:\n{}",
        out
    );
    assert!(
        !out.contains("[ x86"),
        "must not insert whitespace inside `[...]`; got:\n{}",
        out
    );
}

#[test]
fn test_mode_header_trailing_comment_after_positional() {
    // `file(GLOB var  # nolint` should render on one line so line-based lint
    // suppressions stay on the same line as `file(GLOB`.
    let src =
        "file(GLOB tools_versions  # nolint\n    RELATIVE \"${ROOT}\"\n    \"${ROOT}/*\"\n)\n";
    let out = format(src, default_config());
    assert!(
        out.starts_with("file(GLOB tools_versions  # nolint\n"),
        "expected mode keyword, positional, and trailing comment on one line; got:\n{}",
        out
    );
}

#[test]
fn test_mode_header_trailing_comment_between_mode_and_positional() {
    // A trailing comment that sits between the mode keyword and its positional
    // in the source still ends up on the mode-header line. Same rendered shape
    // as if the comment had been written after the positional.
    let src =
        "file(GLOB  # nolint\n    tools_versions\n    RELATIVE \"${ROOT}\"\n    \"${ROOT}/*\"\n)\n";
    let out = format(src, default_config());
    assert!(
        out.starts_with("file(GLOB tools_versions  # nolint\n"),
        "expected comment between mode and positional to render on the header line; got:\n{}",
        out
    );
}

#[test]
fn test_multiple_comments_order() {
    let src = "set(FOO#[[bracket comment]]#[[bracket comment]]#[[bracket comment]]# another comment with space\n)\n";
    let out = format(src, default_config());
    assert_eq!(
        out,
        "set(\n    # another comment with space\n    FOO  #[[bracket comment]]  #[[bracket comment]]  #[[bracket comment]]\n)\n"
    );
}

#[test]
fn test_subparser_with_quoted_hash_inlines() {
    let config = config_with_width(80);
    let src = "install(\n    TARGETS Halide Halide_Generator Halide_GenGen Halide_Runtime\n    LIBRARY COMPONENT \"my # component\"\n)\n";
    let out = format(src, config);
    assert_eq!(
        out,
        "install(\n    TARGETS Halide Halide_Generator Halide_GenGen Halide_Runtime\n    LIBRARY COMPONENT \"my # component\"\n)\n"
    );
}

#[test]
fn test_subparser_with_bracket_argument_hash() {
    let config = config_with_width(80);
    let src = "install(CODE [[message(STATUS \"foo\")  # print]])\n";
    let out = format(src, config);
    assert_eq!(out, "install(CODE [[message(STATUS \"foo\")  # print]])\n");
}

#[test]
fn test_set_target_properties_with_hash_in_target_name() {
    let config = config_with_width(90);
    let src =
        "set_target_properties(\"[[my  # target]]\" PROPERTIES CXX_VISIBILITY_PRESET hidden)\n";
    let out = format(src, config);
    assert_eq!(
        out,
        "set_target_properties(\"[[my  # target]]\" PROPERTIES CXX_VISIBILITY_PRESET hidden)\n"
    );
}

#[test]
fn test_cmake_language_eval_with_bracket_argument_hash() {
    let config = config_with_width(50);
    let src = "cmake_language(EVAL CODE [[message(STATUS \"foo\")  # print]])\n";
    let out = format(src, config);
    assert_eq!(
        out,
        "cmake_language(EVAL\n    CODE [[message(STATUS \"foo\")  # print]]\n)\n"
    );
}

#[test]
fn test_set_with_hash_in_bracket_argument_does_not_wrap_paren() {
    let config = config_with_width(30);
    let src = "set([[val  # h]])\n";
    let out = format(src, config);
    assert_eq!(out, "set([[val  # h]])\n");
}

#[test]
fn test_set_with_hash_in_bracket_argument_forces_paren_wrap_on_overflow() {
    let config = config_with_width(30);
    let src = "set([[a_very_long_var_with_hash_val  # h]])\n";
    let out = format(src, config);
    assert_eq!(out, "set([[a_very_long_var_with_hash_val  # h]]\n)\n");
}
