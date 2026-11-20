use crux_core::EventName as _;

// Payloads deliberately have no reflection, formatting, or serialization traits.
struct Payload;

#[derive(crux_core::macros::EventName)]
enum Event<T> {
    Start,
    Loaded(T),
    Failed { payload: T },
}

#[derive(crux_core::macros::EventName)]
struct Update<T>(T);

#[test]
fn derives_names_without_payload_bounds() {
    assert_eq!(Event::<Payload>::Start.event_name(), "Start");
    assert_eq!(Event::Loaded(Payload).event_name(), "Loaded");
    let event = Event::Failed { payload: Payload };
    assert_eq!(event.event_name(), "Failed");
    let Event::Failed { payload } = event else {
        unreachable!()
    };
    assert_eq!(Update(payload).event_name(), "Update");
}
