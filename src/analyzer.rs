use crate::cst::{CommandNode, SyntaxNode, SyntaxToken};
use crate::schema::{FunctionSchema, SchemaRegistry};
use crate::syntax::SyntaxKind;
use std::collections::HashMap;

pub struct AnalysisResult {
    pub name: String,
    pub schema: FunctionSchema,
    pub is_definitive: bool,
}

pub struct Analyzer;

impl Analyzer {
    pub fn analyze_file(root: &SyntaxNode) -> Vec<AnalysisResult> {
        let commands: Vec<SyntaxNode> = root
            .children()
            .filter(|n| n.kind() == SyntaxKind::COMMAND)
            .collect();
        analyze_commands(&commands)
    }
}

fn analyze_commands(commands: &[SyntaxNode]) -> Vec<AnalysisResult> {
    let mut results = Vec::new();
    let mut i = 0;

    while i < commands.len() {
        let cmd = &commands[i];
        let name = match command_name(cmd) {
            Some(n) => n,
            None => {
                i += 1;
                continue;
            }
        };

        let name_str = name.text();
        let is_function = name_str.eq_ignore_ascii_case("function");
        let is_macro = name_str.eq_ignore_ascii_case("macro");

        if is_function || is_macro {
            let end_keyword = if is_function {
                "endfunction"
            } else {
                "endmacro"
            };
            let args = command_args(cmd);
            let fn_name = match args.first() {
                Some(n) => n.to_lowercase(),
                None => {
                    i += 1;
                    continue;
                }
            };

            // Find matching end keyword tracking nesting depth
            let mut depth = 1usize;
            let mut j = i + 1;
            while j < commands.len() {
                let inner = command_name(&commands[j]);
                let inner_str = inner.as_ref().map(|t| t.text()).unwrap_or("");
                if inner_str.eq_ignore_ascii_case(name_str) {
                    depth += 1;
                } else if inner_str.eq_ignore_ascii_case(end_keyword) {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                j += 1;
            }

            if depth != 0 {
                i += 1;
                continue;
            }

            let body = &commands[i + 1..j];
            let locals = collect_local_vars(body);

            for bcmd in body {
                let bname = match command_name(bcmd) {
                    Some(n) => n,
                    None => continue,
                };
                if bname.text().eq_ignore_ascii_case("cmake_parse_arguments") {
                    let bargs = command_args(bcmd);
                    if let Some((schema, is_definitive)) =
                        parse_cmake_parse_arguments(&bargs, &locals)
                    {
                        results.push(AnalysisResult {
                            name: fn_name.clone(),
                            schema,
                            is_definitive,
                        });
                        break;
                    }
                }
            }

            let nested = analyze_commands(body);
            results.extend(nested);

            i = j + 1;
        } else {
            i += 1;
        }
    }

    results
}

fn command_name(cmd: &SyntaxNode) -> Option<SyntaxToken> {
    CommandNode::cast(cmd.clone())?.name()
}

fn command_args(cmd: &SyntaxNode) -> Vec<String> {
    match CommandNode::cast(cmd.clone()) {
        Some(node) => node
            .args()
            .into_iter()
            .filter(|t| t.kind() != SyntaxKind::COMMENT)
            .map(|t| t.text().to_string())
            .collect(),
        None => vec![],
    }
}

// Maps variable name → (resolved values, is_definitive).
// is_definitive is false if any part of the variable's value came from
// an unresolvable reference.
type Locals = HashMap<String, (Vec<String>, bool)>;

// Iter vars: current foreach loop variable bindings (VAR → single current value).
type IterVars = HashMap<String, String>;

fn collect_local_vars(body: &[SyntaxNode]) -> Locals {
    let mut locals = Locals::new();
    process_commands_for_locals(body, &IterVars::new(), &mut locals);
    locals
}

fn process_commands_for_locals(cmds: &[SyntaxNode], iter_vars: &IterVars, locals: &mut Locals) {
    let mut i = 0;
    while i < cmds.len() {
        let name_tok = command_name(&cmds[i]);
        let name = name_tok.as_ref().map(|t| t.text()).unwrap_or("");
        let args = command_args(&cmds[i]);

        if name.eq_ignore_ascii_case("set") && !args.is_empty() {
            let var = args[0].clone();
            let mut vals = Vec::new();
            let mut ok = true;
            for raw in &args[1..] {
                let (resolved, r) = resolve_arg(raw, locals, iter_vars);
                if !r {
                    ok = false;
                }
                vals.extend(resolved);
            }
            locals.insert(var, (vals, ok));
            i += 1;
        } else if name.eq_ignore_ascii_case("list")
            && args
                .first()
                .is_some_and(|s| s.eq_ignore_ascii_case("APPEND"))
            && args.len() >= 2
        {
            let list_var = args[1].clone();
            let mut new_vals = Vec::new();
            let mut all_ok = true;
            for raw in &args[2..] {
                let (resolved, r) = resolve_arg(raw, locals, iter_vars);
                if !r {
                    all_ok = false;
                }
                new_vals.extend(resolved);
            }
            let entry = locals.entry(list_var).or_insert_with(|| (Vec::new(), true));
            entry.0.extend(new_vals);
            if !all_ok {
                entry.1 = false;
            }
            i += 1;
        } else if name.eq_ignore_ascii_case("foreach") {
            if let Some((loop_var, items, end_idx)) =
                try_parse_foreach_items(&args, cmds, i, locals, iter_vars)
            {
                let loop_body = &cmds[i + 1..end_idx];
                for item in items {
                    let mut new_iter = iter_vars.clone();
                    new_iter.insert(loop_var.clone(), item);
                    process_commands_for_locals(loop_body, &new_iter, locals);
                }
                i = end_idx + 1;
            } else if let Some(end_idx) = find_endforeach(cmds, i) {
                // Non-ITEMS foreach: skip the body entirely
                i = end_idx + 1;
            } else {
                i += 1;
            }
        } else {
            i += 1;
        }
    }
}

fn try_parse_foreach_items(
    args: &[String],
    cmds: &[SyntaxNode],
    foreach_idx: usize,
    locals: &Locals,
    iter_vars: &IterVars,
) -> Option<(String, Vec<String>, usize)> {
    // foreach(VAR IN ITEMS item1 item2 ...)
    let var_name = args.first()?.clone();
    if args.get(1)?.to_uppercase() != "IN" {
        return None;
    }
    if args.get(2)?.to_uppercase() != "ITEMS" {
        return None;
    }

    let items: Vec<String> = args[3..]
        .iter()
        .flat_map(|raw| resolve_arg(raw, locals, iter_vars).0)
        .collect();

    let end_idx = find_endforeach(cmds, foreach_idx)?;
    Some((var_name, items, end_idx))
}

fn find_endforeach(cmds: &[SyntaxNode], start: usize) -> Option<usize> {
    let mut depth = 1usize;
    let mut j = start + 1;
    while j < cmds.len() {
        let n_tok = command_name(&cmds[j]);
        let n = n_tok.as_ref().map(|t| t.text()).unwrap_or("");
        if n.eq_ignore_ascii_case("foreach") {
            depth += 1;
        } else if n.eq_ignore_ascii_case("endforeach") {
            depth -= 1;
            if depth == 0 {
                return Some(j);
            }
        }
        j += 1;
    }
    None
}

fn resolve_arg(raw: &str, locals: &Locals, iter_vars: &IterVars) -> (Vec<String>, bool) {
    // Quoted string
    if raw.starts_with('"') && raw.ends_with('"') && raw.len() >= 2 {
        let inner = &raw[1..raw.len() - 1];
        // Simple: entire inner content is a single ${VAR} reference
        if let Some(rest) = inner.strip_prefix("${")
            && let Some(var_name) = rest.strip_suffix('}')
            && !var_name.contains("${")
        {
            return lookup_var(var_name, locals, iter_vars);
        }
        // General: interpolate embedded ${VAR} refs, then split on `;`
        let (interp, ok) = interpolate_string(inner, locals, iter_vars);
        let parts = split_cmake_list(&interp);
        return (parts, ok);
    }

    // Unquoted ${VAR} reference (whole token)
    if let Some(rest) = raw.strip_prefix("${")
        && let Some(var_name) = rest.strip_suffix('}')
        && !var_name.contains("${")
    {
        return lookup_var(var_name, locals, iter_vars);
    }

    // Plain unquoted token: split on `;`
    (split_cmake_list(raw), true)
}

fn lookup_var(var_name: &str, locals: &Locals, iter_vars: &IterVars) -> (Vec<String>, bool) {
    if let Some(val) = iter_vars.get(var_name) {
        return (vec![val.clone()], true);
    }
    if let Some((vals, ok)) = locals.get(var_name) {
        return (vals.clone(), *ok);
    }
    (vec![], false)
}

fn interpolate_string(s: &str, locals: &Locals, iter_vars: &IterVars) -> (String, bool) {
    let mut result = String::new();
    let mut ok = true;
    let mut remaining = s;

    while let Some(start) = remaining.find("${") {
        result.push_str(&remaining[..start]);
        let rest = &remaining[start + 2..];
        if let Some(end) = rest.find('}') {
            let var_name = &rest[..end];
            if let Some(val) = iter_vars.get(var_name) {
                result.push_str(val);
            } else if let Some((vals, resolved)) = locals.get(var_name) {
                result.push_str(&vals.join(";"));
                if !resolved {
                    ok = false;
                }
            } else {
                ok = false;
            }
            remaining = &rest[end + 1..];
        } else {
            // Malformed ${...
            result.push_str("${");
            remaining = rest;
            ok = false;
        }
    }
    result.push_str(remaining);
    (result, ok)
}

fn split_cmake_list(s: &str) -> Vec<String> {
    s.split(';')
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect()
}

fn parse_cmake_parse_arguments(args: &[String], locals: &Locals) -> Option<(FunctionSchema, bool)> {
    let empty_iter = IterVars::new();
    let (opt_idx, one_idx, multi_idx) = if args.first().map(|s| s.as_str()) == Some("PARSE_ARGV") {
        (3, 4, 5)
    } else {
        (1, 2, 3)
    };

    if args.len() <= multi_idx {
        return None;
    }

    let (options, r1) = resolve_arg(&args[opt_idx], locals, &empty_iter);
    let (one_value, r2) = resolve_arg(&args[one_idx], locals, &empty_iter);
    let (multi_value, r3) = resolve_arg(&args[multi_idx], locals, &empty_iter);

    Some((
        FunctionSchema {
            simple_keywords: vec![],
            options,
            one_value_keywords: one_value,
            multi_value_keywords: multi_value,
            ..Default::default()
        },
        r1 && r2 && r3,
    ))
}

pub fn results_to_registry(results: Vec<AnalysisResult>) -> SchemaRegistry {
    let functions = results.into_iter().map(|r| (r.name, r.schema)).collect();
    SchemaRegistry { functions }
}
