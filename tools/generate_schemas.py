import os
import re
import subprocess

# Determine repo root path relative to this script
script_dir = os.path.dirname(os.path.abspath(__file__))
repo_root = os.path.abspath(os.path.join(script_dir, ".."))

help_command_dir = os.path.join(repo_root, "tests/data/cmake/Help/command")
help_module_dir = os.path.join(repo_root, "tests/data/cmake/Help/module")
output_file = os.path.join(repo_root, "src/generated_schemas.rs")


def extract_signature_blocks_from_text(text, cmd_name):
    sigs = []
    # 1. If signature directives exist, extract from all of them
    matches = list(
        re.finditer(r"\.\.\s+signature::\s*(.*?)\n((?:\s+\S.*\n|\n)*)", text)
    )
    if matches:
        for m in matches:
            inline_val = m.group(1).strip()
            indented_val = m.group(2)
            if inline_val:
                pat = r"\b" + re.escape(cmd_name) + r"\s*\((.*)\)"
                m2 = re.match(pat, inline_val, re.IGNORECASE)
                if m2:
                    sigs.append(m2.group(1).strip())
            if indented_val:
                for line in indented_val.splitlines():
                    line_strip = line.strip()
                    if line_strip.startswith(":") or line_strip.startswith(".."):
                        continue
                    pat = r"\b" + re.escape(cmd_name) + r"\s*\((.*?)\)"
                    m2 = re.search(pat, line_strip, re.IGNORECASE)
                    if m2:
                        sigs.append(m2.group(1).strip())
        if sigs:
            return sigs

    # 2. Otherwise, extract ONLY from the first .. code-block:: cmake block
    m = re.search(r"\.\.\s+code-block::\s+cmake\s*\n((?:\s+\S.*\n|\n)+)", text)
    if m:
        block_text = m.group(1)
        lines = [line.strip() for line in block_text.splitlines()]
        cleaned = " ".join(lines).strip()
        pattern = r"\b" + re.escape(cmd_name) + r"\s*\((.*?)\)"
        cmd_matches = re.findall(pattern, cleaned, re.IGNORECASE)
        for sig in cmd_matches:
            sigs.append(sig.strip())

    return sigs


def parse_signatures_for_doc(text, default_cmd_name=None):
    matches = list(re.finditer(r"\.\.\s+command::\s*(\w+)", text))
    results = []
    if matches:
        for i, m in enumerate(matches):
            cmd_name = m.group(1)
            start = m.start()
            end = matches[i + 1].start() if i + 1 < len(matches) else len(text)
            section_text = text[start:end]
            sigs = extract_signature_blocks_from_text(section_text, cmd_name)
            results.append((cmd_name, sigs))
    elif default_cmd_name:
        sigs = extract_signature_blocks_from_text(text, default_cmd_name)
        results.append((default_cmd_name, sigs))
    return results


def parse_signature(sig):
    options = set()
    one_value = set()
    multi_value = set()

    # Normalize/simplify parenthesized and angle-bracketed alternatives like (AnyNewerVersion|SameMajorVersion)
    # or <AnyNewerVersion|SameMajorVersion> to avoid treating alternatives as separate arguments.
    sig = re.sub(r"[\(<]\s*[^)>]+\s*\|\s*[^)>]+\s*[\)>]", "<alternative>", sig)

    # Extract top level brackets
    bracket_contents = []
    stack = 0
    start = -1
    for i, c in enumerate(sig):
        if c == "[":
            if stack == 0:
                start = i
            stack += 1
        elif c == "]":
            if stack > 0:
                stack -= 1
                if stack == 0:
                    bracket_contents.append(sig[start + 1 : i])

    # Also parse what is outside top-level brackets
    outside_parts = []
    last_idx = 0
    stack = 0
    for i, c in enumerate(sig):
        if c == "[":
            if stack == 0:
                outside_parts.append(sig[last_idx:i])
            stack += 1
        elif c == "]":
            if stack > 0:
                stack -= 1
                if stack == 0:
                    last_idx = i + 1
    if last_idx < len(sig):
        outside_parts.append(sig[last_idx:])
    outside = " ".join(outside_parts)

    token_pat = r"[a-zA-Z0-9_<>@#\-\.]+(?:\.\.\.)?|\.\.\."

    outside_tokens = re.findall(token_pat, outside)
    positional_min = 0
    positional_max = 0
    has_varargs = False

    i = 0
    while i < len(outside_tokens):
        token = outside_tokens[i]
        if token.endswith("...") or token == "...":
            has_varargs = True
        clean_token = token[:-3] if token.endswith("...") else token
        if not clean_token:
            i += 1
            continue

        is_keyword = clean_token.isupper() and not (
            clean_token.startswith("<") and clean_token.endswith(">")
        )
        if is_keyword:
            # Look ahead to see how many values this keyword consumes
            kw_vals = []
            j = i + 1
            while j < len(outside_tokens):
                next_tok = outside_tokens[j]
                next_tok_clean = next_tok[:-3] if next_tok.endswith("...") else next_tok
                # If we encounter another keyword, stop
                if next_tok_clean.isupper() and not (
                    next_tok_clean.startswith("<") and next_tok_clean.endswith(">")
                ):
                    break
                kw_vals.append(next_tok)
                j += 1

            if not kw_vals:
                options.add(clean_token)
            elif len(kw_vals) == 1:
                val_tok = kw_vals[0]
                if val_tok.endswith("...") or val_tok == "...":
                    multi_value.add(clean_token)
                else:
                    one_value.add(clean_token)
            else:
                multi_value.add(clean_token)
            i = j
        else:
            positional_min += 1
            positional_max += 1
            i += 1

    for content in bracket_contents:
        parts = [p.strip() for p in content.split("|")]
        for part in parts:
            tokens = re.findall(token_pat, part)
            if not tokens:
                continue

            for token in tokens:
                if token.endswith("...") or token == "...":
                    has_varargs = True

            first_token = tokens[0]
            first_token_clean = (
                first_token[:-3] if first_token.endswith("...") else first_token
            )
            is_keyword_start = first_token_clean.isupper() and not (
                first_token_clean.startswith("<") and first_token_clean.endswith(">")
            )

            if is_keyword_start:
                keyword = first_token_clean
                clean_tokens = []
                for t in tokens:
                    tc = t[:-3] if t.endswith("...") else t
                    if tc:
                        clean_tokens.append((t, tc))

                if len(clean_tokens) == 1:
                    options.add(keyword)
                elif len(clean_tokens) == 2:
                    val_token, val_token_clean = clean_tokens[1]
                    if val_token.endswith("...") or val_token == "...":
                        multi_value.add(keyword)
                    else:
                        one_value.add(keyword)
                else:
                    if any(
                        t.endswith("...") or t == "..." for t, _ in clean_tokens[1:]
                    ):
                        multi_value.add(keyword)
                    else:
                        multi_value.add(keyword)
            else:
                for token in tokens:
                    token_clean = token[:-3] if token.endswith("...") else token
                    if not token_clean:
                        continue
                    if not token_clean.isupper() or (
                        token_clean.startswith("<") and token_clean.endswith(">")
                    ):
                        positional_max += 1

    return {
        "options": sorted(list(options)),
        "one_value": sorted(list(one_value)),
        "multi_value": sorted(list(multi_value)),
        "positional_min": positional_min,
        "positional_max": 0 if has_varargs else positional_max,
    }


def combine_signatures(sigs):
    combined = {
        "options": set(),
        "one_value": set(),
        "multi_value": set(),
        "positional_min": 0,
        "positional_max": 0,
    }

    if not sigs:
        return combined

    has_unbounded = False
    mins = []
    maxes = []

    for sig in sigs:
        parsed = parse_signature(sig)
        combined["options"].update(parsed["options"])
        combined["one_value"].update(parsed["one_value"])
        combined["multi_value"].update(parsed["multi_value"])

        mins.append(parsed["positional_min"])
        if parsed["positional_max"] == 0:
            has_unbounded = True
        else:
            maxes.append(parsed["positional_max"])

    combined["positional_min"] = min(mins) if mins else 0
    if has_unbounded or not maxes:
        combined["positional_max"] = 0
    else:
        combined["positional_max"] = max(maxes)

    return {
        "options": sorted(list(combined["options"])),
        "one_value": sorted(list(combined["one_value"])),
        "multi_value": sorted(list(combined["multi_value"])),
        "positional_min": combined["positional_min"],
        "positional_max": combined["positional_max"],
    }


def merge_schemas(schema1, schema2):
    if not schema1:
        return schema2
    if not schema2:
        return schema1

    options = set(schema1["options"]) | set(schema2["options"])
    one_value = set(schema1["one_value"]) | set(schema2["one_value"])
    multi_value = set(schema1["multi_value"]) | set(schema2["multi_value"])

    min1 = schema1["positional_min"]
    min2 = schema2["positional_min"]
    max1 = schema1["positional_max"]
    max2 = schema2["positional_max"]

    combined_min = min(min1, min2)
    if max1 == 0 or max2 == 0:
        combined_max = 0
    else:
        combined_max = max(max1, max2)

    return {
        "options": sorted(list(options)),
        "one_value": sorted(list(one_value)),
        "multi_value": sorted(list(multi_value)),
        "positional_min": combined_min,
        "positional_max": combined_max,
    }


def main():
    schemas = {}

    # 1. Scan standard commands
    if os.path.exists(help_command_dir):
        for f in sorted(os.listdir(help_command_dir)):
            if f.endswith(".rst"):
                cmd_name = f[:-4]
                path = os.path.join(help_command_dir, f)
                with open(path, "r", encoding="utf-8") as file:
                    text = file.read()
                res = parse_signatures_for_doc(text, cmd_name)
                for name, sigs in res:
                    name_lower = name.lower()
                    combined = combine_signatures(sigs)
                    schemas[name_lower] = merge_schemas(
                        schemas.get(name_lower), combined
                    )

    # 2. Scan CMake modules
    if os.path.exists(help_module_dir):
        for f in sorted(os.listdir(help_module_dir)):
            if f.endswith(".rst"):
                path = os.path.join(help_module_dir, f)
                with open(path, "r", encoding="utf-8") as file:
                    text = file.read()
                m = re.search(r"\.\.\s+cmake-module::\s*(\S+)", text)
                if m:
                    rel_path = m.group(1)
                    cmake_path = os.path.normpath(
                        os.path.join(os.path.dirname(path), rel_path)
                    )
                    if os.path.exists(cmake_path):
                        with open(cmake_path, "r", encoding="utf-8") as cmake_file:
                            cmake_content = cmake_file.read()
                        brackets = re.findall(
                            r"#\[(=*)\[\.rst:\n(.*?)\n#\]\1\]", cmake_content, re.DOTALL
                        )
                        combined_text = "\n".join(b[1] for b in brackets)
                        res = parse_signatures_for_doc(combined_text)
                        for name, sigs in res:
                            name_lower = name.lower()
                            combined = combine_signatures(sigs)
                            schemas[name_lower] = merge_schemas(
                                schemas.get(name_lower), combined
                            )

    # Write generated_schemas.rs
    with open(output_file, "w", encoding="utf-8") as out:
        out.write("""// This file is autogenerated by tools/generate_schemas.py.
// Do not edit manually.

use crate::schema::{FunctionSchema, PositionalSpec, SchemaRegistry};
use std::collections::HashMap;

pub fn generated_schemas() -> SchemaRegistry {
    let mut functions = HashMap::new();
""")

        for cmd_lower in sorted(schemas.keys()):
            schema = schemas[cmd_lower]
            opts = ", ".join(f'"{o}".into()' for o in schema["options"])
            ov = ", ".join(f'"{o}".into()' for o in schema["one_value"])
            mv = ", ".join(f'"{o}".into()' for o in schema["multi_value"])

            pos_spec = ""
            if schema["positional_min"] > 0 or schema["positional_max"] > 0:
                pos_spec = f"\n            positional: Some(PositionalSpec {{ min: {schema['positional_min']}, max: {schema['positional_max']} }}),"

            opts_spec = f"\n            options: vec![{opts}]," if opts else ""
            ov_spec = f"\n            one_value_keywords: vec![{ov}]," if ov else ""
            mv_spec = f"\n            multi_value_keywords: vec![{mv}]," if mv else ""

            out.write(f"""    functions.insert(
        "{cmd_lower}".to_string(),
        FunctionSchema {{{pos_spec}{opts_spec}{ov_spec}{mv_spec}
            ..Default::default()
        }},
    );
""")

        out.write("""
    SchemaRegistry { functions }
}
""")

    print(f"Generated {len(schemas)} schemas in {output_file}")

    # Run rustfmt
    try:
        subprocess.run(["rustfmt", output_file], check=True)
        print("Formats generated code with rustfmt")
    except Exception as e:
        print(f"Warning: rustfmt failed: {e}")


if __name__ == "__main__":
    main()
