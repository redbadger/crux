use crux_core::{Effect as _, OperationKind, Request, RequestHandle, capability::Operation};

struct Op;

impl Operation for Op {
    type Output = ();
}

#[crux_core::macros::effect]
enum Effect {
    AppChosenName(Op),
}

#[test]
fn generated_effect_exposes_handle_kind_without_static_operation_kind() {
    for (handle, expected) in [
        (RequestHandle::Never, OperationKind::Notify),
        (
            RequestHandle::Once(Box::new(|()| {})),
            OperationKind::Request,
        ),
        (
            RequestHandle::Many(Box::new(|()| Ok(()))),
            OperationKind::Stream,
        ),
    ] {
        let effect = Effect::AppChosenName(Request {
            operation: Op,
            handle,
        });
        assert_eq!(effect.kind(), expected);
        assert_eq!(effect.name(), "AppChosenName");
    }
}
