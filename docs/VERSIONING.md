# Book Versioning

The published documentation site builds two versions of the book:

- **Stable** — matches the currently published `crux_core` crate on crates.io.
- **Latest (master)** — tracks the `master` branch and reflects unreleased changes.

The stable build is driven by a pointer file, `docs/STABLE_REF`, which contains the
name of the git tag the stable book is built from (e.g. `crux_core-v0.17.0`). The
CI workflow reads this file, checks out that tag, and builds both the book and the
`cargo doc` API docs from it.

Because the entire repo — including `examples/` — is checked out at the tag, all
`{{#include}}` paths in the book automatically resolve against the correct version of
the example code. There is no need to maintain a long-lived doc branch.

---

## The common case: a fix that applies to both stable and master

This covers most changes — typos, broken links, clarifications, example corrections.

### 1. Land the fix on `master` first

Work on `master` as normal. Open a PR, get it reviewed, merge it.

**git**
```sh
git checkout master
git checkout -b docs/fix-typo-in-part-1
# make your changes
git commit -am "docs: fix typo in part 1"
# open PR, merge
```

**jj**
```sh
jj new master -m "docs: fix typo in part 1"
# make your changes
# open PR, push to a bookmark
jj git push --change @
```

### 2. Cut a docs patch tag from the current stable tag

Fetch the latest tags, then start a new branch/change from the stable release tag and
cherry-pick the fix onto it.

**git**
```sh
git fetch --tags
git checkout -b docs-patch/v0.17.0 crux_core-v0.17.0
git cherry-pick <sha>   # the commit (or squashed merge commit) from master
# resolve any trivial conflicts, then:
git tag crux_core-v0.17.0-docs.1
git push origin crux_core-v0.17.0-docs.1
# the working branch can be deleted after the tag is pushed
git checkout master
git branch -d docs-patch/v0.17.0
```

**jj**
```sh
jj git fetch
# duplicate the change onto the release tag — this is jj's cherry-pick equivalent;
# it creates a copy of the change without moving it off master
jj duplicate <change-id> -d crux_core-v0.17.0
# jj prints the new change ID in the output; use it to create the tag
jj tag set crux_core-v0.17.0-docs.1 -r <new-change-id>
jj git push --remote origin --named crux_core-v0.17.0-docs.1
```

If `jj duplicate` produces conflicts and you want to interactively pick only some
of the hunks rather than resolving the full conflict, you can use `jj restore -i`
from within the duplicated change:

```sh
jj restore -i --from <change-id>
```

Note that `jj restore --from` gives you the *state* of files at that change, not
the *diff* it introduced — use it only for selective hunk resolution, not as a
cherry-pick substitute.

### 3. Update `docs/STABLE_REF` on `master`

**git**
```sh
git checkout master
git checkout -b chore/bump-stable-docs-ref
echo crux_core-v0.17.0-docs.1 > docs/STABLE_REF
git commit -am "chore(docs): point stable book at crux_core-v0.17.0-docs.1"
# open PR, merge
```

**jj**
```sh
jj new master -m "chore(docs): point stable book at crux_core-v0.17.0-docs.1"
echo crux_core-v0.17.0-docs.1 > docs/STABLE_REF
jj git push --change @
# open PR, merge
```

The pages workflow runs on merge to `master` and republishes both books. The stable
site now carries the fix; `latest_master` already had it from step 1.

---

## Edge case 1: the fix only applies to stable

The relevant section has been rewritten on `master`, so there's nothing to cherry-pick.
Skip step 1 and make the fix directly on a branch/change from the release tag.

**git**
```sh
git fetch --tags
git checkout -b docs-patch/v0.17.0 crux_core-v0.17.0
# make your changes directly
git commit -am "docs: fix section that no longer exists on master"
git tag crux_core-v0.17.0-docs.1
git push origin crux_core-v0.17.0-docs.1
git checkout master
git branch -d docs-patch/v0.17.0
```

**jj**
```sh
jj git fetch
jj new crux_core-v0.17.0 -m "docs: fix section that no longer exists on master"
# make your changes
jj tag set crux_core-v0.17.0-docs.1 -r @
jj git push --remote origin --named crux_core-v0.17.0-docs.1
```

Then update `docs/STABLE_REF` on `master` as in step 3 above.

> **Note on review:** because this fix never goes through a PR on `master`, it gets
> no automatic review. If you want a review surface, push the `docs-patch/v0.17.0`
> branch to the remote and open a PR before tagging. GitHub will let you open a PR
> from a branch that diverges from `master`; it can't be merged normally but it gives
> reviewers a diff to comment on. Tag from the branch tip after approval.

---

## Edge case 2: the fix touches an `{{#include}}`'d example file

The cherry-pick from `master` may:

- **Apply cleanly** — done. Build locally to confirm (see below).
- **Conflict because the example has been refactored on `master`** — hand-edit the
  example *as it existed at the release tag* to fix the underlying issue. Treat the
  cherry-pick as a hint, not a recipe.
- **Not apply at all because the example didn't exist at the release tag** — the fix
  isn't relevant to stable. Don't tag.

In all cases, because you are working on a checkout of the tag, `mdbook serve` will
use the correct `examples/` sources and you can verify the rendered output before
tagging.

---

## Testing locally before tagging

Because the whole repo is at the tag, all `{{#include}}` paths resolve against the
matching example code — exactly as the deployed stable site will see them.

**git**
```sh
git checkout crux_core-v0.17.0-docs.1
cd docs
mdbook serve
```

**jj**
```sh
jj git fetch
# create a new change on top of the tag so the working copy reflects it
jj new crux_core-v0.17.0-docs.1
cd docs
mdbook serve
```

---

## Conventions

- **Tag naming:** `crux_core-v<X.Y.Z>-docs.<N>`, where `<N>` increments from 1 for
  each subsequent doc-only patch to the same release (e.g. `crux_core-v0.17.0-docs.1`,
  `crux_core-v0.17.0-docs.2`). The `-docs.` infix makes them easy to grep and keeps
  them out of release-tooling filters.

- **`docs/STABLE_REF` format:** the file contains exactly one line — the tag name —
  with no trailing whitespace. This is read by the CI workflow with `cat`.

- **Stop backporting at the next release:** once `crux_core` v0.18 is released and
  `STABLE_REF` is updated, stop patching the v0.17 docs unless there is a specific
  reason. The tags still exist and are permanently accessible; the live site moves on.

- **CI sanity check:** the pages workflow verifies that the ref in `STABLE_REF` names
  a tag that actually exists:
  ```sh
  git rev-parse --verify "$(cat docs/STABLE_REF)^{tag}"
  ```
  A typo in `STABLE_REF` fails the build loudly rather than silently deploying the
  wrong thing.

- **Do not maintain a long-lived `book-stable` branch.** The tag is the stable
  artifact. A branch would drift from `master` over time in ways unrelated to the docs
  and recreate the problem this process is designed to avoid.