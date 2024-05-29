use std::collections::HashMap;

use anyhow::{anyhow, Result};
use petgraph::{dot::Dot, graph::NodeIndex, Graph};
use rustdoc_types::{Crate, Enum, Id, Impl, Item, ItemEnum, Type, VariantKind};

pub fn parse(crate_: &Crate) -> Result<String> {
    let mut graph = Graph::new();
    let mut nodes = HashMap::new();

    // nodes
    for (id, item) in crate_.index.clone() {
        nodes.insert(id, graph.add_node(Node { id, item }));
    }

    let node = |id| -> Result<&NodeIndex> {
        nodes
            .get(id)
            .ok_or_else(|| anyhow!("Could not find node with id {:?}", id))
    };

    // edges
    for (id, item) in &crate_.index {
        let source = node(id)?;
        match &item.inner {
            ItemEnum::Module(_module) => (),
            ItemEnum::ExternCrate { name: _, rename: _ } => (),
            ItemEnum::Use(_) => (),
            ItemEnum::Union(_union) => (),
            ItemEnum::Struct(s) => {
                match &s.kind {
                    rustdoc_types::StructKind::Unit => (),
                    rustdoc_types::StructKind::Tuple(fields) => {
                        for field in fields {
                            if let Some(id) = field {
                                graph.add_edge(*source, *node(&id)?, Edge::HasField);
                            }
                        }
                    }
                    rustdoc_types::StructKind::Plain {
                        fields,
                        has_stripped_fields: _,
                    } => {
                        for id in fields {
                            graph.add_edge(*source, *node(&id)?, Edge::HasField);
                        }
                    }
                };
                for id in &s.impls {
                    graph.add_edge(*source, *node(&id)?, Edge::Implements);
                }
            }
            ItemEnum::StructField(_) => (),
            ItemEnum::Enum(Enum { variants, .. }) => {
                for id in variants {
                    graph.add_edge(*source, *node(&id)?, Edge::HasVariant);
                }
            }
            ItemEnum::Variant(v) => {
                match &v.kind {
                    VariantKind::Plain => (),
                    VariantKind::Tuple(_vec) => (),
                    VariantKind::Struct {
                        fields,
                        has_stripped_fields: _,
                    } => {
                        for field in fields {
                            graph.add_edge(*source, *node(&field)?, Edge::HasField);
                        }
                    }
                };
            }
            ItemEnum::Function(_function) => (),
            ItemEnum::Trait(_) => (),
            ItemEnum::TraitAlias(_trait_alias) => (),
            ItemEnum::Impl(Impl {
                for_: Type::ResolvedPath(target),
                items,
                ..
            }) => {
                graph.add_edge(*source, *node(&target.id)?, Edge::ImplFor);
                for id in items {
                    graph.add_edge(*source, *node(&id)?, Edge::AssociatedItem);
                }
            }
            ItemEnum::Impl(_) => (),
            ItemEnum::TypeAlias(_type_alias) => (),
            ItemEnum::Constant {
                type_: _,
                const_: _,
            } => (),
            ItemEnum::Static(_) => (),
            ItemEnum::ExternType => (),
            ItemEnum::Macro(_) => (),
            ItemEnum::ProcMacro(_proc_macro) => (),
            ItemEnum::Primitive(_primitive) => (),
            ItemEnum::AssocConst { type_: _, value: _ } => (),
            ItemEnum::AssocType {
                generics: _,
                bounds: _,
                type_: Some(Type::ResolvedPath(target)),
            } => {
                if let Ok(dest) = node(&target.id) {
                    graph.add_edge(*source, *dest, Edge::AssociatedType);
                }
            }
            ItemEnum::AssocType { .. } => (),
        }
    }
    let out = Dot::new(&graph);
    Ok(format!("{:?}", out))
}

#[derive(Debug)]
struct Node {
    id: Id,
    item: Item,
}

#[derive(Debug)]
enum Edge {
    ImplFor,
    HasField,
    Implements,
    AssociatedItem,
    AssociatedType,
    HasVariant,
}
