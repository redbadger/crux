use rustdoc_types::Type;

use super::nameable_item::NameableItem;

/// A public item in a public API can only be referenced via a path. For example
/// `mod_a::mod_b::StructC`. A `PathComponent` represents one component of such
/// a path. A path component can either be a Rust item, or a Rust type. Normally
/// it is an item. The typical case when it is a type is when there are generic
/// arguments involved. For example, `Option<usize>` is a type. The
/// corresponding item is `Option<T>` (no specific generic args has been
/// specified for the generic parameter T).
#[derive(Clone, Debug)]
pub struct PathComponent<'c> {
    /// The item for this path component.
    pub item: NameableItem<'c>,

    /// The type, if applicable. If we have a type, we'll want to use that
    /// instead of `item`, since the type might have specific generic args.
    pub type_: Option<&'c Type>,

    /// If `true`, do not render this path component to users.
    pub hide: bool,
}
