use super::public_item::PublicItem;

/// The public API of a crate
///
/// Create an instance with [`Builder`].
///
/// ## Rendering the items
///
/// To render the items in the public API you can iterate over the [items](PublicItem).
///
/// You get the `rustdoc_json_str` in the example below as explained in the [crate] documentation, either via
/// [`rustdoc_json`](https://crates.io/crates/rustdoc_json) or by calling `cargo rustdoc` yourself.
///
/// ```no_run
/// use public_api::PublicApi;
/// use std::path::PathBuf;
///
/// # let rustdoc_json: PathBuf = todo!();
/// // Gather the rustdoc content as described in this crates top-level documentation.
/// let public_api = public_api::Builder::from_rustdoc_json(&rustdoc_json).build()?;
///
/// for public_item in public_api.items() {
///     // here we print the items to stdout, we could also write to a string or a file.
///     println!("{}", public_item);
/// }
///
/// // If you want all items of the public API in a single big multi-line String then
/// // you can do like this:
/// let public_api_string = public_api.to_string();
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug)]
#[non_exhaustive] // More fields might be added in the future
pub struct PublicApi {
    /// The items that constitutes the public API. An "item" is for example a
    /// function, a struct, a struct field, an enum, an enum variant, a module,
    /// etc...
    pub(crate) items: Vec<PublicItem>,

    /// See [`Self::missing_item_ids()`]
    pub(crate) missing_item_ids: Vec<String>,
}

impl PublicApi {
    /// Returns an iterator over all public items in the public API
    pub fn items(&self) -> impl Iterator<Item = &'_ PublicItem> {
        self.items.iter()
    }

    /// Like [`Self::items()`], but ownership of all `PublicItem`s are
    /// transferred to the caller.
    pub fn into_items(self) -> impl Iterator<Item = PublicItem> {
        self.items.into_iter()
    }

    /// The rustdoc JSON IDs of missing but referenced items. Intended for use
    /// with `--verbose` flags or similar.
    ///
    /// In some cases, a public item might be referenced from another public
    /// item (e.g. a `mod`), but is missing from the rustdoc JSON file. This
    /// occurs for example in the case of re-exports of external modules (see
    /// <https://github.com/Enselic/cargo-public-api/issues/103>). The entries
    /// in this Vec are what IDs that could not be found.
    ///
    /// The exact format of IDs are to be considered an implementation detail
    /// and must not be be relied on.
    pub fn missing_item_ids(&self) -> impl Iterator<Item = &String> {
        self.missing_item_ids.iter()
    }
}

impl std::fmt::Display for PublicApi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for item in self.items() {
            writeln!(f, "{item}")?;
        }
        Ok(())
    }
}
