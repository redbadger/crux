//! Tests that simulate the WASM single-threaded pattern.
//!
//! On WASM, `thread::current().id()` always returns the same value and
//! `spawn_local` defers work on the same thread. These tests verify that the
//! `EffectResolver` guard correctly allows deferred same-thread resolution
//! (the `spawn_local` pattern) while still catching synchronous calls.

use std::sync::{Arc, Mutex};

use crux_core::{
    Command, Core,
    capability::Operation,
    macros::effect,
    middleware::{EffectMiddleware, EffectResolver, Layer as _},
    render::RenderOperation,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct PingOperation;

#[derive(Debug, PartialEq, Eq, Deserialize)]
struct PingOutput;

impl Operation for PingOperation {
    type Output = PingOutput;
}

#[effect]
#[derive(Debug)]
enum PingEffect {
    Ping(PingOperation),
    Render(RenderOperation),
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
enum PingEvent {
    Go,
    #[serde(skip)]
    Pong(PingOutput),
}

#[derive(Default)]
struct PingApp;

impl crux_core::App for PingApp {
    type Event = PingEvent;
    type Model = ();
    type ViewModel = ();
    type Effect = PingEffect;

    fn update(
        &self,
        event: Self::Event,
        _model: &mut Self::Model,
    ) -> Command<Self::Effect, Self::Event> {
        match event {
            PingEvent::Go => Command::request_from_shell(PingOperation).then_send(PingEvent::Pong),
            PingEvent::Pong(_) => crux_core::render::render(),
        }
    }

    fn view(&self, _model: &Self::Model) -> Self::ViewModel {}
}

/// Middleware that stashes the resolver for deferred (same-thread) resolution.
/// This simulates `spawn_local` on WASM — `try_process_effect` returns, then
/// the resolver is called later on the same thread.
struct DeferredPingMiddleware {
    stash: Arc<Mutex<Option<EffectResolver<PingOutput>>>>,
}

impl DeferredPingMiddleware {
    fn new() -> (Self, Arc<Mutex<Option<EffectResolver<PingOutput>>>>) {
        let stash = Arc::new(Mutex::new(None));
        (
            Self {
                stash: stash.clone(),
            },
            stash,
        )
    }
}

impl EffectMiddleware for DeferredPingMiddleware {
    type Op = PingOperation;

    fn try_process_effect(&self, _operation: PingOperation, resolver: EffectResolver<PingOutput>) {
        // Stash the resolver — don't call resolve() here.
        // This simulates what spawn_local would do: defer the work.
        *self.stash.lock().unwrap() = Some(resolver);
    }
}

/// Verify that deferred same-thread resolution works (the WASM pattern).
/// `try_process_effect` stashes the resolver, returns, then the same thread
/// retrieves and calls `resolve()` — this must NOT panic.
///
/// On WASM, `thread::current().id()` always returns the same value, so this
/// test exercises the exact scenario: same thread ID, but `active` is false
/// because `try_process_effect` has already returned.
#[test]
fn deferred_same_thread_resolve_does_not_panic() {
    let (middleware, stash) = DeferredPingMiddleware::new();

    let (effects_tx, _effects_rx) = crossbeam_channel::unbounded();
    let effect_callback = move |effects: Vec<PingEffect>| effects_tx.send(effects).unwrap();

    let core = Core::<PingApp>::new().handle_effects_using(middleware);

    // This calls try_process_effect, which stashes the resolver and returns.
    // The framework then sets active = false.
    let _effects = core.update(PingEvent::Go, effect_callback);

    // Now, on the SAME thread, retrieve and call resolve().
    // On WASM this is exactly what happens after spawn_local's deferred task runs.
    let mut resolver = stash.lock().unwrap().take().expect("resolver was stashed");
    resolver.resolve(PingOutput); // Must NOT panic
}

/// Verify that synchronous same-thread resolution still panics.
#[test]
#[should_panic(expected = "must not call resolve() synchronously")]
fn synchronous_same_thread_resolve_panics() {
    struct SyncPingMiddleware;

    impl EffectMiddleware for SyncPingMiddleware {
        type Op = PingOperation;

        fn try_process_effect(
            &self,
            _operation: PingOperation,
            mut resolver: EffectResolver<PingOutput>,
        ) {
            // Synchronous call — should panic
            resolver.resolve(PingOutput);
        }
    }

    let (effects_tx, _effects_rx) = crossbeam_channel::unbounded();
    let effect_callback = move |effects: Vec<PingEffect>| effects_tx.send(effects).unwrap();

    let core = Core::<PingApp>::new().handle_effects_using(SyncPingMiddleware);
    let _ = core.update(PingEvent::Go, effect_callback);
}
