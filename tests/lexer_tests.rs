use cmtk::lexer::Lexer;
use cmtk::syntax::SyntaxKind;

#[test]
fn test_bracket_argument_priority() {
    let source = "[[set(Halide_FOUND 1)]]";
    let mut lexer = Lexer::new(source);
    let token = lexer.next().unwrap();
    assert_eq!(token.0, SyntaxKind::BRACKET_ARGUMENT);
    assert_eq!(token.1, "[[set(Halide_FOUND 1)]]");
}

#[test]
fn test_nested_brackets() {
    let source = "[[text]] more";
    let mut lexer = Lexer::new(source);

    let token1 = lexer.next().unwrap();
    assert_eq!(token1.0, SyntaxKind::BRACKET_ARGUMENT);
    assert_eq!(token1.1, "[[text]]");

    let _ws = lexer.next().unwrap(); // whitespace

    let token2 = lexer.next().unwrap();
    assert_eq!(token2.0, SyntaxKind::UNQUOTED_ARGUMENT);
    assert_eq!(token2.1, "more");
}
