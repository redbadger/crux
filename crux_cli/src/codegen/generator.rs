use std::collections::HashMap;

use rustdoc_types::{Type, Variant};

use crate::codegen::format::{ContainerFormat, Named};

use super::{
    format::{Format, VariantFormat},
    parser::Node,
};

pub(crate) fn generate(edges: &[(Node, Node)]) {
    let mut containers = HashMap::new();
    let mut variant_index = 0;
    for (from, to) in edges {
        let Some(name) = get_name(&from) else {
            continue;
        };
        let Some(item) = &from.item else {
            continue;
        };
        let mut container = None;
        match &item.inner {
            rustdoc_types::ItemEnum::Struct(s) => match &s.kind {
                rustdoc_types::StructKind::Unit => {}
                rustdoc_types::StructKind::Tuple(_vec) => {}
                rustdoc_types::StructKind::Plain {
                    fields: _,
                    has_stripped_fields: _,
                } => {
                    let val = ContainerFormat::Struct(vec![]);
                    container = Some(containers.entry(name).or_insert(val));
                }
            },
            rustdoc_types::ItemEnum::Enum(_e) => {
                if containers.contains_key(name) {
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

        let Some(name) = get_name(&to) else {
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
                    value: t.into(),
                });
            }
            rustdoc_types::ItemEnum::Variant(t) => {
                let Some(ContainerFormat::Enum(ref mut v)) = &mut container else {
                    continue;
                };
                v.insert(
                    variant_index,
                    Named {
                        name: name.to_string(),
                        value: t.into(),
                    },
                );
            }
            _ => continue,
        }
        println!("{:?} \n-> {:?}\n", item, to);
    }
    println!();
    println!("{:#?}", containers);
}

impl From<&Type> for Format {
    fn from(value: &Type) -> Self {
        match value {
            Type::ResolvedPath(path) => Format::TypeName(path.name.clone()),
            Type::DynTrait(_dyn_trait) => todo!(),
            Type::Generic(_) => todo!(),
            Type::Primitive(s) => match s.as_ref() {
                "u32" => Format::U32,
                _ => todo!(),
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

fn get_name(node: &Node) -> Option<&str> {
    node.item.as_ref().and_then(|item| {
        let mut new_name = "";
        for attr in &item.attrs {
            if let Some((_, n)) =
                lazy_regex::regex_captures!(r#"\[serde\(rename = "(\w+)"\)\]"#, attr)
            {
                new_name = n;
            }
        }
        if new_name.is_empty() {
            item.name.as_deref()
        } else {
            Some(new_name)
        }
    })
}

#[cfg(test)]
mod test {
    use rustdoc_types::{Generics, Id, Item, ItemEnum, Struct, StructKind, Visibility};

    use super::*;

    fn test_node(name: Option<String>, attrs: Vec<String>) -> Node {
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
            path: None,
        }
    }

    #[test]
    fn test_get_name() {
        let name = Some("Foo".to_string());
        let attrs = vec![];
        let node = test_node(name, attrs);
        assert_eq!(get_name(&node), Some("Foo"));
    }

    #[test]
    fn test_get_name_with_rename() {
        let name = Some("Foo".to_string());
        let attrs = vec![r#"#[serde(rename = "Bar")]"#.to_string()];
        let node = test_node(name, attrs);
        assert_eq!(get_name(&node), Some("Bar"));
    }

    #[test]
    fn test_get_name_with_no_name() {
        let name = None;
        let attrs = vec![];
        let node = test_node(name, attrs);
        assert_eq!(get_name(&node), None);
    }
}
