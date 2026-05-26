use cmtk::config::Config;
use cmtk::formatter::Formatter;
use cmtk::parser::Parser;

#[test]
fn test_space_before_paren_preserved() {
    let src = "if (CONDITION)\n";
    let root = Parser::new(src).parse();
    let out = Formatter::new(Config::default()).format(&root);
    assert_eq!(out, "if (CONDITION)\n");
}
