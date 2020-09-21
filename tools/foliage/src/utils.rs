use std::path::{Path, PathBuf};
use std::env;
use glob::glob;
use log::warn;
use regex::{Regex, Captures};

use super::crates::LocalDependency;

/// Find the root of the project
pub fn find_root<'p>() -> Option<PathBuf> {
    // Travel upwards until we find .redleaf-root

    // TODO: Better error visibility
    let cwd = env::current_dir().ok()?;

    if !cwd.is_absolute() {
        return None;
    }

    for dir in cwd.ancestors() {
        let root_marker = dir.join(Path::new(".redleaf-root"));

        if root_marker.exists() {
            return Some(dir.to_path_buf());
        }
    }

    None
}

/// Find a list of `Cargo.toml`s
pub fn find_cargo_tomls(root: &PathBuf) -> Result<Vec<PathBuf>, &'static str> {
    if !root.is_absolute() {
        return Err("The root must be absolute");
    }

    let mut pattern = root.to_owned();
    pattern.push(PathBuf::from("**/Cargo.toml"));

    let pattern = pattern.to_str().ok_or_else(|| "Could not construct pattern")?;
    let paths_it = match glob(pattern) {
        Ok(it) => it,
        Err(_) => {
            return Err("Could not glob for Cargo.toml files");
        }
    };

    let mut result = Vec::new();

    for glob_result in paths_it {
        match glob_result {
            Ok(pb) => {
                result.push(pb);
            }
            Err(e) => {
                warn!("Failed to read glob entry: {:?}", e);
            }
        }
    }

    Ok(result)
}

/// Find the relative path between two paths
///
/// It assumes the last non-empty component in `from` to be a directory,
/// and the two Paths are rooted in the same directory if they are relative.
///
/// e.g. /projects/redleaf/domains/a/, /projects/redleaf/lib/core/b/
/// would yield ../../lib/core/b
pub fn find_relative_path(from: &Path, to: &Path) -> Result<PathBuf, &'static str> {
    let to = PathBuf::from(to);
    let mut cur = from.to_owned();
    let mut parent_layers: usize = 0;

    // Traverse outwards
    while !to.starts_with(&cur) {
        cur = match cur.parent() {
            Some(p) => p.to_path_buf(),
            None => panic!("This should not happen"),
        };
        parent_layers += 1;
    }

    // Traverse inwards
    let suffix = match to.strip_prefix(cur) {
        Ok(p) => p,
        Err(_) => {
            return Err("Could not construct suffix for relative path");
        }
    };

    let mut relative = PathBuf::new();
    for _ in 0..parent_layers {
        relative.push(PathBuf::from(".."));
    }
    relative.push(suffix);

    Ok(relative)
}

pub fn replace_cargo_toml_deps(mut cargo_toml: String, canonical_path: &Path, new_deps: Vec<LocalDependency>) -> String {
    for dep in new_deps {
        use regex::{Regex, Captures};

        let new_path = find_relative_path(canonical_path, &dep.canonical_path.unwrap());
        if let Ok(new_path) = new_path {
            let replacement: String = format!(r#"{} = {{ path = "{}" }}"#, &dep.name, new_path.to_str().unwrap());

            // Very conservative regex
            let regex = format!(r#"(?m)^{} = \{{ path = "[\\./\-_A-Za-z]+"(, version = "[\\.0-9]+" \}}| \}})$"#, &dep.name);
            let regex = Regex::new(&regex).unwrap();

            cargo_toml = regex.replace(&cargo_toml, |caps: &Captures| {
                replacement.to_owned()
            }).to_string();
        }
    }

    cargo_toml
}
