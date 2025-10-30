use std::marker::PhantomData;

use crate::{MaybeSend, MaybeSync, Resolvable, ResolveError};

use super::Layer;

/// Middleware for converting the effect type to another type.
///
/// Typically, this is used to eliminate some of the effect variants which are processed
/// by the layers below, so that code using this stack is not forced to have extraneous
/// match arms which are never called.
pub struct MapEffectLayer<Next, Effect>
where
    Next: Layer,
    Effect: 'static,
{
    next: Next,
    effect: PhantomData<fn() -> Effect>, // to avoid losing Sync
}

impl<Next, Effect> MapEffectLayer<Next, Effect>
where
    Next: Layer,
{
    pub fn new(next: Next) -> Self {
        Self {
            next,
            effect: PhantomData,
        }
    }

    fn map_effects(effects: Vec<Next::Effect>) -> Vec<Effect>
    where
        Effect: From<Next::Effect> + MaybeSend + 'static,
    {
        effects.into_iter().map(From::from).collect()
    }
}

impl<Next, Effect> Layer for MapEffectLayer<Next, Effect>
where
    Next: Layer,
    Effect: From<Next::Effect> + MaybeSend + 'static,
{
    type Event = Next::Event;
    type Effect = Effect;

    type ViewModel = Next::ViewModel;

    fn update<F>(&self, event: Self::Event, effect_callback: F) -> Vec<Self::Effect>
    where
        F: Fn(Vec<Self::Effect>) + MaybeSync + MaybeSend + 'static,
    {
        Self::map_effects(self.next.update(event, move |effects: Vec<Next::Effect>| {
            effect_callback(Self::map_effects(effects));
        }))
    }

    fn resolve<Output, F>(
        &self,
        request: &mut impl Resolvable<Output>,
        output: Output,
        effect_callback: F,
    ) -> Result<Vec<Self::Effect>, ResolveError>
    where
        F: Fn(Vec<Self::Effect>) + MaybeSync + MaybeSend + 'static,
    {
        Ok(Self::map_effects(self.next.resolve(
            request,
            output,
            move |effects| effect_callback(Self::map_effects(effects)),
        )?))
    }

    fn view(&self) -> Self::ViewModel {
        self.next.view()
    }

    fn process_tasks<F>(&self, effect_callback: F) -> Vec<Self::Effect>
    where
        F: Fn(Vec<Self::Effect>) + MaybeSync + MaybeSend + 'static,
    {
        Self::map_effects(
            self.next
                .process_tasks(move |effects| effect_callback(Self::map_effects(effects))),
        )
    }
}
