use std::{collections::BTreeMap, env, fmt, fs, path::Path};

use anyhow::Result;
use args::Commands;
use clap::Parser;
use console::{style, Style};
use ignore::Walk;
use ramhorns::{Content, Template};
use similar::{capture_diff_slices, Algorithm, ChangeTag, TextDiff};

use crate::args::Cli;

mod args;
mod config;
mod workspace;

#[derive(Content)]
struct CoreContext {
    workspace: String,
    name: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match &cli.command {
        Some(Commands::Doctor { .. }) => {
            let workspace = workspace::read_config()?;
            let mut desired = BTreeMap::new();
            let mut actual = BTreeMap::new();
            let current_dir = &env::current_dir()?;

            for (name, core) in &workspace.cores {
                let root = current_dir.join(&core.source);
                let template_root = current_dir.join(&cli.template_dir).canonicalize()?;
                for entry in Walk::new(&template_root).into_iter().filter_map(|e| e.ok()) {
                    if entry.file_type().expect("should have a file type").is_dir() {
                        continue;
                    }
                    let ctx = CoreContext {
                        workspace: workspace.name.to_ascii_lowercase().replace(" ", "_"),
                        name: name.clone(),
                    };
                    let template = fs::read_to_string(entry.path())?;
                    let template = Template::new(template).unwrap();
                    let rendered = ensure_trailing_newline(&template.render(&ctx));
                    let relative = entry.path().strip_prefix(&template_root)?.to_path_buf();
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
            }

            let old = desired
                .keys()
                .map(|k| k.to_string_lossy())
                .collect::<Vec<_>>();
            let new = actual
                .keys()
                .map(|k| k.to_string_lossy())
                .collect::<Vec<_>>();

            let ops = capture_diff_slices(Algorithm::Myers, &old, &new);
            let missing: Vec<_> = ops
                .iter()
                .flat_map(|x| x.iter_changes(&old, &new))
                .map(|x| (x.tag(), x.value()))
                .filter_map(|(tag, x)| {
                    if tag == ChangeTag::Delete {
                        Some(x.to_string())
                    } else {
                        None
                    }
                })
                .collect();
            println!("Missing files: {:?} \n", missing);

            let existing: Vec<_> = ops
                .iter()
                .flat_map(|x| x.iter_changes(&old, &new))
                .map(|x| (x.tag(), x.value()))
                .filter_map(|(tag, x)| {
                    if tag == ChangeTag::Equal {
                        Some(x.to_string())
                    } else {
                        None
                    }
                })
                .collect();

            for file in existing {
                let desired = desired
                    .get(&Path::new(&file).to_path_buf())
                    .expect("file not in map");
                let actual = actual
                    .get(&Path::new(&file).to_path_buf())
                    .expect("file not in map");
                let diff = TextDiff::from_lines(actual, desired);
                for (idx, group) in diff.grouped_ops(3).iter().enumerate() {
                    if idx == 0 {
                        println!("\n{:-<80}", file);
                    }
                    for op in group {
                        for change in diff.iter_inline_changes(op) {
                            let (sign, s) = match change.tag() {
                                ChangeTag::Delete => ("-", Style::new().red()),
                                ChangeTag::Insert => ("+", Style::new().green()),
                                ChangeTag::Equal => (" ", Style::new().dim()),
                            };
                            print!(
                                "{}{} |{}",
                                style(Line(change.old_index())).dim(),
                                style(Line(change.new_index())).dim(),
                                s.apply_to(sign).bold(),
                            );
                            for (emphasized, value) in change.iter_strings_lossy() {
                                if emphasized {
                                    print!("{}", s.apply_to(value).underlined());
                                } else {
                                    print!("{}", s.apply_to(value));
                                }
                            }
                            if change.missing_newline() {
                                println!();
                            }
                        }
                    }
                }
            }
            workspace::write_config(&workspace)
        }
        None => Ok(()),
    }
}

struct Line(Option<usize>);

impl fmt::Display for Line {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            None => write!(f, "    "),
            Some(idx) => write!(f, "{:>4}", idx + 1),
        }
    }
}

/// Trim whitespace from end of line and ensure trailing newline
fn ensure_trailing_newline(s: &str) -> String {
    let mut s = s.trim_end().to_string();
    s.push('\n');
    s
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
}
