#![allow(non_camel_case_types)]
use logos::{Lexer, Logos};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Logos)]
#[repr(u16)]
pub enum SyntaxKind {
    #[regex(r"[ \t\r\n]+")]
    WHITESPACE = 0,

    #[regex(r"#", lex_comment)]
    COMMENT,

    #[regex(r"\[=*\[", lex_bracket_argument, priority = 3)]
    BRACKET_ARGUMENT,

    #[token("(")]
    L_PAREN,

    #[token(")")]
    R_PAREN,

    // `\` followed by any character (including newline) is a valid escape
    // sequence inside CMake quoted/unquoted arguments — line continuation
    // (backslash-newline) is common in multi-line quoted shell strings.
    // `(?s:\\.)` enables dot-matches-newline locally so `\<NL>` parses.
    #[regex(r#""[^"\\]*(?:(?s:\\.)[^"\\]*)*""#)]
    QUOTED_ARGUMENT,

    #[regex(r"(?:[^\s()#\x22\\\[\]]|(?s:\\.))+")]
    UNQUOTED_ARGUMENT,

    ERROR,

    // Composite nodes
    ROOT,
    COMMAND,
}

impl From<SyntaxKind> for rowan::SyntaxKind {
    fn from(kind: SyntaxKind) -> Self {
        Self(kind as u16)
    }
}

fn lex_comment<'a>(lex: &mut Lexer<'a, SyntaxKind>) -> Option<()> {
    let remainder = lex.remainder();
    let mut chars = remainder.chars();
    if let Some('[') = chars.next() {
        let mut eq_count = 0;
        let mut is_bracket = false;
        for c in chars.by_ref() {
            if c == '=' {
                eq_count += 1;
            } else if c == '[' {
                is_bracket = true;
                break;
            } else {
                break;
            }
        }
        if is_bracket {
            let mut close_pattern = String::from("]");
            close_pattern.push_str(&"=".repeat(eq_count));
            close_pattern.push(']');

            if let Some(idx) = remainder.find(&close_pattern) {
                lex.bump(idx + close_pattern.len());
            } else {
                lex.bump(remainder.len());
            }
            return Some(());
        }
    }

    if let Some(idx) = remainder.find('\n') {
        lex.bump(idx);
    } else {
        lex.bump(remainder.len());
    }
    Some(())
}

fn lex_bracket_argument<'a>(lex: &mut Lexer<'a, SyntaxKind>) -> Option<()> {
    let start_len = lex.slice().len();
    let eq_count = start_len - 2;
    let mut close_pattern = String::from("]");
    close_pattern.push_str(&"=".repeat(eq_count));
    close_pattern.push(']');

    let remainder = lex.remainder();
    if let Some(idx) = remainder.find(&close_pattern) {
        lex.bump(idx + close_pattern.len());
    } else {
        lex.bump(remainder.len());
    }
    Some(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CmakeLanguage {}

impl rowan::Language for CmakeLanguage {
    type Kind = SyntaxKind;

    fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
        assert!(raw.0 <= SyntaxKind::COMMAND as u16);
        unsafe { std::mem::transmute::<u16, SyntaxKind>(raw.0) }
    }

    fn kind_to_raw(kind: Self::Kind) -> rowan::SyntaxKind {
        kind.into()
    }
}
