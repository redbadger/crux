# Crux Time capability

This crate contains the `Time` capability, which can be used to ask the current time from the Shell.

For an example of how to use the capability, see the [integration test](./tests/time_test.rs).

## About Crux Capabilities

Crux capabilities teach Crux how to interact with the shell when performing side effects. They do the following:

1. define a `Request` struct to instruct the Shell how to perform the side effect on behalf of the Core
1. define a `Response` struct to hold the data returned by the Shell after the side effect has completed
1. declare one or more convenience methods for invoking the Shell's capability, each of which creates a `Command` (describing the effect and its continuation) that Crux can "execute"

> Note that because Swift has no namespacing, there is currently a requirement to ensure that `Request` and `Response` are unambiguously named (e.g. `HttpRequest` and `HttpResponse`).
