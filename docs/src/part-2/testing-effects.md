# Testing Crux apps

FIXME: Use Weather tests to demonstrate this

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

For example, here's a test checking that when the weather screen is shown,
a location gets checked and the weather gets refreshed.

```rust
{{#include ../../../examples/weather/shared/src/weather/events.rs:test}}
```

You can see it's a test of a whole interaction with multiple effects, and it runs in 11 ms.

Here's the corresponding code it's testing:

```rust
{{#include ../../../examples/weather/shared/src/weather/events.rs:code}}
```

Hopefully this illustrates that the command API lets you test entire transactions involving effects, without ever executing any.

The full suite of 18 tests runs in 49 milliseconds.

```txt
cargo nextest run
   Compiling shared v0.1.0 (/Users/viktor/Projects/crux/examples/weather/shared)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1.11s
────────────
 Nextest run ID 4f51de83-8f2e-4acf-b75f-03969767e886 with nextest profile: default
    Starting 18 tests across 1 binary
        PASS [   0.020s] shared app::tests::test_navigation
        PASS [   0.020s] shared favorites::events::tests::test_add_multiple_favorites
        PASS [   0.019s] shared favorites::events::tests::test_delete_confirmed
        PASS [   0.020s] shared favorites::events::tests::test_cancel_returns_to_favorites
        PASS [   0.019s] shared favorites::events::tests::test_kv_set_and_load
        PASS [   0.023s] shared favorites::events::tests::test_delete_cancelled
        PASS [   0.023s] shared favorites::events::tests::test_delete_pressed
        PASS [   0.022s] shared favorites::events::tests::test_delete_with_persistence
        PASS [   0.022s] shared favorites::events::tests::test_kv_load_empty
        PASS [   0.013s] shared favorites::events::tests::test_kv_load_error
        PASS [   0.011s] shared favorites::events::tests::test_submit_duplicate_favorite
        PASS [   0.012s] shared favorites::events::tests::test_submit_adds_favorite
        PASS [   0.013s] shared favorites::events::tests::test_submit_persists_favorite
        PASS [   0.011s] shared weather::events::tests::test_fetch_favorites_triggers_fetch_for_all_favorites
        PASS [   0.011s] shared weather::events::tests::test_show_triggers_set_weather
        PASS [   0.012s] shared weather::events::tests::test_fetch_triggers_favorites_fetch_when_favorites_exist
        PASS [   0.027s] shared weather::events::tests::test_current_weather_fetch
        PASS [   0.027s] shared favorites::events::tests::test_search_triggers_api_call
────────────
     Summary [   0.049s] 18 tests run: 18 passed, 0 skipped
```

## Writing a simple test

Crux provides a simple test harness that we can use to write unit tests for our
application code. Strictly speaking it's not needed, but it makes it easier to
avoid boilerplate and to write tests that are easy to read and understand.

Let's take the test from earlier and walk through it step by step.

TODO: step through the test
