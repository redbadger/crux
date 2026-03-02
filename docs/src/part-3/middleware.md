# Middleware

Middleware is a relatively new, and somewhat advanced feature for split
effect handling, i.e. handling some effects in the shell, and some
still in the core, but outside of the Crux state loop.

Middleware can be useful when you have an existing 3rd party library written in Rust
which you want to use, but it isn't written in a sans-I/O way with managed
effects or otherwise isn't compatible with Crux. This is sadly most libraries
with side effects.

It is quite likely most apps will never need to use middleware. Before reaching for middleware,
we encourage you to consider:

- Implementing the side-effect in each Shell using native, platform SDKs. Shared libraries give a
  productivity boost at first, but for the same reason Crux uses Capabilities, they can't always
  be the best platform citizens, and often rely on very low-level system APIs which compromise the
  experience, don't collaborate well with platform security measures, etc.
- Moving coordination logic from the Rust implementation into a custom capability in the core
  and implementing it on top of lower level capabilities, e.g. HTTP. This would be the case for
  HTTP API SDK type libraries, but may well not be practical at first

Only if neither of these is a good option, reach for a middleware. The cost of using it is
that the effect handling becomes less straightforward, which may cause some headaches
debugging effect ordering, etc.

We are also still learning how middleware operates in the wild, and the API may change more
than the rest of Crux tends to.

All that said, the feature is used in production with success today and should work well.

```admonish info
This chapter is not finished, we're working on it. For examples of middleware use you can
read the [API docs](https://docs.rs/crux_core/latest/crux_core/middleware/index.html),
the [tests of the module](https://github.com/redbadger/crux/blob/master/crux_core/tests/middleware.rs)
and one example in the [Counter next example code](https://github.com/redbadger/crux/blob/master/examples/counter-next/shared/src/ffi.rs)
```
