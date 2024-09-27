use rustdoc_types::{Crate, Id, Item};

/// The [`Crate`] type represents the deserialized form of the rustdoc JSON
/// input. This wrapper adds some helpers and state on top.
pub struct CrateWrapper<'c> {
    crate_: &'c Crate,

    /// Normally, an item referenced by [`Id`] is present in the rustdoc JSON.
    /// If [`Self::crate_.index`] is missing an [`Id`], then we add it here, to
    /// aid with debugging. It will typically be missing because of bugs (or
    /// borderline bug such as re-exports of foreign items like discussed in
    /// <https://github.com/rust-lang/rust/pull/99287#issuecomment-1186586518>)
    /// We do not report it to users by default, because they can't do anything
    /// about it. Missing IDs will be printed with `--verbose` however.
    missing_ids: Vec<&'c Id>,
}

impl<'c> CrateWrapper<'c> {
    pub fn new(crate_: &'c Crate) -> Self {
        Self {
            crate_,
            missing_ids: vec![],
        }
    }

    pub fn get_item(&mut self, id: &'c Id) -> Option<&'c Item> {
        self.crate_.index.get(id).or_else(|| {
            self.missing_ids.push(id);
            None
        })
    }

    pub fn missing_item_ids(&self) -> Vec<String> {
        self.missing_ids.iter().map(|m| m.0.clone()).collect()
    }
}
