use cmtk::config::Config;
use cmtk::formatter::Formatter;
use cmtk::parser::Parser;
use cmtk::syntax::{CmakeLanguage, SyntaxKind};
use proptest::prelude::*;

fn whitespace_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just(" ".to_string()),
        Just("\n".to_string()),
        Just(" \n ".to_string()),
        Just("\t".to_string()),
        Just("   ".to_string()),
        Just("\n\n".to_string()),
    ]
}

fn comment_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("# line comment\n".to_string()),
        Just("# another comment with space\n".to_string()),
        Just("#[[bracket comment]]".to_string()),
        Just("#[[multi\nline comment]]".to_string()),
    ]
}

fn unquoted_argument_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("FOO".to_string()),
        Just("BAR".to_string()),
        Just("baz".to_string()),
        Just("my_val".to_string()),
        Just("123".to_string()),
        Just("PUBLIC".to_string()),
        Just("PRIVATE".to_string()),
        Just("${VAR}".to_string()),
        Just("path/to/file.cmake".to_string()),
    ]
}

fn quoted_argument_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("\"hello world\"".to_string()),
        Just("\"nested \\\"quotes\\\"\"".to_string()),
        Just("\"\"".to_string()),
        Just("\"line1\\nline2\"".to_string()),
    ]
}

fn bracket_argument_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("[[bracket content]]".to_string()),
        Just("[[]]".to_string()),
        Just("[[line1\nline2]]".to_string()),
    ]
}

fn argument_strategy() -> impl Strategy<Value = String> {
    let leaf = prop_oneof![
        unquoted_argument_strategy(),
        quoted_argument_strategy(),
        bracket_argument_strategy(),
    ];
    leaf.prop_recursive(
        2, // depth
        8, // size
        2, // branches
        |inner| {
            prop_oneof![
                prop::collection::vec(
                    prop_oneof![inner, comment_strategy(), whitespace_strategy()],
                    0..4
                )
                .prop_map(|items| { format!("({})", items.join("")) })
            ]
        },
    )
}

fn command_name_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("set".to_string()),
        Just("option".to_string()),
        Just("message".to_string()),
        Just("my_custom_command".to_string()),
        Just("if".to_string()),
        Just("endif".to_string()),
        Just("foreach".to_string()),
        Just("endforeach".to_string()),
        Just("block".to_string()),
        Just("endblock".to_string()),
    ]
}

fn command_invocation_strategy() -> impl Strategy<Value = String> {
    let gap_strat = prop_oneof![Just("".to_string()), Just(" ".to_string()),];
    let item_strat = prop_oneof![
        argument_strategy(),
        comment_strategy(),
        whitespace_strategy(),
    ];
    let args_strat = prop::collection::vec(item_strat, 0..8);
    (command_name_strategy(), gap_strat, args_strat).prop_map(|(name, gap, args)| {
        let mut res = format!("{}{}(", name, gap);
        for arg in args {
            res.push_str(&arg);
        }
        res.push(')');
        res
    })
}

fn cmake_file_strategy() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop_oneof![
            command_invocation_strategy(),
            comment_strategy(),
            whitespace_strategy(),
        ],
        1..8,
    )
    .prop_map(|parts| parts.join(""))
}

fn has_errors(node: &rowan::SyntaxNode<CmakeLanguage>) -> bool {
    let mut walk = node.preorder_with_tokens();
    while let Some(event) = walk.next() {
        match event {
            rowan::WalkEvent::Enter(rowan::NodeOrToken::Node(n)) => {
                if n.kind() == SyntaxKind::ERROR.into() {
                    return true;
                }
            }
            rowan::WalkEvent::Enter(rowan::NodeOrToken::Token(t)) => {
                if t.kind() == SyntaxKind::ERROR {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

fn get_structural_tokens(node: &rowan::SyntaxNode<CmakeLanguage>) -> Vec<(SyntaxKind, String)> {
    let mut tokens = Vec::new();
    let mut walk = node.preorder_with_tokens();
    while let Some(event) = walk.next() {
        if let rowan::WalkEvent::Enter(rowan::NodeOrToken::Token(t)) = event {
            let kind = t.kind();
            if kind != SyntaxKind::WHITESPACE && kind != SyntaxKind::COMMENT {
                tokens.push((kind, t.text().to_string()));
            }
        }
    }
    tokens
}

fn get_comment_tokens(node: &rowan::SyntaxNode<CmakeLanguage>) -> Vec<String> {
    let mut comments = Vec::new();
    let mut walk = node.preorder_with_tokens();
    while let Some(event) = walk.next() {
        if let rowan::WalkEvent::Enter(rowan::NodeOrToken::Token(t)) = event {
            if t.kind() == SyntaxKind::COMMENT {
                comments.push(t.text().trim_end().to_string());
            }
        }
    }
    comments
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    #[test]
    fn test_parser_and_formatter_properties(input in cmake_file_strategy()) {
        // --- INVARIANT 1: Parseability ---
        // Verify original parses successfully
        let orig_tree = Parser::new(&input).parse();
        prop_assert!(!has_errors(&orig_tree), "Original input has parse errors: {}", input);

        // Format the tree
        let config = Config::default();
        let formatted = Formatter::new(config.clone()).format(&orig_tree);

        // Verify formatted output parses successfully
        let fmt_tree = Parser::new(&formatted).parse();
        prop_assert!(!has_errors(&fmt_tree), "Formatted output has parse errors. Input: {}\nFormatted: {}", input, formatted);

        // --- INVARIANT 2: Idempotence ---
        let double_formatted = Formatter::new(config).format(&fmt_tree);
        prop_assert_eq!(&formatted, &double_formatted, "Formatter is not idempotent. First format:\n{}\nSecond format:\n{}", formatted, double_formatted);

        // --- INVARIANT 3: Token Preservation ---
        // Compare structural tokens (excluding whitespace and comments)
        let orig_structural = get_structural_tokens(&orig_tree);
        let fmt_structural = get_structural_tokens(&fmt_tree);
        prop_assert_eq!(&orig_structural, &fmt_structural, "Structural tokens modified during formatting.\nOriginal: {:?}\nFormatted: {:?}", &orig_structural, &fmt_structural);

        // Compare comment tokens (excluding whitespace, trimmed)
        let mut orig_comments = get_comment_tokens(&orig_tree);
        let mut fmt_comments = get_comment_tokens(&fmt_tree);
        orig_comments.sort();
        fmt_comments.sort();
        prop_assert_eq!(&orig_comments, &fmt_comments, "Comments modified during formatting.\nOriginal: {:?}\nFormatted: {:?}", &orig_comments, &fmt_comments);
    }
}
