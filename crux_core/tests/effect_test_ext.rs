//! Negative-case tests for the `EffectTestExt` extension trait that
//! `#[effect]` generates on `Command<Effect, Event>`.

use serde::{Deserialize, Serialize};

mod app {
    use crux_core::{
        App, Command,
        render::{RenderOperation, render},
    };
    use crux_macros::effect;
    use serde::{Deserialize, Serialize};

    use super::{PingOperation, ping};

    #[derive(Debug, Serialize, Deserialize)]
    pub enum Event {
        Render,
        Ping,
        Both,
        Echo(()),
    }

    #[effect]
    #[derive(Debug)]
    pub enum Effect {
        Render(RenderOperation),
        Ping(PingOperation),
    }

    #[derive(Default)]
    pub struct PanicApp;

    impl App for PanicApp {
        type Event = Event;
        type Model = ();
        type ViewModel = ();
        type Effect = Effect;

        fn update(&self, event: Self::Event, _model: &mut Self::Model) -> Command<Effect, Event> {
            match event {
                Event::Render => render(),
                Event::Ping => ping().then_send(Event::Echo),
                Event::Both => render().and(ping().then_send(Event::Echo)),
                Event::Echo(()) => Command::done(),
            }
        }

        fn view(&self, _model: &Self::Model) {}
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct PingOperation;

impl crux_core::capability::Operation for PingOperation {
    type Output = ();
}

fn ping() -> crux_core::command::RequestBuilder<
    app::Effect,
    app::Event,
    impl std::future::Future<Output = ()>,
> {
    crux_core::Command::request_from_shell(PingOperation)
}

use app::EffectTestExt as _;
use crux_core::App as _;

#[test]
#[should_panic(expected = "expected Render effect but no more effects remain")]
fn expect_render_panics_on_empty_command() {
    let mut model = ();
    app::PanicApp
        .update(app::Event::Render, &mut model)
        .expect_render()
        .expect_render();
}

#[test]
#[should_panic(expected = "not a Render effect")]
fn expect_render_panics_on_wrong_variant() {
    let mut model = ();
    app::PanicApp
        .update(app::Event::Ping, &mut model)
        .expect_render();
}

#[test]
#[should_panic(expected = "expected an event but got none")]
fn then_event_panics_on_no_events() {
    let mut model = ();
    app::PanicApp
        .update(app::Event::Render, &mut model)
        .then_event(|_| {});
}

#[test]
fn resolve_ping_drives_resulting_event() {
    let mut model = ();
    let event = app::PanicApp
        .update(app::Event::Ping, &mut model)
        .resolve_ping(|_op| ())
        .expect_event();

    assert!(matches!(event, app::Event::Echo(())));
}

#[test]
fn expect_only_render_succeeds_for_single_render() {
    let mut model = ();
    app::PanicApp
        .update(app::Event::Render, &mut model)
        .expect_only_render();
}

#[test]
#[should_panic(expected = "expected command to be done")]
fn expect_only_render_panics_with_extra_effects() {
    let mut model = ();
    // `Both` emits Render plus a Ping; expect_only_render should reject the leftover.
    app::PanicApp
        .update(app::Event::Both, &mut model)
        .expect_only_render();
}
