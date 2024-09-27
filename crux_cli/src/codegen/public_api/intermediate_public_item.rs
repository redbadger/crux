use rustdoc_types::Item;

use super::nameable_item::NameableItem;
use super::path_component::PathComponent;
use super::public_item::PublicItemPath;
use super::render::RenderingContext;
use super::tokens::Token;

/// This struct represents one public item of a crate, but in intermediate form.
/// Conceptually it wraps a single [`Item`] even though the path to the item
/// consists of many [`Item`]s. Later, one [`Self`] will be converted to exactly
/// one [`crate::PublicItem`].
#[derive(Clone, Debug)]
pub struct IntermediatePublicItem<'c> {
    path: Vec<PathComponent<'c>>,
}

impl<'c> IntermediatePublicItem<'c> {
    pub fn new(path: Vec<PathComponent<'c>>) -> Self {
        Self { path }
    }

    #[must_use]
    pub fn item(&self) -> &'c Item {
        self.path()
            .last()
            .expect("path must not be empty")
            .item
            .item
    }

    #[must_use]
    pub fn path(&self) -> &[PathComponent<'c>] {
        &self.path
    }

    /// See [`crate::item_processor::sorting_prefix()`] docs for an explanation why we have this.
    #[must_use]
    pub fn sortable_path(&self, context: &RenderingContext) -> PublicItemPath {
        self.path()
            .iter()
            .map(|p| NameableItem::sortable_name(&p.item, context))
            .collect()
    }

    #[must_use]
    pub fn path_contains_renamed_item(&self) -> bool {
        self.path().iter().any(|m| m.item.overridden_name.is_some())
    }

    pub fn render_token_stream(&self, context: &RenderingContext) -> Vec<Token> {
        context.token_stream(self)
    }
}
