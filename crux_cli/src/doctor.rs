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
        compare(
            current_dir.join(&core.source),
            template_root.join("shared"),
            workspace_name,
            name,
        )?;
        compare(
            current_dir.join(&core.type_gen),
            template_root.join("shared_types"),
            workspace_name,
            name,
        )?;
    }

    workspace::write_config(&workspace)
}

fn compare(
    root: PathBuf,
    template_root: PathBuf,
    workspace_name: &String,
    name: &String,
) -> Result<(), anyhow::Error> {
    println!(
        "{:-<80}\nActual:  {}\nDesired: {}",
        "",
        root.display(),
        template_root.display()
    );
    let (actual, desired) = &read_files(&root, &template_root, workspace_name, name)?;
    missing(actual, desired);
    common(actual, desired);
    Ok(())
}

fn read_files(
    root: &Path,
    template_root: &Path,
    workspace_name: &String,
    name: &String,
) -> Result<(FileMap, FileMap)> {
    let mut actual = FileMap::new();
    for entry in Walk::new(&root).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().expect("should have a file type").is_dir() {
            continue;
        }
        let contents = fs::read_to_string(entry.path())?;
        let relative = entry.path().strip_prefix(&root)?.to_path_buf();
        actual.insert(relative, contents);
    }

    let mut desired = FileMap::new();
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

    Ok((actual, desired))
}

fn missing(actual: &FileMap, desired: &FileMap) {
    let missing = difference(actual, desired);
    if missing.len() == 0 {
        println!("No missing files");
    } else {
        println!("Missing files:");
        for file_name in missing {
            println!("  {}", file_name.to_string_lossy());
        }
        println!("");
    }
}

fn common(actual: &FileMap, desired: &FileMap) {
    for file_name in &intersection(actual, desired) {
        let desired = desired.get(file_name).expect("file not in map");
        let actual = actual.get(file_name).expect("file not in map");
        display::show_diff(file_name, desired, actual);
    }
}

/// Trim whitespace from end of line and ensure trailing newline
fn ensure_trailing_newline(s: &str) -> String {
    let mut s = s.trim_end().to_string();
    s.push('\n');
    s
}

/// files in second but not in first
fn difference(first: &FileMap, second: &FileMap) -> Vec<PathBuf> {
    let mut missing = Vec::new();
    for (k, _) in second {
        if !first.contains_key(k) {
            missing.push(k.clone());
        }
    }
    missing
}

/// files in both first and second
fn intersection(first: &FileMap, second: &FileMap) -> Vec<PathBuf> {
    let mut common = Vec::new();
    for (k, _) in first {
        if second.contains_key(k) {
            common.push(k.clone());
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
        let mut actual_map = FileMap::new();
        actual_map.insert(PathBuf::from("foo"), "foo".to_string());

        let mut desired_map = FileMap::new();
        desired_map.insert(PathBuf::from("foo"), "foo".to_string());
        desired_map.insert(PathBuf::from("bar"), "bar".to_string());

        let expected = vec![PathBuf::from("bar")];
        let actual = difference(&actual_map, &desired_map);
        assert_eq!(expected, actual);
    }

    #[test]
    fn test_find_common_files() {
        let mut actual_map = FileMap::new();
        actual_map.insert(PathBuf::from("foo"), "foo".to_string());

        let mut desired_map = FileMap::new();
        desired_map.insert(PathBuf::from("foo"), "foo".to_string());
        desired_map.insert(PathBuf::from("bar"), "bar".to_string());

        let expected = vec![PathBuf::from("foo")];
        let actual = intersection(&actual_map, &desired_map);
        assert_eq!(expected, actual);
    }
}
