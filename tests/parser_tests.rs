use cmtk::parser::Parser;
use cmtk::syntax::SyntaxKind;

#[test]
fn test_bare_word_command_separation() {
    let source = "@PACKAGE_INIT@

macro(foo)";
    let root = Parser::new(source).parse();

    let command_nodes: Vec<_> = root
        .children()
        .filter(|node| node.kind() == SyntaxKind::COMMAND.into())
        .collect();

    assert_eq!(command_nodes.len(), 2);

    // Verify first command
    let cmd1 = &command_nodes[0];
    let name1 = cmd1
        .children_with_tokens()
        .find_map(|n| n.into_token())
        .unwrap();
    assert_eq!(name1.text(), "@PACKAGE_INIT@");

    // Verify second command
    let cmd2 = &command_nodes[1];
    let name2 = cmd2
        .children_with_tokens()
        .find_map(|n| n.into_token())
        .unwrap();
    assert_eq!(name2.text(), "macro");
}

#[test]
fn test_nested_parentheses_do_not_end_command() {
    let source = "if (HANNK_BUILD_TFLITE AND (Halide_TARGET MATCHES \"wasm\"))\nmessage(FATAL_ERROR \"bad\")\nendif ()\n";
    let root = Parser::new(source).parse();

    let command_nodes: Vec<_> = root
        .children()
        .filter(|node| node.kind() == SyntaxKind::COMMAND.into())
        .collect();

    assert_eq!(command_nodes.len(), 3);
    assert_eq!(
        command_nodes[0].to_string(),
        "if (HANNK_BUILD_TFLITE AND (Halide_TARGET MATCHES \"wasm\"))"
    );
}
