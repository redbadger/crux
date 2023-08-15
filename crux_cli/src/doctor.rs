use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Result};
use ignore::Walk;
use ramhorns::Template;

use crate::{
    diff,
    template::{Context, CoreContext, ShellContext},
    workspace,
};

const SOURCE_CODE_EXTENSIONS: [&str; 9] =
    ["rs", "kt", "swift", "ts", "js", "tsx", "jsx", "html", "css"];

type FileMap = BTreeMap<PathBuf, String>;

pub(crate) fn doctor(
    template_dir: &Path,
    path: Option<&Path>,
    verbosity: u8,
    include_source_code: bool,
) -> Result<()> {
    let workspace = workspace::read_config()?;
    let current_dir = &env::current_dir()?;
    let template_root = current_dir.join(template_dir).canonicalize()?;

    for (_, core) in &workspace.cores {
        let (do_core, do_typegen) = match path {
            Some(path) => (path == &core.source, Some(path) == core.type_gen.as_deref()),
            None => (true, true),
        };

        if do_core {
            compare(
                &current_dir.join(&core.source),
                &template_root.join("shared"),
                &CoreContext::new(&workspace, core),
                verbosity,
                include_source_code,
            )?;
        }

        if do_typegen {
            if let Some(type_gen) = &core.type_gen {
                let templates_typegen = template_root.join("shared_types");
                if templates_typegen.exists() {
                    compare(
                        &current_dir.join(type_gen),
                        &templates_typegen,
                        &CoreContext::new(&workspace, core),
                        verbosity,
                        include_source_code,
                    )?;
                }
            }
        }
    }

    if let Some(shells) = &workspace.shells {
        for (name, shell) in shells {
            let do_shell = match path {
                Some(path) => path == &shell.source,
                None => true,
            };

            if do_shell {
                // TODO support shell having multiple cores
                let core = workspace
                    .cores
                    .get(&shell.cores[0])
                    .expect("core not in workspace");
                let template_root =
                    template_root.join(shell.template.as_deref().unwrap_or(Path::new(&name)));
                if template_root.exists() {
                    compare(
                        &current_dir.join(&shell.source),
                        &template_root,
                        &ShellContext::new(&workspace, core, shell),
                        verbosity,
                        include_source_code,
                    )?;
                }
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
    include_source_code: bool,
) -> Result<(), anyhow::Error> {
    println!(
        "{:-<80}\nActual:  {}\nDesired: {}",
        "",
        root.display(),
        template_root.display()
    );
    let (actual, desired) = &read_files(
        &root,
        &template_root,
        context,
        verbosity,
        include_source_code,
    )?;
    missing(actual, desired);
    common(actual, desired);
    Ok(())
}

fn read_files(
    root: &Path,
    template_root: &Path,
    context: &Context,
    verbosity: u8,
    include_source_code: bool,
) -> Result<(FileMap, FileMap)> {
    validate_path(root)?;
    validate_path(template_root)?;

    let mut actual = FileMap::new();
    for entry in Walk::new(root).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().expect("should have a file type").is_dir() {
            continue;
        }
        let path = entry.path();
        if !include_source_code && is_source_code(path) {
            continue;
        }
        let path_display = path.display();
        if verbosity > 0 {
            println!("Reading: {path_display}");
        }

        match fs::read_to_string(path) {
            Ok(contents) => {
                let relative = path.strip_prefix(root)?.to_path_buf();
                actual.insert(relative, ensure_trailing_newline(&contents));
            }
            Err(e) => match e.kind() {
                std::io::ErrorKind::InvalidData => {
                    if verbosity > 0 {
                        println!("Warning, cannot read: {path_display}, {e}");
                    }
                }
                _ => bail!("Error reading: {path_display}, {e}"),
            },
        };
    }

    let mut desired = FileMap::new();
    for entry in Walk::new(template_root).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().expect("should have a file type").is_dir() {
            continue;
        }
        let path = entry.path();
        if !include_source_code && is_source_code(path) {
            continue;
        }
        let path_display = path.display();
        if verbosity > 0 {
            println!("Reading: {path_display}");
        }

        let template = fs::read_to_string(path)?;
        let template = Template::new(template).unwrap();

        let rendered = match context {
            Context::Core(context) => template.render(context),
            Context::Shell(context) => template.render(context),
        };
        let rendered = ensure_trailing_newline(&rendered);

        let relative = path.strip_prefix(template_root)?.to_path_buf();
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
        diff::show(file_name, desired, actual);
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

/// test if file is source code
fn is_source_code(path: &Path) -> bool {
    if let Some(ext) = path.extension() {
        if let Some(ext) = ext.to_str() {
            return SOURCE_CODE_EXTENSIONS.contains(&ext);
        }
    }
    false
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

    #[test]
    fn test_is_source_code() {
        assert!(is_source_code(Path::new("foo.rs")));
        assert!(is_source_code(Path::new("foo.kt")));
        assert!(is_source_code(Path::new("foo.swift")));
        assert!(is_source_code(Path::new("foo.ts")));
        assert!(is_source_code(Path::new("foo.js")));
        assert!(is_source_code(Path::new("foo.tsx")));
        assert!(is_source_code(Path::new("foo.jsx")));
        assert!(is_source_code(Path::new("foo.html")));
        assert!(is_source_code(Path::new("foo.css")));

        assert!(!is_source_code(Path::new("foo.txt")));
        assert!(!is_source_code(Path::new("foo")));
    }
}
