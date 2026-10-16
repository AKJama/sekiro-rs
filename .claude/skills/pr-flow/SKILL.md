---
name: pr-flow
description: How changes reach main in sekiro-rs - work only on a filed issue, one branch and PR per issue, then code-review, simplify, local checks with a live-game run, the user's playtest, then merge. Use whenever filing a bug, starting a job, committing, pushing, opening a PR or finishing a job in this repo.
---

# PR flow

Nothing is committed straight to `main`.
Work starts from an issue, and every change goes through its own branch and pull request, in this order:

0. Start from a filed issue.
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

## 0. Start from an issue

Every PR works on an existing issue; file or pick one with the [`issues`](../issues/SKILL.md) skill first.
If a job turns up a second problem, file it as a new issue instead of widening the PR.

## Agents

- Use Claude Opus 5.5 at medium effort for the main session and for sub-agents; it is the model that has worked reliably here.
- Claude Sonnet 5.5 sub-agents only as a fallback, and Codex only for trivial, mechanical jobs directed by a Claude session.
- High-priority issues go to the most capable model available (Opus 5.5 or newer).
- Only one agent runs the game at a time, and at most two game windows are open in total (the user may have one).
- Agents share one Cargo build folder (`CARGO_TARGET_DIR`) instead of one each; every full build is several GB.
- Agents do not commit; the main session reviews and commits their work.
- A command an agent is denied permission for is reported back, never worked around.

## 1. Write the PR

Create the branch, commit the change, push, and open the PR with a short body:

```markdown
## What and why
Fixes #N.
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
When the PR merges, move that row to Done with the PR link.

## 2. Code review

Run the `code-review` skill on the PR's diff (for example `/code-review medium`) to find correctness bugs.
Fix the findings that hold up and push them to the same branch.

## 3. Simplify

Run the `simplify` skill on the diff for reuse, simplification and efficiency cleanups, and push the result.

`code-review` and `simplify` are built into Claude Code.
Other agents (Codex and others) do the same two passes by hand: a correctness review of every hunk, its enclosing function and its callers, then a cleanup pass for reuse, redundant state and wasted work.
If neither is possible, say so in the PR body rather than skipping it silently.

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

Then test the change in the live game with the [`live-test`](../live-test/SKILL.md) skill: ask the user to playtest first, and if an agent runs it, wait for the full load, steer with a route and prove the target was hit.
Record time to ready, settled fps, and that the ground and nearby pieces are textured from the first frame.
A PR whose live run is worse than `main` is not ready, whatever the still-image gates say.
If a check fails, fix it on the branch and run the checks again.

## 5. The user plays it

For any PR that changes game code, the user's playtest is the final gate.
Point them at the "What to look out for" section and the build to run, as the [`live-test`](../live-test/SKILL.md) skill describes.
If they ask for changes, make them on the same branch (or rebase the stack), rerun steps 2 to 4, and ask again.

## 6. Merge

Merge only when the checks pass and the user confirms.
Then, every time:
1. Squash-merge the PR (`gh pr merge <N> --squash`); `Fixes #N` closes the issue.
2. Delete the branch on GitHub and locally (`git push origin --delete <branch>`, `git branch -D <branch>`), and delete any stacked branch that merged with it.
3. Pull `main`, and rebase any open stacked branches onto it.
4. Update the shared board (`sekiro-collab/WORK.md`): move the claim row to Done with the PR link and today's date, bump the Done count, commit as AKJama and push.
5. Clear the job's build folder if nothing else uses it, so old builds don't pile up on disk.
