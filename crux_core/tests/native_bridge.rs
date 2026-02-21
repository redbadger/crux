#![cfg(feature = "native_bridge")]

//! Integration tests for the NativeBridge — the typed native FFI bridge
//! that replaces byte serialization with callback-based effect delivery.

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crossbeam_channel::{self, Receiver, Sender};
use crux_core::{
    Core,
    bridge::{EffectId, NativeBridgeError},
    middleware::Layer as _,
};

// ---------------------------------------------------------------------------
// Test App
// ---------------------------------------------------------------------------
mod counter_app {
    use crux_core::bridge::{NativeBridgeError, ResolveNative};
    use crux_core::{
        App, Command,
        capability::Operation,
        macros::effect,
        render::{RenderOperation, render},
    };
    use serde::{Deserialize, Serialize};

    /// A trivial operation whose output is an `i32`.
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct CounterOp;

    impl Operation for CounterOp {
        type Output = i32;
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub enum Event {
        Increment,
        FetchValue,
        Subscribe,
        #[serde(skip)]
        GotValue(i32),
        #[serde(skip)]
        GotStreamValue(i32),
    }

    #[derive(Default)]
    pub struct Model {
        pub count: i32,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ViewModel {
        pub count: i32,
    }

    // The #[effect] macro generates:
    //   - Effect enum with Request<Op> wrapping
    //   - crux_core::Effect impl
    //   - From<Request<Op>> for Effect
    //   - TryFrom<Effect> for Request<Op>
    //   - is_*/into_*/expect_* filter methods
    //
    // We then manually define EffectFfi, EffectOutput, and EffectNative
    // to avoid requiring UniFFI scaffolding in the test binary.
    #[effect]
    #[derive(Debug)]
    pub enum Effect {
        Render(RenderOperation),
        Counter(CounterOp),
    }

    // --- Manual native bridge types ---

    #[derive(Debug)]
    pub enum EffectFfi {
        Render(RenderOperation),
        Counter(CounterOp),
    }

    #[derive(Debug)]
    pub enum EffectOutput {
        Render, // Unit variant for notification effects
        Counter(i32),
    }

    impl crux_core::EffectNative for Effect {
        type Ffi = EffectFfi;
        type Output = EffectOutput;

        fn into_native(self) -> (Self::Ffi, ResolveNative<Self::Output>) {
            match self {
                Effect::Render(req) => req.into_native(EffectFfi::Render, |o| match o {
                    EffectOutput::Render => Ok(()),
                    _ => Err(NativeBridgeError::OutputMismatch {
                        expected: "Render".to_string(),
                    }),
                }),
                Effect::Counter(req) => req.into_native(EffectFfi::Counter, |o| match o {
                    EffectOutput::Counter(v) => Ok(v),
                    _ => Err(NativeBridgeError::OutputMismatch {
                        expected: "Counter".to_string(),
                    }),
                }),
            }
        }
    }

    // --- App ---

    #[derive(Default)]
    pub struct Counter;

    impl App for Counter {
        type Event = Event;
        type Model = Model;
        type ViewModel = ViewModel;
        type Effect = Effect;

        fn update(&self, event: Event, model: &mut Model) -> Command<Effect, Event> {
            match event {
                Event::Increment => {
                    model.count += 1;
                    render()
                }
                Event::FetchValue => {
                    Command::request_from_shell(CounterOp).then_send(Event::GotValue)
                }
                Event::Subscribe => {
                    Command::stream_from_shell(CounterOp).then_send(Event::GotStreamValue)
                }
                Event::GotValue(value) => {
                    model.count = value;
                    render()
                }
                Event::GotStreamValue(value) => {
                    model.count = value;
                    render()
                }
            }
        }

        fn view(&self, model: &Model) -> ViewModel {
            ViewModel { count: model.count }
        }
    }
}

use counter_app::{Counter, EffectFfi, EffectOutput, Event, ViewModel};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

type EffectEntry = (EffectId, EffectFfi);

fn create_bridge() -> (
    crux_core::bridge::NativeBridge<Core<Counter>>,
    Receiver<EffectEntry>,
) {
    let (tx, rx) = crossbeam_channel::unbounded();
    let bridge = Core::<Counter>::new().native_bridge(move |id, effect| {
        tx.send((id, effect)).unwrap();
    });
    (bridge, rx)
}

/// Drain all currently available effects from the channel.
fn drain(rx: &Receiver<EffectEntry>) -> Vec<EffectEntry> {
    rx.try_iter().collect()
}

// ---------------------------------------------------------------------------
// Core tests
// ---------------------------------------------------------------------------

#[test]
fn basic_update_render_view() {
    let (bridge, rx) = create_bridge();

    bridge.update(Event::Increment).unwrap();

    let effects = drain(&rx);
    assert_eq!(effects.len(), 1);
    assert!(matches!(effects[0].1, EffectFfi::Render(_)));

    assert_eq!(bridge.view(), ViewModel { count: 1 });
}

#[test]
fn render_is_fire_and_forget() {
    let (bridge, rx) = create_bridge();

    bridge.update(Event::Increment).unwrap();

    let effects = drain(&rx);
    let render_id = effects[0].0;

    // Resolve fire-and-forget — should error (consistent with serde bridge)
    let result = bridge.resolve(render_id, EffectOutput::Render);
    assert!(
        matches!(result, Err(NativeBridgeError::ResolveNever)),
        "expected ResolveNever for fire-and-forget effect, got {result:?}"
    );

    // Resolving again should fail — entry was cleaned up
    let result = bridge.resolve(render_id, EffectOutput::Render);
    assert!(
        matches!(result, Err(NativeBridgeError::EffectNotFound { .. })),
        "expected EffectNotFound, got {result:?}"
    );
}

#[test]
fn request_resolve_produces_follow_up_effects() {
    let (bridge, rx) = create_bridge();

    // FetchValue → Counter request (Once resolver)
    bridge.update(Event::FetchValue).unwrap();

    let effects = drain(&rx);
    assert_eq!(effects.len(), 1);
    assert!(matches!(effects[0].1, EffectFfi::Counter(_)));
    let counter_id = effects[0].0;

    // Resolve with typed output
    bridge
        .resolve(counter_id, EffectOutput::Counter(42))
        .unwrap();

    // Follow-up: GotValue(42) triggers render
    let effects = drain(&rx);
    assert_eq!(effects.len(), 1);
    assert!(matches!(effects[0].1, EffectFfi::Render(_)));

    assert_eq!(bridge.view(), ViewModel { count: 42 });
}

#[test]
fn output_mismatch_error() {
    let (bridge, rx) = create_bridge();

    // FetchValue → Counter request (Once resolver with extractor)
    bridge.update(Event::FetchValue).unwrap();

    let effects = drain(&rx);
    let counter_id = effects[0].0;

    // Try to resolve Counter request with Render output → mismatch
    let result = bridge.resolve(counter_id, EffectOutput::Render);
    assert!(
        matches!(result, Err(NativeBridgeError::OutputMismatch { .. })),
        "expected OutputMismatch, got {result:?}"
    );
}

#[test]
fn unknown_effect_id_error() {
    let (bridge, _rx) = create_bridge();

    let result = bridge.resolve(EffectId(999), EffectOutput::Render);
    assert!(
        matches!(result, Err(NativeBridgeError::EffectNotFound { .. })),
        "expected EffectNotFound, got {result:?}"
    );
}

#[test]
fn once_resolver_consumed_after_resolve() {
    let (bridge, rx) = create_bridge();

    bridge.update(Event::FetchValue).unwrap();
    let counter_id = drain(&rx)[0].0;

    // First resolve — succeeds, produces follow-up Render
    bridge
        .resolve(counter_id, EffectOutput::Counter(42))
        .unwrap();
    assert_eq!(bridge.view(), ViewModel { count: 42 });

    // The follow-up Render effect may reuse the same Slab slot.
    // Drain and resolve it so the slot is freed.
    let follow_up = drain(&rx);
    assert_eq!(follow_up.len(), 1);
    assert!(matches!(follow_up[0].1, EffectFfi::Render(_)));
    let render_id = follow_up[0].0;
    // Render is fire-and-forget, so resolve returns ResolveNever error
    let result = bridge.resolve(render_id, EffectOutput::Render);
    assert!(
        matches!(result, Err(NativeBridgeError::ResolveNever)),
        "expected ResolveNever for fire-and-forget, got {result:?}"
    );

    // Now both entries are gone — resolving either id should fail
    let result = bridge.resolve(counter_id, EffectOutput::Counter(99));
    assert!(
        matches!(result, Err(NativeBridgeError::EffectNotFound { .. })),
        "expected EffectNotFound after resolver consumed, got {result:?}"
    );

    // View unchanged
    assert_eq!(bridge.view(), ViewModel { count: 42 });
}

#[test]
fn once_resolver_consumed_after_mismatch() {
    let (bridge, rx) = create_bridge();

    bridge.update(Event::FetchValue).unwrap();
    let counter_id = drain(&rx)[0].0;

    // Mismatch — consumes the resolver
    let result = bridge.resolve(counter_id, EffectOutput::Render);
    assert!(matches!(
        result,
        Err(NativeBridgeError::OutputMismatch { .. })
    ));

    // Retry — entry gone
    let result = bridge.resolve(counter_id, EffectOutput::Counter(42));
    assert!(
        matches!(result, Err(NativeBridgeError::EffectNotFound { .. })),
        "expected EffectNotFound after mismatch, got {result:?}"
    );
}

#[test]
fn stream_resolver_survives_multiple_resolves() {
    let (bridge, rx) = create_bridge();

    // Subscribe → Counter stream (Many resolver)
    bridge.update(Event::Subscribe).unwrap();

    let effects = drain(&rx);
    assert_eq!(effects.len(), 1);
    assert!(matches!(effects[0].1, EffectFfi::Counter(_)));
    let stream_id = effects[0].0;

    // First resolve
    bridge
        .resolve(stream_id, EffectOutput::Counter(10))
        .unwrap();
    let effects = drain(&rx);
    assert_eq!(effects.len(), 1, "expected Render after first stream value");
    assert!(matches!(effects[0].1, EffectFfi::Render(_)));
    assert_eq!(bridge.view(), ViewModel { count: 10 });

    // Second resolve — same id, still valid
    bridge
        .resolve(stream_id, EffectOutput::Counter(20))
        .unwrap();
    let effects = drain(&rx);
    assert_eq!(
        effects.len(),
        1,
        "expected Render after second stream value"
    );
    assert!(matches!(effects[0].1, EffectFfi::Render(_)));
    assert_eq!(bridge.view(), ViewModel { count: 20 });

    // Third resolve
    bridge
        .resolve(stream_id, EffectOutput::Counter(30))
        .unwrap();
    assert_eq!(bridge.view(), ViewModel { count: 30 });
}

#[test]
fn view_returns_initial_state() {
    let (bridge, _rx) = create_bridge();
    assert_eq!(bridge.view(), ViewModel { count: 0 });
}

#[test]
fn multiple_sequential_updates() {
    let (bridge, rx) = create_bridge();

    bridge.update(Event::Increment).unwrap();
    bridge.update(Event::Increment).unwrap();
    bridge.update(Event::Increment).unwrap();

    let effects = drain(&rx);
    assert_eq!(effects.len(), 3, "each Increment should produce a Render");

    assert_eq!(bridge.view(), ViewModel { count: 3 });
}

// ---------------------------------------------------------------------------
// Middleware tests
// ---------------------------------------------------------------------------

mod counter_middleware {
    use crux_core::middleware::{EffectMiddleware, EffectResolver};

    use crate::counter_app::CounterOp;

    /// Middleware that handles CounterOp on a background thread,
    /// always resolving with a fixed value.
    pub struct FixedCounter(pub i32);

    impl EffectMiddleware for FixedCounter {
        type Op = CounterOp;

        fn try_process_effect(&self, _operation: CounterOp, mut resolver: EffectResolver<i32>) {
            let value = self.0;
            std::thread::spawn(move || {
                resolver.resolve(value);
            });
        }
    }

    /// Middleware that waits for a signal before resolving.
    pub struct DelayedCounter {
        pub trigger: crossbeam_channel::Receiver<i32>,
    }

    impl EffectMiddleware for DelayedCounter {
        type Op = CounterOp;

        fn try_process_effect(&self, _operation: CounterOp, mut resolver: EffectResolver<i32>) {
            let trigger = self.trigger.clone();
            std::thread::spawn(move || {
                if let Ok(value) = trigger.recv() {
                    resolver.resolve(value);
                }
            });
        }
    }
}

#[test]
fn middleware_async_callback() {
    let (tx, rx): (Sender<EffectEntry>, Receiver<EffectEntry>) = crossbeam_channel::unbounded();

    let bridge = Core::<Counter>::new()
        .handle_effects_using(counter_middleware::FixedCounter(100))
        .native_bridge(move |id, effect| {
            tx.send((id, effect)).unwrap();
        });

    // FetchValue → middleware intercepts Counter, resolves on bg thread
    bridge.update(Event::FetchValue).unwrap();

    // No immediate effects (middleware consumed the Counter request)
    // Wait for the bg thread to deliver the follow-up Render effect
    let (_, effect) = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("timed out waiting for middleware async callback");

    assert!(
        matches!(effect, EffectFfi::Render(_)),
        "expected Render from GotValue, got {effect:?}"
    );

    // Model should be updated by the middleware's resolved value
    assert_eq!(bridge.view(), ViewModel { count: 100 });
}

#[test]
fn middleware_does_not_intercept_other_effects() {
    let (tx, rx): (Sender<EffectEntry>, Receiver<EffectEntry>) = crossbeam_channel::unbounded();

    let bridge = Core::<Counter>::new()
        .handle_effects_using(counter_middleware::FixedCounter(100))
        .native_bridge(move |id, effect| {
            tx.send((id, effect)).unwrap();
        });

    // Increment → Render, not intercepted by CounterMiddleware
    bridge.update(Event::Increment).unwrap();

    let effects = drain(&rx);
    assert_eq!(effects.len(), 1);
    assert!(matches!(effects[0].1, EffectFfi::Render(_)));
    assert_eq!(bridge.view(), ViewModel { count: 1 });
}

#[test]
fn concurrent_update_while_middleware_processing() {
    let (trigger_tx, trigger_rx) = crossbeam_channel::unbounded();
    let (tx, rx): (Sender<EffectEntry>, Receiver<EffectEntry>) = crossbeam_channel::unbounded();

    let bridge = Arc::new(
        Core::<Counter>::new()
            .handle_effects_using(counter_middleware::DelayedCounter {
                trigger: trigger_rx,
            })
            .native_bridge(move |id, effect| {
                tx.send((id, effect)).unwrap();
            }),
    );

    // FetchValue → middleware intercepts, waits for trigger
    bridge.update(Event::FetchValue).unwrap();

    // No effects yet — middleware is waiting
    assert!(
        rx.try_recv().is_err(),
        "no effects should arrive before trigger"
    );

    // Concurrent: send Increment while middleware bg thread is blocked
    bridge.update(Event::Increment).unwrap();

    // Increment's Render should arrive immediately (not intercepted)
    let effects = drain(&rx);
    assert_eq!(effects.len(), 1);
    assert!(matches!(effects[0].1, EffectFfi::Render(_)));
    assert_eq!(bridge.view(), ViewModel { count: 1 });

    // Now trigger the middleware
    trigger_tx.send(200).unwrap();

    // Wait for async Render from middleware resolution (GotValue(200))
    let (_, effect) = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("timed out waiting for delayed middleware");

    assert!(
        matches!(effect, EffectFfi::Render(_)),
        "expected Render from middleware resolution, got {effect:?}"
    );

    // Final view: GotValue(200) overwrites the count
    assert_eq!(bridge.view(), ViewModel { count: 200 });
}

#[test]
fn middleware_dropped_before_resolve() {
    let (trigger_tx, trigger_rx) = crossbeam_channel::unbounded::<i32>();
    let (tx, rx): (Sender<EffectEntry>, Receiver<EffectEntry>) = crossbeam_channel::unbounded();

    let bridge = Core::<Counter>::new()
        .handle_effects_using(counter_middleware::DelayedCounter {
            trigger: trigger_rx,
        })
        .native_bridge(move |id, effect| {
            tx.send((id, effect)).unwrap();
        });

    bridge.update(Event::FetchValue).unwrap();

    // Drop the trigger sender without sending — bg thread's recv() returns Err
    drop(trigger_tx);

    // Give the bg thread time to notice
    thread::sleep(Duration::from_millis(50));
    drop(bridge);

    // No effects should have arrived (middleware never resolved)
    let effects = drain(&rx);
    assert!(
        effects.is_empty(),
        "no effects expected when middleware never resolves, got {} effects",
        effects.len()
    );
}
