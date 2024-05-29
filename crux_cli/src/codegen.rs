use anyhow::{bail, Result};
use rustdoc_types::{Crate, Id, Impl, ItemEnum, Path, StructKind, Type, VariantKind};
use std::{
    fs::File,
    io::{stdout, IsTerminal},
};
use tokio::{process::Command, task::spawn_blocking};

use crate::{args::CodegenArgs, command_runner, graph};

pub async fn codegen(args: &CodegenArgs) -> Result<()> {
    let graph = graph::compute_package_graph()?;

    let Ok(lib) = graph.workspace().member_by_path(&args.lib) else {
        bail!("Could not find workspace package with path {}", args.lib)
    };

    let mut cmd = Command::new("cargo");
    cmd.env("RUSTC_BOOTSTRAP", "1")
        .env(
            "RUSTDOCFLAGS",
            "-Z unstable-options --output-format=json --cap-lints=allow",
        )
        .arg("doc")
        .arg("--no-deps")
        .arg("--manifest-path")
        .arg(lib.manifest_path())
        .arg("--lib");
    if stdout().is_terminal() {
        cmd.arg("--color=always");
    }

    command_runner::run(&mut cmd).await?;

    let target_directory = graph.workspace().target_directory().as_std_path();
    let json_path = target_directory
        .join("doc")
        .join(format!("{}.json", lib.name().replace('-', "_")));

    let crate_: Crate = spawn_blocking(move || -> Result<Crate> {
        let file = File::open(json_path)?;
        let crate_ = serde_json::from_reader(file)?;
        Ok(crate_)
    })
    .await??;

    for (id, associated_items) in find_impls(&crate_, "Effect", &["Ffi"]) {
        println!(
            "\nThe struct that implements crux_core::Effect is {}",
            crate_.paths[id].path.join("::")
        );

        for (name, id) in associated_items {
            visit(0, name, id, &crate_)?;
        }
    }
    println!();
    for (id, associated_items) in find_impls(&crate_, "App", &["Event", "ViewModel"]) {
        println!(
            "\nThe struct that implements crux_core::App is {}",
            crate_.paths[id].path.join("::")
        );

        for (name, id) in associated_items {
            visit(0, name, id, &crate_)?;
        }
    }

    Ok(())
}

fn visit(level: usize, name: &str, id: &Id, crate_: &Crate) -> Result<()> {
    let item = crate_.index.get(id);

    print!(
        "\n{level} {id:18} {} {name:20} ",
        " ".repeat(level * 4),
        id = format!("{:?}", id)
    );

    if let Some(summary) = crate_.paths.get(id) {
        let path_str = summary.path.join("::");
        print!("{path_str}");
    }

    if let Some(item) = item {
        match &item.inner {
            ItemEnum::Struct(ref struct_) => match &struct_.kind {
                StructKind::Unit => {
                    print!("unit struct");
                }
                StructKind::Tuple(fields) => {
                    print!("tuple struct: {fields:?}");
                }
                StructKind::Plain {
                    fields,
                    fields_stripped,
                } => {
                    if *fields_stripped {
                        anyhow::bail!("The {name} struct has private fields. You may need to make them public to use them in your code.");
                    }
                    for id in fields {
                        let item = &crate_.index[id];
                        if let Some(name) = &item.name {
                            visit(level + 1, name, id, crate_)?;
                        }
                    }
                }
            },
            ItemEnum::Enum(ref enum_) => {
                for id in &enum_.variants {
                    let item = &crate_.index[id];
                    if let Some(name) = &item.name {
                        visit(level + 1, name, id, crate_)?;
                    }
                }
            }
            ItemEnum::StructField(Type::ResolvedPath(path)) => {
                visit(level, name, &path.id, crate_)?;
            }
            ItemEnum::StructField(Type::Primitive(name)) => {
                print!("{name}");
            }
            ItemEnum::Module(_) => (),
            ItemEnum::ExternCrate { .. } => (),
            ItemEnum::Import(_) => (),
            ItemEnum::Union(_) => (),
            ItemEnum::Variant(v) => match &v.kind {
                VariantKind::Plain => {}
                VariantKind::Tuple(fields) => {
                    for id in fields {
                        let Some(id) = id else { continue };
                        let item = &crate_.index[id];
                        if let Some(name) = &item.name {
                            visit(level + 1, name, id, crate_)?;
                        }
                    }
                }
                VariantKind::Struct {
                    fields,
                    fields_stripped,
                } => {
                    if *fields_stripped {
                        anyhow::bail!("The {name} struct has private fields. You may need to make them public to use them in your code.");
                    }
                    for id in fields {
                        let item = &crate_.index[id];
                        if let Some(name) = &item.name {
                            visit(level + 1, name, id, crate_)?;
                        }
                    }
                }
            },
            ItemEnum::Function(_) => (),
            ItemEnum::Trait(_) => (),
            ItemEnum::TraitAlias(_) => (),
            ItemEnum::Impl(_) => (),
            ItemEnum::TypeAlias(_) => (),
            ItemEnum::OpaqueTy(_) => (),
            ItemEnum::Constant(_) => (),
            ItemEnum::Static(_) => (),
            ItemEnum::ForeignType => (),
            ItemEnum::Macro(_) => (),
            ItemEnum::ProcMacro(_) => (),
            ItemEnum::Primitive(_) => (),
            ItemEnum::AssocConst { .. } => (),
            ItemEnum::AssocType { .. } => (),
            _ => (),
        }
    }
    Ok(())
}

fn find_impls<'a>(
    crate_: &'a Crate,
    trait_name: &'a str,
    filter: &'a [&'a str],
) -> impl Iterator<Item = (&'a Id, Vec<(&'a str, &'a Id)>)> {
    crate_.index.iter().filter_map(move |(_k, v)| {
        if let ItemEnum::Impl(Impl {
            trait_: Some(Path { name, .. }),
            for_: Type::ResolvedPath(Path { id, .. }),
            items,
            ..
        }) = &v.inner
        {
            if name.as_str() == trait_name {
                let assoc_types = items
                    .iter()
                    .filter_map(|id| {
                        let item = &crate_.index[id];
                        item.name.as_deref().and_then(|name| {
                            if filter.contains(&name) {
                                if let ItemEnum::AssocType {
                                    default: Some(Type::ResolvedPath(Path { id, .. })),
                                    ..
                                } = &item.inner
                                {
                                    Some((name, id))
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        })
                    })
                    .collect();
                Some((id, assoc_types))
            } else {
                None
            }
        } else {
            None
        }
    })
}
