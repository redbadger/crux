use std::collections::HashMap;

use anyhow::Result;
use rustdoc_types::{Crate, Id, Impl, ItemEnum, Path, Type};

use crate::codegen::{
    item_processor::{sorting_prefix, ItemProcessor},
    nameable_item::NameableItem,
    path_component::PathComponent,
    public_item::PublicItem,
    render::RenderingContext,
};

use super::rust_types::{RustEnum, RustStruct, RustTypeAlias};

/// The results of parsing Rust source input.
#[derive(Default, Debug)]
pub struct ParsedData {
    /// Structs defined in the source
    pub structs: HashMap<Id, RustStruct>,
    /// Enums defined in the source
    pub enums: HashMap<Id, RustEnum>,
    /// Type aliases defined in the source
    pub aliases: HashMap<Id, RustTypeAlias>,
}

impl ParsedData {
    pub fn new() -> Self {
        Default::default()
    }
}

pub fn parse(crate_: &Crate) -> Result<ParsedData> {
    let mut item_processor = ItemProcessor::new(crate_);
    add_items(crate_, "Effect", &["Ffi"], &mut item_processor);
    add_items(crate_, "App", &["Event", "ViewModel"], &mut item_processor);
    item_processor.run();

    let context = RenderingContext {
        crate_,
        id_to_items: item_processor.id_to_items(),
    };
    let items: Vec<PublicItem> = item_processor
        .output
        .iter()
        .filter_map(|item| {
            matches!(
                &item.item().inner,
                ItemEnum::Union(_)
                    | ItemEnum::Struct(_)
                    | ItemEnum::StructField(_)
                    | ItemEnum::Enum(_)
                    | ItemEnum::Variant(_)
                    | ItemEnum::Primitive(_)
                    | ItemEnum::TypeAlias(_)
                    | ItemEnum::ForeignType
            )
            .then_some(PublicItem::from_intermediate_public_item(&context, item))
        })
        .collect();
    println!("{items:#?}");
    Ok(ParsedData::new())
}

fn add_items<'c: 'p, 'p>(
    crate_: &'c Crate,
    trait_name: &'c str,
    filter: &'c [&'c str],
    item_processor: &'p mut ItemProcessor<'c>,
) {
    for (id, associated_items) in find_roots(crate_, trait_name, filter) {
        let path = &crate_.paths[id].path;
        println!("{} implements {trait_name}", path.join("::"));

        let item = &crate_.index[id];
        let module = path[..path.len() - 1].join("::");
        for id in associated_items {
            let parent = PathComponent {
                item: NameableItem {
                    item,
                    overridden_name: Some(module.clone()),
                    sorting_prefix: sorting_prefix(item),
                },
                type_: None,
                hide: false,
            };
            item_processor.add_to_work_queue(vec![parent], id);
        }
    }
}

fn find_roots<'a>(
    crate_: &'a Crate,
    trait_name: &'a str,
    filter: &'a [&'a str],
) -> impl Iterator<Item = (&'a Id, Vec<&'a Id>)> {
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
                                    Some(id)
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
