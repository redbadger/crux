use std::collections::HashMap;

use anyhow::Result;
use ascent::ascent;
use rustdoc_types::{
    Crate, Enum, GenericArg, GenericArgs, Id, Impl, Item, ItemEnum, ItemSummary, Path, StructKind,
    Type, VariantKind,
};
use serde::Serialize;

ascent! {
    // input data
    relation edge(Node, Node, Edge);

    // result data
    relation app(Node);
    relation effect(Node);
    relation is_effect_of_app(Node, Node);
    relation root(Node);
    relation parent(Node, Node);
    relation field(Node, Node);
    relation variant(Node, Node);

    // app structs have an implementation of the App trait
    app(app) <--
        edge(app_impl, app_trait, Edge::TraitApp),
        edge(app_impl, app, Edge::Type);

    // effect enums have an implementation of the Effect trait
    effect(effect) <--
        edge(effect_impl, effect_trait, Edge::TraitEffect),
        edge(effect_impl, effect, Edge::Type);

    // an effect belongs to an app if they are in the same module
    is_effect_of_app(app, effect) <--
        app(app),
        effect(effect),
        if are_in_same_module(app, effect);

    // Event and ViewModel types are associated
    // with the root apps (that have no parent)
    root(assoc_type) <--
        edge(app_impl, app_trait, Edge::TraitApp),
        edge(app_impl, app, Edge::Type),
        !parent(_, app),
        edge(app_impl, assoc_item, Edge::AssociatedItem),
        edge(assoc_item, assoc_type, Edge::AssociatedType);
    // Effects belong to the root apps (that have no parent)
    root(effect_enum) <--
        is_effect_of_app(app, effect_enum),
        !parent(_, app);

    // app hierarchy
    parent(parent, child) <--
        app(parent),
        app(child),
        edge(parent, field, Edge::Field),
        edge(field, child, Edge::Type);

    // fields of root structs
    field(struct_, field) <--
        root(struct_),
        edge(struct_, field, ?Edge::Variant|Edge::Field);
    // recursive descent
    field(struct2, field2) <--
        field(struct1, field1),
        edge(field1, struct2, Edge::Type),
        edge(struct2, field2, ?Edge::Variant|Edge::Field);

    // variants of root enums
    variant(enum_, variant) <--
        root(enum_),
        edge(enum_, variant, Edge::Variant);
    // recursive descent
    variant(variant, field) <--
        variant(enum_, variant),
        edge(variant, field, Edge::Field);
}

fn are_in_same_module(app: &Node, effect: &Node) -> bool {
    let (Some(app), Some(effect)) = (&app.summary, &effect.summary) else {
        return false;
    };
    if app.path.len() != effect.path.len() {
        return false;
    };
    app.path[..(app.path.len() - 1)] == effect.path[..(effect.path.len() - 1)]
}

pub fn parse(crate_: &Crate) -> Result<Vec<(Node, Node)>> {
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
            .summary = Some(path.clone());
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
        if item.attrs.contains(&"#[serde(skip)]".to_string()) {
            continue;
        }

        match &item.inner {
            ItemEnum::Module(_module) => (),
            ItemEnum::ExternCrate { name: _, rename: _ } => (),
            ItemEnum::Use(_) => (),
            ItemEnum::Union(_union) => (),
            ItemEnum::Struct(s) => {
                match &s.kind {
                    StructKind::Unit => (),
                    StructKind::Tuple(fields) => {
                        for field in fields {
                            if let Some(id) = field {
                                let Some(dest) = node_by_id(id) else {
                                    continue;
                                };
                                prog.edge.push((source.clone(), dest.clone(), Edge::Field));
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
                            prog.edge.push((source.clone(), dest.clone(), Edge::Field));
                        }
                    }
                };
            }
            ItemEnum::StructField(type_) => match type_ {
                Type::ResolvedPath(path) => {
                    let Some(dest) = node_by_id(&path.id) else {
                        continue;
                    };
                    prog.edge.push((source.clone(), dest.clone(), Edge::Type));

                    if let Some(args) = &path.args {
                        process_args(source, args.as_ref(), &node_by_id, &mut prog);
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
                        .push((source.clone(), dest.clone(), Edge::Variant));
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
                                prog.edge.push((source.clone(), dest.clone(), Edge::Field));
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
                            prog.edge.push((source.clone(), dest.clone(), Edge::Field));
                        }
                    }
                };
            }
            ItemEnum::Function(_function) => (),
            ItemEnum::Trait(_) => (),
            ItemEnum::TraitAlias(_trait_alias) => (),
            ItemEnum::Impl(Impl {
                for_:
                    Type::ResolvedPath(Path {
                        id: for_type_id, ..
                    }),
                trait_:
                    Some(Path {
                        id: trait_id,
                        name: trait_name,
                        args: _,
                    }),
                items,
                ..
            }) => {
                let trait_edge = match trait_name.as_str() {
                    "App" => Edge::TraitApp,
                    "Effect" => Edge::TraitEffect,
                    _ => continue,
                };

                // record an edge for the trait the impl is of
                let Some(dest) = node_by_id(trait_id) else {
                    continue;
                };
                prog.edge.push((source.clone(), dest.clone(), trait_edge));

                // record an edge for the type the impl is for
                let Some(dest) = node_by_id(&for_type_id) else {
                    continue;
                };
                prog.edge.push((source.clone(), dest.clone(), Edge::Type));

                // record edges for the associated items in the impl
                for id in items {
                    let Some(dest) = node_by_id(id) else {
                        continue;
                    };

                    // skip everything except the Event and ViewModel associated types
                    if let Some(Item {
                        name: Some(name), ..
                    }) = &dest.item
                    {
                        if !["Event", "ViewModel", "Capabilities"].contains(&name.as_str()) {
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
                type_: Some(Type::ResolvedPath(target)),
                ..
            } => {
                // skip everything except the Event, ViewModel and Ffi associated types
                if let Item {
                    name: Some(name), ..
                } = &item
                {
                    if !["Event", "ViewModel", "Ffi"].contains(&name.as_str()) {
                        continue;
                    }
                }

                // record an edge for the associated type
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

    // write field and variant edges to disk for debugging
    std::fs::write("/tmp/edge.json", serde_json::to_string(&prog.edge).unwrap())?;
    std::fs::write("/tmp/app.json", serde_json::to_string(&prog.app).unwrap())?;
    std::fs::write(
        "/tmp/effect.json",
        serde_json::to_string(&prog.effect).unwrap(),
    )?;
    std::fs::write("/tmp/root.json", serde_json::to_string(&prog.root).unwrap())?;
    std::fs::write(
        "/tmp/is_effect_of_app.json",
        serde_json::to_string(&prog.is_effect_of_app).unwrap(),
    )?;
    std::fs::write(
        "/tmp/field.json",
        serde_json::to_string(&prog.field).unwrap(),
    )?;
    std::fs::write(
        "/tmp/variant.json",
        serde_json::to_string(&prog.variant).unwrap(),
    )?;
    std::fs::write(
        "/tmp/parent.json",
        serde_json::to_string(&prog.parent).unwrap(),
    )?;

    let mut all = Vec::new();
    all.extend(prog.field);
    all.extend(prog.variant);

    Ok(all)
}

fn process_args<'a>(
    source: &Node,
    args: &GenericArgs,
    node_by_id: &impl Fn(&Id) -> Option<&'a Node>,
    prog: &mut AscentProgram,
) {
    if let GenericArgs::AngleBracketed { args, .. } = args {
        for arg in args {
            if let GenericArg::Type(t) = arg {
                if let Type::ResolvedPath(path) = t {
                    let Some(dest) = node_by_id(&path.id) else {
                        continue;
                    };
                    prog.edge.push((source.clone(), dest.clone(), Edge::Type));

                    if let Some(args) = &path.args {
                        let generic_args = args.as_ref();
                        process_args(source, generic_args, node_by_id, prog);
                    }
                };
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Node {
    pub id: Id,
    pub item: Option<Item>,
    pub summary: Option<ItemSummary>,
}

impl Node {
    fn new(id: Id) -> Self {
        Self {
            id,
            item: None,
            summary: None,
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
    Type,
    Field,
    Variant,
    TraitApp,
    TraitEffect,
}
