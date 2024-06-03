use super::nameable_item::NameableItem;
use super::CrateWrapper;
use super::{
    intermediate_public_item::IntermediatePublicItem, path_component::PathComponent,
    public_item::PublicItem, render::RenderingContext,
};
use rustdoc_types::{
    Crate, Id, Impl, Import, Item, ItemEnum, Module, Struct, StructKind, Type, Variant, VariantKind,
};
use std::convert::identity;
use std::{
    collections::{HashMap, VecDeque},
    vec,
};

/// Items in rustdoc JSON reference each other by Id. The [`ItemProcessor`]
/// essentially takes one Id at a time and figure out what to do with it. Once
/// complete, the item is ready to be listed as part of the public API, and
/// optionally can also be used as part of a path to another (child) item.
///
/// This struct contains a (processed) path to an item that is about to be
/// processed further, and the Id of that item.
#[derive(Debug)]
struct UnprocessedItem<'c> {
    /// The path to the item to process.
    parent_path: Vec<PathComponent<'c>>,

    /// The Id of the item to process.
    id: &'c Id,
}

/// Processes items to find more items and to figure out the path to each item.
/// Some non-obvious cases to take into consideration are:
/// 1. A single item is imported several times.
/// 2. An item is (publicly) imported from another crate
///
/// Note that this implementation iterates over everything, so if the rustdoc
/// JSON is generated with `--document-private-items`, then private items will
/// also be included in the output. Use with `--document-private-items` is not
/// supported.
pub struct ItemProcessor<'c> {
    /// The original and unmodified rustdoc JSON, in deserialized form.
    crate_: CrateWrapper<'c>,

    /// A queue of unprocessed items to process.
    work_queue: VecDeque<UnprocessedItem<'c>>,

    /// The output. A list of processed items. Note that the order is
    /// intentionally "logical", so that e.g. struct fields items follows from
    /// struct items.
    pub output: Vec<IntermediatePublicItem<'c>>,
}

impl<'c> ItemProcessor<'c> {
    pub(crate) fn new(crate_: &'c Crate) -> Self {
        ItemProcessor {
            crate_: CrateWrapper::new(crate_),
            work_queue: VecDeque::new(),
            output: vec![],
        }
    }

    /// Adds an item to the front of the work queue. We want to add items to the
    /// front of the work queue since we process items from the top, and since
    /// we want to "fully" process an item before we move on to the next one. So
    /// when we encounter a struct, we also encounter its struct fields, and we
    /// want to insert the struct fields BEFORE everything else, so that these
    /// items remain grouped together. And the same applies for many kinds of
    /// groupings (enums, impls, etc).
    pub fn add_to_work_queue(&mut self, parent_path: Vec<PathComponent<'c>>, id: &'c Id) {
        self.work_queue
            .push_front(UnprocessedItem { parent_path, id });
    }

    /// Processes the entire work queue. Adds more items based on items it
    /// processes. When this returns, all items and their children and impls
    /// have been recursively processed.
    pub fn run(&mut self) {
        while let Some(unprocessed_item) = self.work_queue.pop_front() {
            if let Some(item) = self.crate_.get_item(unprocessed_item.id) {
                self.process_any_item(item, unprocessed_item);
            }
        }
    }

    /// Process any item. In particular, does the right thing if the item is an
    /// impl or an import.
    fn process_any_item(&mut self, item: &'c Item, unprocessed_item: UnprocessedItem<'c>) {
        match &item.inner {
            ItemEnum::Import(import) => {
                if import.glob {
                    self.process_import_glob_item(import, unprocessed_item, item);
                } else {
                    self.process_import_item(item, import, unprocessed_item);
                }
            }
            ItemEnum::Impl(impl_) => {
                self.process_impl_item(unprocessed_item, item, impl_);
            }
            _ => {
                self.process_item_unless_recursive(unprocessed_item, item, None);
            }
        }
    }

    /// We need to handle `pub use foo::*` specially. In case of such wildcard
    /// imports, `glob` will be `true` and `id` will be the module we should
    /// import all items from, but we should NOT add the module itself. Before
    /// we inline this wildcard import, make sure that the module is not
    /// indirectly trying to import itself. If we allow that, we'll get a stack
    /// overflow.
    fn process_import_glob_item(
        &mut self,
        import: &'c Import,
        unprocessed_item: UnprocessedItem<'c>,
        item: &'c Item,
    ) {
        if let Some(Item {
            inner: ItemEnum::Module(Module { items, .. }),
            ..
        }) = import
            .id
            .as_ref()
            .and_then(|id| self.get_item_if_not_in_path(&unprocessed_item.parent_path, id))
        {
            for item_id in items {
                self.add_to_work_queue(unprocessed_item.parent_path.clone(), item_id);
            }
        } else {
            self.process_item(
                unprocessed_item,
                item,
                Some(format!("<<{}::*>>", import.source)),
            );
        }
    }

    /// Since public imports are part of the public API, we inline them, i.e.
    /// replace the item corresponding to an import with the item that is
    /// imported. If we didn't do this, publicly imported items would show up as
    /// just e.g. `pub use some::function`, which is not sufficient for the use
    /// cases of this tool. We want to show the actual API, and thus also show
    /// type information! There is one exception; for re-exports of primitive
    /// types, there is no item Id to inline with, so they remain as e.g. `pub
    /// use my_i32` in the output.
    fn process_import_item(
        &mut self,
        item: &'c Item,
        import: &'c Import,
        unprocessed_item: UnprocessedItem<'c>,
    ) {
        let mut actual_item = item;

        if let Some(imported_item) = import
            .id
            .as_ref()
            .and_then(|id| self.get_item_if_not_in_path(&unprocessed_item.parent_path, id))
        {
            actual_item = imported_item;
        }

        self.process_item(unprocessed_item, actual_item, Some(import.name.clone()));
    }

    /// Processes impls. Impls are special because we support filtering out e.g.
    /// blanket implementations to reduce output noise.
    fn process_impl_item(
        &mut self,
        unprocessed_item: UnprocessedItem<'c>,
        item: &'c Item,
        impl_: &'c Impl,
    ) {
        if !ImplKind::from(item, impl_).is_active() {
            return;
        }

        self.process_item_for_type(unprocessed_item, item, None, Some(&impl_.for_));
    }

    /// Make sure the item we are about to process is not already part of the
    /// item path. If it is, we have encountered recursion. Stop processing in
    /// that case.
    fn process_item_unless_recursive(
        &mut self,
        unprocessed_item: UnprocessedItem<'c>,
        item: &'c Item,
        overridden_name: Option<String>,
    ) {
        if unprocessed_item
            .parent_path
            .iter()
            .any(|m| m.item.item.id == item.id)
        {
            let recursion_breaker = unprocessed_item.finish(
                item,
                Some(format!("<<{}>>", item.name.as_deref().unwrap_or(""))),
                None,
            );
            self.output.push(recursion_breaker);
        } else {
            self.process_item(unprocessed_item, item, overridden_name);
        }
    }

    /// Process an item. Setup jobs for its children and impls and and then put
    /// it in the output.
    fn process_item(
        &mut self,
        unprocessed_item: UnprocessedItem<'c>,
        item: &'c Item,
        overridden_name: Option<String>,
    ) {
        self.process_item_for_type(unprocessed_item, item, overridden_name, None);
    }

    /// Process an item. Setup jobs for its children and impls and and then put
    /// it in the output.
    fn process_item_for_type(
        &mut self,
        unprocessed_item: UnprocessedItem<'c>,
        item: &'c Item,
        overridden_name: Option<String>,
        type_: Option<&'c Type>,
    ) {
        let finished_item = unprocessed_item.finish(item, overridden_name, type_);

        let children = children_for_item(item);
        let impls = impls_for_item(item).into_iter().flatten();

        if item.id == Id("0:428:2145".to_string()) {
            println!("Processing: {:?}", item.id);
        }
        for id in children {
            let parent_path = finished_item.path().into();
            self.add_to_work_queue(parent_path, id);
        }

        // As usual, impls are special. We want impl items to appear grouped
        // with the trait or type it involves. But when _rendering_ we want to
        // use the type that we implement for, so that e.g. generic arguments
        // can be shown. So hide the "sorting path" of the impl. We'll instead
        // render the path to the type the impl is for.
        for id in impls {
            let mut path = finished_item.path().to_vec();
            for a in &mut path {
                a.hide = true;
            }
            self.add_to_work_queue(path, id);
        }

        self.output.push(finished_item);
    }

    /// Get the rustdoc JSON item with `id`, but only if it is not already part
    /// of the path. This can happen in the case of recursive re-exports, in
    /// which case we need to break the recursion.
    fn get_item_if_not_in_path(
        &mut self,
        parent_path: &[PathComponent<'c>],
        id: &'c Id,
    ) -> Option<&'c Item> {
        if parent_path.iter().any(|m| m.item.item.id == *id) {
            // The item is already in the path! Break import recursion...
            return None;
        }

        self.crate_.get_item(id)
    }

    // Returns a HashMap where a rustdoc JSON Id is mapped to what public items
    // that have this ID. The reason this is a one-to-many mapping is because of
    // re-exports. If an API re-exports a public item in a different place, the
    // same item will be reachable by different paths, and thus the Vec will
    // contain many [`IntermediatePublicItem`]s for that ID.
    //
    // You might think this is rare, but it is actually a common thing in
    // real-world code.
    pub fn id_to_items(&self) -> HashMap<&Id, Vec<&IntermediatePublicItem>> {
        let mut id_to_items: HashMap<&Id, Vec<&IntermediatePublicItem>> = HashMap::new();
        for finished_item in &self.output {
            id_to_items
                .entry(&finished_item.item().id)
                .or_default()
                .push(finished_item);
        }
        id_to_items
    }
}

impl<'c> UnprocessedItem<'c> {
    /// Turns an [`UnprocessedItem`] into a finished [`IntermediatePublicItem`].
    fn finish(
        self,
        item: &'c Item,
        overridden_name: Option<String>,
        type_: Option<&'c Type>,
    ) -> IntermediatePublicItem<'c> {
        // Transfer path ownership to output item
        let mut path = self.parent_path;

        // Complete the path with the last item
        path.push(PathComponent {
            item: NameableItem {
                item,
                overridden_name,
                sorting_prefix: sorting_prefix(item),
            },
            type_,
            hide: false,
        });

        // Done
        IntermediatePublicItem::new(path)
    }
}

/// In order for items in the output to be nicely grouped, we add a prefix to
/// each item in the path to an item. That way, sorting on the name (with this
/// prefix) will group items. But we don't want this prefix to be be visible to
/// users of course, so we do this "behind the scenes".
pub(crate) fn sorting_prefix(item: &Item) -> u8 {
    match &item.inner {
        ItemEnum::ExternCrate { .. } => 1,
        ItemEnum::Import(_) => 2,

        ItemEnum::Primitive(_) => 3,

        ItemEnum::Module(_) => 4,

        ItemEnum::Macro(_) => 5,
        ItemEnum::ProcMacro(_) => 6,

        ItemEnum::Enum(_) => 7,
        ItemEnum::Union(_) => 8,
        ItemEnum::Struct(_) => 9,
        ItemEnum::StructField(_) => 10,
        ItemEnum::Variant(_) => 11,

        ItemEnum::Constant(_) => 12,

        ItemEnum::Static(_) => 13,

        ItemEnum::Trait(_) => 14,

        ItemEnum::AssocType { .. } => 15,
        ItemEnum::AssocConst { .. } => 16,

        ItemEnum::Function(_) => 17,

        ItemEnum::TypeAlias(_) => 19,

        ItemEnum::Impl(impl_) => match ImplKind::from(item, impl_) {
            // To not cause a diff when changing a manual impl to an
            // auto-derived impl (or vice versa), we put them in the same group.
            ImplKind::Inherent => 20,
            ImplKind::Trait | ImplKind::AutoDerived => 21,
            ImplKind::AutoTrait => 23,
            ImplKind::Blanket => 24,
        },

        ItemEnum::ForeignType => 25,

        ItemEnum::OpaqueTy(_) => 26,

        ItemEnum::TraitAlias(_) => 27,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ImplKind {
    /// E.g. `impl Foo` or `impl<'a> Foo<'a>`
    Inherent,

    /// E.g. `impl Bar for Foo {}`
    Trait,

    /// Auto-generated by `#[derive(...)]`
    AutoDerived,

    /// Auto-trait impl such as `impl Sync for Foo`
    AutoTrait,

    /// Blanket impls such as `impl<T> Any for T`
    Blanket,
}

impl ImplKind {
    fn from(impl_item: &Item, impl_: &Impl) -> Self {
        let has_blanket_impl = impl_.blanket_impl.is_some();
        let is_automatically_derived = impl_item
            .attrs
            .iter()
            .any(|a| a == "#[automatically_derived]");

        // See https://github.com/rust-lang/rust/blob/54f20bbb8a7aeab93da17c0019c1aaa10329245a/src/librustdoc/json/conversions.rs#L589-L590
        match (impl_.synthetic, has_blanket_impl) {
            (true, false) => ImplKind::AutoTrait,
            (false, true) => ImplKind::Blanket,
            _ if is_automatically_derived => ImplKind::AutoDerived,
            _ if impl_.trait_.is_none() => ImplKind::Inherent,
            _ => ImplKind::Trait,
        }
    }
}

impl ImplKind {
    fn is_active(&self) -> bool {
        match self {
            ImplKind::Blanket | ImplKind::AutoTrait | ImplKind::AutoDerived => false,
            ImplKind::Inherent | ImplKind::Trait => true,
        }
    }
}

/// Some items contain other items, which is relevant for analysis. Keep track
/// of such relationships.
fn children_for_item(item: &Item) -> Vec<&Id> {
    match &item.inner {
        ItemEnum::Module(m) => m.items.iter().collect(),
        ItemEnum::Union(u) => u.fields.iter().collect(),
        ItemEnum::Struct(Struct {
            kind: StructKind::Plain { fields, .. },
            ..
        }) => fields.iter().collect(),
        ItemEnum::Variant(Variant {
            kind: VariantKind::Struct { fields, .. },
            ..
        }) => fields.iter().collect(),
        ItemEnum::Variant(Variant {
            kind: VariantKind::Plain,
            ..
        }) => vec![],
        ItemEnum::Variant(Variant {
            kind: VariantKind::Tuple(fields),
            ..
        }) => fields.iter().flatten().collect(),
        ItemEnum::Struct(Struct {
            kind: StructKind::Unit,
            ..
        }) => vec![],
        ItemEnum::Struct(Struct {
            kind: StructKind::Tuple(fields),
            ..
        }) => fields.iter().flatten().collect(),
        ItemEnum::Enum(e) => e.variants.iter().collect(),
        ItemEnum::Trait(t) => t.items.iter().collect(),
        ItemEnum::Impl(i) => i.items.iter().collect(),
        ItemEnum::ExternCrate { .. } => vec![],
        ItemEnum::Import(_) => vec![],
        ItemEnum::StructField(_) => vec![],
        ItemEnum::Function(_) => vec![],
        ItemEnum::TraitAlias(_) => vec![],
        ItemEnum::TypeAlias(_) => vec![],
        ItemEnum::OpaqueTy(_) => vec![],
        ItemEnum::Constant(_) => vec![],
        ItemEnum::Static(_) => vec![],
        ItemEnum::ForeignType => vec![],
        ItemEnum::Macro(_) => vec![],
        ItemEnum::ProcMacro(_) => vec![],
        ItemEnum::Primitive(_) => vec![],
        ItemEnum::AssocConst { .. } => vec![],
        ItemEnum::AssocType { .. } => vec![],
    }
}

pub fn impls_for_item(item: &Item) -> Option<&[Id]> {
    match &item.inner {
        ItemEnum::Union(u) => Some(&u.impls),
        ItemEnum::Struct(s) => Some(&s.impls),
        ItemEnum::Enum(e) => Some(&e.impls),
        ItemEnum::Primitive(p) => Some(&p.impls),
        ItemEnum::Trait(t) => Some(&t.implementations),
        _ => None,
    }
}
