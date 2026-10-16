---
name: pr-flow
description: How changes reach main in sekiro-rs - one feature per branch and PR, local checks including a live-game run, succinct PR body. Use whenever committing, pushing, opening a PR or finishing a job in this repo.
---

# PR flow

Nothing is committed straight to `main`.
Every change goes through its own branch and pull request.

## Rules

- One feature or fix per PR.
If a job turns up a second problem, note it and open a separate PR for it.
- One branch per PR, named `<area>/<short-kebab-name>`, for example `map/ground-visibility` or `perf/load-time`.
- Commit as AKJama <92124781+AKJama@users.noreply.github.com>, with no co-author lines.
- Stage explicit file paths only, never `git add -A` or `git add .`, because agent work may be sitting in the tree.
- Agents do not commit, push or open PRs; the main session does after reviewing their work.
- No git worktrees.
- Merge only after the checks below pass and the user says to merge.
Use squash merge, then delete the branch and pull `main`.

## Checks before opening the PR

Run these from the repo root on the branch.
They need the installed game and `cache/`, so they run locally, not in GitHub Actions.

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --release -- -D warnings
cargo test --release --workspace
```

The workspace tests include the data gate.
For any change that can affect rendering, also build the release binary and run both render gates (see `docs/MAP-COMPLETENESS-SWEEP.md`):

```bash
target/release/sekiro-rs.exe --render-gate --map m11_00_00_00 --sweep-out target/render-gate
target/release/sekiro-rs.exe --render-gate --map m11_01_00_00 --sweep-out target/render-gate
```

Then run the live game the way the user plays it: launch from the menu or with `--map m11_00_00_00 --story genichiro-defeated`, wait for the loading screen to end, walk and grapple.
Record time to ready, settled fps, and that the ground and nearby pieces are textured from the first frame.
Use the live-game test from `docs/MAP-COMPLETENESS-SWEEP.md` once it exists.
A PR whose live run is worse than `main` is not ready, whatever the still-image gates say.

## PR body

Keep it short:

```markdown
## What
One or two sentences on the change and why.

## Checks
- fmt, clippy, tests: pass (N passed)
- Render gates: pass / not affected
- Live game: ready in Xs, Y fps settled, ground textured at spawn

## Before / after
Paths under D:\sekiro-rs-media\progress\<date>\ (images stay off the repo).

## Open
Anything left undone, or "None".
```

Do not paste game assets or personal paths beyond the media folder reference.
After opening, add the PR link to the matching row on the collab board (`sekiro-collab/WORK.md`).
