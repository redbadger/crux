use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::Result;
use ignore::Walk;
use ramhorns::{Content, Template};

use crate::{display, workspace};

type FileMap = BTreeMap<PathBuf, String>;

#[derive(Content)]
struct CoreContext {
    workspace: String,
    name: String,
}

pub(crate) fn doctor(template_dir: &Path) -> Result<()> {
    let workspace = workspace::read_config()?;
    let workspace_name = &workspace.name;
    let current_dir = &env::current_dir()?;
    let template_root = current_dir.join(template_dir).canonicalize()?;

    for (name, core) in &workspace.cores {
        let root = current_dir.join(&core.source);
        let (desired, actual) = read_files(&root, &template_root, workspace_name, name)?;

        let missing = find_missing_files(&desired, &actual);
        println!("Missing files: {:?} \n", missing);

        let common: Vec<String> = find_common_files(&desired, &actual);
        for file_name in common {
            let desired = desired
                .get(&PathBuf::from(&file_name))
                .expect("file not in map");
            let actual = actual
                .get(&PathBuf::from(&file_name))
                .expect("file not in map");
            display::show_diff(&file_name, desired, actual);
        }
    }

    workspace::write_config(&workspace)
}

fn read_files(
    root: &Path,
    template_root: &Path,
    workspace_name: &String,
    name: &String,
) -> Result<(FileMap, FileMap)> {
    let mut desired = FileMap::new();
    let mut actual = FileMap::new();
    for entry in Walk::new(template_root).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().expect("should have a file type").is_dir() {
            continue;
        }
        let template = fs::read_to_string(entry.path())?;
        let template = Template::new(template).unwrap();

        let ctx = CoreContext {
            workspace: workspace_name.to_ascii_lowercase().replace(" ", "_"),
            name: name.clone(),
        };
        let rendered = template.render(&ctx);
        let rendered = ensure_trailing_newline(&rendered);

        let relative = entry.path().strip_prefix(template_root)?.to_path_buf();
        desired.insert(relative, rendered);
    }
    for entry in Walk::new(&root).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().expect("should have a file type").is_dir() {
            continue;
        }
        let contents = fs::read_to_string(entry.path())?;
        let relative = entry.path().strip_prefix(&root)?.to_path_buf();
        actual.insert(relative, contents);
    }
    Ok((desired, actual))
}

/// Trim whitespace from end of line and ensure trailing newline
fn ensure_trailing_newline(s: &str) -> String {
    let mut s = s.trim_end().to_string();
    s.push('\n');
    s
}

fn find_missing_files(desired: &FileMap, actual: &FileMap) -> Vec<String> {
    let mut missing = Vec::new();
    for (k, _) in desired {
        if !actual.contains_key(k) {
            missing.push(k.to_string_lossy().to_string());
        }
    }
    missing
}

fn find_common_files(desired: &FileMap, actual: &FileMap) -> Vec<String> {
    let mut common = Vec::new();
    for (k, _) in desired {
        if actual.contains_key(k) {
            common.push(k.to_string_lossy().to_string());
        }
    }
    common
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_ensure_trailing_newline() {
        assert_eq!(ensure_trailing_newline("hello\n"), "hello\n");
        assert_eq!(ensure_trailing_newline("hello\n \t"), "hello\n");
        assert_eq!(ensure_trailing_newline("hello\n\n "), "hello\n");
    }

    #[test]
    fn test_find_missing_files() {
        let mut desired_map = FileMap::new();
        desired_map.insert(PathBuf::from("foo"), "foo".to_string());
        desired_map.insert(PathBuf::from("bar"), "bar".to_string());
        let mut actual_map = FileMap::new();
        actual_map.insert(PathBuf::from("foo"), "foo".to_string());
        let expected = vec!["bar"];
        let actual = find_missing_files(&desired_map, &actual_map);
        assert_eq!(expected, actual);
    }

    #[test]
    fn test_find_common_files() {
        let mut desired_map = FileMap::new();
        desired_map.insert(PathBuf::from("foo"), "foo".to_string());
        desired_map.insert(PathBuf::from("bar"), "bar".to_string());
        let mut actual_map = FileMap::new();
        actual_map.insert(PathBuf::from("foo"), "foo".to_string());
        let expected = vec!["foo"];
        let actual = find_common_files(&desired_map, &actual_map);
        assert_eq!(expected, actual);
    }
}
