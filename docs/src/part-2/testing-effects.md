# Testing with managed effects

We have seen how to use effects, and we have seen a little bit about the testing,
but we should look at that closer.

Crux was expressly designed to support easy, fast, comprehensive testing of your
application. Everyone is generally on board with unit tests and TDD when it comes
to basic pure logic. But as soon as any I/O or UI gets involved, the dread sets in.
We're going to have to set up some fakes, introduce additional traits _just_ to test
things, or just bite the bullet and build tests around a fully integrated app and
wait for them to run (and probably fail on a race condition sometimes). So most people give up.

Managed effects smooth over that big hump. You pay for it a little bit in how the
code is written, but you reap the reward in testing it. This is because the core
that uses managed effects is pure and therefore completely deterministic:
all the side effects are pushed to the shell.

It's straightforward to write an exhaustive set of unit tests that give you
complete confidence in the correctness of your application code. You can test
the behavior of your application independently of platform-specific UI and API
calls.

There is no need to mock/stub anything, and there is no need to write
integration tests.

Not only are the unit tests easy to write, but they run extremely quickly, and
can be run in parallel.

For example, here's a test that drives `LocalWeather` through a full weather fetch: checking location permission, resolving the location, then handling the weather response. A setup helper advances the state machine through the first two events by resolving each effect with a canned response:

```rust
{{#include ../../../examples/weather/shared/src/model/active/home/local.rs:drive_helper}}
```

The test itself picks up from `FetchingWeather`, resolves the HTTP effect, and asserts that the final state is `Fetched` with the expected data:

```rust
{{#include ../../../examples/weather/shared/src/model/active/home/local.rs:full_test}}
```

It's a test of a whole interaction with multiple kinds of effects (location services and HTTP) and it runs in a couple of milliseconds, entirely deterministic. The code being tested is `LocalWeather::update` in [Nested state machines](./nested_state_machines.md); managed effects let us verify the whole transaction without executing any of it.

The full suite of 57 tests of the Weather app runs in around 20 milliseconds on a Mac Mini M4 Pro. In practice, it's rare for a test suite of a Crux app to take longer than compiling it (even incrementally). Apps with thousands of tests usually run them in seconds, though compilation takes longer.

```txt
cargo nextest run
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.28s
────────────
    Starting 57 tests across 1 binary
    ...
     Summary [   0.020s] 57 tests run: 57 passed, 0 skipped
```

## The test steps

Crux gives us a chainable test API that keeps assertions concise, but you still drive the event → update → effect → resolve cycle yourself. You'll see methods like `.expect_only_render()` and `.resolve_http(|op| ...)` on `Command` throughout this chapter. They come from an `EffectTestExt` trait the `#[effect]` macro generates next to your `Effect` enum when `crux_core`'s `testing` feature is enabled, and auto-implements for `Command<Effect, E>` where `E` is an event.

Let's walk through a simpler test from the Weather app step by step:

```rust
{{#include ../../../examples/weather/shared/src/model/active/home/local.rs:simple_test}}
```

First, we build a fresh `LocalWeather::default()`. Its starting state is `CheckingPermission`.

We then call `update` with `LocationEnabled(true)`, as if the shell had just reported that location services are available. `update` returns an `Outcome`, which we destructure with `.expect_continue().into_parts()`. We know this event doesn't complete the state machine, so we assert on `Continue` and get back the updated state plus any command.

We assert the new state is `FetchingLocation`. Then we call `.expect_only_get_location_with(|op| ...)` on the command. That one chained call says "the only effect on this command is a `GetLocation` effect, and here's a closure to inspect its operation." Inside the closure we check the operation is the `GetLocation` we expect.

That's the whole test. `update` is a pure function, so there's nothing to set up beyond the initial state and nothing to tear down.

## More integrated tests and deterministic simulation testing

The tests above resolve each effect with a canned response. We can also test the app in a more integrated way. Instead of a canned response for each HTTP request, the test can forward the request to a small fake of the weather API. The fake keeps its own data and answers each request as the real API does. In the same way, the test can forward the key-value effects to a key-value store that keeps its data in a `HashMap`. The Weather app stores its favourites with these effects, so a test can make sure that the app saves and loads them correctly.

This can sound like mocking, but it is different. We do not implement Redis or build a real HTTP server. Each fake is a small amount of simple code. If we write a fake for each effect that the app uses, and each fake is realistic _enough_, an interesting thing happens. The whole app stack runs in a unit test, without the technical details of the transport.

![Two columns of layers. At runtime, from the top: the app core, the shell's effect handler, the transport (an HTTP client, the network and an HTTP server) and the server logic. In tests: the same app core, an effect handler for the test, and a simplified version of the server logic that runs in the test process. The transport layers are not in tests.](mocking_layers.svg)

With these fakes, we can create an app instance and send it random events. A seed controls the random events, so each run is deterministic. Then we make sure that "nothing bad happens". What that means is specific to each app, but here are some options:

- Add random errors to the fake API, and make sure that the app handles them correctly.
- Lose random data in storage, and make sure that the app recovers.
- Fire timeouts early at random, and make sure that the app handles them correctly.
- Check that other invariants hold. For example, values that relate to time only move forward (counters count up), storage stays referentially consistent, and logically impossible states do not occur. (Ideally, the types make these states impossible to represent, but sometimes that is too difficult.)

We can then run this pseudo-random process for hours, and let it find bugs for us. To reproduce a bug, we need only the random seed of that test run.

In practice, most Crux apps can process thousands of events each second. These tests explore more of the state space than we ever could with manual unit tests.

This type of testing is usually only for consensus algorithms and network protocols. In those systems, anything that can happen _will_ happen, so they must be completely reliable. For other software, the test harness is usually too much work. With managed effects, the harness is a few hundred lines of additional code. For an app of moderate size, it takes a few days to write. Future versions of Crux might include building blocks for such a test harness.
