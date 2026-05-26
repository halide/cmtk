use crate::syntax::{CmakeLanguage, SyntaxKind};

pub type SyntaxNode = rowan::SyntaxNode<CmakeLanguage>;
pub type SyntaxToken = rowan::SyntaxToken<CmakeLanguage>;
pub type SyntaxElement = rowan::SyntaxElement<CmakeLanguage>;

#[derive(Debug, Clone)]
pub struct CommandNode(SyntaxNode);

impl CommandNode {
    pub fn cast(node: SyntaxNode) -> Option<Self> {
        if node.kind() == SyntaxKind::COMMAND {
            Some(Self(node))
        } else {
            None
        }
    }

    pub fn name(&self) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(|element| element.into_token())
            .find(|token| token.kind() == SyntaxKind::UNQUOTED_ARGUMENT)
    }

    pub fn args(&self) -> Vec<SyntaxToken> {
        let mut skipped_name = false;
        let mut paren_depth = 0usize;
        self.0
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .filter(move |t| match t.kind() {
                SyntaxKind::WHITESPACE => false,
                SyntaxKind::UNQUOTED_ARGUMENT if !skipped_name => {
                    skipped_name = true;
                    false
                }
                SyntaxKind::L_PAREN if paren_depth == 0 => {
                    paren_depth = 1;
                    false
                }
                SyntaxKind::L_PAREN => {
                    paren_depth += 1;
                    true
                }
                SyntaxKind::R_PAREN if paren_depth == 1 => {
                    paren_depth = 0;
                    false
                }
                SyntaxKind::R_PAREN => {
                    paren_depth = paren_depth.saturating_sub(1);
                    true
                }
                _ => true,
            })
            .collect()
    }
}
