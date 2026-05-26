use assert_cmd::prelude::*;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use tempfile::NamedTempFile;
use tempfile::TempDir;

fn init_git_repo(dir: &Path) {
    let out = Command::new("git")
        .args(["init", "-q"])
        .current_dir(dir)
        .output()
        .expect("failed to run git init");
    assert!(
        out.status.success(),
        "git init failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn test_check_exits_0_when_already_formatted() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "set(FOO bar)").unwrap();

    let mut cmd = Command::cargo_bin("cmtk").unwrap();
    cmd.arg("format").arg("--check").arg(file.path());
    cmd.assert().success();
}

#[test]
fn test_check_exits_1_when_would_change() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "set(  FOO   bar  )").unwrap();

    let mut cmd = Command::cargo_bin("cmtk").unwrap();
    cmd.arg("format").arg("--check").arg(file.path());
    cmd.assert().failure().code(1);
}

#[test]
fn test_in_place_modifies_file() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "set(  FOO   bar  )").unwrap();

    let mut cmd = Command::cargo_bin("cmtk").unwrap();
    cmd.arg("format").arg("--in-place").arg(file.path());
    cmd.assert().success();

    let content = fs::read_to_string(file.path()).unwrap();
    assert_eq!(content, "set(FOO bar)\n");
}

#[test]
fn test_multiple_files_check() {
    let mut file1 = NamedTempFile::new().unwrap();
    writeln!(file1, "set(FOO bar)").unwrap();
    let mut file2 = NamedTempFile::new().unwrap();
    writeln!(file2, "set(  BAZ   quux  )").unwrap();

    let mut cmd = Command::cargo_bin("cmtk").unwrap();
    cmd.arg("format")
        .arg("--check")
        .arg(file1.path())
        .arg(file2.path());
    cmd.assert().failure().code(1);
}

#[test]
fn test_check_shows_diff_on_stderr() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "set(  FOO   bar  )").unwrap();

    let mut cmd = Command::cargo_bin("cmtk").unwrap();
    cmd.arg("format").arg("--check").arg(file.path());
    let output = cmd.assert().failure().code(1).get_output().clone();
    let stderr = String::from_utf8(output.stderr).unwrap();

    assert!(
        stderr.contains("---"),
        "expected --- header; got:\n{}",
        stderr
    );
    assert!(
        stderr.contains("+++"),
        "expected +++ header; got:\n{}",
        stderr
    );
    assert!(
        stderr.contains("-set(  FOO   bar  )"),
        "expected removed line; got:\n{}",
        stderr
    );
    assert!(
        stderr.contains("+set(FOO bar)"),
        "expected added line; got:\n{}",
        stderr
    );
}

#[test]
fn test_format_auto_scan_applies_discovered_schema() {
    let dir = TempDir::new().unwrap();
    let cmake_path = dir.path().join("CMakeLists.txt");
    fs::write(
        &cmake_path,
        concat!(
            "function(my_func)\n",
            "  cmake_parse_arguments(ARG \"\" \"OUTPUT\" \"SOURCES\" ${ARGN})\n",
            "endfunction()\n",
            "\n",
            "my_func(OUTPUT out.txt SOURCES a.cpp b.cpp c.cpp d.cpp e.cpp f.cpp)\n",
        ),
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("cmtk").unwrap();
    cmd.arg("format").arg(&cmake_path);
    let output = cmd.assert().success().get_output().stdout.clone();
    let formatted = String::from_utf8(output).unwrap();

    // With auto-scan active, schema-aware commands are still allowed to stay inline
    // when they fit within the configured line width.
    assert!(
        formatted.contains("my_func(OUTPUT out.txt SOURCES a.cpp b.cpp c.cpp d.cpp e.cpp f.cpp)"),
        "expected fitting schema-aware command to stay inline; got:\n{}",
        formatted
    );
}

#[test]
fn test_format_uses_schema_from_scan_only_file() {
    let dir = TempDir::new().unwrap();
    let schema_path = dir.path().join("Schema.cmake");
    let format_path = dir.path().join("CMakeLists.txt");

    fs::write(
        &schema_path,
        concat!(
            "function(my_func)\n",
            "  cmake_parse_arguments(ARG \"\" \"OUTPUT\" \"SOURCES\" ${ARGN})\n",
            "endfunction()\n",
        ),
    )
    .unwrap();
    fs::write(
        &format_path,
        "my_func(OUTPUT out.txt SOURCES a.cpp b.cpp c.cpp d.cpp e.cpp f.cpp g.cpp h.cpp i.cpp j.cpp k.cpp l.cpp m.cpp n.cpp)\n",
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("cmtk").unwrap();
    cmd.arg("format")
        .arg(&format_path)
        .arg("--scan-only")
        .arg(&schema_path);
    let output = cmd.assert().success().get_output().stdout.clone();
    let formatted = String::from_utf8(output).unwrap();

    assert!(
        !formatted.contains("function(my_func)"),
        "scan-only file should not be formatted to stdout; got:\n{}",
        formatted
    );
    assert!(
        formatted.contains("\n    OUTPUT out.txt\n    SOURCES"),
        "expected schema from scan-only file to format my_func; got:\n{}",
        formatted
    );
}

#[test]
fn test_format_accepts_multiple_scan_only_files() {
    let dir = TempDir::new().unwrap();
    let schema_one_path = dir.path().join("SchemaOne.cmake");
    let schema_two_path = dir.path().join("SchemaTwo.cmake");
    let format_path = dir.path().join("CMakeLists.txt");

    fs::write(
        &schema_one_path,
        concat!(
            "function(first_func)\n",
            "  cmake_parse_arguments(ARG \"\" \"OUTPUT\" \"\" ${ARGN})\n",
            "endfunction()\n",
        ),
    )
    .unwrap();
    fs::write(
        &schema_two_path,
        concat!(
            "function(second_func)\n",
            "  cmake_parse_arguments(ARG \"\" \"OUTPUT\" \"SOURCES\" ${ARGN})\n",
            "endfunction()\n",
        ),
    )
    .unwrap();
    fs::write(
        &format_path,
        concat!(
            "first_func(OUTPUT out.txt)\n",
            "second_func(OUTPUT out.txt SOURCES a.cpp b.cpp c.cpp d.cpp e.cpp f.cpp g.cpp h.cpp i.cpp j.cpp k.cpp)\n",
        ),
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("cmtk").unwrap();
    cmd.arg("format")
        .arg(&format_path)
        .arg("--scan-only")
        .arg(&schema_one_path)
        .arg(&schema_two_path);
    let output = cmd.assert().success().get_output().stdout.clone();
    let formatted = String::from_utf8(output).unwrap();

    assert!(
        !formatted.contains("function(first_func)") && !formatted.contains("function(second_func)"),
        "scan-only files should not be formatted to stdout; got:\n{}",
        formatted
    );
    assert!(
        formatted.contains("first_func(OUTPUT out.txt)"),
        "expected schema from first scan-only file; got:\n{}",
        formatted
    );
    assert!(
        formatted.contains("\n    OUTPUT out.txt\n    SOURCES"),
        "expected schema from second scan-only file; got:\n{}",
        formatted
    );
}

#[test]
fn test_discover_git_picks_up_tracked_cmake_files() {
    let dir = TempDir::new().unwrap();
    init_git_repo(dir.path());

    let schema_dir = dir.path().join("schemas");
    let build_dir = dir.path().join("build");
    fs::create_dir_all(&schema_dir).unwrap();
    fs::create_dir_all(&build_dir).unwrap();

    let schema_path = schema_dir.join("Schema.cmake");
    let format_path = dir.path().join("CMakeLists.txt");
    let ignored_path = build_dir.join("Ignored.cmake");

    fs::write(
        &schema_path,
        concat!(
            "function(my_func)\n",
            "  cmake_parse_arguments(ARG \"\" \"OUTPUT\" \"SOURCES\" ${ARGN})\n",
            "endfunction()\n",
        ),
    )
    .unwrap();
    fs::write(
        &format_path,
        "my_func(OUTPUT out.txt SOURCES a.cpp b.cpp c.cpp d.cpp e.cpp f.cpp g.cpp h.cpp i.cpp j.cpp k.cpp l.cpp m.cpp n.cpp)\n",
    )
    .unwrap();
    fs::write(
        &ignored_path,
        "function(should_not_appear)\nendfunction()\n",
    )
    .unwrap();
    fs::write(dir.path().join(".gitignore"), "build/\n").unwrap();

    let status = Command::new("git")
        .args(["add", "-A"])
        .current_dir(dir.path())
        .status()
        .unwrap();
    assert!(status.success());

    let mut cmd = Command::cargo_bin("cmtk").unwrap();
    cmd.current_dir(dir.path())
        .arg("format")
        .arg("--discover=git")
        .arg(&format_path);
    let output = cmd.assert().success().get_output().stdout.clone();
    let formatted = String::from_utf8(output).unwrap();

    assert!(
        formatted.contains("\n    OUTPUT out.txt\n    SOURCES"),
        "expected discovered schema to format my_func multi-line; got:\n{}",
        formatted
    );
    assert!(
        !formatted.contains("function(my_func)"),
        "scan-only file should not be formatted to stdout; got:\n{}",
        formatted
    );
    assert!(
        !formatted.contains("should_not_appear"),
        "gitignored file should not be scanned; got:\n{}",
        formatted
    );
}
