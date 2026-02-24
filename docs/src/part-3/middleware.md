# Middleware

Middleware is a relatively new, and still somewhat experimental support
for split effect handling – handling some effects in the shell, and some
still in the core, but outside of the Crux state loop.

This is useful when you have an existing 3rd party library written in Rust
which you want to use, but it isn't written in a sans-I/O way with managed
effects or otherwise isn't compatible with Crux. This is most libraries
with side effects.

```admonish info
This chapter is not finished, we're working on it. For examples of middleware use you can
read the [API docs](https://docs.rs/crux_core/latest/crux_core/middleware/index.html),
the [tests of the module](https://github.com/redbadger/crux/blob/master/crux_core/tests/middleware.rs)
and one example in the [Counter next example code](https://github.com/redbadger/crux/blob/master/examples/counter-next/shared/src/ffi.rs)
```
