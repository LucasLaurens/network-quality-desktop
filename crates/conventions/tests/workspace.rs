//! Every Rust source of the workspace follows the method-naming rule.

use std::path::{Path, PathBuf};

use conventions::find_misnamed_functions;

const SOURCE_FOLDERS: [&str; 2] = ["crates", "src-tauri/src"];

/// O(f) for f files under `folder`.
fn list_rust_files(folder: &Path, files: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(folder).expect("a readable folder") {
        let path = entry.expect("an entry").path();
        if path.is_dir() {
            list_rust_files(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
}

#[test]
fn names_every_function_with_a_verb() {
    // Arrange
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = Vec::new();
    for folder in SOURCE_FOLDERS {
        list_rust_files(&root.join(folder), &mut files);
    }

    // Act
    let misnamed: Vec<String> = files
        .iter()
        .flat_map(|file| {
            let source = std::fs::read_to_string(file).expect("a readable file");
            let names = find_misnamed_functions(&source).expect("valid Rust");
            names
                .into_iter()
                .map(move |name| format!("{}: {name}", file.display()))
        })
        .collect();

    // Assert
    assert!(
        misnamed.is_empty(),
        "start these with a verb:\n{}",
        misnamed.join("\n")
    );
}
