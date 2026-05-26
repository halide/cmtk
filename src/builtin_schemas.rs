use crate::schema::{
    CompoundListKeyword, FunctionSchema, ListType, PositionalSpec, SchemaRegistry,
};
use std::collections::HashMap;

// install(EXPORT <name> NAMESPACE <ns> [FILE <file>.cmake] [DESTINATION <dir>]
//         [PERMISSIONS ...] [CONFIGURATIONS ...] [COMPONENT <component>]
//         [EXCLUDE_FROM_ALL] [EXPORT_LINK_INTERFACE_LIBRARIES])
fn install_export_schema() -> FunctionSchema {
    FunctionSchema {
        positional: Some(PositionalSpec { min: 1, max: 1 }),
        options: vec![
            "EXCLUDE_FROM_ALL".into(),
            "EXPORT_LINK_INTERFACE_LIBRARIES".into(),
        ],
        one_value_keywords: vec![
            "NAMESPACE".into(),
            "FILE".into(),
            "DESTINATION".into(),
            "COMPONENT".into(),
            "CXX_MODULES_DIRECTORY".into(),
        ],
        multi_value_keywords: vec!["PERMISSIONS".into(), "CONFIGURATIONS".into()],
        ..Default::default()
    }
}

// Keywords supported per-type inside install(TARGETS ...). Most per-type
// subparsers (RUNTIME, LIBRARY, ARCHIVE, FRAMEWORK, BUNDLE, OBJECTS,
// PUBLIC_HEADER, PRIVATE_HEADER, RESOURCE) accept the same set; LIBRARY also
// accepts NAMELINK_COMPONENT/NAMELINK_ONLY/NAMELINK_SKIP.
fn install_target_kind_schema(namelink: bool) -> FunctionSchema {
    let mut options: Vec<String> = vec!["OPTIONAL".into(), "EXCLUDE_FROM_ALL".into()];
    let mut one_value_keywords: Vec<String> =
        vec!["DESTINATION".into(), "COMPONENT".into(), "RENAME".into()];
    if namelink {
        options.push("NAMELINK_ONLY".into());
        options.push("NAMELINK_SKIP".into());
        one_value_keywords.push("NAMELINK_COMPONENT".into());
    }
    FunctionSchema {
        options,
        one_value_keywords,
        multi_value_keywords: vec!["PERMISSIONS".into(), "CONFIGURATIONS".into()],
        ..Default::default()
    }
}

fn install_targets_subparsers() -> HashMap<String, FunctionSchema> {
    let pattern_or_regex = FunctionSchema {
        positional: Some(PositionalSpec { min: 1, max: 1 }),
        options: vec!["EXCLUDE".into()],
        multi_value_keywords: vec!["PERMISSIONS".into()],
        ..Default::default()
    };
    let file_set = FunctionSchema {
        positional: Some(PositionalSpec { min: 1, max: 1 }),
        one_value_keywords: vec!["COMPONENT".into(), "DESTINATION".into()],
        multi_value_keywords: vec!["BASE_DIRS".into(), "FILES".into()],
        list_keyword_types: [
            ("BASE_DIRS".into(), ListType::Path),
            ("FILES".into(), ListType::Path),
        ]
        .into_iter()
        .collect(),
        ..Default::default()
    };
    let mut subparsers: HashMap<String, FunctionSchema> = HashMap::new();
    subparsers.insert("PATTERN".into(), pattern_or_regex.clone());
    subparsers.insert("REGEX".into(), pattern_or_regex);
    subparsers.insert("FILE_SET".into(), file_set);
    for kind in &[
        "RUNTIME",
        "ARCHIVE",
        "FRAMEWORK",
        "BUNDLE",
        "OBJECTS",
        "PUBLIC_HEADER",
        "PRIVATE_HEADER",
        "RESOURCE",
    ] {
        subparsers.insert(kind.to_string(), install_target_kind_schema(false));
    }
    subparsers.insert("LIBRARY".into(), install_target_kind_schema(true));
    subparsers.insert("EXPORT".into(), install_export_schema());
    subparsers
}

pub fn builtin_schemas() -> SchemaRegistry {
    let mut registry = crate::generated_schemas::generated_schemas();
    let mut functions = HashMap::new();

    functions.insert(
        "if".to_string(),
        FunctionSchema {
            default_list_type: ListType::Condition,
            ..Default::default()
        },
    );

    functions.insert(
        "elseif".to_string(),
        FunctionSchema {
            default_list_type: ListType::Condition,
            ..Default::default()
        },
    );

    functions.insert(
        "while".to_string(),
        FunctionSchema {
            default_list_type: ListType::Condition,
            ..Default::default()
        },
    );

    functions.insert(
        "project".to_string(),
        FunctionSchema {
            positional: Some(PositionalSpec { min: 1, max: 1 }),
            simple_keywords: vec![],
            one_value_keywords: vec![
                "VERSION".into(),
                "DESCRIPTION".into(),
                "HOMEPAGE_URL".into(),
            ],
            multi_value_keywords: vec!["LANGUAGES".into()],
            ..Default::default()
        },
    );

    functions.insert(
        "cmake_minimum_required".to_string(),
        FunctionSchema {
            simple_keywords: vec![],
            options: vec!["FATAL_ERROR".into()],
            one_value_keywords: vec!["VERSION".into()],
            ..Default::default()
        },
    );

    functions.insert(
        "fetchcontent_declare".to_string(),
        FunctionSchema {
            simple_keywords: vec![],
            options: vec!["OVERRIDE_FIND_PACKAGE".into(), "SYSTEM".into()],
            one_value_keywords: vec![
                "GIT_REPOSITORY".into(),
                "GIT_TAG".into(),
                "GIT_SUBMODULES_RECURSE".into(),
                "URL".into(),
                "URL_MD5".into(),
                "URL_HASH".into(),
                "SOURCE_DIR".into(),
                "BINARY_DIR".into(),
                "DOWNLOAD_EXTRACT_TIMESTAMP".into(),
            ],
            multi_value_keywords: vec!["GIT_SUBMODULES".into()],
            ..Default::default()
        },
    );

    functions.insert(
        "externalproject_add".to_string(),
        FunctionSchema {
            simple_keywords: vec![],
            options: vec![],
            one_value_keywords: vec![
                "GIT_REPOSITORY".into(),
                "GIT_TAG".into(),
                "SOURCE_DIR".into(),
                "BINARY_DIR".into(),
                "INSTALL_DIR".into(),
                "PREFIX".into(),
                "DOWNLOAD_DIR".into(),
                "STAMP_DIR".into(),
                "LOG_DIR".into(),
                "TMP_DIR".into(),
                "WORKING_DIRECTORY".into(),
                "COMMENT".into(),
                "CONFIGURE_COMMAND".into(),
                "BUILD_COMMAND".into(),
                "INSTALL_COMMAND".into(),
                "TEST_COMMAND".into(),
                "UPDATE_COMMAND".into(),
                "CONFIGURE_HANDLED_BY_BUILD".into(),
            ],
            multi_value_keywords: vec![
                "CMAKE_ARGS".into(),
                "CMAKE_CACHE_ARGS".into(),
                "DEPENDS".into(),
                "STEP_TARGETS".into(),
            ],
            ..Default::default()
        },
    );

    functions.insert(
        "define_property".to_string(),
        FunctionSchema {
            simple_keywords: vec![],
            options: vec![
                "GLOBAL".into(),
                "DIRECTORY".into(),
                "TARGET".into(),
                "SOURCE".into(),
                "TEST".into(),
                "VARIABLE".into(),
                "CACHED_VARIABLE".into(),
                "INHERITED".into(),
            ],
            one_value_keywords: vec!["PROPERTY".into(), "BRIEF_DOCS".into(), "FULL_DOCS".into()],
            ..Default::default()
        },
    );

    functions.insert(
        "add_custom_target".to_string(),
        FunctionSchema {
            positional: Some(PositionalSpec { min: 1, max: 1 }),
            no_break_first_argument: true,
            simple_keywords: vec![],
            options: vec![
                "ALL".into(),
                "VERBATIM".into(),
                "USES_TERMINAL".into(),
                "COMMAND_EXPAND_LISTS".into(),
            ],
            one_value_keywords: vec![
                "COMMENT".into(),
                "WORKING_DIRECTORY".into(),
                "JOB_POOL".into(),
                "JOB_SERVER_AWARE".into(),
            ],
            multi_value_keywords: vec![
                "COMMAND".into(),
                "DEPENDS".into(),
                "SOURCES".into(),
                "BYPRODUCTS".into(),
            ],
            list_keyword_types: [
                ("COMMAND".into(), ListType::CommandArgv),
                ("DEPENDS".into(), ListType::Path),
                ("SOURCES".into(), ListType::Path),
                ("BYPRODUCTS".into(), ListType::Path),
            ]
            .into_iter()
            .collect(),
            ..Default::default()
        },
    );

    functions.insert(
        "add_custom_command".to_string(),
        FunctionSchema {
            simple_keywords: vec![],
            options: vec![
                "PRE_BUILD".into(),
                "PRE_LINK".into(),
                "POST_BUILD".into(),
                "VERBATIM".into(),
                "USES_TERMINAL".into(),
                "COMMAND_EXPAND_LISTS".into(),
                "APPEND".into(),
            ],
            one_value_keywords: vec![
                "TARGET".into(),
                "WORKING_DIRECTORY".into(),
                "COMMENT".into(),
                "MAIN_DEPENDENCY".into(),
                "DEPFILE".into(),
            ],
            multi_value_keywords: vec![
                "OUTPUT".into(),
                "COMMAND".into(),
                "DEPENDS".into(),
                "BYPRODUCTS".into(),
                "IMPLICIT_DEPENDS".into(),
            ],
            list_keyword_types: [
                ("OUTPUT".into(), ListType::Path),
                ("COMMAND".into(), ListType::CommandArgv),
                ("BYPRODUCTS".into(), ListType::Path),
            ]
            .into_iter()
            .collect(),
            ..Default::default()
        },
    );

    functions.insert(
        "foreach".to_string(),
        FunctionSchema {
            no_break_first_argument: true,
            compound_list_keywords: vec![CompoundListKeyword {
                name: "IN".into(),
                headers: vec![
                    vec!["ITEMS".into()],
                    vec!["LISTS".into()],
                    vec!["ZIP_LISTS".into()],
                ],
                list_type: ListType::one_per_line(),
            }],
            ..Default::default()
        },
    );

    functions.insert(
        "set_property".to_string(),
        FunctionSchema {
            no_break_first_argument: true,
            simple_keywords: vec![],
            options: vec!["APPEND".into(), "APPEND_STRING".into()],
            one_value_keywords: vec![
                "CACHE".into(),
                "GLOBAL".into(),
                "DIRECTORY".into(),
                "TARGET".into(),
                "SOURCE".into(),
                "INSTALL".into(),
                "TEST".into(),
                "VARIABLE".into(),
            ],
            multi_value_keywords: vec!["PROPERTY".into()],
            ..Default::default()
        },
    );

    functions.insert(
        "get_property".to_string(),
        FunctionSchema {
            simple_keywords: vec![],
            options: vec![
                "SET".into(),
                "DEFINED".into(),
                "BRIEF_DOCS".into(),
                "FULL_DOCS".into(),
            ],
            one_value_keywords: vec![
                "GLOBAL".into(),
                "DIRECTORY".into(),
                "TARGET".into(),
                "SOURCE".into(),
                "INSTALL".into(),
                "TEST".into(),
                "VARIABLE".into(),
                "CACHE".into(),
                "PROPERTY".into(),
            ],
            ..Default::default()
        },
    );

    functions.insert(
        "target_include_directories".to_string(),
        FunctionSchema {
            positional: Some(PositionalSpec { min: 1, max: 1 }),
            simple_keywords: vec!["PUBLIC".into(), "PRIVATE".into(), "INTERFACE".into()],
            options: vec!["SYSTEM".into(), "BEFORE".into(), "AFTER".into()],
            multi_value_keywords: vec!["PUBLIC".into(), "PRIVATE".into(), "INTERFACE".into()],
            list_keyword_types: [
                ("PUBLIC".into(), ListType::Path),
                ("PRIVATE".into(), ListType::Path),
                ("INTERFACE".into(), ListType::Path),
            ]
            .into_iter()
            .collect(),
            ..Default::default()
        },
    );

    functions.insert(
        "target_compile_options".to_string(),
        FunctionSchema {
            positional: Some(PositionalSpec { min: 1, max: 1 }),
            simple_keywords: vec!["PUBLIC".into(), "PRIVATE".into(), "INTERFACE".into()],
            options: vec!["BEFORE".into()],
            multi_value_keywords: vec!["PUBLIC".into(), "PRIVATE".into(), "INTERFACE".into()],
            // Values are independent compile-flag strings, not a program+args
            // invocation, so the default Packed layout (with source-vertical
            // preservation) fits — CommandArgv would wrongly group them as a
            // single command line.
            ..Default::default()
        },
    );

    functions.insert(
        "target_compile_definitions".to_string(),
        FunctionSchema {
            positional: Some(PositionalSpec { min: 1, max: 1 }),
            simple_keywords: vec!["PUBLIC".into(), "PRIVATE".into(), "INTERFACE".into()],
            multi_value_keywords: vec!["PUBLIC".into(), "PRIVATE".into(), "INTERFACE".into()],
            ..Default::default()
        },
    );

    functions.insert(
        "target_compile_features".to_string(),
        FunctionSchema {
            positional: Some(PositionalSpec { min: 1, max: 1 }),
            simple_keywords: vec!["PUBLIC".into(), "PRIVATE".into(), "INTERFACE".into()],
            multi_value_keywords: vec!["PUBLIC".into(), "PRIVATE".into(), "INTERFACE".into()],
            ..Default::default()
        },
    );

    functions.insert(
        "target_link_options".to_string(),
        FunctionSchema {
            positional: Some(PositionalSpec { min: 1, max: 1 }),
            simple_keywords: vec!["PUBLIC".into(), "PRIVATE".into(), "INTERFACE".into()],
            options: vec!["BEFORE".into()],
            multi_value_keywords: vec!["PUBLIC".into(), "PRIVATE".into(), "INTERFACE".into()],
            // Values are independent linker-option strings, not a program+args
            // invocation, so the default Packed layout (with source-vertical
            // preservation) fits — CommandArgv would wrongly group them as a
            // single command line.
            ..Default::default()
        },
    );

    functions.insert(
        "target_link_directories".to_string(),
        FunctionSchema {
            positional: Some(PositionalSpec { min: 1, max: 1 }),
            simple_keywords: vec!["PUBLIC".into(), "PRIVATE".into(), "INTERFACE".into()],
            options: vec!["BEFORE".into()],
            multi_value_keywords: vec!["PUBLIC".into(), "PRIVATE".into(), "INTERFACE".into()],
            list_keyword_types: [
                ("PUBLIC".into(), ListType::Path),
                ("PRIVATE".into(), ListType::Path),
                ("INTERFACE".into(), ListType::Path),
            ]
            .into_iter()
            .collect(),
            ..Default::default()
        },
    );

    functions.insert(
        "target_precompile_headers".to_string(),
        FunctionSchema {
            positional: Some(PositionalSpec { min: 1, max: 1 }),
            simple_keywords: vec!["PUBLIC".into(), "PRIVATE".into(), "INTERFACE".into()],
            one_value_keywords: vec!["REUSE_FROM".into()],
            multi_value_keywords: vec!["PUBLIC".into(), "PRIVATE".into(), "INTERFACE".into()],
            list_keyword_types: [
                ("PUBLIC".into(), ListType::Path),
                ("PRIVATE".into(), ListType::Path),
                ("INTERFACE".into(), ListType::Path),
            ]
            .into_iter()
            .collect(),
            ..Default::default()
        },
    );

    functions.insert(
        "target_sources".to_string(),
        FunctionSchema {
            positional: Some(PositionalSpec { min: 1, max: 1 }),
            simple_keywords: vec!["PUBLIC".into(), "PRIVATE".into(), "INTERFACE".into()],
            multi_value_keywords: vec!["PUBLIC".into(), "PRIVATE".into(), "INTERFACE".into()],
            list_keyword_types: [
                ("PUBLIC".into(), ListType::Path),
                ("PRIVATE".into(), ListType::Path),
                ("INTERFACE".into(), ListType::Path),
            ]
            .into_iter()
            .collect(),
            subparsers: [(
                "FILE_SET".into(),
                FunctionSchema {
                    positional: Some(PositionalSpec { min: 1, max: 1 }),
                    one_value_keywords: vec!["TYPE".into(), "COMPONENT".into()],
                    multi_value_keywords: vec!["BASE_DIRS".into(), "FILES".into()],
                    list_keyword_types: [
                        ("BASE_DIRS".into(), ListType::Path),
                        ("FILES".into(), ListType::Path),
                    ]
                    .into_iter()
                    .collect(),
                    ..Default::default()
                },
            )]
            .into_iter()
            .collect(),
            ..Default::default()
        },
    );

    functions.insert(
        "add_library".to_string(),
        FunctionSchema {
            simple_keywords: vec![],
            options: vec![
                "STATIC".into(),
                "SHARED".into(),
                "MODULE".into(),
                "OBJECT".into(),
                "INTERFACE".into(),
                "IMPORTED".into(),
                "ALIAS".into(),
                "EXCLUDE_FROM_ALL".into(),
            ],
            ..Default::default()
        },
    );

    functions.insert(
        "add_executable".to_string(),
        FunctionSchema {
            simple_keywords: vec![],
            options: vec![
                "WIN32".into(),
                "MACOSX_BUNDLE".into(),
                "EXCLUDE_FROM_ALL".into(),
                "IMPORTED".into(),
                "ALIAS".into(),
            ],
            ..Default::default()
        },
    );

    functions.insert(
        "install".to_string(),
        FunctionSchema {
            simple_keywords: vec![],
            options: vec![
                "OPTIONAL".into(),
                "NAMELINK_ONLY".into(),
                "NAMELINK_SKIP".into(),
                "EXCLUDE_FROM_ALL".into(),
            ],
            one_value_keywords: vec![
                "DESTINATION".into(),
                "RENAME".into(),
                "COMPONENT".into(),
                "TYPE".into(),
            ],
            multi_value_keywords: vec![
                "TARGETS".into(),
                "FILES".into(),
                "PROGRAMS".into(),
                "DIRECTORY".into(),
                "CONFIGURATIONS".into(),
                "PERMISSIONS".into(),
                "FILE_PERMISSIONS".into(),
                "DIRECTORY_PERMISSIONS".into(),
            ],
            list_keyword_types: [
                ("FILES".into(), ListType::Path),
                ("PROGRAMS".into(), ListType::Path),
                ("DIRECTORY".into(), ListType::Path),
            ]
            .into_iter()
            .collect(),
            subparsers: install_targets_subparsers(),
            ..Default::default()
        },
    );

    functions.insert(
        "set_target_properties".to_string(),
        FunctionSchema {
            positional: Some(PositionalSpec { min: 1, max: 0 }),
            no_break_first_argument: true,
            simple_keywords: vec![],
            multi_value_keywords: vec!["PROPERTIES".into()],
            list_keyword_types: [("PROPERTIES".into(), ListType::NPerLine { n: 2 })]
                .into_iter()
                .collect(),
            ..Default::default()
        },
    );

    functions.insert(
        "set_source_files_properties".to_string(),
        FunctionSchema {
            positional: Some(PositionalSpec { min: 1, max: 0 }),
            simple_keywords: vec![],
            one_value_keywords: vec!["DIRECTORY".into(), "TARGET_DIRECTORY".into()],
            multi_value_keywords: vec!["PROPERTIES".into()],
            list_keyword_types: [("PROPERTIES".into(), ListType::NPerLine { n: 2 })]
                .into_iter()
                .collect(),
            ..Default::default()
        },
    );

    functions.insert(
        "set_directory_properties".to_string(),
        FunctionSchema {
            multi_value_keywords: vec!["PROPERTIES".into()],
            list_keyword_types: [("PROPERTIES".into(), ListType::NPerLine { n: 2 })]
                .into_iter()
                .collect(),
            ..Default::default()
        },
    );

    functions.insert(
        "set_tests_properties".to_string(),
        FunctionSchema {
            positional: Some(PositionalSpec { min: 1, max: 0 }),
            simple_keywords: vec![],
            one_value_keywords: vec!["DIRECTORY".into()],
            multi_value_keywords: vec!["PROPERTIES".into()],
            list_keyword_types: [("PROPERTIES".into(), ListType::NPerLine { n: 2 })]
                .into_iter()
                .collect(),
            ..Default::default()
        },
    );

    // add_test(NAME ... COMMAND ... [CONFIGURATIONS ...] [WORKING_DIRECTORY ...]
    //          [COMMAND_EXPAND_LISTS] [VERBATIM])
    // VERBATIM is documented for add_test (and add_custom_command/target). Without
    // listing it as an option here, it falls inside the CommandArgv group for
    // COMMAND and renders indented as if it were a shell arg.
    functions.insert(
        "add_test".to_string(),
        FunctionSchema {
            options: vec!["COMMAND_EXPAND_LISTS".into(), "VERBATIM".into()],
            one_value_keywords: vec!["NAME".into(), "WORKING_DIRECTORY".into()],
            multi_value_keywords: vec!["COMMAND".into(), "CONFIGURATIONS".into()],
            list_keyword_types: [("COMMAND".into(), ListType::CommandArgv)]
                .into_iter()
                .collect(),
            ..Default::default()
        },
    );

    functions.insert(
        "execute_process".to_string(),
        FunctionSchema {
            options: vec![
                "INPUT_QUIET".into(),
                "OUTPUT_QUIET".into(),
                "ERROR_QUIET".into(),
                "OUTPUT_STRIP_TRAILING_WHITESPACE".into(),
                "ERROR_STRIP_TRAILING_WHITESPACE".into(),
                "ECHO_OUTPUT_VARIABLE".into(),
                "ECHO_ERROR_VARIABLE".into(),
            ],
            one_value_keywords: vec![
                "WORKING_DIRECTORY".into(),
                "TIMEOUT".into(),
                "RESULT_VARIABLE".into(),
                "RESULTS_VARIABLE".into(),
                "OUTPUT_VARIABLE".into(),
                "ERROR_VARIABLE".into(),
                "INPUT_FILE".into(),
                "OUTPUT_FILE".into(),
                "ERROR_FILE".into(),
                "COMMAND_ECHO".into(),
                "COMMAND_ERROR_IS_FATAL".into(),
            ],
            multi_value_keywords: vec!["COMMAND".into()],
            list_keyword_types: [("COMMAND".into(), ListType::CommandArgv)]
                .into_iter()
                .collect(),
            ..Default::default()
        },
    );

    functions.insert(
        "find_package".to_string(),
        FunctionSchema {
            positional: Some(PositionalSpec { min: 1, max: 2 }),
            simple_keywords: vec![],
            options: vec![
                "REQUIRED".into(),
                "QUIET".into(),
                "NO_POLICY_SCOPE".into(),
                "EXACT".into(),
                "MODULE".into(),
                "CONFIG".into(),
            ],
            one_value_keywords: vec!["VERSION".into()],
            multi_value_keywords: vec![
                "COMPONENTS".into(),
                "OPTIONAL_COMPONENTS".into(),
                "HINTS".into(),
                "PATHS".into(),
            ],
            ..Default::default()
        },
    );

    functions.insert(
        "target_link_libraries".to_string(),
        FunctionSchema {
            positional: Some(PositionalSpec { min: 1, max: 1 }),
            simple_keywords: vec![],
            multi_value_keywords: vec!["PUBLIC".into(), "PRIVATE".into(), "INTERFACE".into()],
            ..Default::default()
        },
    );

    // find_package_handle_standard_args(<Name>
    //   [FOUND_VAR <var>] [VERSION_VAR <var>] [FAIL_MESSAGE <msg>]
    //   [REASON_FAILURE_MESSAGE <msg>] REQUIRED_VARS <var>...
    //   [HANDLE_COMPONENTS] [HANDLE_VERSION_RANGE] [CONFIG_MODE]
    //   [NAME_MISMATCHED])
    //
    // REQUIRED_VARS values are CMake variable names but read like a
    // small grep-friendly list, so use Path semantics: single short
    // entry rides with the keyword, multiple entries go one-per-line.
    functions.insert(
        "find_package_handle_standard_args".to_string(),
        FunctionSchema {
            positional: Some(PositionalSpec { min: 1, max: 1 }),
            options: vec![
                "HANDLE_COMPONENTS".into(),
                "HANDLE_VERSION_RANGE".into(),
                "CONFIG_MODE".into(),
                "NAME_MISMATCHED".into(),
            ],
            one_value_keywords: vec![
                "FOUND_VAR".into(),
                "VERSION_VAR".into(),
                "FAIL_MESSAGE".into(),
                "REASON_FAILURE_MESSAGE".into(),
            ],
            multi_value_keywords: vec!["REQUIRED_VARS".into()],
            list_keyword_types: [("REQUIRED_VARS".into(), ListType::Path)]
                .into_iter()
                .collect(),
            ..Default::default()
        },
    );

    functions.insert(
        "option".to_string(),
        FunctionSchema {
            positional: Some(PositionalSpec { min: 1, max: 1 }),
            no_break_first_argument: true,
            ..Default::default()
        },
    );

    functions.insert(
        "cmake_dependent_option".to_string(),
        FunctionSchema {
            positional: Some(PositionalSpec { min: 1, max: 1 }),
            no_break_first_argument: true,
            ..Default::default()
        },
    );

    functions.insert(
        "set".to_string(),
        FunctionSchema {
            positional: Some(PositionalSpec { min: 1, max: 1 }),
            no_break_first_argument: true,
            subparsers: [(
                "CACHE".into(),
                FunctionSchema {
                    positional: Some(PositionalSpec { min: 2, max: 2 }),
                    ..Default::default()
                },
            )]
            .into_iter()
            .collect(),
            ..Default::default()
        },
    );

    functions.insert(
        "file".to_string(),
        FunctionSchema {
            no_break_first_argument: true,
            options: vec![
                "GENERATE".into(),
                "MAKE_DIRECTORY".into(),
                "COPYONLY".into(),
                "ESCAPE_QUOTES".into(),
                "@ONLY".into(),
                "REMOVE".into(),
                "REMOVE_RECURSE".into(),
                "COPY".into(),
                "INSTALL".into(),
                "DOWNLOAD".into(),
                "UPLOAD".into(),
                "READ".into(),
                "WRITE".into(),
                "APPEND".into(),
            ],
            one_value_keywords: vec![
                "OUTPUT".into(),
                "INPUT".into(),
                "CONTENT".into(),
                "DESTINATION".into(),
                "RENAME".into(),
                "TYPE".into(),
                "RESULT_VARIABLE".into(),
                "STATUS".into(),
                "LOG".into(),
                "TIMEOUT".into(),
            ],
            multi_value_keywords: vec!["PERMISSIONS".into(), "FILES_MATCHING".into()],
            list_keyword_types: [
                ("OUTPUT".into(), ListType::Path),
                ("INPUT".into(), ListType::Path),
                ("DESTINATION".into(), ListType::Path),
            ]
            .into_iter()
            .collect(),
            modes: [
                (
                    "CONFIGURE".into(),
                    FunctionSchema {
                        options: vec!["COPYONLY".into(), "ESCAPE_QUOTES".into(), "@ONLY".into()],
                        one_value_keywords: vec![
                            "OUTPUT".into(),
                            "INPUT".into(),
                            "CONTENT".into(),
                            "NEWLINE_STYLE".into(),
                        ],
                        list_keyword_types: [
                            ("OUTPUT".into(), ListType::Path),
                            ("INPUT".into(), ListType::Path),
                        ]
                        .into_iter()
                        .collect(),
                        ..Default::default()
                    },
                ),
                (
                    "GLOB".into(),
                    FunctionSchema {
                        positional: Some(PositionalSpec { min: 1, max: 1 }),
                        options: vec!["CONFIGURE_DEPENDS".into()],
                        one_value_keywords: vec!["LIST_DIRECTORIES".into(), "RELATIVE".into()],
                        ..Default::default()
                    },
                ),
                (
                    "GLOB_RECURSE".into(),
                    FunctionSchema {
                        positional: Some(PositionalSpec { min: 1, max: 1 }),
                        options: vec!["CONFIGURE_DEPENDS".into(), "FOLLOW_SYMLINKS".into()],
                        one_value_keywords: vec!["LIST_DIRECTORIES".into(), "RELATIVE".into()],
                        ..Default::default()
                    },
                ),
            ]
            .into(),
            ..Default::default()
        },
    );

    functions.insert(
        "export".to_string(),
        FunctionSchema {
            options: vec!["APPEND".into(), "EXPORT_LINK_INTERFACE_LIBRARIES".into()],
            one_value_keywords: vec!["NAMESPACE".into(), "FILE".into(), "ANDROID_MK".into()],
            multi_value_keywords: vec!["TARGETS".into()],
            list_keyword_types: [("FILE".into(), ListType::Path)].into_iter().collect(),
            ..Default::default()
        },
    );

    functions.insert(
        "cmake_path".to_string(),
        FunctionSchema {
            no_break_first_argument: true,
            options: vec![
                "NORMALIZE".into(),
                "HAS_ROOT_NAME".into(),
                "HAS_ROOT_DIRECTORY".into(),
                "HAS_ROOT_PATH".into(),
                "HAS_FILENAME".into(),
                "HAS_EXTENSION".into(),
                "HAS_STEM".into(),
                "HAS_RELATIVE_PART".into(),
                "HAS_PARENT_PATH".into(),
                "IS_ABSOLUTE".into(),
                "IS_RELATIVE".into(),
            ],
            one_value_keywords: vec![
                "BASE_DIRECTORY".into(),
                "OUTPUT_VARIABLE".into(),
                "FILENAME".into(),
                "EXTENSION".into(),
                "STEM".into(),
                "RELATIVE_PART".into(),
                "PARENT_PATH".into(),
                "ROOT_NAME".into(),
                "ROOT_DIRECTORY".into(),
                "ROOT_PATH".into(),
            ],
            modes: [
                (
                    "SET".into(),
                    FunctionSchema {
                        positional: Some(PositionalSpec { min: 1, max: 1 }),
                        options: vec!["NORMALIZE".into()],
                        ..Default::default()
                    },
                ),
                (
                    "APPEND".into(),
                    FunctionSchema {
                        positional: Some(PositionalSpec { min: 1, max: 1 }),
                        options: vec!["OUTPUT_VARIABLE".into()],
                        ..Default::default()
                    },
                ),
                (
                    "APPEND_STRING".into(),
                    FunctionSchema {
                        positional: Some(PositionalSpec { min: 1, max: 1 }),
                        options: vec!["OUTPUT_VARIABLE".into()],
                        ..Default::default()
                    },
                ),
                (
                    "GET".into(),
                    FunctionSchema {
                        positional: Some(PositionalSpec { min: 1, max: 2 }),
                        one_value_keywords: vec!["OUTPUT_VARIABLE".into()],
                        ..Default::default()
                    },
                ),
                (
                    "CONVERT".into(),
                    FunctionSchema {
                        positional: Some(PositionalSpec { min: 1, max: 1 }),
                        options: vec!["NORMALIZE".into()],
                        one_value_keywords: vec!["OUTPUT_VARIABLE".into()],
                        ..Default::default()
                    },
                ),
                (
                    "ABSOLUTE_PATH".into(),
                    FunctionSchema {
                        positional: Some(PositionalSpec { min: 1, max: 1 }),
                        options: vec!["NORMALIZE".into()],
                        one_value_keywords: vec!["BASE_DIRECTORY".into(), "OUTPUT_VARIABLE".into()],
                        ..Default::default()
                    },
                ),
                (
                    "RELATIVE_PATH".into(),
                    FunctionSchema {
                        positional: Some(PositionalSpec { min: 1, max: 1 }),
                        one_value_keywords: vec!["BASE_DIRECTORY".into(), "OUTPUT_VARIABLE".into()],
                        ..Default::default()
                    },
                ),
            ]
            .into(),
            ..Default::default()
        },
    );

    functions.insert(
        "cmake_language".to_string(),
        FunctionSchema {
            no_break_first_argument: true,
            modes: [
                (
                    "CALL".into(),
                    FunctionSchema {
                        // <function-name> is the only positional; remaining
                        // args belong to the called function and are formatted
                        // as a generic packed list. We do not consult the
                        // called function's schema — the function name is
                        // typically an expanded variable, not a literal.
                        positional: Some(PositionalSpec { min: 1, max: 1 }),
                        ..Default::default()
                    },
                ),
                (
                    "EVAL".into(),
                    FunctionSchema {
                        // cmake_language(EVAL CODE <code>...) — CODE is the
                        // multi-value keyword that collects the code strings.
                        multi_value_keywords: vec!["CODE".into()],
                        ..Default::default()
                    },
                ),
                (
                    "DEFER".into(),
                    FunctionSchema {
                        // DEFER is itself multi-modal:
                        //   [DIRECTORY <d>] [ID <id>|ID_VAR <var>] CALL <cmd> [<args>...]
                        //   [DIRECTORY <d>] GET_CALL_IDS <var>
                        //   [DIRECTORY <d>] GET_CALL <id> <var>
                        //   [DIRECTORY <d>] CANCEL_CALL <id>...
                        // Model as one flat schema: only one of CALL /
                        // GET_CALL_IDS / GET_CALL / CANCEL_CALL appears per
                        // call, so co-listing them is unambiguous.
                        one_value_keywords: vec![
                            "ID".into(),
                            "ID_VAR".into(),
                            "GET_CALL_IDS".into(),
                        ],
                        multi_value_keywords: vec![
                            "DIRECTORY".into(),
                            "CALL".into(),
                            "GET_CALL".into(),
                            "CANCEL_CALL".into(),
                        ],
                        list_keyword_types: [("DIRECTORY".into(), ListType::Path)]
                            .into_iter()
                            .collect(),
                        ..Default::default()
                    },
                ),
                (
                    "GET_MESSAGE_LOG_LEVEL".into(),
                    FunctionSchema {
                        positional: Some(PositionalSpec { min: 1, max: 1 }),
                        ..Default::default()
                    },
                ),
                (
                    "SET_DEPENDENCY_PROVIDER".into(),
                    FunctionSchema {
                        positional: Some(PositionalSpec { min: 1, max: 1 }),
                        multi_value_keywords: vec!["SUPPORTED_METHODS".into()],
                        ..Default::default()
                    },
                ),
                (
                    "GET_EXPERIMENTAL_FEATURE_ENABLED".into(),
                    FunctionSchema {
                        positional: Some(PositionalSpec { min: 2, max: 2 }),
                        ..Default::default()
                    },
                ),
            ]
            .into_iter()
            .collect(),
            ..Default::default()
        },
    );

    functions.insert(
        "list".to_string(),
        FunctionSchema {
            no_break_first_argument: true,
            options: vec![
                "LENGTH".into(),
                "GET".into(),
                "JOIN".into(),
                "SUBLIST".into(),
                "FIND".into(),
                "FILTER".into(),
                "INSERT".into(),
                "POP_BACK".into(),
                "POP_FRONT".into(),
                "REMOVE_ITEM".into(),
                "REMOVE_AT".into(),
                "REMOVE_DUPLICATES".into(),
                "SORT".into(),
                "REVERSE".into(),
                "COMPARE".into(),
                "CASE".into(),
            ],
            one_value_keywords: vec!["OUTPUT_VARIABLE".into()],
            modes: [
                (
                    "APPEND".into(),
                    FunctionSchema {
                        positional: Some(PositionalSpec { min: 1, max: 1 }),
                        ..Default::default()
                    },
                ),
                (
                    "PREPEND".into(),
                    FunctionSchema {
                        positional: Some(PositionalSpec { min: 1, max: 1 }),
                        ..Default::default()
                    },
                ),
                (
                    "TRANSFORM".into(),
                    FunctionSchema {
                        positional: Some(PositionalSpec { min: 1, max: 1 }),
                        options: vec![
                            "TOLOWER".into(),
                            "TOUPPER".into(),
                            "STRIP".into(),
                            "GENEX_STRIP".into(),
                        ],
                        one_value_keywords: vec!["OUTPUT_VARIABLE".into(), "REGEX".into()],
                        multi_value_keywords: vec!["AT".into(), "FOR".into()],
                        subparsers: [
                            (
                                "APPEND".into(),
                                FunctionSchema {
                                    positional: Some(PositionalSpec { min: 1, max: 1 }),
                                    ..Default::default()
                                },
                            ),
                            (
                                "PREPEND".into(),
                                FunctionSchema {
                                    positional: Some(PositionalSpec { min: 1, max: 1 }),
                                    ..Default::default()
                                },
                            ),
                            (
                                "REPLACE".into(),
                                FunctionSchema {
                                    positional: Some(PositionalSpec { min: 2, max: 2 }),
                                    ..Default::default()
                                },
                            ),
                        ]
                        .into_iter()
                        .collect(),
                        ..Default::default()
                    },
                ),
            ]
            .into_iter()
            .collect(),
            ..Default::default()
        },
    );

    functions.insert(
        "string".to_string(),
        FunctionSchema {
            no_break_first_argument: true,
            options: vec!["ASCII".into(), "RANDOM".into()],
            one_value_keywords: vec![
                "OUTPUT_VARIABLE".into(),
                "LENGTH".into(),
                "ALPHABET".into(),
                "RANDOM_SEED".into(),
            ],
            modes: [
                (
                    "CONCAT".into(),
                    FunctionSchema {
                        positional: Some(PositionalSpec { min: 1, max: 1 }),
                        ..Default::default()
                    },
                ),
                (
                    "JOIN".into(),
                    FunctionSchema {
                        positional: Some(PositionalSpec { min: 2, max: 2 }),
                        ..Default::default()
                    },
                ),
                (
                    "APPEND".into(),
                    FunctionSchema {
                        positional: Some(PositionalSpec { min: 1, max: 1 }),
                        ..Default::default()
                    },
                ),
                (
                    "PREPEND".into(),
                    FunctionSchema {
                        positional: Some(PositionalSpec { min: 1, max: 1 }),
                        ..Default::default()
                    },
                ),
                (
                    "FIND".into(),
                    FunctionSchema {
                        positional: Some(PositionalSpec { min: 3, max: 3 }),
                        ..Default::default()
                    },
                ),
                (
                    "REPLACE".into(),
                    FunctionSchema {
                        positional: Some(PositionalSpec { min: 3, max: 3 }),
                        ..Default::default()
                    },
                ),
                (
                    "TOLOWER".into(),
                    FunctionSchema {
                        positional: Some(PositionalSpec { min: 2, max: 2 }),
                        ..Default::default()
                    },
                ),
                (
                    "TOUPPER".into(),
                    FunctionSchema {
                        positional: Some(PositionalSpec { min: 2, max: 2 }),
                        ..Default::default()
                    },
                ),
                (
                    "LENGTH".into(),
                    FunctionSchema {
                        positional: Some(PositionalSpec { min: 2, max: 2 }),
                        ..Default::default()
                    },
                ),
                (
                    "SUBSTRING".into(),
                    FunctionSchema {
                        positional: Some(PositionalSpec { min: 4, max: 4 }),
                        ..Default::default()
                    },
                ),
                (
                    "STRIP".into(),
                    FunctionSchema {
                        positional: Some(PositionalSpec { min: 2, max: 2 }),
                        ..Default::default()
                    },
                ),
                (
                    "GENEX_STRIP".into(),
                    FunctionSchema {
                        positional: Some(PositionalSpec { min: 2, max: 2 }),
                        ..Default::default()
                    },
                ),
                (
                    "COMPARE".into(),
                    FunctionSchema {
                        positional: Some(PositionalSpec { min: 4, max: 4 }),
                        ..Default::default()
                    },
                ),
                (
                    "HEX".into(),
                    FunctionSchema {
                        positional: Some(PositionalSpec { min: 2, max: 2 }),
                        ..Default::default()
                    },
                ),
                (
                    "CONFIGURE".into(),
                    FunctionSchema {
                        positional: Some(PositionalSpec { min: 2, max: 2 }),
                        ..Default::default()
                    },
                ),
                (
                    "MAKE_C_IDENTIFIER".into(),
                    FunctionSchema {
                        positional: Some(PositionalSpec { min: 2, max: 2 }),
                        ..Default::default()
                    },
                ),
                (
                    "TIMESTAMP".into(),
                    FunctionSchema {
                        positional: Some(PositionalSpec { min: 1, max: 1 }),
                        ..Default::default()
                    },
                ),
                (
                    "UUID".into(),
                    FunctionSchema {
                        positional: Some(PositionalSpec { min: 1, max: 1 }),
                        ..Default::default()
                    },
                ),
                (
                    "REGEX".into(),
                    FunctionSchema {
                        subparsers: [
                            (
                                "MATCH".into(),
                                FunctionSchema {
                                    positional: Some(PositionalSpec { min: 2, max: 2 }),
                                    ..Default::default()
                                },
                            ),
                            (
                                "MATCHALL".into(),
                                FunctionSchema {
                                    positional: Some(PositionalSpec { min: 2, max: 2 }),
                                    ..Default::default()
                                },
                            ),
                            (
                                "REPLACE".into(),
                                FunctionSchema {
                                    positional: Some(PositionalSpec { min: 3, max: 3 }),
                                    ..Default::default()
                                },
                            ),
                        ]
                        .into_iter()
                        .collect(),
                        ..Default::default()
                    },
                ),
                (
                    "MD5".into(),
                    FunctionSchema {
                        positional: Some(PositionalSpec { min: 2, max: 2 }),
                        ..Default::default()
                    },
                ),
                (
                    "SHA1".into(),
                    FunctionSchema {
                        positional: Some(PositionalSpec { min: 2, max: 2 }),
                        ..Default::default()
                    },
                ),
                (
                    "SHA224".into(),
                    FunctionSchema {
                        positional: Some(PositionalSpec { min: 2, max: 2 }),
                        ..Default::default()
                    },
                ),
                (
                    "SHA256".into(),
                    FunctionSchema {
                        positional: Some(PositionalSpec { min: 2, max: 2 }),
                        ..Default::default()
                    },
                ),
                (
                    "SHA384".into(),
                    FunctionSchema {
                        positional: Some(PositionalSpec { min: 2, max: 2 }),
                        ..Default::default()
                    },
                ),
                (
                    "SHA512".into(),
                    FunctionSchema {
                        positional: Some(PositionalSpec { min: 2, max: 2 }),
                        ..Default::default()
                    },
                ),
                (
                    "SHA3_224".into(),
                    FunctionSchema {
                        positional: Some(PositionalSpec { min: 2, max: 2 }),
                        ..Default::default()
                    },
                ),
                (
                    "SHA3_256".into(),
                    FunctionSchema {
                        positional: Some(PositionalSpec { min: 2, max: 2 }),
                        ..Default::default()
                    },
                ),
                (
                    "SHA3_384".into(),
                    FunctionSchema {
                        positional: Some(PositionalSpec { min: 2, max: 2 }),
                        ..Default::default()
                    },
                ),
                (
                    "SHA3_512".into(),
                    FunctionSchema {
                        positional: Some(PositionalSpec { min: 2, max: 2 }),
                        ..Default::default()
                    },
                ),
            ]
            .into_iter()
            .collect(),
            ..Default::default()
        },
    );

    let message_mode_schema = FunctionSchema {
        positional: Some(PositionalSpec { min: 0, max: 1 }),
        ..Default::default()
    };
    functions.insert(
        "message".to_string(),
        FunctionSchema {
            positional: Some(PositionalSpec { min: 0, max: 100 }),
            modes: [
                ("FATAL_ERROR".into(), message_mode_schema.clone()),
                ("SEND_ERROR".into(), message_mode_schema.clone()),
                ("WARNING".into(), message_mode_schema.clone()),
                ("AUTHOR_WARNING".into(), message_mode_schema.clone()),
                ("DEPRECATION".into(), message_mode_schema.clone()),
                ("NOTICE".into(), message_mode_schema.clone()),
                ("STATUS".into(), message_mode_schema.clone()),
                ("VERBOSE".into(), message_mode_schema.clone()),
                ("DEBUG".into(), message_mode_schema.clone()),
                ("TRACE".into(), message_mode_schema.clone()),
                ("CHECK_START".into(), message_mode_schema.clone()),
                ("CHECK_PASS".into(), message_mode_schema.clone()),
                ("CHECK_FAIL".into(), message_mode_schema.clone()),
                ("CONFIGURE_LOG".into(), message_mode_schema),
            ]
            .into_iter()
            .collect(),
            ..Default::default()
        },
    );

    // cmake_parse_arguments(<prefix> <options> <one_value_keywords>
    //                       <multi_value_keywords> <args>...)
    // cmake_parse_arguments(PARSE_ARGV <N> <prefix> <options>
    //                       <one_value_keywords> <multi_value_keywords>)
    //
    // All arguments are positional — there are no keyword groups inside the
    // call. Without a schema, the generic ALL-CAPS heuristic treats short
    // uppercase prefix names (e.g. `ARG`) as logical-group breakpoints,
    // which fractures the strict positional layout. Naming the call here
    // suppresses that heuristic. The PARSE_ARGV mode glues N, prefix, and
    // the options string to the mode line; the two long keyword-list strings
    // pack onto continuation lines below.
    functions.insert(
        "cmake_parse_arguments".to_string(),
        FunctionSchema {
            positional: Some(PositionalSpec { min: 4, max: 100 }),
            modes: [(
                "PARSE_ARGV".into(),
                FunctionSchema {
                    positional: Some(PositionalSpec { min: 3, max: 3 }),
                    ..Default::default()
                },
            )]
            .into_iter()
            .collect(),
            ..Default::default()
        },
    );

    // vcpkg portfile helpers (https://github.com/microsoft/vcpkg). Not standard
    // CMake, but widely used in per-port portfile.cmake scripts. Without
    // schemas, the generic ALL-CAPS heuristic misclassifies keyword/value pairs
    // like `OUT_SOURCE_PATH SOURCE_PATH` and breaks them across lines.

    functions.insert(
        "vcpkg_check_linkage".to_string(),
        FunctionSchema {
            options: vec!["ONLY_STATIC_LIBRARY".into(), "ONLY_DYNAMIC_LIBRARY".into()],
            ..Default::default()
        },
    );

    functions.insert(
        "vcpkg_cmake_config_fixup".to_string(),
        FunctionSchema {
            options: vec![
                "DO_NOT_DELETE_PARENT_CONFIG_PATH".into(),
                "NO_PREFIX_CORRECTION".into(),
            ],
            one_value_keywords: vec![
                "PACKAGE_NAME".into(),
                "CONFIG_PATH".into(),
                "TOOLS_PATH".into(),
            ],
            ..Default::default()
        },
    );

    functions.insert(
        "vcpkg_cmake_configure".to_string(),
        FunctionSchema {
            options: vec![
                "DISABLE_PARALLEL_CONFIGURE".into(),
                "NO_CHARSET_FLAG".into(),
                "WINDOWS_USE_MSBUILD".into(),
            ],
            one_value_keywords: vec![
                "SOURCE_PATH".into(),
                "LOGFILE_BASE".into(),
                "GENERATOR".into(),
            ],
            multi_value_keywords: vec![
                "OPTIONS".into(),
                "OPTIONS_DEBUG".into(),
                "OPTIONS_RELEASE".into(),
                "MAYBE_UNUSED_VARIABLES".into(),
            ],
            ..Default::default()
        },
    );

    functions.insert(
        "vcpkg_cmake_install".to_string(),
        FunctionSchema {
            options: vec!["DISABLE_PARALLEL".into(), "ADD_BIN_TO_PATH".into()],
            ..Default::default()
        },
    );

    functions.insert(
        "vcpkg_from_github".to_string(),
        FunctionSchema {
            one_value_keywords: vec![
                "OUT_SOURCE_PATH".into(),
                "REPO".into(),
                "REF".into(),
                "SHA512".into(),
                "HEAD_REF".into(),
                "FILE_DISAMBIGUATOR".into(),
                "AUTHORIZATION_TOKEN".into(),
            ],
            multi_value_keywords: vec!["PATCHES".into()],
            ..Default::default()
        },
    );

    functions.insert(
        "vcpkg_install_copyright".to_string(),
        FunctionSchema {
            one_value_keywords: vec!["COMMENT".into()],
            multi_value_keywords: vec!["FILE_LIST".into()],
            ..Default::default()
        },
    );

    functions.insert(
        "cmake_instrumentation".to_string(),
        FunctionSchema {
            one_value_keywords: vec!["API_VERSION".into(), "DATA_VERSION".into()],
            multi_value_keywords: vec![
                "HOOKS".into(),
                "OPTIONS".into(),
                "CALLBACK".into(),
                "CUSTOM_CONTENT".into(),
            ],
            list_keyword_types: [("CALLBACK".into(), ListType::CommandArgv)]
                .into_iter()
                .collect(),
            ..Default::default()
        },
    );

    functions.insert(
        "find_file".to_string(),
        FunctionSchema {
            positional: Some(PositionalSpec { min: 2, max: 0 }),
            options: vec![
                "NO_CACHE".into(),
                "REQUIRED".into(),
                "OPTIONAL".into(),
                "NO_DEFAULT_PATH".into(),
                "NO_PACKAGE_ROOT_PATH".into(),
                "NO_CMAKE_PATH".into(),
                "NO_CMAKE_ENVIRONMENT_PATH".into(),
                "NO_SYSTEM_ENVIRONMENT_PATH".into(),
                "NO_CMAKE_SYSTEM_PATH".into(),
                "NO_CMAKE_INSTALL_PREFIX".into(),
                "CMAKE_FIND_ROOT_PATH_BOTH".into(),
                "ONLY_CMAKE_FIND_ROOT_PATH".into(),
                "NO_CMAKE_FIND_ROOT_PATH".into(),
            ],
            one_value_keywords: vec!["REGISTRY_VIEW".into(), "VALIDATOR".into(), "DOC".into()],
            multi_value_keywords: vec![
                "NAMES".into(),
                "HINTS".into(),
                "PATHS".into(),
                "PATH_SUFFIXES".into(),
            ],
            ..Default::default()
        },
    );

    functions.insert(
        "find_library".to_string(),
        FunctionSchema {
            positional: Some(PositionalSpec { min: 2, max: 0 }),
            options: vec![
                "NAMES_PER_DIR".into(),
                "NO_CACHE".into(),
                "REQUIRED".into(),
                "OPTIONAL".into(),
                "NO_DEFAULT_PATH".into(),
                "NO_PACKAGE_ROOT_PATH".into(),
                "NO_CMAKE_PATH".into(),
                "NO_CMAKE_ENVIRONMENT_PATH".into(),
                "NO_SYSTEM_ENVIRONMENT_PATH".into(),
                "NO_CMAKE_SYSTEM_PATH".into(),
                "NO_CMAKE_INSTALL_PREFIX".into(),
                "CMAKE_FIND_ROOT_PATH_BOTH".into(),
                "ONLY_CMAKE_FIND_ROOT_PATH".into(),
                "NO_CMAKE_FIND_ROOT_PATH".into(),
            ],
            one_value_keywords: vec!["REGISTRY_VIEW".into(), "VALIDATOR".into(), "DOC".into()],
            multi_value_keywords: vec![
                "NAMES".into(),
                "HINTS".into(),
                "PATHS".into(),
                "PATH_SUFFIXES".into(),
            ],
            ..Default::default()
        },
    );

    functions.insert(
        "find_path".to_string(),
        FunctionSchema {
            positional: Some(PositionalSpec { min: 2, max: 0 }),
            options: vec![
                "NO_CACHE".into(),
                "REQUIRED".into(),
                "OPTIONAL".into(),
                "NO_DEFAULT_PATH".into(),
                "NO_PACKAGE_ROOT_PATH".into(),
                "NO_CMAKE_PATH".into(),
                "NO_CMAKE_ENVIRONMENT_PATH".into(),
                "NO_SYSTEM_ENVIRONMENT_PATH".into(),
                "NO_CMAKE_SYSTEM_PATH".into(),
                "NO_CMAKE_INSTALL_PREFIX".into(),
                "CMAKE_FIND_ROOT_PATH_BOTH".into(),
                "ONLY_CMAKE_FIND_ROOT_PATH".into(),
                "NO_CMAKE_FIND_ROOT_PATH".into(),
            ],
            one_value_keywords: vec!["REGISTRY_VIEW".into(), "VALIDATOR".into(), "DOC".into()],
            multi_value_keywords: vec![
                "NAMES".into(),
                "HINTS".into(),
                "PATHS".into(),
                "PATH_SUFFIXES".into(),
            ],
            ..Default::default()
        },
    );

    functions.insert(
        "find_program".to_string(),
        FunctionSchema {
            positional: Some(PositionalSpec { min: 2, max: 0 }),
            options: vec![
                "NAMES_PER_DIR".into(),
                "NO_CACHE".into(),
                "REQUIRED".into(),
                "OPTIONAL".into(),
                "NO_DEFAULT_PATH".into(),
                "NO_PACKAGE_ROOT_PATH".into(),
                "NO_CMAKE_PATH".into(),
                "NO_CMAKE_ENVIRONMENT_PATH".into(),
                "NO_SYSTEM_ENVIRONMENT_PATH".into(),
                "NO_CMAKE_SYSTEM_PATH".into(),
                "NO_CMAKE_INSTALL_PREFIX".into(),
                "CMAKE_FIND_ROOT_PATH_BOTH".into(),
                "ONLY_CMAKE_FIND_ROOT_PATH".into(),
                "NO_CMAKE_FIND_ROOT_PATH".into(),
            ],
            one_value_keywords: vec!["REGISTRY_VIEW".into(), "VALIDATOR".into(), "DOC".into()],
            multi_value_keywords: vec![
                "NAMES".into(),
                "HINTS".into(),
                "PATHS".into(),
                "PATH_SUFFIXES".into(),
            ],
            ..Default::default()
        },
    );

    registry.functions.extend(functions);
    registry
}
