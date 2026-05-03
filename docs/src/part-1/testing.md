# Testing the Counter app

In this chapter we'll write some basic tests for our counter app. It is tempting
to skip reading this, but please don't. Testing and testability is one of the
most important benefits of Crux, and even in this simple case, subtle things are going on,
which we'll build on later.

## The first test

Technically, we've already broken [the rules](https://tddbuddy.com/references/tdd-cycle.html) and
written code without having a failing test for it. We're going to let that slip in the name of education,
but let's fix that before someone alerts the TDD authorities.

The first test we're going to write will check that resetting the count renders the UI.

```rust,noplayground
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn renders() {
        let app = Counter;
        let mut model = Model::default();

        // Check update asked us to `Render`, and only that
        app.update(Event::Reset, &mut model).expect_only_render();
    }
}
```

We create an instance of the app, and an instance of the model. Then we call update with the `Event::Reset` event.
As you may remember we get back a `Command`, which we expect to carry a request for a render operation.
The `#[effect]` macro on the `Effect` enum we declared earlier generates chainable test-helper methods for `Command` (technically, this is implemented a a trait called `EffectTestExt`, which needs to be in scope). One of them is `expect_only_render`, which asserts
"the next effect is a Render and there are no others." It panics if either condition fails.
The trait is generated alongside the `Effect` declaration, so `use super::*;` brings it into scope automatically.

That test should pass (check with `cargo nextest run`). Next up, we can check that the view model is rendered
correctly

```rust,noplayground
#[test]
fn shows_initial_count() {
    let app = Counter;
    let model = Model::default();

    let actual_view = app.view(&model).count;
    let expected_view = "Count is: 0";

    assert_eq!(actual_view, expected_view);
}
```

This is a lot more basic, just a simple equality assertion. Let's try something a bit more interesting

```rust,noplayground
#[test]
fn increments_count() {
    let app = Counter;
    let mut model = Model::default();

    // Check update asked us to `Render`, and only that
    app.update(Event::Increment, &mut model).expect_only_render();

    let actual_view = app.view(&model).count;
    let expected_view = "Count is: 1";
    assert_eq!(actual_view, expected_view);
}
```

When we send the increment event, we expect to be told to render, and we expect the view to show `"Count is: 1"`.

You could just as well test just the model state, this is really up to you, what is more convenient and whether
you prefer your tests to know about how your state works and to what extent.

By now you get the gist, so here's all the tests to satisfy ourselves that the app does in fact work:

```rust,noplayground
{{#include ../../../examples/counter/shared/src/app.rs:test}}
```

You can see that occasionally, we test for the render to be requested. This will be important later, because
we'll be able to not only check for the effects, but also _resolve_ them – provide the value they requested,
for example the response to a HTTP request. The same `EffectTestExt` extension generates a `resolve_<variant>`
method per effect variant for that purpose, which we'll meet in [Testing with Effects](../part-2/testing-effects.md).

That will let us test entire user flows calling web APIs, working with local storage and timers, and anything
else, all at the speed of unit test and without ever touching the external world or writing a single fake (and maintaining it later).

For now though, let's actually give this thing some user interface. Time to [build a Shell](./shell.md).
