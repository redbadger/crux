/// A stable label for the event which starts an observed command.
///
/// Use `#[derive(crux_core::macros::EventName)]` for enum variant names or a
/// struct's type name. Payloads are not formatted or recorded. This trait is
/// required only by `Core::with_runtime_observer`, not by `App`.
pub trait EventName {
    fn event_name(&self) -> &'static str;
}
