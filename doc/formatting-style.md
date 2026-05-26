# Proposed CMake Formatting Style

This document is a draft style target for `cmtk`. It is intentionally
normative: it describes the behavior we want, not the formatter's current
behavior.

The core design goal is stable, readable CMake that preserves useful search
patterns while avoiding visually random continuation indentation. Formatting
should be driven by a command schema whenever one is available.

## Vocabulary

A command invocation has a command name and an argument list:

```cmake
command(arg1 arg2 KEYWORD value)
```

Arguments are classified by schema:

- Positional arguments: values consumed in order before keyword parsing, or as
  operands to a parser.
- Options: standalone flags with no value.
- One-value keywords: keywords that consume one value.
- List keywords: keywords that consume zero or more values until another
  keyword in the same parser is encountered.
- Sub-parsers: keywords or options that introduce their own nested argument
  grammar.

For example, in:

```cmake
install(
    TARGETS Halide
    RUNTIME COMPONENT Halide_Runtime
)
```

`TARGETS` is a list keyword in the `install` parser. `RUNTIME` is a sub-parser
within `install(TARGETS ...)`, and `COMPONENT` is a keyword inside the
`RUNTIME` sub-parser.

## General Layout

Keep a command on one line if it fits and has no comments that force a split:

```cmake
enable_testing()
find_package(Threads REQUIRED)
option(BUILD_SHARED_LIBS "Build shared libraries" ON)
```

When a command does not fit, use block indentation. Do not use hanging
alignment under the opening parenthesis. Continuation lines are always
indented by one fixed step from the command name; the formatter never aligns
continuation arguments under the opening parenthesis, regardless of the
layout of the source. Indentation width comes from configuration and is
never inferred by sampling the file.

Preferred:

```cmake
find_package(
    Halide_LLVM 21...99 REQUIRED
    COMPONENTS WebAssembly X86
    OPTIONAL_COMPONENTS AArch64 ARM Hexagon NVPTX PowerPC RISCV
)
```

Avoid:

```cmake
find_package(Halide_LLVM 21...99 REQUIRED
        COMPONENTS WebAssembly X86
        OPTIONAL_COMPONENTS AArch64 ARM Hexagon NVPTX PowerPC RISCV)
```

Avoid:

```cmake
find_package(
    Halide_LLVM
    21...99
    REQUIRED
    COMPONENTS WebAssembly
    X86
)
```

The closing parenthesis of a multiline command appears on its own line aligned
with the command name:

```cmake
project(
    Halide
    VERSION 22.0.0
    DESCRIPTION "Halide compiler and libraries"
    HOMEPAGE_URL "https://halide-lang.org"
)
```

## Positional Arguments

Positional arguments are grouped separately from parsed keyword lists. A
command's positional prefix should stay together when it fits.

Preferred:

```cmake
find_package(
    pybind11 2.8
    REQUIRED
    COMPONENTS comp1 comp2 comp3 comp4
)
```

Avoid:

```cmake
find_package(
    pybind11
    2.8
    REQUIRED
    COMPONENTS comp1 comp2 comp3 comp4
)
```

If the positional prefix itself is too long, wrap it as a list under the same
logical group:

```cmake
some_command(
    first_positional second_positional
    third_positional fourth_positional
    KEYWORD value
)
```

The schema should be able to specify a soft upper bound for how many
positional arguments a parser accepts before keyword parsing begins. This is a
grouping hint, not an exact arity rule. Some positional arguments are optional,
such as the version argument to `find_package`, and the formatter should not
force an optional positional argument onto its own line simply because the
hint's maximum is greater than the number of arguments present.

Examples:

```cmake
find_package(
    pybind11 2.8
    REQUIRED
)
```

```cmake
project(
    Halide
    VERSION 22.0.0
    DESCRIPTION "Halide compiler and libraries"
)
```

```cmake
add_library(
    Halide SHARED
    src/Func.cpp src/Module.cpp src/Pipeline.cpp
)
```

## Options

Options are standalone flags. In multiline form, they usually occupy their own
logical slot, but they may share a line with a positional group if that is part
of the parser.

```cmake
find_package(
    Halide_LLVM 21...99 REQUIRED
    COMPONENTS WebAssembly X86
)
```

```cmake
add_custom_command(
    OUTPUT generated.cpp
    COMMAND generator --emit generated.cpp
    DEPENDS generator input.txt
    VERBATIM
)
```

While the layout is still in *header mode* — the positional prefix and any
chained options before the first value-bearing keyword — an option may glue
onto the preceding header line if it fits. Once a one-value keyword,
multi-value keyword, compound list keyword, or sub-parser has emitted, header
mode ends and any subsequent option falls onto its own line so it cannot be
mistaken for a value of the preceding group. Re-arming header mode on a later
option is intentional: the new last line is itself an option line, and the
next option can safely ride on it.

```cmake
find_package(Halide_LLVM 21...99 REQUIRED EXACT)

find_package(
    Halide_LLVM 21...99 REQUIRED EXACT
    COMPONENTS WebAssembly X86
    OPTIONAL_COMPONENTS AArch64 ARM
)
```

## One-Value Keywords

A one-value keyword stays with its value when it fits:

```cmake
project(
    Halide
    VERSION 22.0.0
    DESCRIPTION "Halide compiler and libraries"
    HOMEPAGE_URL "https://halide-lang.org"
)
```

```cmake
get_property(
    is_multi_config
    GLOBAL PROPERTY GENERATOR_IS_MULTI_CONFIG
)
```

A one-value keyword's value stays on the keyword line even if it slightly
exceeds the configured line width. The value is only moved to its own
indented continuation line when keeping it on the keyword line would push
*other* content over the width, or when the value is itself the sole reason
the command cannot fit on a single line. When the value does wrap, it is
placed on the next line indented one further level than the keyword:

```cmake
message(
    FATAL_ERROR
        "Halide requires C++17 or newer but CMAKE_CXX_STANDARD=${CMAKE_CXX_STANDARD}"
)
```

## List Keywords

A list keyword starts a logical group. Values are packed onto the keyword line
while they fit unless the schema gives the list a more specific type.

```cmake
target_link_libraries(
    Halide
    PUBLIC Halide_Runtime Halide_Headers
    PRIVATE LLVMCore LLVMSupport
)
```

```cmake
find_package(
    Halide_LLVM 21...99 REQUIRED
    COMPONENTS WebAssembly X86
    OPTIONAL_COMPONENTS AArch64 ARM Hexagon NVPTX PowerPC RISCV
)
```

If values do not fit, wrap the values beneath the same logical group. For
non-path values, continue packing by width:

```cmake
target_link_libraries(
    Halide
    PRIVATE
        LLVMAnalysis LLVMBitWriter LLVMCore LLVMExecutionEngine LLVMInstCombine LLVMipo LLVMScalarOpts
        LLVMSupport
    PUBLIC Halide_Runtime Halide_Headers Halide_Targets
)
```

This means list keyword values are not necessarily one-per-line. The default
list type is `packed`: values are packed by width, and wrapped values continue
under the same logical group.

Schemas should model list behavior with an explicit `list_type` enum rather
than a collection of independent booleans. The initial list types are:

- `packed`: the default. Values are packed by width. This is appropriate for
  target names, package components, feature lists, library lists, and unknown
  lists.
- `path`: values are file or directory paths. A single short path stays on the
  keyword line; multiple paths, or a path that does not fit with the keyword,
  format one-per-line under the keyword.
- `command_argv`: values are an executable plus command-line arguments. The
  formatter keeps the command and its leading arguments together while they
  fit, then wraps continuation arguments one indentation level deeper.
- `n_per_line`: values are chopped into fixed-size groups of `n` tokens each,
  one group per line. `n` defaults to `1`, in which case every value appears
  on its own line (the diff-friendly form used by schemas that want stable
  one-value-per-line output even when the values are not paths). `n = 2`
  models key/value pair lists such as the `PROPERTIES` argument of
  `set_target_properties` (see Property Pair Lists below). Larger `n` values
  generalize to triplets and beyond, e.g. RGB defaults or cross-compilation
  target triples. See the `n_per_line` rules below.

If a schema marks a keyword as a list but does not specify `list_type`, the
formatter treats it as `packed`.

When a list keyword's values do not all fit on the keyword line, the keyword
sits alone on its own line and the values are emitted on continuation lines
below. The formatter never emits a mixed form where the first value rides
with the keyword and later values wrap below. Pick a lane: either everything
fits on the keyword line, or the keyword is alone with values beneath it.

How those continuation values lay out depends on the list type:

- `packed`: values continue packing by width across one or more continuation
  lines below the keyword.
- `path`, `n_per_line`, `command_argv`, and a `packed` list promoted to
  vertical layout by the source-vertical-list heuristic: each value (or
  `n`-token group) appears on its own continuation line. "Chopped means
  chopped" — the keyword is alone and every value, including the first,
  occupies its own indented line.

### `n_per_line` semantics

- If the entire list fits on the keyword line with the keyword, it is left
  there.
- Otherwise, the list is chopped into groups of exactly `n` consecutive
  tokens. Each group is emitted on its own line, indented one level under
  the keyword. The keyword sits on its own line. The first group goes on its
  own indented line; the keyword line never carries the first group.
- Tokens within a group are joined by a single space.
- If a single group is too long to fit on one line, that group's trailing
  tokens (typically the value, for `n = 2`) wrap onto continuation lines at
  a further indent. The group's first token stays on the group's first line
  so that prefix text searches (for example, `KEY ` in a `PROPERTIES` list)
  continue to work.
- Comments attach to a whole group, not to an individual token within a
  group.
- The total number of values in an `n_per_line` list must be a multiple of
  `n`. If it is not, the formatter falls back to `packed` for that list and
  reports a diagnostic; it does not attempt to silently realign.

Some CMake list forms have a compound header: two or more fixed tokens that
introduce the actual value list. Schemas should model those as compound list
keywords rather than as nested sub-parsers or as ordinary values. For example,
`foreach(var IN ITEMS a b c)` has the logical list header `IN ITEMS`; `a b c`
are the list values. When a compound header is used, the formatter glues the
header tokens backward to the command's argument header when possible, and
applies the list type only to the values after the header:

```cmake
foreach (hdr_name IN ITEMS
    HalideBuffer.h
    HalideRuntime.h
    HalideRuntimeCuda.h
)
```

This avoids treating selector tokens such as `ITEMS`, `LISTS`, or `ZIP_LISTS`
as list elements, and avoids adding an indentation level for each selector.
Compound list keywords are still schemas, not command-specific formatting
rules: each entry names the keyword, the allowed header alternatives, and the
list type for the trailing values.

## Property Pair Lists

Several commands consume a flat sequence of `KEY VALUE KEY VALUE ...` pairs
following a `PROPERTIES` keyword: `set_target_properties`,
`set_source_files_properties`, `set_directory_properties`,
`set_tests_properties`, and the `PROPERTIES` argument inside
`install(TARGETS ... PROPERTIES ...)`. These are modeled as `n_per_line` with
`n = 2`: each key/value pair appears on its own line, with the key and value
separated by a single space:

```cmake
set_target_properties(Halide
    PROPERTIES
    CXX_STANDARD 17
    CXX_STANDARD_REQUIRED YES
    CXX_EXTENSIONS NO
    EXPORT_NAME Halide
    OUTPUT_NAME Halide
    VERSION ${Halide_VERSION}
    SOVERSION ${Halide_SOVERSION}
)
```

`set_target_properties` uses `no_break_first_argument`, so the target name
rides with the opening parenthesis (see Searchability below). `PROPERTIES`
is the only multi-line group in the call, so the trailing-block flat rule
emits its pair list flush with `PROPERTIES` rather than indented one further
level.

If a single pair's value is too long to share a line with its key, that
pair's value wraps onto the next line at a further indent, but the key stays
on the pair's first line so that `KEY ` text searches still find it.

The formatter does not sort `PROPERTIES` pairs. If sorting is desired, wrap
the list with `keep-sorted` markers and run that tool separately.

Packed lists may still preserve an existing vertical source layout. This is a
source-layout heuristic, not an integration with any specific sorting tool or
comment convention. If the first `source_vertical_list_threshold` values in a
packed value run each begin on their own source line, preserve that run as a
vertical list. Comments inside the run do not count as values.

The default threshold is `3`:

```cmake
set(extra_output_names
    ASSEMBLY
    BITCODE
    COMPILER_LOG
    C_SOURCE
)
```

The threshold is configurable. `0` means always preserve vertical list layout
for packed value runs that must be formatted in multiline form. `-1` means
never infer vertical list layout from the source.

Path lists are the primary case where one-per-line formatting is preferred,
but that preference is for actual lists. A short singleton path stays with its
keyword:

```cmake
add_custom_command(
    OUTPUT generated.cpp
    COMMAND generator --emit generated.cpp
    DEPENDS generator input.txt
    VERBATIM
)
```

Multiple paths are usually easier to scan, diff, and edit when each path has
its own line:

```cmake
target_sources(
    Halide
    PRIVATE
        src/Argument.cpp
        src/Bounds.cpp
        src/Buffer.cpp
        src/CodeGen_C.cpp
        src/CodeGen_LLVM.cpp
        src/CompilerLogger.cpp
    PUBLIC
        include/Halide.h
        include/HalideBuffer.h
)
```

Whether a list keyword contains paths should be part of the schema. For
example, `target_sources(... PRIVATE ...)` and `install(... FILES ...)` should
be path-like, while `find_package(... COMPONENTS ...)` should remain packed by
width.

Command argument lists are not path lists and should not be formatted
one-per-line by default. For example:

```cmake
add_custom_command(
    OUTPUT generated.cpp
    COMMAND generator --emit generated.cpp
    DEPENDS generator input.txt
    VERBATIM
)
```

If a command argv list needs to wrap, continuation arguments are indented under
the `COMMAND` group rather than treated as independent CMake keyword groups:

```cmake
add_custom_command(
    OUTPUT generated.cpp
    COMMAND
        generator --emit generated.cpp --target host
        --output "${CMAKE_CURRENT_BINARY_DIR}/generated.cpp"
        --extra-flag
    DEPENDS generator input.txt
    VERBATIM
)
```

Chopped-means-chopped applies: when the command and its arguments do not all
fit on the keyword line, the `COMMAND` keyword sits alone and every argument
emits on its own continuation line, indented one level under `COMMAND`. The
formatter does not emit the hybrid form where the program rides with the
keyword and later arguments wrap below.

## Recursive Sub-Parsers

Some command arguments contain nested grammars. When formatting reaches such an
argument, it should recursively apply a parser for that argument's sub-language
instead of flattening everything into one list.

For `install(TARGETS ...)`, `RUNTIME`, `LIBRARY`, `ARCHIVE`, and `FILE_SET`
introduce sub-parsers:

```cmake
install(
    TARGETS Halide Halide_Generator Halide_GenGen
    EXPORT Halide_Targets
    RUNTIME COMPONENT Halide_Runtime
    LIBRARY COMPONENT Halide_Runtime NAMELINK_COMPONENT Halide_Development
    ARCHIVE COMPONENT Halide_Development
    FILE_SET HEADERS COMPONENT Halide_Development
)
```

Here:

- `TARGETS` is a list keyword containing target names.
- `EXPORT` is a one-value keyword.
- `RUNTIME`, `LIBRARY`, and `ARCHIVE` are sub-parsers.
- `COMPONENT` is a one-value keyword inside those sub-parsers.
- `FILE_SET` is a sub-parser with one positional argument, `HEADERS`, and a
  one-value keyword, `COMPONENT`.

If a sub-parser line becomes too long, recursively split inside that sub-parser:

```cmake
install(
    TARGETS Halide Halide_Generator Halide_GenGen
    EXPORT Halide_Targets
    RUNTIME COMPONENT Halide_Runtime
    LIBRARY
        DESTINATION ${CMAKE_INSTALL_LIBDIR}
        COMPONENT Halide_Runtime
        NAMELINK_COMPONENT Halide_Development
    ARCHIVE COMPONENT Halide_Development
    FILE_SET HEADERS COMPONENT Halide_Development
)
```

For a longer `FILE_SET`, split according to the `FILE_SET` parser rather than
the outer `install` parser:

```cmake
install(
    TARGETS Halide
    FILE_SET HEADERS
        BASE_DIRS include
        FILES
            include/Halide.h
            include/HalideBuffer.h
            include/HalideRuntime.h
        COMPONENT Halide_Development
    ARCHIVE COMPONENT Halide_Development
)
```

`FILES` is a path list; with multiple paths it formats one-per-line below the
keyword. The `ARCHIVE` sub-block keeps `FILE_SET` from being the only
sub-parser in the call, so the trailing-block flat rule does not fire and
`FILE_SET`'s children stay at the deeper indent.

The exact indentation for recursively split sub-parser contents is a policy
choice. The important rule is that splitting is based on the nested parser, not
on a flat argument list.

Chopped-means-chopped applies to sub-parsers as well as to list keywords. When
a sub-parser does not fit on one line, the only place its first child may
glue onto the sub-parser keyword line is the *positional-pairing* case: a
sub-parser that declares a positional argument can render as `KEYWORD value`
on one line and continue its remaining keywords below.
`install(EXPORT name ...)` and `install(FILE_SET HEADERS ...)` use this form.
A sub-parser without a positional argument — `RUNTIME`, `LIBRARY`, `ARCHIVE` —
puts the keyword on its own line and emits every child below; the hybrid
"first keyword child rides with the sub-parser, later siblings wrap" form is
not emitted.

```cmake
install(
    TARGETS Halide Halide_Generator
    EXPORT Halide_Targets
    LIBRARY
        DESTINATION ${CMAKE_INSTALL_LIBDIR}
        COMPONENT Halide_Runtime
        NAMELINK_COMPONENT Halide_Development
    ARCHIVE COMPONENT Halide_Development
)
```

### Trailing-block flat indentation

The contents of a sub-parser or a wrapping multi-value list keyword are
normally indented one extra level beneath the keyword. That extra level is
*omitted* when both of the following hold at the point the block opens:

- (a) no earlier sub-parser or wrapping multi-value group in the same
  parser scope has been emitted with an extra indent level, and
- (b) no further arguments follow this block in the same parser scope.

"Same parser scope" means the body of the current parser: the top-level
call, the body of a sub-parser, or the body of a mode. A mode body that
contains a single wrapping group qualifies — `cmake_language(EVAL CODE
...)` has only `CODE` in the `EVAL` body, so the rule fires and `CODE`'s
values land at `CODE`'s own indent rather than one level deeper:

```cmake
cmake_language(EVAL
    CODE
    "set(some_variable_name \"a value\")"
    "message(STATUS \"variables are configured\")"
)
```

When both conditions hold, the block's contents are emitted at the same
indentation as the block's keyword itself. The keyword's own line position
is unaffected by this rule: the keyword always follows the normal multiline
rule of one logical group per line, and is never tucked onto the preceding
positional line.

The rule applies to both layers of nesting:

- A trailing sub-parser (`install(... LIBRARY ...)` where `LIBRARY` is the
  only sub-block) emits its child keywords flush with the sub-parser
  keyword.
- A trailing multi-value list keyword whose values wrap below it
  (`target_sources(target PRIVATE ...)`) emits its continuation values flush
  with the keyword.

A block that does not actually emit any indented children — typically an
inline `EXPORT name` with no following nested keywords, or a multi-value
keyword whose values fit on the keyword line — does not count toward
condition (a). This keeps `install(EXPORT name ...)` followed by a final
sub-block from blocking the final sub-block from flattening, even though
`EXPORT` is itself a sub-parser.

`command_argv` lists are excluded from the rule. Their source-vertical
groups model distinct shell invocations, and flattening would erase the
keyword anchor that separates the program-and-args layer from the outer
command's other keywords.

With the rule active for a trailing multi-value keyword:

```cmake
target_sources(
    Halide
    PRIVATE
    src/Argument.cpp
    src/Bounds.cpp
    src/Buffer.cpp
)
```

With the rule active for a trailing sub-parser:

```cmake
install(
    TARGETS Halide
    LIBRARY
    DESTINATION ${CMAKE_INSTALL_LIBDIR}
    COMPONENT Halide_Runtime
    NAMELINK_COMPONENT Halide_Development
)
```

When there is surrounding structure to disambiguate, the extra indent level
is preserved:

```cmake
target_sources(
    Halide
    PRIVATE
        src/Argument.cpp
        src/Bounds.cpp
    PUBLIC
        include/Halide.h
)
```

The known cost of this rule is asymmetry under editing: appending a second
sub-parser or multi-value group to a single-block call flips condition (b)
and causes the existing group to re-indent. We accept that cost because the
single-block case is common and the flat layout reads more cleanly when
there is no sibling block to align against.

## Control Flow Expressions

Commands like `if()`, `elseif()`, and `while()` should not use the standard `packed` list logic for their conditions. Instead, they should format semantically, breaking lines preferentially before logical operators (`AND`, `OR`) and indenting correctly within nested parentheses.

```cmake
if (
    CMAKE_SYSTEM_NAME MATCHES "Linux"
    AND NOT (CMAKE_CXX_COMPILER_ID STREQUAL "MSVC")
    AND NOT (CMAKE_CXX_COMPILER_ID STREQUAL "GNU")
)
```

If a clause is itself too long, the formatter wraps inside its parentheses
and continues at a deeper indent:

```cmake
if (
    CMAKE_SYSTEM_NAME MATCHES "Linux"
    AND NOT (CMAKE_CXX_COMPILER_ID STREQUAL "MSVC"
        AND CMAKE_CXX_COMPILER_VERSION VERSION_LESS 19.16)
)
```

## Searchability: `set()` and `option()`

Commands like `set()`, `option()`, and `cmake_dependent_option()` are special cases for searchability. They should not break immediately
after the opening parenthesis. The variable name should remain on the first
line so that simple text searches for `set(NAME` or `option(NAME` remain useful.

Preferred:

```cmake
set(Halide_INSTALL_TOOLSDIR "${CMAKE_INSTALL_DATADIR}/tools"
    CACHE STRING "Path to Halide build-time tools and sources"
)
```

Avoid:

```cmake
set(
    Halide_INSTALL_TOOLSDIR "${CMAKE_INSTALL_DATADIR}/tools"
    CACHE STRING "Path to Halide build-time tools and sources"
)
```

This should be modeled as a general schema option, not as hard-coded formatter
behavior:

- `no_break_first_argument = true`
- positional prefix: variable name, then value
- `CACHE` sub-parser: two positional arguments, type and documentation string

For a long cache doc string, split recursively inside the `CACHE` sub-parser.
Quoted strings should follow the same line-width rules as unquoted arguments:
they should not be placed alone on a line merely because they are strings.
There should be an independent formatting reason, such as the full line
exceeding the configured width.

```cmake
set(Halide_INSTALL_TOOLSDIR "${CMAKE_INSTALL_DATADIR}/tools"
    CACHE STRING
    "Path to Halide build-time tools and sources. I've added a bunch of text here to make it absurdly long. I hope nobody ever does this"
)
```

For a normal non-cache variable, keep the variable name and value together when
they fit:

```cmake
set(CMAKE_WINDOWS_EXPORT_ALL_SYMBOLS ON)
```

If the value list is long, keep the variable name on the first line and wrap
the values after it:

```cmake
set(Halide_CCACHE_PARAMS
    CCACHE_CPP2=yes CCACHE_HASHDIR=yes
    CCACHE_SLOPPINESS=pch_defines,time_macros,include_file_mtime,include_file_ctime
    CACHE STRING "Parameters to pass through to ccache"
)
```

## Comments

Comments do not force a command into multiline form by themselves.

The formatter should preserve comment attachment:

- Trailing comments attach to the preceding argument or logical group.
- Standalone comments that precede an argument attach to the following argument
  or logical group.

Trailing comments may remain trailing if the containing line still fits:

```cmake
find_package(Halide_LLVM 21...99 REQUIRED)  # minimum-only constraint
```

Trailing comments may also remain trailing in multiline commands if their line
still fits:

```cmake
find_package(
    Halide_LLVM 21...99 REQUIRED  # minimum-only constraint
    COMPONENTS WebAssembly X86
)
```

If the line does not fit, a trailing comment may move to its own line, but it
still belongs to the same argument or group. In that case, prefer promoting the
comment to a leading comment for that argument or group. This preserves the
same attachment rule used for standalone comments and avoids making the comment
look like it describes the following group:

```cmake
find_package(
    # Use 99 to fake a minimum-only constraint
    Halide_LLVM 21...99 REQUIRED
    COMPONENTS WebAssembly X86
)
```

In this example, the comment describes `Halide_LLVM 21...99 REQUIRED`, so it is
placed before that group rather than between that group and `COMPONENTS`.

Comments inside a keyword value group do not end that group. If more values from
the same group follow the comment, keep those values at the same continuation
indentation as the group's other wrapped values:

```cmake
target_link_libraries(
    benchmark
    PRIVATE
    tflite_parser interpreter error_util file_util hannk_log_stderr
    Halide::Tools  # for halide_benchmark.h
    Halide::Runtime
)
```

The mid-list trailing comment forces the chopped-means-chopped lane:
`PRIVATE` sits alone, and the values continue below at the trailing-block
flat indent. The value the comment attaches to (`Halide::Tools`) is placed
on its own continuation line when needed so that the trailing comment can
stay on that value's line rather than being promoted to a leading comment
for the whole group.

Any mid-list comment — trailing or standalone — that is followed by more
values forces the containing keyword group into chopped form. The comment
itself cannot share the keyword's line, the next value also cannot ride with
the keyword, and emitting only the *pre-comment* values on the keyword line
would produce the hybrid "wrap and chop" layout that the chopped-means-chopped
rule rejects. The keyword sits alone and every value continues below.

Standalone comments that already precede an argument should remain before that
argument:

```cmake
target_compile_options(
    Halide
    PRIVATE
    $<$<CXX_COMPILER_ID:GNU,Clang,AppleClang>:-Wall>
    # variable length arrays in C++ are a Clang extension
    $<$<CXX_COMPILER_ID:Clang,AppleClang>:-Wvla-extension>
)
```

Here, the VLA comment describes the following
`$<$<CXX_COMPILER_ID:Clang,AppleClang>:-Wvla-extension>` argument.

A comment that lands at the *tail* of a keyword's argument slice — no more
values follow inside that slice — sits between this keyword group and whatever
follows in the outer call. Render it at the keyword's own indent rather than
under the value continuations, so it visually aligns with the sibling keyword
group it is logically attached to:

```cmake
install(
    FILES
        "${CMAKE_CURRENT_BINARY_DIR}/HalideConfig.cmake"
        "${CMAKE_CURRENT_BINARY_DIR}/../HalideConfigVersion.cmake"
    # It's okay to hard-code the destination because this code is only
    # called by scikit-build-core.
    DESTINATION "${SKBUILD_DATA_DIR}/share/cmake/Halide"
    COMPONENT Halide_Python
)
```

### Trailing `#` as a manual line-break hint

Some lists pack by width because the formatter has no schema for the eventual
consumer — for example, `set(generator_args COMMAND ... DEPENDS ...)` where
the variable will later be expanded into a command-line and the values *look*
like keyword/value pairs but are just a list to CMake, or
`string(CONCAT ... fragment fragment fragment ...)` where source-vertical
preservation does not fire.

A bare trailing `#` (a `#` comment with no body) attached to a value forces a
line break after that value. The empty trailing comment flushes the current
value group, so the formatter emits the value alone on its line and any
following values continue below at the same continuation indent. This is the
documented escape hatch for "I want one fragment per line here" without
adding global type-inference. Idempotence is preserved: a second format pass
sees the same trailing `#` tokens and produces the same output.

```cmake
set(generator_args
    COMMAND $<TARGET_FILE:my_gen>  #
    OUTPUT "${out_dir}/lib.a"  #
    DEPENDS my_gen
)
```

## Unknown Commands

Unknown commands use a conservative generic parser. The formatter does not
invent a full keyword/value/option grammar for them, but it does recognize
ALL-CAPS tokens as logical-group breakpoints so that authored layouts of the
form `KEYWORD value value ...` group correctly even without a schema:

```cmake
unknown_command(
    arg1 arg2 arg3
    KEYWORD value1 value2
    OTHER_KEYWORD value3
)
```

A token counts as a generic keyword when:

- it is at least three characters long,
- its first character is an ASCII uppercase letter, and
- every remaining character is an ASCII uppercase letter, digit, or
  underscore.

A small closed set of common ALL-CAPS boolean literals — `YES`, `OFF`, `TRUE`,
`FALSE` — is excluded so they format as values rather than as group
breakpoints. (`NO` and `ON` are already excluded by the length minimum.)
Tokens containing lowercase letters, equal signs, or other punctuation never
count, which keeps `EMCC_FLAGS=-O2` style values out of the heuristic. If a
single-token group is followed by a single value (the common `KEY value` pair
shape, such as `SHA512 <hash>`), the pair stays on one line rather than being
chopped.

This is an unconditional layout heuristic, not a keyword *guess* — the
formatter does not classify the token as a one-value or multi-value keyword,
it just uses it to break a flat argument list into logical groups for packing.

If a project wants better formatting for a command, it should add a manual
schema. Manual schemas must support the same parser features as built-in
schemas:

- positional prefix counts
- options
- one-value keywords
- list keywords
- list types
- modes
- recursive sub-parsers
- no-break-first-argument

Modes are distinct from sub-parsers. A mode is a leading argument that selects
the grammar for the rest of the command and is part of the command header:

```cmake
file(CONFIGURE
    OUTPUT "${out}"
    CONTENT "${content}"
)

string(CONCAT stub_text
    "#include <Halide.h>\n"
    "HALIDE_GENERATOR_PYSTUB(...)\n"
)
```

Sub-parsers are for peer sections inside an already-selected grammar, such as
`install(TARGETS ... RUNTIME ... FILE_SET ...)`. Adding mode information should
improve formatting. Removing mode information from a schema should never be
required to get the preferred layout.

### Mode-header chopping on overflow

When a mode declares positionals, the mode keyword and its positionals form
the command header and are normally emitted together on one line. When the
header doesn't fit and the mode has two or more positionals, the mode
keyword stays glued to the open paren and each positional chops onto its
own line at `arg_indent`:

```cmake
cmake_language(GET_EXPERIMENTAL_FEATURE_ENABLED
    CXX_MODULES
    is_enabled_long_var
)
```

A mode with at most one positional keeps mode + positional glued even on
overflow. The pair is semantically a single header unit — `FATAL_ERROR
"text"`, `CONCAT output_var`, `WRITE path "content"` — and forcing a break
between them is more disruptive than accepting the bounded overflow:

```cmake
message(
    FATAL_ERROR "Halide requires Node v16.13 or later, but found ..."
)
```

## Schema Sketch

This is not final syntax, but it outlines the information the formatter needs.

```toml
line_width = 100
source_vertical_list_threshold = 3

[FIND_PACKAGE]
positional = { min = 1, max = 2 }
options = ["REQUIRED", "QUIET", "EXACT", "MODULE", "CONFIG"]
list_keywords = [
    { name = "COMPONENTS", list_type = "packed" },
    { name = "OPTIONAL_COMPONENTS", list_type = "packed" },
]

[PROJECT]
positional = { min = 1, max = 1 }
one_value_keywords = ["VERSION", "DESCRIPTION", "HOMEPAGE_URL"]
list_keywords = [{ name = "LANGUAGES", list_type = "packed" }]

[FOREACH]
no_break_first_argument = true
compound_list_keywords = [
    { name = "IN", headers = [["ITEMS"], ["LISTS"], ["ZIP_LISTS"]], list_type = "n_per_line" },  # n defaults to 1
]

[SET]
no_break_first_argument = true
positional = { min = 1, max = 1 }

[SET.subparsers.CACHE]
positional = { min = 2, max = 2 }

[FILE]
no_break_first_argument = true

[FILE.modes.CONFIGURE]
one_value_keywords = ["OUTPUT", "INPUT", "CONTENT", "NEWLINE_STYLE"]
options = ["@ONLY", "COPYONLY", "ESCAPE_QUOTES"]

[INSTALL]
list_keywords = [
    { name = "TARGETS", list_type = "packed" },
    { name = "PROPERTIES", list_type = "n_per_line", n = 2 },
]
one_value_keywords = ["EXPORT"]

[SET_TARGET_PROPERTIES]
positional = { min = 1 }
list_keywords = [
    { name = "PROPERTIES", list_type = "n_per_line", n = 2 },
]

[INSTALL.subparsers.RUNTIME]
one_value_keywords = ["COMPONENT"]

[INSTALL.subparsers.LIBRARY]
one_value_keywords = ["COMPONENT", "NAMELINK_COMPONENT"]

[INSTALL.subparsers.ARCHIVE]
one_value_keywords = ["COMPONENT"]

[INSTALL.subparsers.FILE_SET]
positional = { min = 1, max = 1 }
one_value_keywords = ["COMPONENT"]
list_keywords = [
    { name = "BASE_DIRS", list_type = "path" },
    { name = "FILES", list_type = "path" },
]

[INSTALL.subparsers.PATTERN]
positional = { min = 1, max = 1 }
options = ["EXCLUDE"]
list_keywords = [{ name = "PERMISSIONS", list_type = "packed" }]

[INSTALL.subparsers.REGEX]
positional = { min = 1, max = 1 }
options = ["EXCLUDE"]
list_keywords = [{ name = "PERMISSIONS", list_type = "packed" }]

[ADD_CUSTOM_COMMAND]
options = ["VERBATIM", "APPEND", "USES_TERMINAL"]
one_value_keywords = ["TARGET", "COMMENT", "WORKING_DIRECTORY"]
list_keywords = [
    { name = "OUTPUT", list_type = "path" },
    { name = "COMMAND", list_type = "command_argv" },
    { name = "DEPENDS", list_type = "packed" },
    { name = "BYPRODUCTS", list_type = "path" },
]
```

Open questions:

- How should mutually exclusive command modes be represented, such as the many
  forms of `install`?

  For now, defer this until a real schema conflict appears. In practice, the
  different command forms usually accept different or compatible option
  vocabularies, so the formatter can often select the right parser by the first
  mode keyword. We should only add a more elaborate representation once we encounter
  a case where the same keyword must mean different things in different modes,
  such as a keyword being valid both inside and outside of a sub-parser.
