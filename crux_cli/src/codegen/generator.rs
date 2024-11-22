use std::collections::HashMap;

use rustdoc_types::{Crate, GenericArg, GenericArgs, ItemSummary, Type, Variant};

use crate::codegen::format::{ContainerFormat, Named};

use super::{
    format::{Format, VariantFormat},
    parser::Node,
};

pub(crate) fn generate(edges: &[(Node, Node)], crate_: &Crate) {
    let mut containers = HashMap::new();
    let mut variant_index = 0;
    for (from, to) in edges {
        let Some(name) = get_name(&from, true) else {
            continue;
        };
        let Some(item) = &from.item else {
            continue;
        };
        let mut container = None;
        match &item.inner {
            rustdoc_types::ItemEnum::Struct(s) => match &s.kind {
                rustdoc_types::StructKind::Unit => (),
                rustdoc_types::StructKind::Tuple(_vec) => (),
                rustdoc_types::StructKind::Plain {
                    fields: _,
                    has_stripped_fields: _,
                } => {
                    let val = ContainerFormat::Struct(vec![]);
                    container = Some(containers.entry(name).or_insert(val));
                }
            },
            rustdoc_types::ItemEnum::Enum(_e) => {
                if containers.contains_key(&name) {
                    variant_index += 1;
                } else {
                    variant_index = 0;
                }
                container = Some(
                    containers
                        .entry(name)
                        .or_insert(ContainerFormat::Enum(Default::default())),
                );
            }
            _ => continue,
        }

        let Some(name) = get_name(&to, false) else {
            continue;
        };
        let Some(item) = &to.item else {
            continue;
        };
        match &item.inner {
            rustdoc_types::ItemEnum::StructField(t) => {
                let Some(ContainerFormat::Struct(ref mut v)) = &mut container else {
                    continue;
                };
                v.push(Named {
                    name: name.to_string(),
                    value: (crate_, t).into(),
                });
            }
            rustdoc_types::ItemEnum::Variant(t) => {
                let Some(ContainerFormat::Enum(ref mut v)) = &mut container else {
                    continue;
                };
                let value = Named {
                    name: name.to_string(),
                    value: t.into(),
                };
                if v.values().find(|v| v.name == name).is_none() {
                    v.insert(variant_index, value);
                } else {
                    variant_index -= 1;
                }
            }
            _ => continue,
        }
        // println!("{:?} \n-> {:?}\n", item, to);
    }
    // println!();
    println!("{:#?}", containers);
}

impl From<(&Crate, &Type)> for Format {
    fn from(value: (&Crate, &Type)) -> Self {
        match value.1 {
            Type::ResolvedPath(path) => {
                if let Some(args) = &path.args {
                    match args.as_ref() {
                        GenericArgs::AngleBracketed {
                            args,
                            constraints: _,
                        } => {
                            if path.name == "Option" {
                                let format = match args[0] {
                                    GenericArg::Type(ref t) => (value.0, t).into(),
                                    _ => todo!(),
                                };
                                Format::Option(Box::new(format))
                            } else {
                                let name = qualify_name(value.0.paths.get(&path.id), &path.name);
                                Format::TypeName(name)
                            }
                        }
                        GenericArgs::Parenthesized {
                            inputs: _,
                            output: _,
                        } => todo!(),
                    }
                } else {
                    Format::TypeName(path.name.clone())
                }
            }
            Type::DynTrait(_dyn_trait) => todo!(),
            Type::Generic(_) => todo!(),
            Type::Primitive(s) => match s.as_ref() {
                "bool" => Format::Bool,
                "char" => Format::Char,
                "isize" => match std::mem::size_of::<isize>() {
                    4 => Format::I32,
                    8 => Format::I64,
                    _ => panic!("unsupported isize size"),
                },
                "i8" => Format::I8,
                "i16" => Format::I16,
                "i32" => Format::I32,
                "i64" => Format::I64,
                "i128" => Format::I128,
                "usize" => match std::mem::size_of::<usize>() {
                    4 => Format::U32,
                    8 => Format::U64,
                    _ => panic!("unsupported usize size"),
                },
                "u8" => Format::U8,
                "u16" => Format::U16,
                "u32" => Format::U32,
                "u64" => Format::U64,
                "u128" => Format::U128,
                s => panic!("need to implement primitive {s}"),
            },
            Type::FunctionPointer(_function_pointer) => todo!(),
            Type::Tuple(_vec) => todo!(),
            Type::Slice(_) => todo!(),
            Type::Array { type_: _, len: _ } => todo!(),
            Type::Pat {
                type_: _,
                __pat_unstable_do_not_use,
            } => todo!(),
            Type::ImplTrait(_vec) => todo!(),
            Type::Infer => todo!(),
            Type::RawPointer {
                is_mutable: _,
                type_: _,
            } => todo!(),
            Type::BorrowedRef {
                lifetime: _,
                is_mutable: _,
                type_: _,
            } => todo!(),
            Type::QualifiedPath {
                name: _,
                args: _,
                self_type: _,
                trait_: _,
            } => todo!(),
        }
    }
}

impl From<&Variant> for VariantFormat {
    fn from(value: &Variant) -> Self {
        match &value.kind {
            rustdoc_types::VariantKind::Plain => VariantFormat::Unit,
            rustdoc_types::VariantKind::Tuple(_vec) => VariantFormat::Tuple(vec![]),
            rustdoc_types::VariantKind::Struct {
                fields: _,
                has_stripped_fields: _,
            } => todo!(),
        }
    }
}

fn get_name(node: &Node, qualified: bool) -> Option<String> {
    node.item.as_ref().and_then(|item| {
        let mut new_name = "";
        for attr in &item.attrs {
            if let Some((_, n)) =
                lazy_regex::regex_captures!(r#"\[serde\(rename\s*=\s*"(\w+)"\)\]"#, attr)
            {
                new_name = n;
            }
        }
        let name = if new_name.is_empty() {
            item.name.as_deref()
        } else {
            Some(new_name)
        };
        let item_summary = node.summary.as_ref();
        name.map(|name| {
            if qualified {
                qualify_name(item_summary, name)
            } else {
                name.to_string()
            }
        })
    })
}

fn qualify_name(item_summary: Option<&ItemSummary>, name: &str) -> String {
    item_summary
        .map(|p| {
            [&p.path[..(p.path.len() - 1)], &[name.to_string()]]
                .concat()
                .join("::")
        })
        .unwrap_or(name.to_string())
}

#[cfg(test)]
mod test {
    use rustdoc_types::{
        Generics, Id, Item, ItemEnum, ItemKind, ItemSummary, Struct, StructKind, Visibility,
    };

    use super::*;

    fn test_node(name: Option<String>, path: Option<Vec<String>>, attrs: Vec<String>) -> Node {
        let path = path.map(|path| ItemSummary {
            crate_id: 0,
            path,
            kind: ItemKind::Struct,
        });
        Node {
            item: Some(Item {
                name,
                attrs,
                inner: ItemEnum::Struct(Struct {
                    kind: StructKind::Plain {
                        fields: vec![],
                        has_stripped_fields: false,
                    },
                    generics: Generics {
                        params: vec![],
                        where_predicates: vec![],
                    },
                    impls: vec![],
                }),
                id: Id(0),
                crate_id: 0,
                span: None,
                visibility: Visibility::Public,
                docs: None,
                links: Default::default(),
                deprecation: None,
            }),
            id: Id(0),
            summary: path,
        }
    }

    #[test]
    fn test_get_name() {
        let name = Some("Foo".to_string());
        let path = vec!["shared".to_string(), "app".to_string(), "Foo".to_string()];
        let attrs = vec![];
        let node = test_node(name, Some(path), attrs);
        assert_eq!(get_name(&node, false), Some("Foo".to_string()));
    }

    #[test]
    fn test_get_name_qualified() {
        let name = Some("Foo".to_string());
        let path = vec!["shared".to_string(), "app".to_string(), "Foo".to_string()];
        let attrs = vec![];
        let node = test_node(name, Some(path), attrs);
        assert_eq!(get_name(&node, true), Some("shared::app::Foo".to_string()));
    }

    #[test]
    fn test_get_name_with_rename() {
        let name = Some("Foo".to_string());
        let path = vec!["shared".to_string(), "app".to_string(), "Foo".to_string()];
        let attrs = vec![r#"#[serde(rename = "Bar")]"#.to_string()];
        let node = test_node(name, Some(path), attrs);
        assert_eq!(get_name(&node, true), Some("shared::app::Bar".to_string()));
    }

    #[test]
    fn test_get_name_with_rename_no_whitespace() {
        let name = Some("Foo".to_string());
        let attrs = vec![r#"#[serde(rename="Bar")]"#.to_string()];
        let node = test_node(name, None, attrs);
        assert_eq!(get_name(&node, false), Some("Bar".to_string()));
    }

    #[test]
    fn test_get_name_with_no_name() {
        let name = None;
        let attrs = vec![];
        let node = test_node(name, None, attrs);
        assert_eq!(get_name(&node, false), None);
    }
}
