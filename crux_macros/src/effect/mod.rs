pub mod macro_impl;
#[cfg(feature = "facet_typegen")]
#[cfg(test)]
pub mod test_facet;
#[cfg(not(feature = "facet_typegen"))]
#[cfg(test)]
pub mod test_serde;
