use std::cmp::Ordering;
use std::fmt::Display;
use std::hash::Hash;

use super::intermediate_public_item::IntermediatePublicItem;
use super::render::RenderingContext;
use super::tokens::tokens_to_string;
use super::tokens::Token;

/// Each public item (except `impl`s) have a path that is displayed like
/// `first::second::third`. Internally we represent that with a `vec!["first",
/// "second", "third"]`. This is a type alias for that internal representation
/// to make the code easier to read.
pub(crate) type PublicItemPath = Vec<String>;

/// Represent a public item of an analyzed crate, i.e. an item that forms part
/// of the public API of a crate. Implements [`Display`] so it can be printed. It
/// also implements [`Ord`], but how items are ordered are not stable yet, and
/// will change in later versions.
#[derive(Clone)]
pub struct PublicItem {
    /// Read [`crate::item_processor::sorting_prefix()`] docs for more info
    pub(crate) sortable_path: PublicItemPath,

    /// The rendered item as a stream of [`Token`]s
    pub(crate) tokens: Vec<Token>,
}

impl PublicItem {
    pub(crate) fn from_intermediate_public_item(
        context: &RenderingContext,
        public_item: &IntermediatePublicItem<'_>,
    ) -> PublicItem {
        PublicItem {
            sortable_path: public_item.sortable_path(context),
            tokens: public_item.render_token_stream(context),
        }
    }

    /// The rendered item as a stream of [`Token`]s
    pub fn tokens(&self) -> impl Iterator<Item = &Token> {
        self.tokens.iter()
    }

    /// Special version of [`cmp`](Ord::cmp) that is used to sort public items in a way that
    /// makes them grouped logically. For example, struct fields will be put
    /// right after the struct they are part of.
    #[must_use]
    pub fn grouping_cmp(&self, other: &Self) -> std::cmp::Ordering {
        // This will make e.g. struct and struct fields be grouped together.
        if let Some(ordering) = different_or_none(&self.sortable_path, &other.sortable_path) {
            return ordering;
        }

        // Fall back to lexical sorting if the above is not sufficient
        self.to_string().cmp(&other.to_string())
    }
}

impl PartialEq for PublicItem {
    fn eq(&self, other: &Self) -> bool {
        self.tokens == other.tokens
    }
}

impl Eq for PublicItem {}

impl Hash for PublicItem {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.tokens.hash(state);
    }
}

/// We want pretty-printing (`"{:#?}"`) of [`crate::diff::PublicApiDiff`] to print
/// each public item as `Display`, so implement `Debug` with `Display`.
impl std::fmt::Debug for PublicItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

/// One of the basic uses cases is printing a sorted `Vec` of `PublicItem`s. So
/// we implement `Display` for it.
impl Display for PublicItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", tokens_to_string(&self.tokens))
    }
}

/// Returns `None` if two items are equal. Otherwise their ordering is returned.
fn different_or_none<T: Ord>(a: &T, b: &T) -> Option<Ordering> {
    match a.cmp(b) {
        Ordering::Equal => None,
        c => Some(c),
    }
}
