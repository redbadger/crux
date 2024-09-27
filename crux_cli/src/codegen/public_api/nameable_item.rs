use rustdoc_types::{Item, ItemEnum};

use super::render::RenderingContext;

/// Wraps an [`Item`] and allows us to override its name.
#[derive(Clone, Debug)]
pub struct NameableItem<'c> {
    /// The item we are effectively wrapping.
    pub item: &'c Item,

    /// If `Some`, this overrides [Item::name], which happens in the case of
    /// renamed imports (`pub use other::Item as Foo;`).
    ///
    /// We can't calculate this on-demand, because we can't know the final name
    /// until we have checked if we need to break import recursion.
    pub overridden_name: Option<String>,

    /// See [`crate::item_processor::sorting_prefix()`] docs for an explanation why we have this.
    pub sorting_prefix: u8,
}

impl<'c> NameableItem<'c> {
    /// The regular name of the item. Shown to users.
    pub fn name(&self) -> Option<&str> {
        self.overridden_name
            .as_deref()
            .or(self.item.name.as_deref())
    }

    /// The name that, when sorted on, will group items nicely. Is never shown
    /// to a user.
    pub fn sortable_name(&self, context: &RenderingContext) -> String {
        // Note that in order for the prefix to sort properly lexicographically,
        // we need to pad it with leading zeroes.
        let mut sortable_name = format!("{:0>3}-", self.sorting_prefix);

        if let Some(name) = self.name() {
            sortable_name.push_str(name);
        } else if let ItemEnum::Impl(impl_) = &self.item.inner {
            // In order for items of impls to be grouped together with its impl,
            // add the "name" of the impl to the sorting prefix. Ignore `!` when
            // sorting however, because that just messes the expected order up.
            sortable_name.push_str(&super::tokens::tokens_to_string(&context.render_impl(
                impl_,
                &[],
                true, /* disregard_negativity */
            )));

            // If this is an inherent impl, additionally add the concatenated
            // names of all associated items to the "name" of the impl. This makes
            // multiple inherent impls group together, even if they have the
            // same "name".
            //
            // For example, consider this code:
            //
            //   pub struct MultipleInherentImpls;
            //
            //   impl MultipleInherentImpls {
            //       pub fn impl_one() {}
            //   }
            //
            //   impl MultipleInherentImpls {
            //       pub fn impl_two() {}
            //   }
            //
            // In this case, we want to group the two impls together. So
            // the name of the first impl should be
            //
            //   impl MultipleInherentImpls-impl_one
            //
            // and the second one
            //
            //   impl MultipleInherentImpls-impl_two
            //
            if impl_.trait_.is_none() {
                let mut assoc_item_names: Vec<&str> = impl_
                    .items
                    .iter()
                    .filter_map(|id| context.crate_.index.get(id))
                    .filter_map(|item| item.name.as_ref())
                    .map(String::as_str)
                    .collect();
                assoc_item_names.sort_unstable();

                sortable_name.push('-');
                sortable_name.push_str(&assoc_item_names.join("-"));
            }
        }

        sortable_name
    }
}
