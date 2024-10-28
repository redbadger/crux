use std::collections::HashMap;

use anyhow::Result;
use ascent::ascent;
use rustdoc_types::{
    Crate, Enum, GenericArg, GenericArgs, Id, Impl, Item, ItemEnum, ItemSummary, Path, StructKind,
    Type, VariantKind,
};
use serde::Serialize;

ascent! {
    relation edge(Node, Node, Edge);

    relation field(Node, Node);
    relation variant(Node, Node);
    relation root(Node, Node);

    // root for Event and ViewModel
    root(impl_, type_) <--
        edge(impl_, trait_, Edge::Trait),
        edge(impl_, item, Edge::AssociatedItem),
        edge(item, type_, Edge::AssociatedType);
    // root for Effect
    root(impl_, type_) <--
        edge(impl_, type_, Edge::AssociatedType),
        if let Some(i) = impl_.item.as_ref(),
        if let Some(n) = i.name.as_ref(),
        if n == &"Ffi".to_string();

    field(struct_, field) <--
        root(impl_, struct_),
        edge(struct_, field, ?Edge::HasField|Edge::Unit);
    field(struct2, field2) <--
        field(struct1, field1),
        edge(field1, struct2, Edge::ForType),
        edge(struct2, field2, Edge::HasField);

    variant(enum_, variant) <--
        root(impl_, enum_),
        edge(enum_, variant, Edge::HasVariant);
    variant(variant, field) <--
        variant(enum_, variant),
        edge(variant, field, Edge::HasField);
}

pub fn parse(crate_: &Crate) -> Result<String> {
    let mut prog = AscentProgram::default();
    let mut nodes_by_id = HashMap::new();

    // items
    for (id, item) in &crate_.index {
        nodes_by_id
            .entry(*id)
            .or_insert_with(|| Node::new(*id))
            .item = Some(item.clone());
    }

    // paths
    for (id, path) in &crate_.paths {
        nodes_by_id
            .entry(*id)
            .or_insert_with(|| Node::new(*id))
            .path = Some(path.clone());
    }

    let node_by_id = |id: &Id| -> Option<&Node> {
        let node = nodes_by_id
            .get(id)
            .expect("node should exist for all items and paths");
        let skip = match &node.item {
            Some(x) => x.attrs.contains(&"#[serde(skip)]".to_string()),
            _ => false,
        };
        (!skip).then_some(node)
    };

    // edges
    for (id, item) in &crate_.index {
        let Some(source) = node_by_id(id) else {
            continue;
        };

        match &item.inner {
            ItemEnum::Module(_module) => (),
            ItemEnum::ExternCrate { name: _, rename: _ } => (),
            ItemEnum::Use(_) => (),
            ItemEnum::Union(_union) => (),
            ItemEnum::Struct(s) => {
                match &s.kind {
                    StructKind::Unit => {
                        prog.edge.push((source.clone(), source.clone(), Edge::Unit));
                    }
                    StructKind::Tuple(fields) => {
                        for field in fields {
                            if let Some(id) = field {
                                let Some(dest) = node_by_id(id) else {
                                    continue;
                                };
                                prog.edge
                                    .push((source.clone(), dest.clone(), Edge::HasField));
                            }
                        }
                    }
                    StructKind::Plain {
                        fields,
                        has_stripped_fields: _,
                    } => {
                        for id in fields {
                            let Some(dest) = node_by_id(id) else {
                                continue;
                            };
                            prog.edge
                                .push((source.clone(), dest.clone(), Edge::HasField));
                        }
                    }
                };
            }
            ItemEnum::StructField(type_) => match type_ {
                // TODO: make recursive
                Type::ResolvedPath(path) => {
                    if let Some(args) = &path.args {
                        if let GenericArgs::AngleBracketed { args, .. } = args.as_ref() {
                            for arg in args {
                                if let GenericArg::Type(t) = arg {
                                    if let Type::ResolvedPath(path) = t {
                                        let Some(dest) = node_by_id(&path.id) else {
                                            continue;
                                        };
                                        prog.edge.push((
                                            source.clone(),
                                            dest.clone(),
                                            Edge::ForType,
                                        ));
                                    };
                                }
                            }
                        }
                    }
                }
                _ => (),
            },
            ItemEnum::Enum(Enum { variants, .. }) => {
                for id in variants {
                    let Some(dest) = node_by_id(id) else {
                        continue;
                    };
                    prog.edge
                        .push((source.clone(), dest.clone(), Edge::HasVariant));
                }
            }
            ItemEnum::Variant(v) => {
                match &v.kind {
                    VariantKind::Plain => (),
                    VariantKind::Tuple(fields) => {
                        for id in fields {
                            if let Some(id) = id {
                                let Some(dest) = node_by_id(id) else {
                                    continue;
                                };
                                prog.edge
                                    .push((source.clone(), dest.clone(), Edge::HasField));
                            }
                        }
                    }
                    VariantKind::Struct {
                        fields,
                        has_stripped_fields: _,
                    } => {
                        for id in fields {
                            let Some(dest) = node_by_id(id) else {
                                continue;
                            };
                            prog.edge
                                .push((source.clone(), dest.clone(), Edge::HasField));
                        }
                    }
                };
            }
            ItemEnum::Function(_function) => (),
            ItemEnum::Trait(_) => (),
            ItemEnum::TraitAlias(_trait_alias) => (),
            ItemEnum::Impl(Impl {
                trait_:
                    Some(Path {
                        id: trait_id,
                        name,
                        args: _,
                    }),
                for_: Type::ResolvedPath(target),
                items,
                ..
            }) => {
                if !["App", "Effect"].contains(&name.as_str()) {
                    continue;
                }

                let Some(dest) = node_by_id(&target.id) else {
                    continue;
                };
                prog.edge
                    .push((source.clone(), dest.clone(), Edge::ForType));

                let Some(dest) = node_by_id(trait_id) else {
                    continue;
                };
                prog.edge.push((source.clone(), dest.clone(), Edge::Trait));
                for id in items {
                    let Some(dest) = node_by_id(id) else {
                        continue;
                    };
                    if let Some(Item {
                        name: Some(name), ..
                    }) = &dest.item
                    {
                        if !["Event", "ViewModel"].contains(&name.as_str()) {
                            continue;
                        }
                    }

                    prog.edge
                        .push((source.clone(), dest.clone(), Edge::AssociatedItem));
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
                if let Item {
                    name: Some(name), ..
                } = &item
                {
                    if !["Event", "ViewModel", "Ffi"].contains(&name.as_str()) {
                        continue;
                    }
                }
                let Some(dest) = node_by_id(&target.id) else {
                    continue;
                };
                prog.edge
                    .push((source.clone(), dest.clone(), Edge::AssociatedType));
            }
            ItemEnum::AssocType { .. } => (),
        }
    }

    prog.run();

    std::fs::write("/tmp/edge.json", serde_json::to_string(&prog.edge).unwrap())?;
    std::fs::write(
        "/tmp/field.json",
        serde_json::to_string(&prog.field).unwrap(),
    )?;
    std::fs::write(
        "/tmp/variant.json",
        serde_json::to_string(&prog.variant).unwrap(),
    )?;

    Ok(format!(""))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct Node {
    id: Id,
    item: Option<Item>,
    path: Option<ItemSummary>,
}

impl Node {
    fn new(id: Id) -> Self {
        Self {
            id,
            item: None,
            path: None,
        }
    }
}

impl std::hash::Hash for Node {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
enum Edge {
    AssociatedItem,
    AssociatedType,
    ForType,
    HasField,
    HasVariant,
    Trait,
    Unit,
}
