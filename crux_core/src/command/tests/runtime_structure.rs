use crate::{Command, Core, Request, capability::Operation};

#[derive(Default)]
struct TestApp;

#[derive(Debug, PartialEq, Eq)]
enum Event {
    Start,
    FollowUp,
    FollowUpDone,
    AfterDone,
}

#[derive(Debug, PartialEq, Eq)]
enum Op {
    FollowUp,
    After,
}

impl Operation for Op {
    type Output = ();
}

enum Effect {
    Op(Request<Op>),
}

impl crate::Effect for Effect {}

impl From<Request<Op>> for Effect {
    fn from(value: Request<Op>) -> Self {
        Self::Op(value)
    }
}

impl crate::App for TestApp {
    type Event = Event;
    type Model = ();
    type ViewModel = ();
    type Effect = Effect;

    fn update(
        &self,
        event: Self::Event,
        _model: &mut Self::Model,
    ) -> Command<Self::Effect, Self::Event> {
        match event {
            Event::Start => Command::event(Event::FollowUp)
                .then(Command::request_from_shell(Op::After).then_send(|()| Event::AfterDone)),
            Event::FollowUp => {
                Command::request_from_shell(Op::FollowUp).then_send(|()| Event::FollowUpDone)
            }
            Event::FollowUpDone | Event::AfterDone => Command::done(),
        }
    }

    fn view(&self, _model: &Self::Model) -> Self::ViewModel {}
}

#[test]
fn follow_up_commands_are_hosted_in_the_command_that_emitted_the_event() {
    let core = Core::new_with(TestApp, ());

    let mut effects = core.process_event(Event::Start);
    assert_eq!(effects.len(), 1);

    let Effect::Op(mut request) = effects.remove(0);
    assert_eq!(request.operation, Op::FollowUp);

    let mut effects = core.resolve(&mut request, ()).unwrap();
    assert_eq!(effects.len(), 1);

    let Effect::Op(request) = effects.remove(0);
    assert_eq!(request.operation, Op::After);
}
