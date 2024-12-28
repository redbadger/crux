mod basic {
    use super::super::Command;
    use crate::{render::RenderOperation, Request};

    enum Effect {
        Render(Request<RenderOperation>),
    }

    impl From<Request<RenderOperation>> for Effect {
        fn from(request: Request<RenderOperation>) -> Self {
            Self::Render(request)
        }
    }

    #[test]
    fn done_can_be_created() {
        let cmd: Command<Effect> = Command::done();

        assert!(cmd.is_done())
    }

    #[test]
    fn notify_can_be_created_with_an_operation() {
        let cmd: Command<Effect> = Command::notify_shell(RenderOperation);

        assert!(!cmd.is_done())
    }

    #[test]
    fn notify_effect_can_be_inspected() {
        let mut cmd = Command::notify_shell(RenderOperation);

        let mut effects = cmd.effects();

        assert!(!effects.is_empty());

        let Effect::Render(request) = effects.remove(0);

        assert_eq!(request.operation, RenderOperation)
    }
}
