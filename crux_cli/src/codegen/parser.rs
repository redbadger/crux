use std::collections::HashMap;

use anyhow::Result;
use rustdoc_types::{
    Crate, GenericArg, GenericArgs, Id, Impl, ItemEnum, Path, StructKind, Type, VariantKind,
};

use crate::codegen::{
    item_processor::ItemProcessor,
    public_item::PublicItem,
    render::RenderingContext,
    rust_types::{RustField, SpecialRustType},
};

use super::rust_types::{RustEnum, RustStruct, RustType, RustTypeAlias};

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
    for (id, associated_items) in find_roots(crate_, "Effect", &["Ffi"]) {
        println!(
            "\nThe struct that implements crux_core::Effect is {}",
            crate_.paths[id].path.join("::")
        );

        for id in associated_items {
            item_processor.add_to_work_queue(vec![], id);
        }
    }

    for (id, associated_items) in find_roots(crate_, "App", &["Event", "ViewModel"]) {
        println!(
            "\nThe struct that implements crux_core::App is {}",
            crate_.paths[id].path.join("::")
        );

        for id in associated_items {
            item_processor.add_to_work_queue(vec![], id);
        }
    }

    item_processor.run();

    let context = RenderingContext {
        crate_,
        id_to_items: item_processor.id_to_items(),
    };
    let items = item_processor
        .output
        .iter()
        .filter_map(|item| {
            if match &item.item().inner {
                ItemEnum::Union(_) => true,
                ItemEnum::Struct(_) => true,
                ItemEnum::StructField(_) => true,
                ItemEnum::Enum(_) => true,
                ItemEnum::Variant(_) => true,
                ItemEnum::Primitive(_) => true,
                ItemEnum::TypeAlias(_) => true,
                ItemEnum::ForeignType => true,

                ItemEnum::Module(_) => false,
                ItemEnum::ExternCrate { .. } => false,
                ItemEnum::Import(_) => false,
                ItemEnum::Function(_) => false,
                ItemEnum::Trait(_) => false,
                ItemEnum::TraitAlias(_) => false,
                ItemEnum::Impl(_) => false,
                ItemEnum::OpaqueTy(_) => false,
                ItemEnum::Constant(_) => true,
                ItemEnum::Static(_) => false,
                ItemEnum::Macro(_) => false,
                ItemEnum::ProcMacro(_) => false,
                ItemEnum::AssocConst { .. } => false,
                ItemEnum::AssocType { .. } => false,
            } {
                Some(PublicItem::from_intermediate_public_item(&context, item))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    println!("{items:#?}");
    Ok(ParsedData::new())
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
