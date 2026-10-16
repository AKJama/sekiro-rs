---
name: pr-flow
description: How changes reach main in sekiro-rs - one feature per branch and PR, then code-review, simplify, local checks with a live-game run, then merge. Use whenever committing, pushing, opening a PR or finishing a job in this repo.
---

# PR flow

Nothing is committed straight to `main`.
Every change goes through its own branch and pull request, in this order:

1. Write the PR.
2. Run the `code-review` skill.
3. Run the `simplify` skill.
4. Run the checks.
5. The user plays the game and confirms (for PRs that change game code).
6. Merge.

## Rules

- One feature or fix per PR.
If a job turns up a second problem, note it and open a separate PR for it.
- One branch per PR, named `<area>/<short-kebab-name>`, for example `map/ground-visibility` or `perf/load-time`.
- PRs may be stacked: branch from an open PR's branch and set that branch as the base.
When a lower PR changes or merges, rebase the stacked branches onto it and force-push with `--force-with-lease`.
- Commit as AKJama <92124781+AKJama@users.noreply.github.com>, with no co-author lines.
- Stage explicit file paths only, never `git add -A` or `git add .`, because agent work may be sitting in the tree.
- Agents do not commit, push or open PRs; the main session does after reviewing their work.
- No git worktrees.

## 1. Write the PR

Create the branch, commit the change, push, and open the PR with a short body:

```markdown
## What and why
In plain words: what this tries to implement and why.

## What to look out for when playing
Where to go and what to check, for example "load into the Outskirts: the loading bar should finish with the ground already textured".

## Checks
- code-review: N findings, M fixed
- simplify: done
- fmt, clippy, tests: pass (N passed)
- Render gates: pass / not affected
- Live game: ready in Xs, Y fps settled, ground textured at spawn

## Before / after
Paths under D:\sekiro-rs-media\progress\<date>\ (images stay off the repo).

## Open
Anything left undone, or "None".
```

Fill in the Checks lines as steps 2 to 4 finish, and edit the PR body to match.
Do not paste game assets or personal paths beyond the media folder reference.
Add the PR link to the matching row on the collab board (`sekiro-collab/WORK.md`).

## 2. Code review

Run the `code-review` skill on the PR's diff (for example `/code-review medium`) to find correctness bugs.
Fix the findings that hold up and push them to the same branch.

## 3. Simplify

Run the `simplify` skill on the diff for reuse, simplification and efficiency cleanups, and push the result.

If either skill is not installed on this machine, say so in the PR body rather than skipping it silently.

## 4. Checks

Run these last, on the final state of the branch, from the repo root.
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
If a check fails, fix it on the branch and run the checks again.

## 5. The user plays it

For any PR that changes game code, the user's playtest is the final gate.
Point them at the "What to look out for" section and the build to run (copy the branch's release binary to `target/play/sekiro-rs.exe`).
If they ask for changes, make them on the same branch (or rebase the stack), rerun steps 2 to 4, and ask again.

## 6. Merge

Merge only when the checks pass and the user confirms.
Use squash merge, then delete the branch and pull `main`.
