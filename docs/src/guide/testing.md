# A guide to testing Crux apps

## Introduction

One of the most compelling consequences of the Crux architecture is that it
becomes trivial to comprehensively test your application. This is because the
core is pure and therefore completely deterministic — all the side effects are
pushed to the shell.

It's straightforward to write an exhaustive set of unit tests that give you
complete confidence in the correctness of your application code — you can test
the behavior of your application independently of platform-specific UI and API
calls.

There is no need to mock/stub anything, and there is no need to write
integration tests.

Not only are the unit tests easy to write, but they run extremely quickly, and
can be run in parallel.

For example, the
[Notes example app](https://github.com/redbadger/crux/tree/master/examples/notes)
contains complex logic related to collaborative text-editing using Conflict-free
Replicated Data Types (CRDTs). The test suite consists of 25 tests that give us
high coverage and high confidence of correctness. Many of the tests include
instantiating two instances (alice and bob) and checking that, even during
complex edits, the synchronization between them works correctly.

This test, for example, ensures that when Alice and Bob both insert text at the
same time, they both end up with the same result. It runs in 4 milliseconds.

```rust,ignore,no_run
#[test]
fn two_way_sync() {
    let (mut alice, mut bob) = make_alice_and_bob();

    alice.update(Event::Insert("world".to_string()));
    let edits = alice.edits.drain(0..).collect::<Vec<_>>();

    bob.send_edits(edits.as_ref());

    // Alice's inserts should go in front of Bob's cursor
    // so we break the ambiguity of same cursor position
    // as quickly as possible
    bob.update(Event::Insert("Hello ".to_string()));
    let edits = bob.edits.drain(0..).collect::<Vec<_>>();

    alice.send_edits(edits.as_ref());

    let alice_view = alice.view();
    let bob_view = bob.view();

    assert_eq!(alice_view.text, "Hello world".to_string());
    assert_eq!(alice_view.text, bob_view.text);
}
```

And the full suite of 25 tests runs in 16 milliseconds.

```txt
cargo nextest run --release -p shared
    Finished release [optimized] target(s) in 0.07s
    Starting 25 tests across 2 binaries
        PASS [   0.005s] shared app::editing_tests::handles_emoji
        PASS [   0.005s] shared app::editing_tests::removes_character_before_cursor
        PASS [   0.005s] shared app::editing_tests::moves_cursor
        PASS [   0.006s] shared app::editing_tests::inserts_text_at_cursor_and_renders
        PASS [   0.005s] shared app::editing_tests::removes_selection_on_backspace
        PASS [   0.005s] shared app::editing_tests::removes_character_after_cursor
        PASS [   0.005s] shared app::editing_tests::removes_selection_on_delete
        PASS [   0.007s] shared app::editing_tests::changes_selection
        PASS [   0.006s] shared app::editing_tests::renders_text_and_cursor
        PASS [   0.006s] shared app::editing_tests::replaces_empty_range_and_renders
        PASS [   0.005s] shared app::editing_tests::replaces_range_and_renders
        PASS [   0.005s] shared app::note::test::splices_text
        PASS [   0.005s] shared app::editing_tests::replaces_selection_and_renders
        PASS [   0.004s] shared app::save_load_tests::opens_a_document
        PASS [   0.005s] shared app::note::test::inserts_text
        PASS [   0.005s] shared app::save_load_tests::saves_document_when_typing_stops
        PASS [   0.005s] shared app::save_load_tests::starts_a_timer_after_an_edit
        PASS [   0.006s] shared app::save_load_tests::creates_a_document_if_it_cant_open_one
        PASS [   0.005s] shared app::sync_tests::concurrent_clean_edits
        PASS [   0.005s] shared app::sync_tests::concurrent_conflicting_edits
        PASS [   0.005s] shared app::sync_tests::one_way_sync
        PASS [   0.005s] shared app::sync_tests::remote_delete_moves_cursor
        PASS [   0.005s] shared app::sync_tests::remote_insert_behind_cursor
        PASS [   0.004s] shared app::sync_tests::two_way_sync
        PASS [   0.005s] shared app::sync_tests::receiving_own_edits
------------
     Summary [   0.016s] 25 tests run: 25 passed, 0 skipped
```

## Writing a simple test

Crux provides a simple test harness that we can use to write unit tests for our
application code. Strictly speaking it's not needed, but it makes it easier to
avoid boilerplate and to write tests that are easy to read and understand.

Let's take a
[really simple test](https://github.com/redbadger/crux/blob/master/examples/notes/shared/src/app.rs#L379)
from the
[Notes example app](https://github.com/redbadger/crux/tree/master/examples/notes)
and walk through it step by step — the test replaces some selected text in a
document and checks that the correct text is rendered.

The first thing to do is create an instance of the `AppTester` test harness,
which runs our app (`NoteEditor`) and makes it easy to analyze the `Event`s and
`Effect`s that are generated.

```rust,ignore,no_run
let app = AppTester::<NoteEditor, _>::default();
```

The `Model` is normally private to the app (`NoteEditor`), but `AppTester`
allows us to set it up for our test. In this case the document contains the
string `"hello"` with the last three characters selected.

```rust,ignore,no_run
let mut model = Model {
    note: Note::with_text("hello"),
    cursor: TextCursor::Selection(3..5),
    ..Default::default()
};
```

Let's insert the text under the selection range. We simply create an `Event`
that captures the user's action and pass it into the app's `update()` method,
along with the Model we just created (which we will be able to inspect
afterwards).

```rust,ignore,no_run
let event = Event::Insert("ter skelter".to_string());
let update = app.update(event, &mut model);
```

We can check that the shell was asked to render by using the
[`assert_effect!`](https://docs.rs/crux_core/latest/crux_core/macro.assert_effect.html)
macro, which panics if none of the effects generated by the update matches the
specified pattern.

```rust,ignore,no_run
assert_effect!(update, Effect::Render(_));
```

Finally we can ask the app for its `ViewModel` and use it to check that the text
was inserted correctly and that the cursor position was updated.

```rust,ignore,no_run
let view = app.view(&model);

assert_eq!(view.text, "helter skelter".to_string());
assert_eq!(view.cursor, TextCursor::Position(14));
```

## Writing a more complicated test

Now let's take a
[more complicated test](https://github.com/redbadger/crux/blob/master/examples/notes/shared/src/app.rs#L630)
and walk through that. This test checks that a "save" timer is restarted each
time the user edits the document (after a second of no activity the document is
stored). Note that the _actual_ timer is run by the shell (because it is a side
effect, which would make it really tricky to test) — but all we need to do is
check that the behavior of the timer is correct (i.e. started, finished and
cancelled correctly).

Again, the first thing we need to do is create an instance of the `AppTester`
test harness, which runs our app (`NoteEditor`) and makes it easy to analyze the
`Event`s and `Effect`s that are generated.

```rust,ignore,no_run
let app = AppTester::<NoteEditor, _>::default();
```

We again need to set up a `Model` that we can pass to the `update()` method.

```rust,ignore,no_run
let mut model = Model {
    note: Note::with_text("hello"),
    cursor: TextCursor::Selection(2..4),
    ..Default::default()
};
```

We send an `Event` (e.g. raised in response to a user action) into our app in
order to check that it does the right thing.

Here we send an Insert event, which should start a timer. We filter out just the
`Effect`s that were created by the `Timer` Capability, mapping them to their
inner `Request<TimerOperation>` type.

```rust,ignore,no_run
let requests = &mut app
    .update(Event::Insert("something".to_string()), &mut model)
    .into_effects()
    .filter_map(Effect::into_timer);
```

There are a few things to discuss here. Firstly, the `update()` method returns
an `Update` struct, which contains vectors of `Event`s and `Effect`s. We are
only interested in the `Effect`s, so we call `into_effects()` to consume them as
an `Iterator` (there are also `effects()` and `effects_mut()` methods that allow
us to borrow the `Effect`s instead of consuming them, but we don't need that
here). Secondly, we use the `filter_map()` method to filter out just the
`Effect`s that were created by the `Timer` Capability, using
`Effect::into_timer` to map the `Effect`s to their inner
`Request<TimerOperation>`.

The [`Effect`](https://github.com/redbadger/crux/tree/master/crux_macros) derive
macro generates filters and maps for each capability that we are using. So if
our `Capabilities` struct looked like this...

```rust,ignore,no_run

#[cfg_attr(feature = "typegen", derive(crux_macros::Export))]
#[derive(Effect)]
#[effect(app = "NoteEditor")]
pub struct Capabilities {
    timer: Timer<Event>,
    render: Render<Event>,
    pub_sub: PubSub<Event>,
    key_value: KeyValue<Event>,
}
```

... we would get the following filters and filter_maps:

```rust,ignore,no_run
// filters
Effect::is_timer(&self) -> bool
Effect::is_render(&self) -> bool
Effect::is_pub_sub(&self) -> bool
Effect::is_key_value(&self) -> bool
// filter_maps
Effect::into_timer(self) -> Option<Request<TimerOperation>>
Effect::into_render(self) -> Option<Request<RenderOperation>>
Effect::into_pub_sub(self) -> Option<Request<PubSubOperation>>
Effect::into_key_value(self) -> Option<Request<KeyValueOperation>>
```

We want to check that the first request is a `Start` operation, and that the
timer is set to fire in 1000 milliseconds. The macro
[`assert_let!()`](https://docs.rs/assert_let/0.1.0/assert_let/) does a pattern
match for us and assigns the `id` to a local variable called `first_id`, which
we'll use later. Finally, we don't expect any more timer requests to have been
generated.

```rust,ignore,no_run
let mut request = requests.next().unwrap(); // this is mutable so we can resolve it later
assert_let!(
    TimerOperation::Start {
        id: first_id,
        millis: 1000
    },
    request.operation.clone()
);
assert!(requests.next().is_none());
```

At this point the shell would start the timer (this is something the core can't
do as it is a side effect) and so we need to tell the app that it was created.
We do this by "resolving" the request.

Remember that `Request`s either resolve zero times (fire-and-forget, e.g. for
`Render`), once (request/response, e.g. for `Http`), or many times (for streams,
e.g. `Sse` — Server-Sent Events). The `Timer` capability falls into the
"request/response" category, so we need to resolve the `Start` request with a
`Created` response. This tells the app that the timer has been started, and
allows it to cancel the timer if necessary.

Note that resolving a request could call the app's `update()` method resulting
in more `Event`s being generated, which we need to feed back into the app.

```rust,ignore,no_run
let update = app
    .resolve(&mut request, TimerOutput::Created { id: first_id }).unwrap();
for event in update.events {
    app.update(event, &mut model);
}
```

Before the timer fires, we'll insert another character, which should cancel the
existing timer and start a new one.

```rust,ignore,no_run
let mut requests = app
    .update(Event::Replace(1, 2, "a".to_string()), &mut model)
    .into_effects()
    .filter_map(Effect::into_timer);

let cancel_request = requests.next().unwrap();
assert_let!(
    TimerOperation::Cancel { id: cancel_id },
    cancel_request.operation
);
assert_eq!(cancel_id, first_id);

let start_request = &mut requests.next().unwrap(); // this is mutable so we can resolve it later
assert_let!(
    TimerOperation::Start {
        id: second_id,
        millis: 1000
    },
    start_request.operation.clone()
);
assert_ne!(first_id, second_id);

assert!(requests.next().is_none());
```

Now we need to tell the app that the second timer was created.

```rust,ignore,no_run
let update = app
    .resolve(start_request, TimerOutput::Created { id: second_id })
    .unwrap();
for event in update.events {
    app.update(event, &mut model);
}
```

In the real world, time passes and the timer fires, but all we have to do is to
resolve our start request again, but this time with a `Finished` response.

```rust,ignore,no_run
let update = app
    .resolve(start_request, TimerOutput::Finished { id: second_id })
    .unwrap();
for event in update.events {
    app.update(event, &mut model);
}
```

Another edit should result in another timer, but not in a cancellation:

```rust,ignore,no_run
let update = app.update(Event::Backspace, &mut model);
let mut timer_requests = update.into_effects().filter_map(Effect::into_timer);

assert_let!(
    TimerOperation::Start {
        id: third_id,
        millis: 1000
    },
    timer_requests.next().unwrap().operation
);
assert!(timer_requests.next().is_none()); // no cancellation

assert_ne!(third_id, second_id);
```

Note that this test was not about testing whether the model was updated
correctly (that is covered in other tests) so we don't call the app's `view()`
method — it's just about checking that the timer is started, cancelled and
restarted correctly.
