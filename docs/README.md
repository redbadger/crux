# Documentation Guide

This is built with [mdBook](https://github.com/rust-lang/mdBook). If you want to
change the docs, follow the
[installation](https://rust-lang.github.io/mdBook/guide/installation.html) and
[getting started guide](https://rust-lang.github.io/mdBook/guide/creating.html).

## mdbook plugins

To get the full styling, you need to install the following plugins:

```sh
cargo install mdbook-admonish
```

We also use the [linkcheck](https://github.com/Michael-F-Bryan/mdbook-linkcheck)
plugin to check for broken links:

```sh
cargo install mdbook-linkcheck
```
