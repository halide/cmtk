use cmtk::parser::Parser;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

fn check_reconstruction(path: &Path) {
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(_) => return, // Ignore files with invalid UTF-8
    };
    let parser = Parser::new(&source);
    let parsed_tree = parser.parse();
    let reconstructed = parsed_tree.to_string();

    assert_eq!(
        source,
        reconstructed,
        "Reconstruction failed for file: {}",
        path.display()
    );
}

#[test]
fn test_reconstruction() {
    let data_dirs = ["tests/data/cmake", "tests/data/halide"];

    for dir in data_dirs {
        for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "cmake" || path.file_name().unwrap() == "CMakeLists.txt" {
                        check_reconstruction(path);
                    }
                } else if path.file_name().unwrap() == "CMakeLists.txt" {
                    check_reconstruction(path);
                }
            }
        }
    }
}
