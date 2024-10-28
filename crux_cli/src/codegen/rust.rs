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
        let from = from.item.as_ref().unwrap();
        let mut container = None;
        let Some(name) = from.name.as_deref() else {
            continue;
        };
        match &from.inner {
            rustdoc_types::ItemEnum::Struct(s) => match &s.kind {
                rustdoc_types::StructKind::Unit => {}
                rustdoc_types::StructKind::Tuple(vec) => {}
                rustdoc_types::StructKind::Plain {
                    fields,
                    has_stripped_fields,
                } => {
                    let val = ContainerFormat::Struct(vec![]);
                    container = Some(containers.entry(name).or_insert(val));
                }
            },
            rustdoc_types::ItemEnum::Enum(e) => {
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
        let to = to.item.as_ref().unwrap();
        let Some(name) = to.name.as_deref() else {
            continue;
        };
        match &to.inner {
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
        println!("{:?} \n-> {:?}\n", from, to);
    }
    println!();
    println!("{:#?}", containers);
}

impl From<&Type> for Format {
    fn from(value: &Type) -> Self {
        match value {
            Type::ResolvedPath(path) => Format::TypeName(path.name.clone()),
            Type::DynTrait(dyn_trait) => todo!(),
            Type::Generic(_) => todo!(),
            Type::Primitive(s) => match s.as_ref() {
                "u32" => Format::U32,
                _ => todo!(),
            },
            Type::FunctionPointer(function_pointer) => todo!(),
            Type::Tuple(vec) => todo!(),
            Type::Slice(_) => todo!(),
            Type::Array { type_, len } => todo!(),
            Type::Pat {
                type_,
                __pat_unstable_do_not_use,
            } => todo!(),
            Type::ImplTrait(vec) => todo!(),
            Type::Infer => todo!(),
            Type::RawPointer { is_mutable, type_ } => todo!(),
            Type::BorrowedRef {
                lifetime,
                is_mutable,
                type_,
            } => todo!(),
            Type::QualifiedPath {
                name,
                args,
                self_type,
                trait_,
            } => todo!(),
        }
    }
}

impl From<&Variant> for VariantFormat {
    fn from(value: &Variant) -> Self {
        match &value.kind {
            rustdoc_types::VariantKind::Plain => VariantFormat::Unit,
            rustdoc_types::VariantKind::Tuple(vec) => VariantFormat::Tuple(vec![]),
            rustdoc_types::VariantKind::Struct {
                fields,
                has_stripped_fields,
            } => todo!(),
        }
    }
}
