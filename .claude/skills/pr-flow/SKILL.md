---
name: pr-flow
description: How changes reach main in sekiro-rs - work only on a filed issue, one branch and PR per issue, then code-review, simplify, local checks with a live-game run, the user's playtest, then merge. Use whenever filing a bug, starting a job, committing, pushing, opening a PR or finishing a job in this repo.
---

# PR flow

Nothing is committed straight to `main`.
Work starts from an issue, and every change goes through its own branch and pull request, in this order:

0. Start from a filed issue.
1. Write the PR.
2. Run the `code-review` skill at medium effort and collect its findings.
3. Run the `simplify` review and collect its findings, then apply both sets in one fix pass.
4. Run the full checks, once.
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

## Outside contributors

- Work from your fork and open the PR against `AKJama/sekiro-rs` `main`.
- Claim the issue by opening a draft PR with `Fixes #N` before writing code, instead of a board row; mark it ready for review when steps 1 to 4 are done.
- One issue per PR, never several: if the work covers more (a whole menu system, all of parkour), file one issue per playable piece with the `issues` skill and make one PR each.
- Branch from the latest upstream `main`, and rebase on it just before step 4 (`git fetch upstream`, `git rebase upstream/main`), so the full checks run once on the final base; resolve conflicts and push with `--force-with-lease`.
- Commit under your own name; the AKJama identity rule is for the maintainer.
- Skip the collab-board steps.
- Run step 4 and play the game yourself, and describe what you saw in the PR; step 5 becomes the maintainer's review and playtest.
- Stop at step 5: only the maintainer merges, so step 6 is not yours.
- See [CONTRIBUTING.md](../../../CONTRIBUTING.md) for the contribution rules.

## Reviewing a contributor's PR (maintainer)

1. Check it links an open issue and stays on that one issue.
2. Run `code-review` at medium effort on it, and check out the branch (`gh pr checkout <N>`).
3. Run the step 4 checks once, then build a playtest binary and play before and after.
4. Request changes on the PR, or merge with step 6.

## 0. Start from an issue

Every PR works on an existing issue; file or pick one with the [`issues`](../issues/SKILL.md) skill first.
If a job turns up a second problem, file it as a new issue instead of widening the PR.

## Agents

- Use Claude Opus 5.5 at medium effort for the main session and for sub-agents; it is the model that has worked reliably here.
- Claude Sonnet 5.5 sub-agents only as a fallback, and Codex only for trivial, mechanical jobs directed by a Claude session.
- High-priority issues go to the most capable model available (Opus 5.5 or newer).
- Only one agent runs the game at a time, and at most two game windows are open in total (the user may have one).
- Agents share one Cargo build folder (`CARGO_TARGET_DIR=target/agent`) instead of one each; every full build is several GB.
- Agents do not commit; the main session reviews and commits their work.
- A command an agent is denied permission for is reported back, never worked around.
- Each agent brief sets a time budget (about 30 minutes) and puts cheap, likely wins first; the agent stops and reports when it hits the budget.
- Changing Cargo profiles, debug info or the linker recompiles all of Bevy, so an agent tries at most one such setting unless the user approves more.

## Testing while working

Builds and tests are most of a PR's time, so the implementing agent tests as little as it can and the full checks run once, at step 4.

- The implementing agent writes the code and its tests, and checks that it compiles.
- New tests stay within the budget in [AGENTS.md](../../../AGENTS.md#tests): usually 1 to 3, each failing without the change.
- It runs only the new tests and the tests in the files it touched, once, by name filter.
It does not run the full suite, and does not rerun tests while iterating.
- When it only needs to know whether something works in the game, it builds the release binary once and stops.
It reports 1 to 3 specific things for the user to try, and the user playtests; the user is quicker than an agent game run.
- The main session runs the full checks (step 4) once at the end: fmt, clippy, the workspace tests, the three render gates and the live test.

The fast commands, from PowerShell in the repo root:

```powershell
$env:CARGO_TARGET_DIR = "target/agent"
# Does it compile (no codegen, fastest):
cargo check -p sekiro-sim --all-targets --release
# One sim integration test file, or one test, by name filter:
cargo test --release -p sekiro-sim --test it object_collision::
cargo test --release -p sekiro-sim --test it -- --exact duel::deflect_chain_depths_send_back_posture
# One game unit test module:
cargo test --release -p sekiro-game --bin sekiro-rs map::world::tests::
# The release binary for a playtest:
cargo build --release -p sekiro-game --bin sekiro-rs
```

Use the release profile for tests: the sim tests step real fights and are slow unoptimised, and sharing the release build with the playtest binary avoids a second full build of Bevy.
Measured on 2026-10-16 (16 threads), after touching one sim file: release binary about 95 s, `cargo test --release --workspace --no-run` about 2 minutes.
The whole workspace test run takes about 2.5 minutes; most of it is the sim `it` binary (about 90 s).
So build the test binaries once and filter, instead of rebuilding for each run.
Data loaded from `cache/` that never changes, such as the enemy registry, is read once per process and shared, so tests that load a duel or an arena stay cheap.
Keep new shared data read-only; a test that changes state loads or clones its own.

## 1. Write the PR

Create the branch, commit the change, push, and open the PR with the body from [`.github/pull_request_template.md`](../../../.github/pull_request_template.md), filling in every section.
The `issue-link` check fails a ready PR whose body misses a section, leaves one empty or keeps a template line, so open it as a draft, fill in How it was tested as steps 2 to 4 finish, then mark it ready.
Screenshots stay off the repo: attach them in the PR conversation, or for maintainers give the path under `D:\sekiro-rs-media\progress\<date>\`.
Do not paste game assets or personal paths beyond the media folder reference.
Add the PR link to the matching row on the collab board (`sekiro-collab/WORK.md`).
When the PR merges, move that row to Done with the PR link.

## 2. Code review

Run the `code-review` skill on the PR's diff at medium effort (`/code-review medium`), never higher, to find correctness bugs.
Collect the findings that hold up; do not fix them yet.

## 3. Simplify, then one fix pass

Run the `simplify` review on the same diff for reuse, simplification, efficiency and altitude cleanups, and collect its findings too.
Then apply the code-review and simplify findings together in one pass (one agent, or the main session), checking that it compiles and running only the tests in the files it touched, and push the result.
Keep simplify to cleanups of the diff: a deeper refactor it suggests becomes its own issue instead of growing the PR.

`code-review` and `simplify` are built into Claude Code.
Every PR must run both in Claude Code, `/code-review` at medium and then `/simplify`, on its final diff, whatever agent wrote the code.
A review done by hand or by another agent (Codex and others) does not count, and a PR without both is sent back.

## 4. Checks

Run these once, last, on the final state of the branch, from the repo root.
Agents that worked on the branch do not run them as well.
They need the installed game and `cache/`, so they run locally, not in GitHub Actions.

```powershell
$env:CARGO_TARGET_DIR = "target/agent"
cargo fmt --all --check
cargo clippy --workspace --all-targets --release -- -D warnings
cargo test --release --workspace
```

The workspace tests include the data gate.
For any change that can affect rendering, also build the release binary and run the three render gates, two noon and one opening (see `docs/MAP-COMPLETENESS-SWEEP.md`):

```bash
target/release/sekiro-rs.exe --render-gate --map m11_00_00_00 --sweep-out target/render-gate
target/release/sekiro-rs.exe --render-gate --map m11_01_00_00 --sweep-out target/render-gate
target/release/sekiro-rs.exe --render-gate --map m11_00_00_00 --gate-fixture opening --sweep-out target/render-gate
```

Then test the change in the live game with the [`live-test`](../live-test/SKILL.md) skill: ask the user to playtest first, and if an agent runs it, wait for the full load, steer with a route and prove the target was hit.
Record time to ready, settled fps, and that the ground and nearby pieces are textured from the first frame.
A PR whose live run is worse than `main` is not ready, whatever the still-image gates say.
If a check fails, fix it on the branch and run the checks again.

## 5. The user plays it

For any PR that changes game code, the user's playtest is the final gate.
Point them at the "What to look out for" section and the build to run, as the [`live-test`](../live-test/SKILL.md) skill describes.
If they ask for changes, make them on the same branch (or rebase the stack), rerun steps 2 to 4, and ask again.

## 6. Merge (maintainer only)

Merge only when the checks pass and the user confirms.
Then, every time:
1. Squash-merge the PR; `Fixes #N` closes the issue and moves it to Done on the project board.
2. GitHub deletes the merged branch; delete it locally too (`git branch -D <branch>`), and any stacked branch that merged with it.
3. Pull `main`, and rebase any open stacked branches onto it.
4. Update the shared board (`sekiro-collab/WORK.md`): move the claim row to Done with the PR link and today's date, bump the Done count, commit as AKJama and push.
5. Clear the job's build folder if nothing else uses it, so old builds don't pile up on disk.
