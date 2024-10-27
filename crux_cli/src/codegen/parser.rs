use std::collections::HashMap;

use anyhow::{anyhow, Result};
use ascent::ascent;
use rustdoc_types::{
    Crate, Enum, Id, Impl, Item, ItemEnum, ItemSummary, Path, StructKind, Type, VariantKind,
};
use serde::Serialize;

ascent! {
    relation edge(Node, Node, Edge);

    relation implements(Node, Node);
    relation struct_fields(Node, Node);
    relation enum_variants(Node, Node);
    relation associated_type(Node, Node);

    implements(type_, trait_) <--
        edge(impl_, type_, Edge::ForType),
        edge(impl_, trait_, Edge::Trait);

    associated_type(impl_, type_) <--
        edge(impl_, trait_, Edge::Trait),
        edge(impl_, item, Edge::AssociatedItem),
        edge(item, type_, Edge::AssociatedType);

    struct_fields(struct_, field) <--
        associated_type(impl_, struct_),
        (edge(struct_, field, Edge::HasField) || edge(struct_, field, Edge::Unit));

    enum_variants(enum_, variant) <--
        associated_type(impl_, enum_),
        edge(enum_, variant, Edge::HasVariant);

    enum_variants(variant, field) <--
        enum_variants(enum_, variant),
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

    let node_by_id = |id: &Id| -> Result<&Node> {
        nodes_by_id
            .get(id)
            .ok_or_else(|| anyhow!("Could not find node with id {:?}", id))
    };

    // edges
    for (id, item) in &crate_.index {
        let source = node_by_id(id)?.clone();

        match &item.inner {
            ItemEnum::Module(_module) => (),
            ItemEnum::ExternCrate { name: _, rename: _ } => (),
            ItemEnum::Use(_) => (),
            ItemEnum::Union(_union) => (),
            ItemEnum::Struct(s) => {
                match &s.kind {
                    StructKind::Unit => {
                        prog.edge.push((source.clone(), source, Edge::Unit));
                    }
                    StructKind::Tuple(fields) => {
                        for field in fields {
                            if let Some(id) = field {
                                prog.edge.push((
                                    source.clone(),
                                    node_by_id(&id)?.clone(),
                                    Edge::HasField,
                                ));
                            }
                        }
                    }
                    StructKind::Plain {
                        fields,
                        has_stripped_fields: _,
                    } => {
                        for id in fields {
                            prog.edge.push((
                                source.clone(),
                                node_by_id(&id)?.clone(),
                                Edge::HasField,
                            ));
                        }
                    }
                };
            }
            ItemEnum::StructField(_) => (),
            ItemEnum::Enum(Enum { variants, .. }) => {
                for id in variants {
                    prog.edge
                        .push((source.clone(), node_by_id(&id)?.clone(), Edge::HasVariant));
                }
            }
            ItemEnum::Variant(v) => {
                match &v.kind {
                    VariantKind::Plain => (),
                    VariantKind::Tuple(fields) => {
                        for id in fields {
                            if let Some(id) = id {
                                prog.edge.push((
                                    source.clone(),
                                    node_by_id(id)?.clone(),
                                    Edge::HasField,
                                ));
                            }
                        }
                    }
                    VariantKind::Struct {
                        fields,
                        has_stripped_fields: _,
                    } => {
                        for field in fields {
                            prog.edge.push((
                                source.clone(),
                                node_by_id(field)?.clone(),
                                Edge::HasField,
                            ));
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
                prog.edge.push((
                    source.clone(),
                    node_by_id(&target.id)?.clone(),
                    Edge::ForType,
                ));
                prog.edge
                    .push((source.clone(), node_by_id(trait_id)?.clone(), Edge::Trait));
                for id in items {
                    prog.edge.push((
                        source.clone(),
                        node_by_id(&id)?.clone(),
                        Edge::AssociatedItem,
                    ));
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
                if let Ok(dest) = node_by_id(&target.id) {
                    prog.edge
                        .push((source.clone(), dest.clone(), Edge::AssociatedType));
                }
            }
            ItemEnum::AssocType { .. } => (),
        }
    }

    prog.run();

    std::fs::write(
        "/tmp/struct_fields.json",
        serde_json::to_string(&prog.struct_fields).unwrap(),
    )?;
    std::fs::write(
        "/tmp/enum_variants.json",
        serde_json::to_string(&prog.enum_variants).unwrap(),
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
