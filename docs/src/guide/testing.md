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

Let's take a really simple test from the
[Notes example app](https://github.com/redbadger/crux/tree/master/examples/notes)
and walk through it step by step — the test replaces some selected text in a
document and checks that the correct text is rendered.

```rust,ignore,no_run
{{#include ../../../examples/notes/shared/src/app.rs:replaces_selection_and_renders}}
```

The first thing to do is create an instance of our app (`NoteEditor`) and set it up
with a model for our test. In this case the document contains the
string `"hello"` with the last two characters selected.

Let's insert some text under the selection range. We simply create an `Event`
that captures the user's action and pass it into the app's `update()` method,
along with the Model we just created (which we will be able to inspect
afterwards).

```rust,ignore,no_run
let event = Event::Insert("ter skelter".to_string());
let mut cmd = app.update(event, &mut model);
```

````admonish
The `update()` method we called above does not take a `Capabilities` argument.
It is actually our own `update()` method that we delegate to in the `NoteEditor` app.
```rust,ignore
{{#include ../../../examples/notes/shared/src/app.rs:update}}
```
Once the migration to the new `Command` API is complete, the signature of this method
will be changed in the `App` trait and this delegation will no longer be required.
````

We can check that the shell was asked to render by using the
[`assert_effect!`](https://docs.rs/crux_core/latest/crux_core/macro.assert_effect.html)
macro, which panics if none of the effects generated by the update matches the
specified pattern.

```rust,ignore,no_run
assert_effect!(cmd, Effect::Render(_));
```

Finally we can ask the app for its `ViewModel` and use it to check that the text
was inserted correctly and that the cursor position was updated.

```rust,ignore,no_run
let view = app.view(&model);

assert_eq!(view.text, "helter skelter".to_string());
assert_eq!(view.cursor, TextCursor::Position(14));
```

## Writing a more complicated test

Now let's take a more complicated test and walk through that.
```rust,ignore,no_run
{{#include ../../../examples/notes/shared/src/app.rs:starts_a_timer_after_an_edit}}
```
This test checks that a "save" timer is restarted each
time the user edits the document (after a second of no activity the document is
stored). We will use the [`Time`](https://crates.io/crates/crux_time)
capability to manage this. Note that the _actual_ timer is run by the shell
(because it is a side effect, which would make it really tricky to test) —
but all we need to do is check that the behavior of the timer is correct
(i.e. started, finished and cancelled correctly).

Again, the first thing we need to do is create an instance of our app (`NoteEditor`),
supply a model to represent our starting state, and analyze the
`Event`s and `Effect`s that are generated.

We send an `Event` (e.g. raised in response to a user action) into our app in
order to check that it does the right thing.

Here we send an Insert event, which should start a timer. We filter out just the
`Effect`s that were created by the `Time` Capability, mapping them to their
inner `Request<TimeRequest>` type.

```rust,ignore,no_run
let event = Event::Insert("something".to_string());
let mut cmd1 = app.update(event, &mut model);
let mut requests = cmd1.effects().filter_map(Effect::into_timer);
```

There are a few things to discuss here. Firstly, the `update()` method returns
a `Command`, which gives us access to the `Event`s and `Effect`s. We are
only interested in the `Effect`s, so we call `effects()` to consume them as
an `Iterator`. Secondly, we use the `filter_map()` method to filter out just the
`Effect`s that were created by the `Time` Capability, using
`Effect::into_timer` to map the `Effect`s to their inner
`Request<TimeRequest>`.

The [`Effect`](https://github.com/redbadger/crux/tree/master/crux_macros) derive
macro generates filters and maps for each capability that we are using. So if
our `Capabilities` struct looked like this...

```rust,ignore,no_run

#[cfg_attr(feature = "typegen", derive(crux_core::macros::Export))]
#[derive(Effect)]
pub struct Capabilities {
    timer: Time<Event>,
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
Effect::into_timer(self) -> Option<Request<TimeRequest>>
Effect::into_render(self) -> Option<Request<RenderOperation>>
Effect::into_pub_sub(self) -> Option<Request<PubSubOperation>>
Effect::into_key_value(self) -> Option<Request<KeyValueOperation>>
```

We want to check that the first request is a `NotifyAfter` operation, and that the
timer is set to fire in 1000 milliseconds. So let's do a
pattern match and assign the `id` to a local variable called `first_id`,
which we'll use later. Finally, we don't expect any more timer requests to have
been generated.

```rust,ignore,no_run
let request = requests.next().unwrap();
let (first_id, duration) = match &request.operation {
    TimeRequest::NotifyAfter { id, duration } => (id.clone(), duration),
    _ => panic!("expected a NotifyAfter"),
};
assert_eq!(duration, &Duration::from_secs(1).into());
assert!(requests.next().is_none());
```

````admonish Note
There are other ways to analyze effects from the update.

You can take all the effects that match a predicate out of the update:

```rust,ignore,no_run
let mut effects = cmd2.effects().filter(|effect| effect.is_timer());
// or
let mut effects = cmd2.effects().filter(Effect::is_timer);
```

Or you can filter and map at the same time:

```rust,ignore,no_run
let mut requests = cmd2.effects().filter_map(Effect::into_timer);
```

There are also `expect_*` methods that allow you to assert and return a certain
type of effect:

```rust,ignore,no_run
let request = cmd.expect_one_effect().expect_render();
```
````

At this point the shell would start the timer (this is something the core can't
do as it is a side effect) and so we need to tell the app that it was created.
We do this by "resolving" the request.

Remember that `Request`s either resolve zero times (fire-and-forget, e.g. for
`Render`), once (request/response, e.g. for `Http`), or many times (for streams,
e.g. `Sse` — Server-Sent Events). The `Time` capability falls into the
"request/response" category, so at some point, we should resolve the `NotifyAfter`
request with a `DurationElapsed` response.

However, before the timer fires, we'll insert another character, which should cancel the
existing timer (still on `cmd1`) and start a new one (on `cmd2`).

```rust,ignore,no_run
let mut cmd2 = app.update(Event::Replace(1, 2, "a".to_string()), &mut model);
let mut requests = cmd2.effects().filter_map(Effect::into_timer);

// but first, the original request (cmd1) should resolve with a clear
let cancel_request = cmd1
    .effects()
    .filter_map(Effect::into_timer)
    .next()
    .unwrap();
let cancel_id = match &cancel_request.operation {
    TimeRequest::Clear { id } => id.clone(),
    _ => panic!("expected a Clear"),
};
assert_eq!(cancel_id, first_id);
```

In the real world, time passes and the timer fires, but all we have to do is to
resolve our start request again, but this time with a `DurationElapsed` response.

```rust,ignore,no_run
start_request
    .resolve(TimeResponse::DurationElapsed { id: second_id })
    .unwrap();
```

Another edit should result in another timer, but not in a cancellation:

```rust,ignore,no_run
// One more edit. Should result in a new timer
let mut cmd4 = app.update(Event::Backspace, &mut model);
let mut effects = cmd4.effects();

let _publish = effects.next().unwrap().expect_pub_sub();
let timer = effects.next().unwrap().expect_timer();
assert_eq!(
    timer.operation,
    TimeRequest::NotifyAfter {
        id: TimerId(3),
        duration: crux_time::Duration::from_millis(1000)
    }
);
```

Note that this test was not about testing whether the model was updated
correctly (that is covered in other tests) so we don't call the app's `view()`
method — it's just about checking that the timer is started, cancelled and
restarted correctly.
