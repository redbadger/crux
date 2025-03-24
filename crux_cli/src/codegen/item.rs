use iter_tools::Itertools;
use rustdoc_types::{
    Enum, Id, Impl, Item, ItemEnum, Path, Struct, StructKind, Type, Variant, VariantKind,
};

pub fn is_relevant(item: &Item) -> bool {
    is_impl(item)
        || is_struct_field(item)
        || is_enum_variant(item)
        || is_struct(item)
        || is_enum(item)
        || is_associated_type(item)
}

pub fn is_struct(item: &Item) -> bool {
    matches!(
        item,
        Item {
            inner: ItemEnum::Struct(_),
            ..
        }
    )
}

pub fn is_struct_unit(item: &Item) -> bool {
    matches!(
        item,
        Item {
            inner: ItemEnum::Struct(Struct {
                kind: StructKind::Unit,
                ..
            }),
            ..
        }
    )
}

pub fn is_struct_plain(item: &Item) -> bool {
    matches!(
        item,
        Item {
            inner: ItemEnum::Struct(Struct {
                kind: StructKind::Plain { .. },
                ..
            }),
            ..
        }
    )
}

pub fn is_struct_tuple(item: &Item) -> bool {
    matches!(
        item,
        Item {
            inner: ItemEnum::Struct(Struct {
                kind: StructKind::Tuple(_),
                ..
            }),
            ..
        }
    )
}

pub fn has_field(item: &Item, field: &Item) -> bool {
    match item {
        Item {
            inner: ItemEnum::Struct(Struct { kind, .. }),
            ..
        } => match kind {
            StructKind::Unit => false,
            StructKind::Tuple(fields) => fields.contains(&Some(field.id)),
            StructKind::Plain {
                fields,
                has_stripped_fields: _,
            } => fields.contains(&field.id),
        },
        Item {
            inner: ItemEnum::Variant(Variant { kind, .. }),
            ..
        } => match kind {
            VariantKind::Plain => false,
            VariantKind::Tuple(fields) => fields.contains(&Some(field.id)),
            VariantKind::Struct { fields, .. } => fields.contains(&field.id),
        },
        _ => false,
    }
}

pub fn field_ids(item: &Item) -> Vec<Id> {
    match item {
        Item {
            inner: ItemEnum::Struct(Struct { kind, .. }),
            ..
        } => match kind {
            StructKind::Plain { fields, .. } => fields.to_vec(),
            StructKind::Tuple(fields) => fields
                .iter()
                .filter_map(|f| f.as_ref())
                .cloned()
                .collect_vec(),
            StructKind::Unit => vec![],
        },
        Item {
            inner: ItemEnum::Variant(Variant { kind, .. }),
            ..
        } => match kind {
            VariantKind::Plain => vec![],
            VariantKind::Tuple(fields) => fields
                .iter()
                .filter_map(|f| f.as_ref())
                .cloned()
                .collect_vec(),
            VariantKind::Struct { fields, .. } => fields.to_vec(),
        },
        _ => vec![],
    }
}

fn is_struct_field(item: &Item) -> bool {
    matches!(
        item,
        Item {
            inner: ItemEnum::StructField(_),
            ..
        }
    )
}

pub fn is_enum(item: &Item) -> bool {
    matches!(
        item,
        Item {
            inner: ItemEnum::Enum(_),
            ..
        }
    )
}

fn is_enum_variant(item: &Item) -> bool {
    matches!(
        item,
        Item {
            inner: ItemEnum::Variant(_),
            ..
        }
    )
}

pub fn has_variant(item: &Item, variant: &Item) -> bool {
    match item {
        Item {
            inner: ItemEnum::Enum(Enum { variants, .. }),
            ..
        } => variants.contains(&variant.id),
        _ => false,
    }
}

pub fn variant_ids(item: &Item) -> Vec<Id> {
    match item {
        Item {
            inner: ItemEnum::Enum(Enum { variants, .. }),
            ..
        } => variants.to_vec(),
        _ => vec![],
    }
}

pub fn is_plain_variant(item: &Item) -> bool {
    matches!(
        item,
        Item {
            inner: ItemEnum::Variant(Variant {
                kind: VariantKind::Plain,
                ..
            }),
            ..
        }
    )
}

pub fn is_struct_variant(item: &Item) -> bool {
    matches!(
        item,
        Item {
            inner: ItemEnum::Variant(Variant {
                kind: VariantKind::Struct { .. },
                ..
            }),
            ..
        }
    )
}

pub fn is_tuple_variant(item: &Item) -> bool {
    matches!(
        item,
        Item {
            inner: ItemEnum::Variant(Variant {
                kind: VariantKind::Tuple(_),
                ..
            }),
            ..
        }
    )
}

pub fn is_impl(item: &Item) -> bool {
    matches!(
        item,
        Item {
            inner: ItemEnum::Impl(Impl {
                trait_: Some(Path { path, .. }),
                ..
            }),
            ..
            } if (["App", "Effect", "Capability", "Operation"]).contains(&path.as_str())
    )
}

pub fn is_impl_for(item: &Item, for_: &Item, trait_name: &str) -> bool {
    matches!(item, Item {
            inner:
                ItemEnum::Impl(Impl {
                    trait_: Some(Path { path, .. }),
                    for_: Type::ResolvedPath(Path { id, .. }),
                    ..
                }),
            ..
        } if path == trait_name && id == &for_.id)
}

fn is_associated_type(item: &Item) -> bool {
    matches!(
        item,
        Item {
            inner: ItemEnum::AssocType { .. },
            ..
        }
    )
}

pub fn has_associated_item(item: &Item, associated_item: &Item, with_name: &str) -> bool {
    match item {
        Item {
            inner: ItemEnum::Impl(Impl { items, .. }),
            ..
        } => match &associated_item {
            Item {
                name: Some(name), ..
            } if with_name == name => items.contains(&associated_item.id),
            _ => false,
        },
        _ => false,
    }
}
