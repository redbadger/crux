use rustdoc_types::{
    Enum, GenericArg, GenericArgs, Impl, Item, ItemEnum, Path, StructKind, Type, VariantKind,
};

use super::data::{Data, Edge, Node};

pub fn parse(data: &Data) -> Vec<(Node, Node, Edge)> {
    let mut edges = Vec::new();

    // edges
    for (id, item) in &data.crate_.index {
        let Some(source) = data.node_by_id(id) else {
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
                    StructKind::Unit => edges.push((source.clone(), source.clone(), Edge::Field)),
                    StructKind::Tuple(fields) => {
                        for field in fields {
                            if let Some(id) = field {
                                let Some(dest) = data.node_by_id(id) else {
                                    continue;
                                };
                                edges.push((source.clone(), dest.clone(), Edge::Field));
                            }
                        }
                    }
                    StructKind::Plain {
                        fields,
                        has_stripped_fields: _,
                    } => {
                        for id in fields {
                            let Some(dest) = data.node_by_id(id) else {
                                continue;
                            };
                            edges.push((source.clone(), dest.clone(), Edge::Field));
                        }
                    }
                };
            }
            ItemEnum::StructField(type_) => match type_ {
                Type::ResolvedPath(path) => {
                    let Some(dest) = data.node_by_id(&path.id) else {
                        continue;
                    };
                    edges.push((source.clone(), dest.clone(), Edge::Type));

                    if let Some(args) = &path.args {
                        process_args(source, args.as_ref(), &data, &mut edges);
                    }
                }
                _ => (),
            },
            ItemEnum::Enum(Enum { variants, .. }) => {
                for id in variants {
                    let Some(dest) = data.node_by_id(id) else {
                        continue;
                    };
                    edges.push((source.clone(), dest.clone(), Edge::Variant));
                }
            }
            ItemEnum::Variant(v) => {
                match &v.kind {
                    VariantKind::Plain => {
                        edges.push((source.clone(), source.clone(), Edge::Field));
                    }
                    VariantKind::Tuple(fields) => {
                        for id in fields {
                            if let Some(id) = id {
                                let Some(dest) = data.node_by_id(id) else {
                                    continue;
                                };
                                edges.push((source.clone(), dest.clone(), Edge::Field));
                            }
                        }
                    }
                    VariantKind::Struct {
                        fields,
                        has_stripped_fields: _,
                    } => {
                        for id in fields {
                            let Some(dest) = data.node_by_id(id) else {
                                continue;
                            };
                            edges.push((source.clone(), dest.clone(), Edge::Field));
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
                let Some(dest) = data.node_by_id(trait_id) else {
                    continue;
                };
                edges.push((source.clone(), dest.clone(), trait_edge));

                // record an edge for the type the impl is for
                let Some(dest) = data.node_by_id(&for_type_id) else {
                    continue;
                };
                edges.push((source.clone(), dest.clone(), Edge::Type));

                // record edges for the associated items in the impl
                for id in items {
                    let Some(dest) = data.node_by_id(id) else {
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

                    edges.push((source.clone(), dest.clone(), Edge::AssociatedItem));
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
                let Some(dest) = data.node_by_id(&target.id) else {
                    continue;
                };
                edges.push((source.clone(), dest.clone(), Edge::AssociatedType));
            }
            ItemEnum::AssocType { .. } => (),
        }
    }

    edges
}

fn process_args(
    source: &Node,
    args: &GenericArgs,
    data: &Data,
    edges: &mut Vec<(Node, Node, Edge)>,
) {
    if let GenericArgs::AngleBracketed { args, .. } = args {
        for arg in args {
            if let GenericArg::Type(t) = arg {
                if let Type::ResolvedPath(path) = t {
                    let Some(dest) = data.node_by_id(&path.id) else {
                        continue;
                    };
                    edges.push((source.clone(), dest.clone(), Edge::Type));

                    if let Some(args) = &path.args {
                        let generic_args = args.as_ref();
                        process_args(source, generic_args, data, edges);
                    }
                };
            }
        }
    }
}
