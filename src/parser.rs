use crate::lexer::Lexer;
use crate::syntax::{CmakeLanguage, SyntaxKind};
use rowan::{GreenNodeBuilder, Language};

pub struct Parser<'a> {
    lexer: Lexer<'a>,
    builder: GreenNodeBuilder<'static>,
    current_token: Option<(SyntaxKind, &'a str)>,
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a str) -> Self {
        let mut lexer = Lexer::new(input);
        let current_token = lexer.next();
        Self {
            lexer,
            builder: GreenNodeBuilder::new(),
            current_token,
        }
    }

    fn advance(&mut self) {
        self.current_token = self.lexer.next();
    }

    fn eat_trivia(&mut self) {
        while let Some((kind, text)) = self.current_token {
            if kind == SyntaxKind::WHITESPACE || kind == SyntaxKind::COMMENT {
                self.builder.token(CmakeLanguage::kind_to_raw(kind), text);
                self.advance();
            } else {
                break;
            }
        }
    }

    pub fn parse(mut self) -> rowan::SyntaxNode<CmakeLanguage> {
        self.builder
            .start_node(CmakeLanguage::kind_to_raw(SyntaxKind::ROOT));

        while self.current_token.is_some() {
            self.eat_trivia();
            if self.current_token.is_none() {
                break;
            }

            // A command invocation
            self.parse_command();
        }

        self.builder.finish_node();
        rowan::SyntaxNode::new_root(self.builder.finish())
    }

    fn parse_command(&mut self) {
        self.builder
            .start_node(CmakeLanguage::kind_to_raw(SyntaxKind::COMMAND));

        // Identifier
        if let Some((kind, text)) = self.current_token {
            if kind == SyntaxKind::UNQUOTED_ARGUMENT {
                // Command name
                self.builder.token(CmakeLanguage::kind_to_raw(kind), text);
                self.advance();
            } else {
                // Unexpected token, we must consume something to advance
                self.builder
                    .token(CmakeLanguage::kind_to_raw(SyntaxKind::ERROR), text);
                self.advance();
            }
        }

        self.eat_trivia();

        // L_PAREN
        if let Some((kind, text)) = self.current_token {
            if kind == SyntaxKind::L_PAREN {
                self.builder.token(CmakeLanguage::kind_to_raw(kind), text);
                self.advance();
            } else {
                // No L_PAREN: finish the command here; don't consume further tokens.
                self.builder.finish_node();
                return;
            }
        } else {
            // No L_PAREN: finish the command here
            self.builder.finish_node();
            return;
        }

        // Arguments
        let mut paren_depth = 1;
        loop {
            self.eat_trivia();
            if let Some((kind, text)) = self.current_token {
                if kind == SyntaxKind::L_PAREN {
                    paren_depth += 1;
                    self.builder.token(CmakeLanguage::kind_to_raw(kind), text);
                    self.advance();
                } else if kind == SyntaxKind::R_PAREN {
                    paren_depth -= 1;
                    self.builder.token(CmakeLanguage::kind_to_raw(kind), text);
                    self.advance();
                    if paren_depth == 0 {
                        break;
                    }
                } else if kind == SyntaxKind::UNQUOTED_ARGUMENT
                    || kind == SyntaxKind::QUOTED_ARGUMENT
                    || kind == SyntaxKind::BRACKET_ARGUMENT
                    || kind == SyntaxKind::ERROR
                {
                    self.builder.token(CmakeLanguage::kind_to_raw(kind), text);
                    self.advance();
                } else {
                    // Stop on L_PAREN or anything unexpected?
                    // Actually any other token is unexpected, wrap in error.
                    self.builder
                        .token(CmakeLanguage::kind_to_raw(SyntaxKind::ERROR), text);
                    self.advance();
                }
            } else {
                break;
            }
        }

        self.builder.finish_node();
    }
}
