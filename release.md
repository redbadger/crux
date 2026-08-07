# Cutting a release of crux

## Dependencies between crates

The Crux crates depend on one another, and need to be released in this order:

1. `crux_macros`
2. `crux_core`
3. Capability crates (`crux_http`, `crux_kv`, `crux_time`)

## Deciding the versions

`release-plz update` will move the versions according to the changes to each crate's
API. It isn't bulletproof, because it doesn't consider every change that is in fact
breaking to be breaking.

Things it misses (may not be a complete list):

- changes to `crux_macros` that generate incompatible code
- changes to capability operation types

**Every crate is pre-1.0, so a minor bump is the breaking bump.**

**WARNING**: if `crux_macros` or `crux_core` had a breaking change, it usually means
breaking changes in the capabilities too, because of trait changes and the like. The
capability crates need a minor bump even when their own API has not changed — a
capability left on the older `crux_core` alongside one on the newer pulls two
incompatible `crux_core` versions into the same tree, with two distinct `Effect` and
`Operation` trait sets.

You can also just bump the versions by hand. Either way, the version bumps and the
dated changelog entries go through a normal release PR, reviewed and merged like
anything else.

## Steps

1. Land the version bumps and changelog entries on `master` via a PR.

2. Wait for CI to be green on the merge commit — `build`, `examples` and `pages`.
   This is the commit the release is cut from, so it needs to be good.

3. Tag that commit, one tag per crate, named `<crate>-vX.Y.Z`. Every crate being
   released gets a tag, including the ones that only moved to stay in lockstep.

   ```sh
   jj tag set crux_core-v0.20.0 crux_http-v0.20.0 crux_kv-v0.14.0 \
     crux_macros-v0.10.1 crux_time-v0.18.0 -r master
   jj git push --tag crux_core-v0.20.0 --tag crux_http-v0.20.0 \
     --tag crux_kv-v0.14.0 --tag crux_macros-v0.10.1 --tag crux_time-v0.18.0
   ```

   These are lightweight tags, which is what every previous release used, and what
   the pages workflow's `^{commit}` check expects.

4. Publish to crates.io in the dependency order above, one at a time. `cargo publish`
   waits for each crate to appear on the index before returning, so the next one can
   depend on it.

   ```sh
   cargo publish -p crux_macros
   cargo publish -p crux_core
   cargo publish -p crux_http
   cargo publish -p crux_kv
   cargo publish -p crux_time
   ```

   Note that `cargo publish --dry-run` cannot verify a capability crate before
   `crux_core` is on the index — it requires the exact `crux_core` version, which does
   not exist on crates.io yet. Those three are only verifiable at publish time.

5. Create a GitHub release for the `crux_core` tag, and for any other crate whose
   release is substantive in its own right (`crux_http` has had its own release when
   it carried significant changes). The body is that crate's CHANGELOG section
   verbatim, and the title matches the tag.

   ```sh
   # notes.md holds that crate's CHANGELOG section for this version
   gh release create crux_core-v0.20.0 --title crux_core-v0.20.0 \
     --notes-file notes.md --verify-tag
   gh release edit crux_core-v0.20.0 --latest
   ```

   `--verify-tag` makes the release attach to the tag pushed in step 3 rather than
   silently creating a new one. GitHub gives "Latest" to whichever release was created
   most recently, so set it explicitly on `crux_core` if you cut more than one.

   Releases can be cut before publishing instead, but then they announce a version
   that is not yet on crates.io — if publishing then fails, the announcement is
   already public.

6. Update `docs/STABLE_REF` to the new `crux_core` tag, so the stable book is built
   from the release. It can only move once the tag is pushed, because the pages
   workflow verifies the ref resolves. See `docs/VERSIONING.md`.

7. Give Zulip a heads up about the good news :)
