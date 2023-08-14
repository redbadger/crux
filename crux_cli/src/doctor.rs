use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Result};
use ignore::Walk;
use ramhorns::{Content, Template};

use crate::{display, workspace};

type FileMap = BTreeMap<PathBuf, String>;

enum Context {
    Core(CoreContext),
    Shell(ShellContext),
}

#[derive(Content)]
struct CoreContext {
    workspace: String,
    name: String,
}

#[derive(Content)]
struct ShellContext {
    workspace: String,
    core: String,
    type_gen: String,
    name: String,
}

pub(crate) fn doctor(template_dir: &Path, verbosity: u8) -> Result<()> {
    let workspace = workspace::read_config()?;
    let workspace_name = &workspace.name;
    let current_dir = &env::current_dir()?;
    let template_root = current_dir.join(template_dir).canonicalize()?;

    for (name, core) in &workspace.cores {
        let context = Context::Core(CoreContext {
            workspace: workspace_name.to_ascii_lowercase().replace(" ", "_"),
            name: name.to_string(),
        });
        let root = current_dir.join(&core.source);
        let template_root = template_root.join("shared");
        compare(&root, &template_root, &context, verbosity)?;

        let root = current_dir.join(&core.type_gen);
        let template_root = template_root.join("shared_types");
        if template_root.exists() {
            compare(&root, &template_root, &context, verbosity)?;
        }
    }

    if let Some(shells) = &workspace.shells {
        for (name, shell) in shells {
            // TODO support shell having multiple cores
            let core = workspace
                .cores
                .get(&shell.cores[0])
                .expect("core not in workspace");
            let context = Context::Shell(ShellContext {
                workspace: workspace_name.to_ascii_lowercase().replace(" ", "_"),
                core: core.source.to_string_lossy().to_string(),
                type_gen: core.type_gen.to_string_lossy().to_string(),
                name: name.to_string(),
            });
            let root = current_dir.join(&shell.source);
            let template_root =
                template_root.join(shell.template.as_deref().unwrap_or(Path::new(&name)));
            if template_root.exists() {
                compare(&root, &template_root, &context, verbosity)?;
            }
        }
    }

    workspace::write_config(&workspace)
}

fn compare(
    root: &Path,
    template_root: &Path,
    context: &Context,
    verbosity: u8,
) -> Result<(), anyhow::Error> {
    println!(
        "{:-<80}\nActual:  {}\nDesired: {}",
        "",
        root.display(),
        template_root.display()
    );
    let (actual, desired) = &read_files(&root, &template_root, verbosity, context)?;
    missing(actual, desired);
    common(actual, desired);
    Ok(())
}

fn read_files(
    root: &Path,
    template_root: &Path,
    verbosity: u8,
    context: &Context,
) -> Result<(FileMap, FileMap)> {
    validate_path(root)?;
    validate_path(template_root)?;

    let mut actual = FileMap::new();
    for entry in Walk::new(root).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().expect("should have a file type").is_dir() {
            continue;
        }
        if verbosity > 0 {
            println!("Reading: {}", entry.path().display());
        }

        match fs::read_to_string(entry.path()) {
            Ok(contents) => {
                let relative = entry.path().strip_prefix(root)?.to_path_buf();
                actual.insert(relative, ensure_trailing_newline(&contents));
            }
            Err(e) => match e.kind() {
                std::io::ErrorKind::InvalidData => {
                    if verbosity > 0 {
                        println!("Warning, cannot read: {}, {e}", entry.path().display());
                    }
                }
                _ => {}
            },
        }
    }

    let mut desired = FileMap::new();
    for entry in Walk::new(template_root).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().expect("should have a file type").is_dir() {
            continue;
        }
        if verbosity > 0 {
            println!("Reading: {:?}", entry);
        }

        let template = fs::read_to_string(entry.path())?;
        let template = Template::new(template).unwrap();

        let rendered = match context {
            Context::Core(context) => template.render(context),
            Context::Shell(context) => template.render(context),
        };
        let rendered = ensure_trailing_newline(&rendered);

        let relative = entry.path().strip_prefix(template_root)?.to_path_buf();
        desired.insert(relative, rendered);
    }

    Ok((actual, desired))
}

fn validate_path(path: &Path) -> Result<()> {
    if !path.exists() {
        bail!("{} does not exist", path.display());
    }
    if !path.is_absolute() {
        bail!("{} is not an absolute path", path.display());
    }
    Ok(())
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
