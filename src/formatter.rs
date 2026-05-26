use crate::config::{Config, IndentStyle};
use crate::cst::{CommandNode, SyntaxNode, SyntaxToken};
use crate::schema::{FunctionSchema, ListType};
use crate::syntax::SyntaxKind;
use rowan::NodeOrToken;

pub struct Formatter {
    config: Config,
    output: String,
    indent_level: usize,
    at_line_start: bool,
}

#[derive(Clone)]
enum Item {
    Arg {
        text: String,
        index: usize,
        source_line: usize,
    },
    Comment {
        text: String,
        trailing: bool,
        target_arg: Option<usize>,
        previous_arg: Option<String>,
    },
}

#[derive(Debug, Clone)]
struct LayoutLine {
    indent: usize,
    text: String,
    has_comment: bool,
}

#[derive(Clone)]
struct ArgItem {
    text: String,
    is_comment: bool,
    is_trailing_comment: bool,
    source_line: Option<usize>,
}

impl Formatter {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            output: String::new(),
            indent_level: 0,
            at_line_start: true,
        }
    }

    pub fn format(mut self, root: &SyntaxNode) -> String {
        self.visit_node(root);
        self.output
    }

    fn is_block_open(name: &str) -> bool {
        name.eq_ignore_ascii_case("if")
            || name.eq_ignore_ascii_case("foreach")
            || name.eq_ignore_ascii_case("while")
            || name.eq_ignore_ascii_case("function")
            || name.eq_ignore_ascii_case("macro")
            || name.eq_ignore_ascii_case("block")
    }

    fn is_block_close(name: &str) -> bool {
        name.eq_ignore_ascii_case("endif")
            || name.eq_ignore_ascii_case("endforeach")
            || name.eq_ignore_ascii_case("endwhile")
            || name.eq_ignore_ascii_case("endfunction")
            || name.eq_ignore_ascii_case("endmacro")
            || name.eq_ignore_ascii_case("endblock")
    }

    fn is_block_mid(name: &str) -> bool {
        name.eq_ignore_ascii_case("elseif") || name.eq_ignore_ascii_case("else")
    }

    fn node_command_name(node: &SyntaxNode) -> Option<SyntaxToken> {
        node.children_with_tokens()
            .filter_map(|e| e.into_token())
            .find(|t| t.kind() == SyntaxKind::UNQUOTED_ARGUMENT)
    }

    fn normalize_whitespace(s: &str) -> std::borrow::Cow<'_, str> {
        // Only normalize tokens that span lines; preserve same-line horizontal
        // whitespace (e.g. spaces before trailing comments on the same line).
        if !s.contains('\n') && !s.contains('\r') {
            return std::borrow::Cow::Borrowed(s);
        }
        // Strip trailing spaces/tabs after each newline so block indentation
        // can be regenerated from indent_level rather than inherited from source.
        let mut result = String::new();
        let mut pending_spaces = String::new();
        for ch in s.chars() {
            match ch {
                ' ' | '\t' => pending_spaces.push(ch),
                '\n' | '\r' => {
                    pending_spaces.clear();
                    result.push(ch);
                }
                _ => {
                    result.push_str(&pending_spaces);
                    pending_spaces.clear();
                    result.push(ch);
                }
            }
        }
        // Discard trailing pending_spaces (= indentation of the next line)
        std::borrow::Cow::Owned(result)
    }

    fn visit_node(&mut self, node: &SyntaxNode) {
        if node.kind() == SyntaxKind::COMMAND {
            let name = Self::node_command_name(node);
            let name_str = name.as_ref().map(|t| t.text()).unwrap_or("");
            if Self::is_block_close(name_str) || Self::is_block_mid(name_str) {
                self.indent_level = self.indent_level.saturating_sub(1);
            }
            self.format_command(node);
            if Self::is_block_open(name_str) || Self::is_block_mid(name_str) {
                self.indent_level += 1;
            }
        } else {
            for element in node.children_with_tokens() {
                match element {
                    NodeOrToken::Node(n) => self.visit_node(&n),
                    NodeOrToken::Token(t) => {
                        if t.kind() == SyntaxKind::WHITESPACE {
                            let norm = Self::normalize_whitespace(t.text());
                            if norm.contains('\n') {
                                self.at_line_start = true;
                            }
                            self.output.push_str(&norm);
                        } else {
                            if self.at_line_start {
                                self.output.push_str(&self.indent_str(self.indent_level));
                                self.at_line_start = false;
                            }
                            self.output.push_str(t.text());
                        }
                    }
                }
            }
        }
    }

    fn indent_str(&self, level: usize) -> String {
        match self.config.indent_style {
            IndentStyle::Space => " ".repeat(self.config.indent_width * level),
            IndentStyle::Tab => "\t".repeat(level),
        }
    }

    fn current_line(&self) -> &str {
        self.output
            .rsplit_once('\n')
            .map(|(_, tail)| tail)
            .unwrap_or(&self.output)
    }

    fn command_items(&self, node: &SyntaxNode) -> Vec<Item> {
        let mut skipped_name = false;
        let mut paren_depth = 0usize;
        let mut last_whitespace_had_newline = true;
        // CMake's lexical grammar: whitespace is the only separator between
        // arguments. Adjacent non-paren argument tokens (e.g. unquoted segments
        // glued to a following quoted argument like -DKEY="${VAL}", or the
        // [/]-bracketed keyword tokens that the lexer emits as ERRORs) with no
        // whitespace between them form ONE argument. Parens remain structural
        // tokens — `19.16)` in a condition is `19.16` followed by a closing
        // paren, not a concatenated arg.
        let mut prev_arg_concatenable = false;
        let mut next_arg_index = 0usize;
        let mut line_first_arg = None;
        let mut source_line = 0usize;
        let mut items = Vec::new();

        for token in node.children_with_tokens().filter_map(|e| e.into_token()) {
            match token.kind() {
                SyntaxKind::WHITESPACE => {
                    last_whitespace_had_newline = token.text().contains('\n');
                    prev_arg_concatenable = false;
                    if last_whitespace_had_newline {
                        line_first_arg = None;
                        source_line += token.text().matches('\n').count();
                    }
                }
                SyntaxKind::UNQUOTED_ARGUMENT if !skipped_name => {
                    skipped_name = true;
                    last_whitespace_had_newline = false;
                }
                SyntaxKind::L_PAREN if paren_depth == 0 => {
                    paren_depth = 1;
                    last_whitespace_had_newline = false;
                }
                SyntaxKind::L_PAREN => {
                    paren_depth += 1;
                    Self::push_arg(
                        &mut items,
                        token.text(),
                        &mut next_arg_index,
                        &mut line_first_arg,
                        source_line,
                    );
                    last_whitespace_had_newline = false;
                    prev_arg_concatenable = false;
                }
                SyntaxKind::R_PAREN if paren_depth == 1 => {
                    paren_depth = 0;
                    last_whitespace_had_newline = false;
                }
                SyntaxKind::R_PAREN => {
                    paren_depth = paren_depth.saturating_sub(1);
                    Self::push_arg(
                        &mut items,
                        token.text(),
                        &mut next_arg_index,
                        &mut line_first_arg,
                        source_line,
                    );
                    last_whitespace_had_newline = false;
                    prev_arg_concatenable = false;
                }
                SyntaxKind::COMMENT => {
                    let trailing = !last_whitespace_had_newline;
                    let prev_arg_text = if trailing {
                        items.iter().rev().find_map(|item| match item {
                            Item::Arg { text, .. } => Some(text.clone()),
                            _ => None,
                        })
                    } else {
                        None
                    };
                    items.push(Item::Comment {
                        text: token.text().trim_end().to_string(),
                        trailing,
                        target_arg: if trailing { line_first_arg } else { None },
                        previous_arg: prev_arg_text,
                    });
                    last_whitespace_had_newline = false;
                    prev_arg_concatenable = false;
                }
                _ => {
                    if prev_arg_concatenable {
                        Self::extend_last_arg(&mut items, token.text());
                    } else {
                        Self::push_arg(
                            &mut items,
                            token.text(),
                            &mut next_arg_index,
                            &mut line_first_arg,
                            source_line,
                        );
                    }
                    last_whitespace_had_newline = false;
                    prev_arg_concatenable = true;
                }
            }
        }

        items
    }

    fn push_arg(
        items: &mut Vec<Item>,
        text: &str,
        next_arg_index: &mut usize,
        line_first_arg: &mut Option<usize>,
        source_line: usize,
    ) {
        let index = *next_arg_index;
        if line_first_arg.is_none() {
            *line_first_arg = Some(index);
        }
        *next_arg_index += 1;
        items.push(Item::Arg {
            text: text.to_string(),
            index,
            source_line,
        });
    }

    fn extend_last_arg(items: &mut [Item], text: &str) {
        if let Some(Item::Arg {
            text: prev_text, ..
        }) = items.last_mut()
        {
            prev_text.push_str(text);
        }
    }

    fn place_schema_comments(items: &[Item], schema: &FunctionSchema, width: usize) -> Vec<Item> {
        let mut leading_comments: Vec<(usize, String)> = Vec::new();
        let mut kept = Vec::new();
        let mut previous_arg = None;

        for item in items {
            match item {
                Item::Arg { index, .. } => {
                    let index = *index;
                    previous_arg = Some(index);
                    kept.push(item.clone());
                }
                Item::Comment {
                    text,
                    trailing,
                    target_arg: Some(target_arg),
                    previous_arg: Some(previous_arg_text),
                } if *trailing
                    && previous_arg != Some(*target_arg)
                    && schema.is_option(previous_arg_text)
                    && Self::trailing_line_overflows(items, *target_arg, text, width) =>
                {
                    leading_comments.push((*target_arg, text.clone()));
                }
                Item::Comment { .. } => kept.push(item.clone()),
            }
        }

        if leading_comments.is_empty() {
            return kept;
        }

        let mut placed = Vec::new();
        for item in kept {
            if let Item::Arg { index, .. } = &item {
                for (_, text) in leading_comments
                    .iter()
                    .filter(|(target_arg, _)| target_arg == index)
                {
                    placed.push(Item::Comment {
                        text: text.clone(),
                        trailing: false,
                        target_arg: None,
                        previous_arg: None,
                    });
                }
            }
            placed.push(item);
        }

        placed
    }

    // True if the source line beginning at `target_arg` would not fit when
    // rendered as `<args> <"  "> <comment_text>` in the available arg-line
    // width. Used by place_schema_comments to decide whether to promote a
    // trailing comment to a leading comment for the line's first arg: if the
    // trailing form still fits, leave it alone — the spec keeps trailing
    // comments trailing whenever the resulting line is within width.
    fn trailing_line_overflows(
        items: &[Item],
        target_arg: usize,
        comment_text: &str,
        width: usize,
    ) -> bool {
        let target_source_line = items.iter().find_map(|item| match item {
            Item::Arg {
                index, source_line, ..
            } if *index == target_arg => Some(*source_line),
            _ => None,
        });
        let Some(target_line) = target_source_line else {
            return true;
        };
        let mut total = 0usize;
        let mut first = true;
        for item in items {
            if let Item::Arg {
                text, source_line, ..
            } = item
                && *source_line == target_line
            {
                if !first {
                    total += 1;
                }
                total += text.len();
                first = false;
            }
        }
        total += 2;
        total += comment_text.len();
        total > width
    }

    fn format_command(&mut self, node: &SyntaxNode) {
        let cmd = match CommandNode::cast(node.clone()) {
            Some(c) => c,
            None => return,
        };
        let name_tok = match cmd.name() {
            Some(t) => t,
            None => return,
        };
        let cmd_name = name_tok.text();

        let has_parens = node
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .any(|t| t.kind() == SyntaxKind::L_PAREN);

        let name_paren_gap = self.name_paren_gap(node, cmd_name);
        let items = self.command_items(node);

        // A comment on the same source line as the opening `(`, before any
        // positional argument, is rendered as a trailing comment on the
        // `name(` line. This preserves markers (e.g. `# nolint`) that
        // downstream tools associate with the command's own line.
        let head_comment: Option<String> = match items.first() {
            Some(Item::Comment {
                text,
                trailing: true,
                target_arg: None,
                ..
            }) => Some(text.clone()),
            _ => None,
        };
        let layout_items: &[Item] = if head_comment.is_some() {
            &items[1..]
        } else {
            &items[..]
        };

        let schema = self.config.function_schemas.get(cmd_name);
        let base_indent = self.indent_str(self.indent_level);
        let arg_indent = format!("{}{}", base_indent, self.indent_str(1));
        let continuation_indent = format!("{}{}", base_indent, self.indent_str(2));

        let single_line = self.single_line(cmd_name, &name_paren_gap, has_parens, &items);
        let current_col = base_indent.len();
        if current_col + single_line.len() <= self.config.line_width
            && !items.iter().any(|i| matches!(i, Item::Comment { .. }))
        {
            if self.at_line_start {
                self.output.push_str(&base_indent);
                self.at_line_start = false;
            }
            self.output.push_str(&single_line);
            return;
        }

        let arg_width = self.config.line_width.saturating_sub(arg_indent.len());
        // The first line glues to `<cmd_name><name_paren_gap>(` under
        // no_break_first_argument, so its budget is line_width minus that
        // prefix. For ordinary calls, it equals arg_width (the first line
        // sits at arg_indent like the rest).
        let no_break_first = schema.is_some_and(|s| s.no_break_first_argument);
        let header_budget = if no_break_first {
            let prefix_len = cmd_name.len() + name_paren_gap.len() + usize::from(has_parens);
            self.config.line_width.saturating_sub(prefix_len)
        } else {
            arg_width
        };

        let lines = match schema {
            Some(s) => self.layout_schema_items(layout_items, s, arg_width, header_budget),
            None => self.layout_generic_items(layout_items, arg_width),
        };

        if self.at_line_start {
            self.output.push_str(&base_indent);
            self.at_line_start = false;
        }
        self.output.push_str(cmd_name);
        self.output.push_str(&name_paren_gap);
        if has_parens {
            self.output.push('(');
        }
        if let Some(comment) = &head_comment {
            self.output.push_str("  ");
            self.output.push_str(comment);
        }

        if lines.is_empty() {
            if has_parens {
                if head_comment.is_some() {
                    self.output.push('\n');
                    self.output.push_str(&base_indent);
                }
                self.output.push(')');
            }
            return;
        }

        if schema.is_some_and(|s| s.no_break_first_argument)
            && head_comment.is_none()
            && !lines[0].text.starts_with('#')
        {
            self.render_no_break_first(&lines, &base_indent, &arg_indent, &continuation_indent);
        } else {
            self.render_block(&lines, &base_indent, &arg_indent, &continuation_indent);
        }
    }

    fn name_paren_gap(&self, node: &SyntaxNode, cmd_name: &str) -> String {
        let is_control_flow = matches!(
            cmd_name.to_lowercase().as_str(),
            "if" | "elseif"
                | "else"
                | "endif"
                | "foreach"
                | "endforeach"
                | "while"
                | "endwhile"
                | "block"
                | "endblock"
        );

        if is_control_flow {
            return " ".to_string();
        }

        let mut tokens = node
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .skip_while(|t| t.kind() != SyntaxKind::UNQUOTED_ARGUMENT);
        tokens.next();
        match tokens.next() {
            Some(t) if t.kind() == SyntaxKind::WHITESPACE => t.text().to_string(),
            _ => String::new(),
        }
    }

    fn single_line(
        &self,
        cmd_name: &str,
        name_paren_gap: &str,
        has_parens: bool,
        items: &[Item],
    ) -> String {
        let mut single_line = String::from(cmd_name);
        single_line.push_str(name_paren_gap);
        if has_parens {
            single_line.push('(');
        }
        for (i, item) in items.iter().enumerate() {
            match item {
                Item::Arg { text, .. } => {
                    if i == 0 {
                        single_line.push_str(text);
                    } else {
                        Self::append_value(&mut single_line, text);
                    }
                }
                Item::Comment { text, .. } => {
                    if i > 0 {
                        single_line.push(' ');
                    }
                    single_line.push_str(text);
                }
            }
        }
        if has_parens {
            single_line.push(')');
        }
        single_line
    }

    fn render_block(
        &mut self,
        lines: &[LayoutLine],
        base_indent: &str,
        arg_indent: &str,
        continuation_indent: &str,
    ) {
        for line in lines {
            self.output.push('\n');
            self.push_layout_indent(line.indent, arg_indent, continuation_indent);
            self.output.push_str(&line.text);
        }
        self.output.push('\n');
        self.output.push_str(base_indent);
        self.output.push(')');
    }

    fn render_no_break_first(
        &mut self,
        lines: &[LayoutLine],
        base_indent: &str,
        arg_indent: &str,
        continuation_indent: &str,
    ) {
        self.output.push_str(&lines[0].text);
        for line in &lines[1..] {
            self.output.push('\n');
            self.push_layout_indent(line.indent, arg_indent, continuation_indent);
            self.output.push_str(&line.text);
        }

        let current_line = self.current_line();
        let last_line_indent = lines.last().map(|line| line.indent).unwrap_or(0);
        if current_line.len() < self.config.line_width
            && last_line_indent == 0
            && !lines[0].has_comment
            && lines.len() == 1
        {
            self.output.push(')');
        } else {
            self.output.push('\n');
            self.output.push_str(base_indent);
            self.output.push(')');
        }
    }

    fn push_layout_indent(&mut self, indent: usize, arg_indent: &str, continuation_indent: &str) {
        if indent == 0 {
            self.output.push_str(arg_indent);
        } else if indent == 1 {
            self.output.push_str(continuation_indent);
        } else {
            self.output.push_str(arg_indent);
            self.output.push_str(&self.indent_str(indent));
        }
    }

    fn is_generic_keyword(s: &str) -> bool {
        // Common ALL-CAPS boolean literals — these appear as values, not as
        // logical-group breakpoints, so exclude them even though they look
        // like keywords by shape. (NO and ON are 2 chars and already
        // excluded by the length check.)
        if matches!(s, "YES" | "OFF" | "TRUE" | "FALSE") {
            return false;
        }
        s.len() >= 3
            && s.chars().next().is_some_and(|c| c.is_ascii_uppercase())
            && s.chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
    }

    fn layout_generic_items(&self, items: &[Item], width: usize) -> Vec<LayoutLine> {
        let mut lines: Vec<LayoutLine> = Vec::new();
        let mut current_prefix: Vec<String> = Vec::new();
        let mut current_values: Vec<ArgItem> = Vec::new();

        for item in items {
            match item {
                Item::Arg {
                    text, source_line, ..
                } => {
                    if Self::is_generic_keyword(text) {
                        // Single-value groups always inline (e.g. SHA512 <hash>).
                        if current_prefix.len() == 1 && current_values.len() == 1 {
                            lines.push(LayoutLine {
                                indent: 0,
                                text: format!("{} {}", current_prefix[0], current_values[0].text),
                                has_comment: false,
                            });
                        } else {
                            lines.extend(self.pack_value_items(
                                0,
                                current_prefix,
                                current_values,
                                width,
                                ListType::Packed,
                                0,
                            ));
                        }
                        current_prefix = vec![text.clone()];
                        current_values = Vec::new();
                    } else {
                        current_values.push(ArgItem {
                            text: text.clone(),
                            is_comment: false,
                            is_trailing_comment: false,
                            source_line: Some(*source_line),
                        });
                    }
                }
                Item::Comment { text, trailing, .. } => {
                    if current_prefix.len() == 1 && current_values.len() == 1 {
                        lines.push(LayoutLine {
                            indent: 0,
                            text: format!("{} {}", current_prefix[0], current_values[0].text),
                            has_comment: false,
                        });
                    } else {
                        lines.extend(self.pack_value_items(
                            0,
                            current_prefix,
                            current_values,
                            width,
                            ListType::Packed,
                            0,
                        ));
                    }
                    current_prefix = Vec::new();
                    current_values = Vec::new();
                    Self::push_comment(&mut lines, 0, text, *trailing, width);
                }
            }
        }

        if current_prefix.len() == 1 && current_values.len() == 1 {
            lines.push(LayoutLine {
                indent: 0,
                text: format!("{} {}", current_prefix[0], current_values[0].text),
                has_comment: false,
            });
        } else {
            lines.extend(self.pack_value_items(
                0,
                current_prefix,
                current_values,
                width,
                ListType::Packed,
                0,
            ));
        }
        lines
    }

    fn layout_schema_items(
        &self,
        items: &[Item],
        schema: &FunctionSchema,
        width: usize,
        header_budget: usize,
    ) -> Vec<LayoutLine> {
        let schema_items = Self::place_schema_comments(items, schema, width);
        let args = schema_items
            .iter()
            .map(|i| match i {
                Item::Arg {
                    text, source_line, ..
                } => ArgItem {
                    text: text.clone(),
                    is_comment: false,
                    is_trailing_comment: false,
                    source_line: Some(*source_line),
                },
                Item::Comment { text, trailing, .. } => ArgItem {
                    text: text.clone(),
                    is_comment: true,
                    is_trailing_comment: *trailing,
                    source_line: None,
                },
            })
            .collect::<Vec<_>>();
        self.layout_schema_args(&args, schema, width, header_budget, 0)
    }

    fn layout_schema_args(
        &self,
        args: &[ArgItem],
        schema: &FunctionSchema,
        width: usize,
        header_budget: usize,
        consumed_positionals: usize,
    ) -> Vec<LayoutLine> {
        let mut lines = Vec::new();
        let mut index = 0;

        if let Some(mode) = args
            .first()
            .filter(|arg| !arg.is_comment)
            .and_then(|arg| schema.mode(&arg.text).map(|mode| (arg.text.clone(), mode)))
        {
            let (mode_keyword, mode_schema) = mode;
            let (mode_lines, rest_start) =
                self.layout_mode_header(&mode_keyword, mode_schema, &args[1..], header_budget);
            lines.extend(mode_lines);
            let rest_schema = Self::schema_without_positional(mode_schema);
            lines.extend(self.layout_schema_args(
                &args[1 + rest_start..],
                &rest_schema,
                width,
                width,
                consumed_positionals + rest_start,
            ));
            return lines;
        }

        let mut positional_max = schema
            .positional
            .as_ref()
            .map(|p| p.max)
            .filter(|max| *max > 0);

        if positional_max == Some(1) && schema.subparser("CACHE").is_some() {
            let mut val_count = 0;
            let mut cache_found = false;
            for arg in &args[index..] {
                if arg.is_comment {
                    continue;
                }
                if schema.is_any_keyword(&arg.text) {
                    if arg.text == "CACHE" {
                        cache_found = true;
                    }
                    break;
                }
                val_count += 1;
            }
            if cache_found && val_count == 2 {
                positional_max = Some(2);
            }
        }
        let mut positional: Vec<ArgItem> = Vec::new();
        while index < args.len()
            && !args[index].is_comment
            && !schema.is_any_keyword(&args[index].text)
        {
            if let Some(max) = positional_max
                && positional.len() >= max
            {
                break;
            }
            positional.push(args[index].clone());
            index += 1;
        }
        // Route positionals through pack_value_items so the source-vertical
        // preservation heuristic applies — otherwise calls like
        // `list(APPEND var v1 v2 v3 ...)`, where the mode consumes the only
        // keyword and the rest are positionals, would always pack by width
        // even when the author chopped them one-per-line in the source.
        lines.extend(self.pack_value_items(
            0,
            Vec::new(),
            positional,
            width,
            schema.default_list_type,
            consumed_positionals,
        ));

        let mut any_indented_subparser = false;
        let mut any_keyword_seen = false;
        // Options may glue back onto the previous line while the layout is
        // still in "header mode" — positional args and chained option flags
        // only. Once a value-bearing keyword (one-value, multi-value, compound,
        // or subparser) has emitted, gluing would make the option look like a
        // value of that group, so switch off.
        let mut option_can_glue = true;
        while index < args.len() {
            let keyword = &args[index].text;
            if args[index].is_comment {
                Self::push_comment(
                    &mut lines,
                    0,
                    keyword,
                    args[index].is_trailing_comment,
                    width,
                );
                index += 1;
                continue;
            }

            if schema.is_option(keyword) {
                // Bare flag keywords (find_package(... EXACT REQUIRED ...),
                // install(... OPTIONAL ...), etc.) read idiomatically when they
                // ride on the preceding positional/option line. Re-arm
                // option_can_glue after each option emission: even if a
                // value-bearing keyword interrupted the header chain, the new
                // last line is itself an option line and the *next* option can
                // safely glue onto it.
                if option_can_glue && Self::append_to_last_flat_line(&mut lines, keyword, width) {
                    // glued onto previous line
                } else {
                    lines.push(LayoutLine {
                        indent: 0,
                        text: keyword.clone(),
                        has_comment: false,
                    });
                }
                index += 1;
                any_keyword_seen = true;
                option_can_glue = true;
                continue;
            }

            if schema.is_one_value_keyword(keyword) {
                let value = args
                    .get(index + 1)
                    .filter(|item| !item.is_comment)
                    .map(|item| item.text.clone());
                match value {
                    Some(value) => {
                        lines.push(LayoutLine {
                            indent: 0,
                            text: format!("{keyword} {value}"),
                            has_comment: false,
                        });
                    }
                    None => lines.push(LayoutLine {
                        indent: 0,
                        text: keyword.clone(),
                        has_comment: false,
                    }),
                }
                index += 1 + usize::from(args.get(index).is_some_and(|item| !item.is_comment));
                any_keyword_seen = true;
                option_can_glue = false;
                continue;
            }

            if let Some(subparser) = schema.subparser(keyword) {
                let start = index + 1;
                let end = self.next_subparser_end(args, start, schema, subparser);
                // Flatten the subparser's contents flush with the subparser
                // keyword when this is the only sub-block in the call —
                // install(EXPORT name NAMESPACE ... FILE ...) reads cleanly
                // without an extra indent level. Unlike multi-value keywords
                // we don't require prior keywords or positionals: if a call
                // starts directly with a subparser and nothing follows, the
                // sub-block has no sibling to align against and the extra
                // indent only widens the layout.
                let no_further_args = args[end..].iter().all(|a| a.is_comment);
                let flat = !any_indented_subparser && no_further_args;
                let before = lines.len();
                lines.extend(self.layout_subparser(
                    keyword,
                    &args[start..end],
                    subparser,
                    width,
                    flat,
                ));
                // Only mark the call as "had an indented sub-block" if this
                // subparser actually emitted indented children. A subparser
                // that fit inline or had no children doesn't introduce a
                // sub-block, so a later trailing subparser can still flatten.
                if lines[before..].iter().any(|line| line.indent > 0) {
                    any_indented_subparser = true;
                }
                index = end;
                any_keyword_seen = true;
                option_can_glue = false;
                continue;
            }

            if let Some(compound) = schema.compound_list_keyword(keyword) {
                let start = index + 1;
                let end = self.next_outer_keyword(args, start, schema);
                let header_len = self.compound_header_len(&args[start..end], &compound.headers);
                let mut prefix = vec![keyword.clone()];
                prefix.extend(
                    args[start..start + header_len]
                        .iter()
                        .map(|item| item.text.clone()),
                );
                let header = Self::join_values(&prefix);
                let value_indent = if Self::append_to_last_flat_line(&mut lines, &header, width) {
                    0
                } else {
                    lines.push(LayoutLine {
                        indent: 0,
                        text: header,
                        has_comment: false,
                    });
                    0
                };
                lines.extend(self.pack_arg_items(
                    value_indent,
                    Vec::new(),
                    &args[start + header_len..end],
                    width,
                    compound.list_type,
                ));
                index = end;
                any_keyword_seen = true;
                option_can_glue = false;
                continue;
            }

            if schema.is_multi_value_keyword(keyword) || schema.is_simple_keyword(keyword) {
                let start = index + 1;
                let end = self.next_outer_keyword(args, start, schema);
                let no_further_args = args[end..].iter().all(|a| a.is_comment);
                let list_type = schema.list_type(keyword);
                // Flat pulls the trailing keyword's value layer flush with the
                // keyword so the closing paren reads cleanly under a single
                // logical block — but it preserves relative offsets within the
                // block. A wrapped NPerLine pair's value, indented one level
                // under its key, stays one level under after flattening; only
                // the keyword/value-base distinction is collapsed. CommandArgv
                // is excluded entirely: its source-vertical groups model
                // distinct shell invocations, and flattening would erase the
                // keyword anchor that separates the program-and-args layer.
                // We don't require a prior positional or keyword here: a body
                // that starts directly with a multi-value keyword and has
                // nothing after it (cmake_language(EVAL CODE ...)) has no
                // sibling group to align against, and the extra indent only
                // widens the layout — symmetric with the sub-parser path.
                let flat_eligible = !matches!(list_type, ListType::CommandArgv);
                let flat = flat_eligible && !any_indented_subparser && no_further_args;
                let mut kw_lines = self.pack_arg_items(
                    0,
                    vec![keyword.clone()],
                    &args[start..end],
                    width,
                    list_type,
                );
                if flat {
                    for line in &mut kw_lines {
                        line.indent = line.indent.saturating_sub(1);
                    }
                } else if kw_lines.iter().any(|l| l.indent > 0) {
                    any_indented_subparser = true;
                }
                lines.extend(kw_lines);
                index = end;
                any_keyword_seen = true;
                option_can_glue = false;
                continue;
            }

            let start = index;
            let end = self.next_outer_keyword(args, start + 1, schema);
            lines.extend(self.pack_arg_items(
                0,
                Vec::new(),
                &args[start..end],
                width,
                schema.default_list_type,
            ));
            index = end;
            // Only end header mode once a real keyword has emitted. Positional
            // fall-through (e.g. when a leading comment broke the positional
            // loop) still lives in the header chain — the next option can
            // still glue back onto the just-emitted positional line.
            if any_keyword_seen {
                option_can_glue = false;
            }
        }

        lines
    }

    fn layout_mode_header(
        &self,
        mode_keyword: &str,
        schema: &FunctionSchema,
        args: &[ArgItem],
        budget: usize,
    ) -> (Vec<LayoutLine>, usize) {
        let positional_max = schema
            .positional
            .as_ref()
            .map(|p| p.max)
            .filter(|max| *max > 0)
            .unwrap_or(0);

        // A trailing comment that sits inside the mode header in the source —
        // either right after the mode keyword (e.g. `file(GLOB  # nolint`) or
        // right after the last positional (e.g. `file(GLOB var  # nolint`) —
        // belongs on the rendered header line, not on its own line below.
        // Line-based lint suppressions (`# nolint`) rely on this placement.
        let mut index = 0;
        let mut header_comment: Option<String> = None;
        if let Some(arg) = args.first()
            && arg.is_comment
            && arg.is_trailing_comment
        {
            header_comment = Some(arg.text.clone());
            index = 1;
        }

        let positional_start = index;
        let mut positionals: Vec<String> = Vec::new();
        while index < args.len()
            && (index - positional_start) < positional_max
            && !args[index].is_comment
            && !schema.is_any_keyword(&args[index].text)
        {
            positionals.push(args[index].text.clone());
            index += 1;
        }

        if !positionals.is_empty()
            && header_comment.is_none()
            && let Some(arg) = args.get(index)
            && arg.is_comment
            && arg.is_trailing_comment
        {
            header_comment = Some(arg.text.clone());
            index += 1;
        }

        let mut single = mode_keyword.to_string();
        for positional in &positionals {
            Self::append_value(&mut single, positional);
        }
        if let Some(comment) = &header_comment {
            single.push_str("  ");
            single.push_str(comment);
        }
        // Single positional bound to the mode is the conventional "header
        // continuation" — message(FATAL_ERROR "..."), string(CONCAT out ...),
        // file(WRITE path "..."), etc. Keep that pair glued even if it
        // overflows; the unit is more readable than a forced break, and the
        // overflow is bounded by the size of one positional.
        if positionals.len() <= 1 || single.len() <= budget {
            return (
                vec![LayoutLine {
                    indent: 0,
                    text: single,
                    has_comment: false,
                }],
                index,
            );
        }

        // Two or more positionals that don't fit: emit the mode keyword on the
        // open-paren line and chop each positional onto its own line at
        // arg_indent. The mode keyword alone may still overflow the budget
        // when the command name itself is long — but chopping its positionals
        // can only reduce overflow on subsequent lines, never introduce any.
        let mut keyword_line = mode_keyword.to_string();
        if let Some(comment) = &header_comment {
            keyword_line.push_str("  ");
            keyword_line.push_str(comment);
        }
        let mut lines = vec![LayoutLine {
            indent: 0,
            text: keyword_line,
            has_comment: false,
        }];
        for positional in positionals {
            lines.push(LayoutLine {
                indent: 0,
                text: positional,
                has_comment: false,
            });
        }
        (lines, index)
    }

    fn schema_without_positional(schema: &FunctionSchema) -> FunctionSchema {
        let mut schema = schema.clone();
        schema.positional = None;
        schema
    }

    fn append_to_last_flat_line(lines: &mut [LayoutLine], text: &str, width: usize) -> bool {
        let Some(last) = lines.last_mut() else {
            return false;
        };
        if last.indent != 0 || last.has_comment {
            return false;
        }
        let separator_width = Self::separator_width(&last.text, text);
        if last.text.len() + separator_width + text.len() > width {
            return false;
        }
        Self::append_value(&mut last.text, text);
        true
    }

    fn compound_header_len(&self, args: &[ArgItem], headers: &[Vec<String>]) -> usize {
        headers
            .iter()
            .find(|header| {
                header.len() <= args.len()
                    && args
                        .iter()
                        .zip(header.iter())
                        .all(|(arg, token)| !arg.is_comment && arg.text.eq_ignore_ascii_case(token))
            })
            .map_or(0, Vec::len)
    }

    // flat=true: experimental sub-parser-no-indent rule (spec §Recursive Sub-Parsers).
    // When both conditions hold (no prior indented sub-parser, no further args), contents
    // are emitted at the same indent as the sub-parser keyword rather than one level deeper.
    fn layout_subparser(
        &self,
        keyword: &str,
        args: &[ArgItem],
        schema: &FunctionSchema,
        width: usize,
        flat: bool,
    ) -> Vec<LayoutLine> {
        let child_width = width.saturating_sub(keyword.len());
        let child_lines = self.layout_schema_args(args, schema, child_width, child_width, 0);
        if child_lines.is_empty() {
            return vec![LayoutLine {
                indent: 0,
                text: keyword.to_string(),
                has_comment: false,
            }];
        }

        let has_comments = args.iter().any(|arg| arg.is_comment);
        // Try fully inline (all children fit on the keyword line)
        let inline =
            (!has_comments && child_lines.iter().all(|line| line.indent == 0)).then(|| {
                let mut text = keyword.to_string();
                for line in &child_lines {
                    Self::append_value(&mut text, &line.text);
                }
                text
            });
        if let Some(text) = inline
            && text.len() <= width
        {
            return vec![LayoutLine {
                indent: 0,
                text,
                has_comment: false,
            }];
        }

        // Flat mode: contents emit at the sub-parser keyword's own indent — no
        // extra indent level. Chopped-means-chopped applies here too: if we
        // didn't fit on one line above, the only place we glue a child onto
        // the keyword is the positional case (e.g. `EXPORT name` reads as one
        // logical header). Subparsers without a positional put the keyword on
        // its own line and every child flat below.
        if flat {
            let has_positional = schema.positional.as_ref().is_some_and(|p| p.max > 0);
            let first = &child_lines[0];
            let first_text = format!("{keyword} {}", first.text);
            if has_positional && first.indent == 0 && first_text.len() <= width {
                let mut lines = vec![LayoutLine {
                    indent: 0,
                    text: first_text,
                    has_comment: false,
                }];
                for line in &child_lines[1..] {
                    lines.push(LayoutLine {
                        indent: line.indent,
                        text: line.text.clone(),
                        has_comment: false,
                    });
                }
                return lines;
            }
            let mut lines = vec![LayoutLine {
                indent: 0,
                text: keyword.to_string(),
                has_comment: false,
            }];
            for line in child_lines {
                lines.push(LayoutLine {
                    indent: line.indent,
                    text: line.text,
                    has_comment: false,
                });
            }
            return lines;
        }

        // Non-flat, multi-line: chopped-means-chopped applies to subparsers
        // too. The only place we glue a child onto the keyword line is the
        // positional case — install(EXPORT <name> ...) reads naturally as
        // `EXPORT <name>` with the configuration keywords below. Subparsers
        // without a positional (e.g. install(TARGETS ... LIBRARY ...)) put
        // the keyword on its own line and every child below — otherwise the
        // first keyword child would ride with the subparser keyword while
        // later siblings wrap below, the hybrid form we don't emit.
        let mut lines = Vec::new();
        let has_positional = schema.positional.as_ref().is_some_and(|p| p.max > 0);
        let first = &child_lines[0];
        let first_text = format!("{keyword} {}", first.text);
        if has_positional && first.indent == 0 && first_text.len() <= width {
            lines.push(LayoutLine {
                indent: 0,
                text: first_text,
                has_comment: false,
            });
            for line in &child_lines[1..] {
                lines.push(LayoutLine {
                    indent: line.indent + 1,
                    text: line.text.clone(),
                    has_comment: false,
                });
            }
        } else {
            lines.push(LayoutLine {
                indent: 0,
                text: keyword.to_string(),
                has_comment: false,
            });
            for line in child_lines {
                lines.push(LayoutLine {
                    indent: line.indent + 1,
                    text: line.text,
                    has_comment: false,
                });
            }
        }
        lines
    }

    fn next_outer_keyword(&self, args: &[ArgItem], start: usize, schema: &FunctionSchema) -> usize {
        let mut index = start;
        while index < args.len()
            && (args[index].is_comment || !schema.is_any_keyword(&args[index].text))
        {
            index += 1;
        }
        index
    }

    fn next_subparser_end(
        &self,
        args: &[ArgItem],
        start: usize,
        parent: &FunctionSchema,
        child: &FunctionSchema,
    ) -> usize {
        let mut index = start;
        while index < args.len() {
            if parent.is_any_keyword(&args[index].text) && !child.is_any_keyword(&args[index].text)
            {
                break;
            }
            index += 1;
        }
        index
    }

    fn pack_values(
        &self,
        indent: usize,
        prefix: Vec<String>,
        values: Vec<String>,
        width: usize,
        list_type: ListType,
    ) -> Vec<LayoutLine> {
        if prefix.is_empty() && values.is_empty() {
            return Vec::new();
        }

        if list_type == ListType::Condition {
            return self.pack_condition(indent, prefix, values, width);
        }

        if list_type == ListType::CommandArgv {
            return self.pack_command_argv(indent, prefix, values, width);
        }

        if list_type == ListType::Path
            && values.len() == 1
            && !prefix.is_empty()
            && Self::joined_len_with_value(&prefix, &values[0]) <= width
        {
            let mut text = Self::join_values(&prefix);
            Self::append_value(&mut text, &values[0]);
            return vec![LayoutLine {
                indent,
                text,
                has_comment: false,
            }];
        }

        if list_type == ListType::Path && !values.is_empty() {
            let value_indent = if prefix.is_empty() {
                indent
            } else {
                indent + 1
            };
            let mut lines = Vec::new();
            if !prefix.is_empty() {
                lines.push(LayoutLine {
                    indent,
                    text: Self::join_values(&prefix),
                    has_comment: false,
                });
            }
            lines.extend(values.into_iter().map(|text| LayoutLine {
                indent: value_indent,
                text,
                has_comment: false,
            }));
            return lines;
        }

        if let ListType::NPerLine { n } = list_type {
            let n = n as usize;
            let value_indent = if prefix.is_empty() {
                indent
            } else {
                indent + 1
            };
            let mut lines = Vec::new();
            if !prefix.is_empty() {
                lines.push(LayoutLine {
                    indent,
                    text: Self::join_values(&prefix),
                    has_comment: false,
                });
            }
            if n <= 1 || values.is_empty() {
                lines.extend(values.into_iter().map(|text| LayoutLine {
                    indent: value_indent,
                    text,
                    has_comment: false,
                }));
                return lines;
            }
            // n >= 2: group into n-tuples; malformed (not multiple of n) falls through to Packed
            if values.len().is_multiple_of(n) {
                for chunk in values.chunks(n) {
                    let joined: String = chunk.join(" ");
                    if joined.len() <= width {
                        lines.push(LayoutLine {
                            indent: value_indent,
                            text: joined,
                            has_comment: false,
                        });
                    } else {
                        lines.push(LayoutLine {
                            indent: value_indent,
                            text: chunk[0].clone(),
                            has_comment: false,
                        });
                        for tok in &chunk[1..] {
                            lines.push(LayoutLine {
                                indent: value_indent + 1,
                                text: tok.clone(),
                                has_comment: false,
                            });
                        }
                    }
                }
                return lines;
            }
            // malformed: fall through to Packed
        }

        // Try single-line layout first: prefix + all values on one line.
        if !prefix.is_empty() {
            let mut single_line = Self::join_values(&prefix);
            for value in &values {
                Self::append_value(&mut single_line, value);
            }
            if single_line.len() <= width {
                return vec![LayoutLine {
                    indent,
                    text: single_line,
                    has_comment: false,
                }];
            }
        }

        // Multi-line: keyword (if any) sits alone; values pack below.
        let mut lines = Vec::new();
        let value_indent = if prefix.is_empty() {
            indent
        } else {
            lines.push(LayoutLine {
                indent,
                text: Self::join_values(&prefix),
                has_comment: false,
            });
            indent + 1
        };

        let mut current = String::new();
        for value in values {
            let separator_width = Self::separator_width(&current, &value);
            if current.is_empty() {
                current = value;
            } else if current.len() + separator_width + value.len() <= width {
                Self::append_value(&mut current, &value);
            } else {
                lines.push(LayoutLine {
                    indent: value_indent,
                    text: current,
                    has_comment: false,
                });
                current = value;
            }
        }

        if !current.is_empty() {
            lines.push(LayoutLine {
                indent: value_indent,
                text: current,
                has_comment: false,
            });
        }

        lines
    }

    fn pack_arg_items(
        &self,
        indent: usize,
        prefix: Vec<String>,
        items: &[ArgItem],
        width: usize,
        list_type: ListType,
    ) -> Vec<LayoutLine> {
        let mut lines = Vec::new();
        let mut current_prefix = prefix;
        let mut values = Vec::new();
        let mut current_indent = indent;
        let comment_indent = if current_prefix.is_empty() {
            indent
        } else {
            indent + 1
        };

        // A comment that splits the value list (any comment followed by more
        // values, trailing or standalone) forces multi-line layout: the comment
        // occupies its own line by definition, and the values it splits cannot
        // all share the keyword line anyway. Chopped-means-chopped requires the
        // keyword to sit alone too. Without this, pre-comment values can be
        // short enough to pack with the keyword, leaving later values wrapped
        // below in the hybrid form — and re-formatting can re-pack differently
        // than the first pass. A comment at the END of the value list is fine:
        // nothing follows to ambiguously merge, so keyword + value(s) + trailing
        // comment can stay single-line.
        let mut splitter_seen = false;
        for (idx, item) in items.iter().enumerate() {
            if item.is_comment {
                let more_values_after = items[idx + 1..].iter().any(|later| !later.is_comment);
                if more_values_after {
                    splitter_seen = true;
                    break;
                }
            }
        }
        if splitter_seen && !current_prefix.is_empty() {
            lines.push(LayoutLine {
                indent,
                text: Self::join_values(&current_prefix),
                has_comment: false,
            });
            current_prefix = Vec::new();
            current_indent = comment_indent;
        }

        for (idx, item) in items.iter().enumerate() {
            if item.is_comment {
                // A trailing comment attaches to the value just before it.
                // When the comment is mid-list AND the packed value run would
                // not have room for the trailing comment on its last line,
                // split the tail value into its own pack so the comment can
                // stay trailing on the tail's own line. Without this split,
                // push_comment falls back to inserting the comment *before*
                // the packed line, which reads as a leading comment for the
                // whole group rather than a note about the tail value. We
                // only split when needed because trailing-# escape hatches
                // (e.g. set(... COMMAND ${cmd}  #  DEPENDS ${dep}  # ...))
                // depend on the keyword/value pair staying together when the
                // comment marker fits at the end.
                let more_after = items[idx + 1..].iter().any(|later| !later.is_comment);
                let trailing_split_needed =
                    item.is_trailing_comment && more_after && !values.is_empty() && {
                        let packed = self.pack_value_items(
                            current_indent,
                            current_prefix.clone(),
                            values.clone(),
                            width,
                            list_type,
                            0,
                        );
                        packed
                            .last()
                            .is_some_and(|last| last.text.len() + 2 + item.text.len() > width)
                    };
                if trailing_split_needed {
                    let tail = values.pop().unwrap();
                    lines.extend(self.pack_value_items(
                        current_indent,
                        current_prefix,
                        values,
                        width,
                        list_type,
                        0,
                    ));
                    current_prefix = Vec::new();
                    values = vec![tail];
                }
                lines.extend(self.pack_value_items(
                    current_indent,
                    current_prefix,
                    values,
                    width,
                    list_type,
                    0,
                ));
                current_prefix = Vec::new();
                values = Vec::new();
                // A comment at the END of a keyword's args slice (no more
                // values follow) sits between this keyword group and whatever
                // follows in the outer call, not inside the value list. Place
                // it at the keyword's own indent so it visually aligns with
                // sibling keywords rather than with the value continuations.
                let more_after = items[idx + 1..].iter().any(|later| !later.is_comment);
                let this_comment_indent = if more_after { comment_indent } else { indent };
                current_indent = this_comment_indent;
                Self::push_comment(
                    &mut lines,
                    this_comment_indent,
                    &item.text,
                    item.is_trailing_comment,
                    width,
                );
            } else {
                values.push(item.clone());
            }
        }

        lines.extend(self.pack_value_items(
            current_indent,
            current_prefix,
            values,
            width,
            list_type,
            0,
        ));
        lines
    }

    fn pack_value_items(
        &self,
        indent: usize,
        prefix: Vec<String>,
        items: Vec<ArgItem>,
        width: usize,
        list_type: ListType,
        consumed_positionals: usize,
    ) -> Vec<LayoutLine> {
        if list_type == ListType::CommandArgv {
            return self.pack_command_argv_items(indent, prefix, items, width);
        }

        let inferred_list_type =
            if self.should_preserve_vertical_list(&items, list_type, consumed_positionals) {
                ListType::one_per_line()
            } else {
                list_type
            };
        let values = items.into_iter().map(|item| item.text).collect();
        self.pack_values(indent, prefix, values, width, inferred_list_type)
    }

    fn should_preserve_vertical_list(
        &self,
        items: &[ArgItem],
        list_type: ListType,
        consumed_positionals: usize,
    ) -> bool {
        if list_type == ListType::CommandArgv
            || list_type == ListType::Condition
            || items.is_empty()
        {
            return false;
        }

        let threshold = self.config.source_vertical_list_threshold;
        if threshold < 0 {
            return false;
        }
        if threshold == 0 {
            return true;
        }

        let threshold = threshold as usize;
        let adjusted_threshold = threshold.saturating_sub(consumed_positionals).max(2);
        if items.len() < adjusted_threshold {
            return false;
        }

        let mut previous_line = None;
        for item in items.iter().take(adjusted_threshold) {
            let Some(source_line) = item.source_line else {
                return false;
            };
            if previous_line == Some(source_line) {
                return false;
            }
            previous_line = Some(source_line);
        }
        true
    }

    fn pack_command_argv_items(
        &self,
        indent: usize,
        prefix: Vec<String>,
        items: Vec<ArgItem>,
        width: usize,
    ) -> Vec<LayoutLine> {
        let source_groups = self.source_line_groups(&items);
        if self.should_preserve_command_argv_layout(&source_groups) {
            let mut lines = Vec::new();
            let value_indent = if prefix.is_empty() {
                indent
            } else {
                lines.push(LayoutLine {
                    indent,
                    text: Self::join_values(&prefix),
                    has_comment: false,
                });
                indent + 1
            };
            for group in source_groups {
                lines.extend(self.pack_command_argv(value_indent, Vec::new(), group, width));
            }
            return lines;
        }

        let values = items.into_iter().map(|item| item.text).collect();
        self.pack_command_argv(indent, prefix, values, width)
    }

    fn source_line_groups(&self, items: &[ArgItem]) -> Vec<Vec<String>> {
        let mut groups = Vec::new();
        let mut current_line = None;
        let mut current = Vec::new();

        for item in items {
            if current_line.is_some() && current_line != item.source_line {
                groups.push(current);
                current = Vec::new();
            }
            current_line = item.source_line;
            current.push(item.text.clone());
        }

        if !current.is_empty() {
            groups.push(current);
        }
        groups
    }

    fn should_preserve_command_argv_layout(&self, groups: &[Vec<String>]) -> bool {
        let threshold = self.config.source_vertical_list_threshold;
        if threshold < 0 || groups.len() <= 1 {
            return false;
        }
        threshold == 0 || groups.len() >= threshold as usize
    }

    fn pack_command_argv(
        &self,
        indent: usize,
        prefix: Vec<String>,
        values: Vec<String>,
        width: usize,
    ) -> Vec<LayoutLine> {
        if prefix.is_empty() {
            // Wraps stay at the caller's indent so that re-formatting an already-
            // formatted file is a fixpoint: when an overlong source-vertical
            // command group has to wrap, the wrapped lines become new source
            // lines on the next pass and must not drift to a deeper indent.
            let values = Self::group_command_argv(values);
            let mut lines = Vec::new();
            let mut current = String::new();
            for value in values {
                let separator_width = Self::separator_width(&current, &value);
                if current.is_empty() {
                    current = value;
                } else if current.len() + separator_width + value.len() <= width {
                    Self::append_value(&mut current, &value);
                } else {
                    lines.push(LayoutLine {
                        indent,
                        text: current,
                        has_comment: false,
                    });
                    current = value;
                }
            }
            if !current.is_empty() {
                lines.push(LayoutLine {
                    indent,
                    text: current,
                    has_comment: false,
                });
            }
            return lines;
        }

        // Try single-line layout first: prefix + all grouped values on one line.
        let grouped = Self::group_command_argv(values);
        let mut single_line = Self::join_values(&prefix);
        for value in &grouped {
            Self::append_value(&mut single_line, value);
        }
        if single_line.len() <= width {
            return vec![LayoutLine {
                indent,
                text: single_line,
                has_comment: false,
            }];
        }

        // Multi-line: keyword alone, then values pack below. Continuation lines
        // stay at value_indent (not deeper) so the layout is a fixpoint under
        // re-formatting and visually consistent with the source-vertical path.
        let mut lines = vec![LayoutLine {
            indent,
            text: Self::join_values(&prefix),
            has_comment: false,
        }];
        let value_indent = indent + 1;
        let mut current = String::new();
        for value in grouped {
            let separator_width = Self::separator_width(&current, &value);
            if current.is_empty() {
                current = value;
            } else if current.len() + separator_width + value.len() <= width {
                Self::append_value(&mut current, &value);
            } else {
                lines.push(LayoutLine {
                    indent: value_indent,
                    text: current,
                    has_comment: false,
                });
                current = value;
            }
        }
        if !current.is_empty() {
            lines.push(LayoutLine {
                indent: value_indent,
                text: current,
                has_comment: false,
            });
        }
        lines
    }

    fn pack_condition(
        &self,
        indent: usize,
        prefix: Vec<String>,
        values: Vec<String>,
        width: usize,
    ) -> Vec<LayoutLine> {
        let mut lines = Vec::new();
        let mut current_line = Self::join_values(&prefix);
        let mut current_indent = indent;
        let mut paren_depth = 0;

        for value in values {
            let is_op = value == "AND" || value == "OR";
            let is_close = value == ")";

            if is_close && paren_depth > 0 {
                paren_depth -= 1;
            }

            let break_before = is_op && !current_line.is_empty();

            let separator_width = Self::separator_width(&current_line, &value);

            if break_before
                || (!current_line.is_empty()
                    && current_line.len() + separator_width + value.len() > width)
            {
                lines.push(LayoutLine {
                    indent: current_indent,
                    text: current_line,
                    has_comment: false,
                });
                current_line = String::new();
                current_indent = indent + paren_depth + if is_op { 0 } else { 1 };
            }

            if current_line.is_empty() {
                current_line = value.clone();
            } else {
                Self::append_value(&mut current_line, &value);
            }

            if value == "(" {
                paren_depth += 1;
            }
        }

        if !current_line.is_empty() {
            lines.push(LayoutLine {
                indent: current_indent,
                text: current_line,
                has_comment: false,
            });
        }

        lines
    }

    fn group_command_argv(values: Vec<String>) -> Vec<String> {
        let mut grouped = Vec::new();
        let mut iter = values.into_iter().peekable();
        let mut after_separator = false;

        while let Some(value) = iter.next() {
            if value == "--" {
                after_separator = true;
                grouped.push(value);
                continue;
            }

            if !after_separator
                && Self::starts_flag_value_pair(&value)
                && iter
                    .peek()
                    .is_some_and(|next| Self::can_be_flag_value(next))
            {
                let next = iter.next().expect("peeked value exists");
                grouped.push(format!("{value} {next}"));
            } else {
                grouped.push(value);
            }
        }

        grouped
    }

    fn starts_flag_value_pair(value: &str) -> bool {
        value.starts_with('-') && value != "-" && value != "--" && !value.contains('=')
    }

    fn can_be_flag_value(value: &str) -> bool {
        !value.starts_with('-') || value == "-" || Self::is_negative_number(value)
    }

    fn is_negative_number(value: &str) -> bool {
        value.starts_with('-') && value != "-" && value.parse::<f64>().is_ok()
    }

    fn join_values(values: &[String]) -> String {
        let mut joined = String::new();
        for value in values {
            Self::append_value(&mut joined, value);
        }
        joined
    }

    fn joined_len_with_value(values: &[String], value: &str) -> usize {
        let joined = Self::join_values(values);
        joined.len() + Self::separator_width(&joined, value) + value.len()
    }

    fn append_value(line: &mut String, value: &str) {
        if line.is_empty() || value == ")" || line.ends_with('(') {
            line.push_str(value);
        } else {
            line.push(' ');
            line.push_str(value);
        }
    }

    fn separator_width(line: &str, value: &str) -> usize {
        if line.is_empty() || value == ")" || line.ends_with('(') {
            0
        } else {
            1
        }
    }

    fn push_comment(
        lines: &mut Vec<LayoutLine>,
        indent: usize,
        comment: &str,
        trailing: bool,
        width: usize,
    ) {
        if trailing
            && let Some(last) = lines.last_mut()
            && !last.text.starts_with('#')
        {
            if last.text.len() + 2 + comment.len() <= width {
                last.text.push_str("  ");
                last.text.push_str(comment);
                last.has_comment = true;
            } else {
                let comment_indent = last.indent;
                let index = lines.len().saturating_sub(1);
                lines.insert(
                    index,
                    LayoutLine {
                        indent: comment_indent,
                        text: comment.to_string(),
                        has_comment: true,
                    },
                );
            }
            return;
        }

        lines.push(LayoutLine {
            indent,
            text: comment.to_string(),
            has_comment: true,
        });
    }
}
