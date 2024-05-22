# Community contributed capabilities

This is a space for the Crux user community to contribute and share their
[capability](https://redbadger.github.io/crux/guide/capabilities.html)
implementations, so that others can use them, and we can observe the commonalities
and eventually promote some of the semantics into an official implementation,
if it seems like an obviously good idea.

## Contributing

1. Fork the repo
1. Find or make a directory for the effect type your capability is handling (e.g. `websocket/`)
1. Inside create a uniquely named directory for the implementation (e.g. your github handle,
   company name, app name, etc.), with a `rust` subdirectory containing your code.
   (feel free to include a `Cargo.toml` to make it a valid rust project with tests, and
   a Readme is always a nice bonus)
1. If you want to contribute shell-side code as well (fantastic!), add a `swift`, `kotlin`
   or `ts` directory with the code next to `rust` directory.
1. Open a Pull Request back to `redbadger/crux`, and we should merge it shortly.

## Disclaimer

This is community contributed code and the Crux team can't guarantee any level of quality
(be it worse or better than Crux itself) or provide support for it, the code is provided as is.

By contributing the code, you are also publishing it under the same open source license Crux
itself is published under ([Apache 2.0](../../LICENSE))
