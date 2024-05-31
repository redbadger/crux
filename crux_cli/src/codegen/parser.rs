use std::collections::HashMap;

use anyhow::Result;
use rustdoc_types::{
    Crate, GenericArg, GenericArgs, Id, Impl, ItemEnum, Path, StructKind, Type, VariantKind,
};

use crate::codegen::rust_types::{RustField, SpecialRustType};

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

/// An import visitor that collects all use or
/// qualified referenced items.
#[derive(Default)]
pub struct Visitor {
    parsed_data: ParsedData,
}

impl Visitor {
    pub fn new() -> Self {
        Self {
            parsed_data: ParsedData::new(),
        }
    }

    fn visit_item(
        &mut self,
        level: usize,
        name: &str,
        parent: &Id,
        id: &Id,
        crate_: &Crate,
    ) -> Result<()> {
        print!(
            "\n{level} {id:18} {} {name:20} ",
            " ".repeat(level * 4),
            id = format!("{:?}", id)
        );

        if let Some(summary) = crate_.paths.get(id) {
            let path_str = summary.path.join("::");
            print!("{path_str}");
        }

        if let Some(item) = crate_.index.get(id) {
            match &item.inner {
                ItemEnum::Struct(ref struct_) => match &struct_.kind {
                    StructKind::Unit => {
                        print!("unit struct");
                    }
                    StructKind::Tuple(fields) => {
                        print!("tuple struct: {fields:?}");
                    }
                    StructKind::Plain {
                        fields,
                        fields_stripped,
                    } => {
                        if *fields_stripped {
                            anyhow::bail!("The {name} struct has private fields. You may need to make them public to use them in your code.");
                        }
                        let parent = id.clone();
                        let _entry = self
                            .parsed_data
                            .structs
                            .entry(parent.clone())
                            .or_insert_with(|| RustStruct::new(parent.clone().into()));
                        for field in fields {
                            let item = &crate_.index[field];
                            if let Some(name) = &item.name {
                                self.visit_item(level + 1, name, &parent, field, crate_)?;
                            }
                        }
                    }
                },
                ItemEnum::Enum(ref enum_) => {
                    let parent = id;
                    for id in &enum_.variants {
                        let item = &crate_.index[id];
                        if let Some(name) = &item.name {
                            if !&item.attrs.contains(&"#[serde(skip)]".to_string()) {
                                self.visit_item(level + 1, name, parent, id, crate_)?;
                            }
                        }
                    }
                }
                ItemEnum::StructField(ty) => {
                    if let Some(ty) = self.visit_type(level, "", id, ty, crate_)? {
                        self.parsed_data
                            .structs
                            .get_mut(parent)
                            .unwrap()
                            .fields
                            .push(RustField {
                                id: id.clone().into(),
                                ty,
                                comments: vec![],
                                has_default: false,
                            });
                    }
                }
                ItemEnum::Module(_) => (),
                ItemEnum::ExternCrate { .. } => (),
                ItemEnum::Import(_) => (),
                ItemEnum::Union(_) => (),
                ItemEnum::Variant(v) => match &v.kind {
                    VariantKind::Plain => {}
                    VariantKind::Tuple(fields) => {
                        let parent = id;
                        for id in fields {
                            let Some(id) = id else { continue };
                            let item = &crate_.index[id];
                            if let Some(name) = &item.name {
                                self.visit_item(level + 1, name, parent, id, crate_)?;
                            }
                        }
                    }
                    VariantKind::Struct {
                        fields,
                        fields_stripped,
                    } => {
                        if *fields_stripped {
                            anyhow::bail!("The {name} struct has private fields. You may need to make them public to use them in your code.");
                        }
                        let parent = id;
                        for id in fields {
                            let item = &crate_.index[id];
                            if let Some(name) = &item.name {
                                self.visit_item(level + 1, name, parent, id, crate_)?;
                            }
                        }
                    }
                },
                ItemEnum::Function(_) => (),
                ItemEnum::Trait(_) => (),
                ItemEnum::TraitAlias(_) => (),
                ItemEnum::Impl(_) => (),
                ItemEnum::TypeAlias(_) => (),
                ItemEnum::OpaqueTy(_) => (),
                ItemEnum::Constant(_) => (),
                ItemEnum::Static(_) => (),
                ItemEnum::ForeignType => (),
                ItemEnum::Macro(_) => (),
                ItemEnum::ProcMacro(_) => (),
                ItemEnum::Primitive(_) => (),
                ItemEnum::AssocConst { .. } => (),
                ItemEnum::AssocType { .. } => (),
            }
        }
        Ok(())
    }

    fn visit_type(
        &mut self,
        level: usize,
        name: &str,
        parent: &Id,
        ty: &Type,
        crate_: &Crate,
    ) -> Result<Option<RustType>> {
        let out_type: Option<RustType> = match ty {
            Type::ResolvedPath(path) => {
                self.visit_item(level + 1, name, parent, &path.id, crate_)?;
                if let Some(args) = &path.args {
                    match args.as_ref() {
                        GenericArgs::AngleBracketed { args, bindings: _ } => {
                            for (i, arg) in args.iter().enumerate() {
                                match arg {
                                    GenericArg::Lifetime(_) => todo!(),
                                    GenericArg::Type(ty) => {
                                        print!("  ");
                                        self.visit_type(level, &i.to_string(), parent, ty, crate_)?;
                                    }
                                    GenericArg::Const(_) => todo!(),
                                    GenericArg::Infer => todo!(),
                                }
                            }
                            None
                        }
                        GenericArgs::Parenthesized { .. } => None,
                    }
                } else {
                    None
                }
            }
            Type::DynTrait(_) => None,
            Type::Generic(s) => {
                print!("{s}");
                None
            }
            Type::Primitive(name) => {
                print!("{name}");
                let out_type = SpecialRustType::try_from(name.as_str())
                    .map_err(|e| anyhow::anyhow!(e.to_owned()))?;
                Some(RustType::Special(out_type))
            }
            Type::FunctionPointer(_) => None,
            Type::Tuple(types) => {
                for (i, ty) in types.iter().enumerate() {
                    self.visit_type(level, &i.to_string(), parent, ty, crate_)?;
                }
                None
            }
            Type::Slice(_) => None,
            Type::Array { type_: _, len: _ } => None,
            Type::ImplTrait(_) => None,
            Type::Infer => None,
            Type::RawPointer { .. } => None,
            Type::BorrowedRef { .. } => None,
            Type::QualifiedPath {
                name: _,
                args: _,
                self_type: _,
                trait_: _,
            } => None,
        };
        Ok(out_type)
    }
}

pub fn parse(crate_: Crate) -> Result<ParsedData> {
    let mut visitor = Visitor::new();
    for (id, associated_items) in find_impls(&crate_, "Effect", &["Ffi"]) {
        println!(
            "\nThe struct that implements crux_core::Effect is {}",
            crate_.paths[id].path.join("::")
        );

        let parent = id;
        for (name, id) in associated_items {
            visitor.visit_item(0, name, parent, id, &crate_)?;
        }
    }
    println!();
    for (id, associated_items) in find_impls(&crate_, "App", &["Event", "ViewModel"]) {
        println!(
            "\nThe struct that implements crux_core::App is {}",
            crate_.paths[id].path.join("::")
        );

        let parent = id;
        for (name, id) in associated_items {
            visitor.visit_item(0, name, parent, id, &crate_)?;
        }
    }
    Ok(visitor.parsed_data)
}

fn find_impls<'a>(
    crate_: &'a Crate,
    trait_name: &'a str,
    filter: &'a [&'a str],
) -> impl Iterator<Item = (&'a Id, Vec<(&'a str, &'a Id)>)> {
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
                                    Some((name, id))
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
