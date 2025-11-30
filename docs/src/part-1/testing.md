# Testing the Counter app

In this chapter we'll write some basic tests for our counter app. It is tempting
to skip reading this, but please don't. Testing and testability is one of the
most important benefits of Crux, and even in this simple case, subtle things are going on,
which we'll build on later on.

## The first test

Technically, we've already broken [the rules](https://tddbuddy.com/references/tdd-cycle.html) and
wrote code without having a failing test for it. We're going to let that slip in the name of education,
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

        let mut cmd = app.update(Event::Reset, &mut model);

        // Check update asked us to `Render`
        cmd.expect_one_effect().expect_render();
    }
}
```

We create an instance of the app, and an instance of the model. Then we call update with the `Event::Reset` event.
As you may remember we get back a `Command`, which we expect to carry a request for a render operation. Using the
expectation helper API of the Command type, we check we got one effect, and that the effect is a render. Both methods will panic, if they don't succeed (they are also `#[cfg(test)]` only, don't use them outside of tests).

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

    let mut cmd = app.update(Event::Increment, &mut model);

    // Check update asked us to `Render`
    cmd.expect_one_effect().expect_render();

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
{{#include ../../../examples/simple_counter/shared/src/app.rs:test}}
```

You can see that occasionally, we test for the render to be requested. This will be important later, because
we'll be able to not only check for the effects, but also _resolve_ them – provide the value they requested,
for example the response to a HTTP request.

That will let us test entire user flows calling web APIs, working with local storage and timers, and anything
else, all at the speed of unit test and without ever touching the external world or writing a single fake (and maintaining it later).

For now though, let's actually give this thing some user interface. Time to build a Shell.
