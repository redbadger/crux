use crux_core::capability::Operation;
use crux_core::macros::effect;
use crux_core::middleware::{EffectMiddleware, Layer};
use crux_core::render::RenderOperation;
use crux_core::{Command, Core, Request, RequestHandle};
use facet::Facet;
use serde::{Deserialize, Serialize};

#[derive(Default)]
pub struct App;

impl crux_core::App for App {
    type Effect = Effect;
    type Event = Event;
    type Model = Model;
    type ViewModel = ViewModel;

    fn update(&self, _: Event, _: &mut Model) -> Command<Effect, Event> {
        Command::new(async |ctx| {
            for _ in 0..100_000 {
                ctx.request_from_shell(PingOperation).await;
            }
        })
    }

    fn view(&self, _: &Model) -> Self::ViewModel {
        ViewModel
    }
}

#[derive(Facet, Serialize, Deserialize)]
pub struct PingOperation;

impl Operation for PingOperation {
    type Output = PingOutput;
}

#[derive(Facet, Serialize, Deserialize)]
pub struct PingOutput;

pub struct PingMiddleware;

impl<Effect> EffectMiddleware<Effect> for PingMiddleware
where
    Effect: TryInto<Request<PingOperation>, Error = Effect>,
{
    type Op = PingOperation;

    fn try_process_effect_with(
        &self,
        effect: Effect,
        resolve: impl FnOnce(&mut RequestHandle<PingOutput>, PingOutput) + Send + 'static,
    ) -> Result<(), Effect> {
        let mut request = effect.try_into()?;

        resolve(&mut request.handle, PingOutput);

        Ok(())
    }
}

#[effect(facet_typegen)]
pub enum Effect {
    Ping(PingOperation),
    Render(RenderOperation),
}

#[derive(Facet, Serialize, Deserialize)]
#[repr(C)]
pub enum Event {
    Init,
}

#[derive(Default)]
pub struct Model;

#[derive(Facet, Serialize, Deserialize)]
pub struct ViewModel;

#[test]
fn test() {
    let effects = Core::<App>::new()
        .handle_effects_using(PingMiddleware)
        .update(Event::Init, |_| todo!());

    assert!(
        effects.is_empty(),
        "All effects must have been dealt with by the middleware"
    );
}
