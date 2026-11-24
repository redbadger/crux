//! What type generation knows about the app itself.
//!
//! [`TypeRegistry::register_app`](super::TypeRegistry::register_app) already
//! registers `App::Event` and `App::ViewModel`; [`AppMeta`] is what it
//! remembers about *which* of the registered types they are, so that the
//! generated `Core` can name them in a signature.

use facet_generate::reflection::format::QualifiedTypeName;

/// The two types the generated `Core` names: the event it serializes and the
/// view model it deserializes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppMeta {
    /// The registry name of `App::Event`, after any `#[facet(rename)]`.
    pub event: QualifiedTypeName,
    /// The registry name of `App::ViewModel`, after any `#[facet(rename)]`.
    pub view_model: QualifiedTypeName,
}
