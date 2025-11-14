#[cfg(feature = "facet_typegen")]
use crate::type_generation::facet::TypeGenError;

/// Operation trait links together input and output of a side-effect.
///
/// You implement `Operation` on the payload sent by the capability to the shell using [`CapabilityContext::request_from_shell`].
///
/// For example (from `crux_http`):
///
/// ```rust,ignore
/// impl Operation for HttpRequest {
///     type Output = HttpResponse;
/// }
/// ```
pub trait Operation: Send + 'static {
    /// `Output` assigns the type this request results in.
    type Output: Send + Unpin + 'static;

    #[cfg(feature = "typegen")]
    #[allow(clippy::missing_errors_doc)]
    fn register_types(
        generator: &mut crate::type_generation::serde::TypeGen,
    ) -> crate::type_generation::serde::Result
    where
        Self: serde::Serialize + for<'de> serde::de::Deserialize<'de>,
        Self::Output: for<'de> serde::de::Deserialize<'de>,
    {
        generator.register_type::<Self>()?;
        generator.register_type::<Self::Output>()?;
        Ok(())
    }

    #[cfg(feature = "facet_typegen")]
    #[allow(clippy::missing_errors_doc)]
    fn register_types_facet<'a>(
        generator: &mut crate::type_generation::facet::TypeRegistry,
    ) -> Result<&mut crate::type_generation::facet::TypeRegistry, TypeGenError>
    where
        Self: facet::Facet<'a> + serde::Serialize + for<'de> serde::de::Deserialize<'de>,
        <Self as Operation>::Output: facet::Facet<'a> + for<'de> serde::de::Deserialize<'de>,
    {
        generator
            .register_type::<Self>()
            .map_err(|e| TypeGenError::Generation(e.to_string()))?
            .register_type::<Self::Output>()
            .map_err(|e| TypeGenError::Generation(e.to_string()))?;

        Ok(generator)
    }
}
