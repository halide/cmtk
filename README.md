# cmtk 🛠️

[![Rust](https://img.shields.io/badge/rust-2024-orange.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)

`cmtk` (CMake Toolkit) is a high-performance, schema-driven CMake code formatter and analyzer built in Rust. It utilizes a lossless Concrete Syntax Tree (CST) pipeline to guarantee that formatting operations never destroy comments, whitespace, or any other source details.

Unlike general-purpose formatters or simple regular-expression tools, `cmtk` formats CMake files according to known function signature schemas. It can automatically discover schemas for custom CMake functions/macros by analyzing the codebase (e.g. looking for uses of `cmake_parse_arguments`), allowing for precise, semantic layouts without tedious manual configuration.

---

## Key Features

- **Lossless CST Architecture**: Built on top of the [Rowan](https://github.com/rust-analyzer/rowan) library (used in `rust-analyzer`) and the [Logos](https://github.com/maciejhirsz/logos) lexer. The formatting pipeline guarantees `tree.to_string() == original_source` prior to styling, meaning comments, bracket arguments, and trivia are fully preserved.
- **Schema-Driven Formatting**: Employs structural layouts based on CMake commands' signatures. It distinguishes positional arguments, options (flags), one-value keywords, and list keywords.
- **Auto-Schema Discovery**: The `cmtk scan` subcommand inspects your macros and functions, parsing their inner `cmake_parse_arguments` calls to automatically generate configuration schemas.
- **Advanced List Wrapping Rules**:
  - **Packed**: Packed by width (default for target names, components).
  - **Path**: One-per-line formatting for file lists, keeping singletons inline.
  - **Command Arguments (`command_argv`)**: Retains executable and initial flags on the same line, wrapping trailing arguments.
  - **N per Line**: Groups arguments into sets of `n` (e.g., `n = 2` for properties key-value pairs).
- **Searchability Preservation**: Keeps the first argument (e.g., target names in `set_target_properties`, or variable names in `set` / `option`) on the opening line to maintain searchability via simple tools like `grep`.
- **Configurable**: Settings are easily configured using a `.cmtkrc` file or a `[tool.cmtk]` section in `pyproject.toml`.

---

## Installation

Ensure you have the Rust toolchain installed. Build the project using `cargo`:

```bash
git clone https://github.com/halide/cmtk.git
cd cmtk
cargo build --release
```

The resulting binary will be available at `target/release/cmtk`.

---

## Usage

`cmtk` provides a CLI with three subcommands: `format`, `scan`, and `parse`.

### 1. Formatting Files

To format a CMake file and print the result to stdout:
```bash
cmtk format CMakeLists.txt
```

To format files in-place:
```bash
cmtk format -i CMakeLists.txt src/CMakeLists.txt
```

To check if files are formatted (useful in CI pipelines):
```bash
cmtk format --check CMakeLists.txt
```

#### Automated Schema Discovery during Format
You can point the formatter to files containing custom definitions (`--scan-only`) to teach it your custom command signatures on-the-fly:
```bash
cmtk format CMakeLists.txt --scan-only cmake/MyMacros.cmake
```

You can also instruct it to find all CMake files in your Git repository automatically:
```bash
cmtk format CMakeLists.txt --discover=git
```

### 2. Scanning / Schema Discovery

To scan custom CMake files, extract schemas, and output them in TOML:
```bash
cmtk scan cmake/MyMacros.cmake
```

To write/append the scanned schemas directly to your local `.cmtkrc`:
```bash
cmtk scan cmake/MyMacros.cmake --write
```

### 3. Parsing (Debug representation)

To inspect the raw lossless syntax tree produced by the parser:
```bash
cmtk parse CMakeLists.txt
```

---

## Formatting Style Guidelines

For multiline commands, `cmtk` avoids visually random continuation indentation or alignment under the opening parenthesis. Instead, it aligns the closing parenthesis with the command name and applies block indentation.

### Preferred (Block Indentation)
```cmake
find_package(
    Halide_LLVM 21...99 REQUIRED
    COMPONENTS WebAssembly X86
    OPTIONAL_COMPONENTS AArch64 ARM Hexagon NVPTX PowerPC RISCV
)
```

### List Layouts (`list_type`)

* **`packed`** (Default): Packs items as long as they fit the line, wrapping them when they exceed the limit:
  ```cmake
  target_link_libraries(
      MyTarget
      PUBLIC Halide_Runtime Halide_Headers Halide_Compiler Halide_Tools
      PRIVATE LLVMCore LLVMSupport LLVMAnalysis LLVMTarget
  )
  ```
* **`path`**: Paths format one-per-line when multiple, but singletons stay inline.

  When there are multiple keyword blocks, they are all indented:
  ```cmake
  target_sources(
      MyTarget
      PUBLIC
          include/Halide.h
          include/HalideBuffer.h
      PRIVATE
          src/Argument.cpp
          src/Bounds.cpp
          src/Buffer.cpp
  )
  ```

  When a list is at the end of the argument list (the last block) and no prior blocks were indented, it is flattened/de-dented to align with the keyword:
  ```cmake
  target_sources(
      MyTarget
      PRIVATE
      src/file1.cpp
      src/file2.cpp
      src/file3.cpp
  )
  ```

  Similarly, if a singleton path is too long to fit inline and is in the last block, it wraps and is de-dented:
  ```cmake
  target_sources(
      MyTarget
      PRIVATE
      extremely/long/path/to/some/nested/source/file/that/will/definitely/exceed/the/column/limit/to/verify/the/dedented/singleton/rule.cpp
  )
  ```
* **`n_per_line`** (e.g. `n = 2` for property pairs):
  ```cmake
  set_target_properties(MyTarget
      PROPERTIES
      CXX_STANDARD 17
      CXX_STANDARD_REQUIRED YES
      EXPORT_NAME MyTarget
  )
  ```

---

## Configuration

`cmtk` automatically discovers configuration by searching for `.cmtkrc` in the current directory first, then fallback to `[tool.cmtk]` in `pyproject.toml`.

### Configuration Options

| Option | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `indent_style` | string | `"space"` | Indentation style: `"space"` or `"tab"` |
| `indent_width` | integer | `4` | Indent size in spaces |
| `line_width` | integer | `100` | Target column boundary for wrapping |
| `source_vertical_list_threshold` | integer | `3` | Number of items in the source after which packed lists are forced vertical (`-1` to disable) |
| `function_schemas` | table | `{}` | Mappings of lowercase CMake functions to custom format schemas |

### Example `.cmtkrc` (TOML)

```toml
indent_style = "space"
indent_width = 4
line_width = 100
source_vertical_list_threshold = 3

[function_schemas.my_custom_install]
no_break_first_argument = true
options = ["FORCE", "VERBOSE"]
one_value_keywords = ["DESTINATION", "RENAME"]
multi_value_keywords = [
    { name = "TARGETS", list_type = "packed" },
    { name = "FILES", list_type = "path" }
]
```

### Example `pyproject.toml` integration

```toml
[tool.cmtk]
indent_style = "space"
indent_width = 4
line_width = 80

[tool.cmtk.function_schemas.add_halide_library]
no_break_first_argument = true
one_value_keywords = ["GENERATOR"]
multi_value_keywords = [
    { name = "SOURCES", list_type = "path" }
]
```

---

## Architecture Overview

1. **`syntax.rs`**: Defines token and node types (`SyntaxKind`) via `logos` derive macros and implements the `rowan::Language` trait (`CmakeLanguage`).
2. **`lexer.rs`**: Feeds tokens, whitespace, and comments to the parser.
3. **`parser.rs`**: Lossless recursive-descent parser that constructs a `rowan::GreenNode` syntax tree.
4. **`cst.rs`**: Strongly typed AST wrappers for raw Rowans (such as `CommandNode`).
5. **`schema.rs`**: Definitions for schema representations of CMake commands.
6. **`analyzer.rs`**: Infers custom schemas by analyzing local CMake source trees.
7. **`formatter.rs`**: Implements the layout engine, making formatting decisions based on line widths and schemas.

---

## Development

Run tests, check formatting, and lint with the following cargo commands:

```bash
# Run unit, integration, and golden tests
cargo test

# Force-update the golden test specs
UPDATE_GOLDENS=1 cargo test

# Verify formatting
cargo fmt -- --check

# Run lints
cargo clippy -- -D warnings
```

---

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE) or project files for details.
