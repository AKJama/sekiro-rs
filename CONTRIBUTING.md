# Contributing

## Before you start (what you need)

You need Windows, your own legitimately bought Steam copy of Sekiro v1.6.0.0, Rust (stable, MSVC), the Visual Studio C++ build tools, and about 50 GB free.
Follow the one-time setup in [docs/GETTING-STARTED.md](docs/GETTING-STARTED.md).
Read the [README disclaimer](README.md#disclaimer) before starting.

## Pick an issue

Any open issue without a linked PR is free to take; `help wanted` marks ones that suit newcomers.
Claim it by opening a draft PR with `Fixes #N` in its body before you write any code, so everyone can see it's taken; you don't need to wait for anyone.
The [project board](https://github.com/users/AKJama/projects/2) shows what is free (Todo), claimed (In Progress) and waiting for review.
One person works on each issue, so skip issues that already show a linked PR, and check the [collaboration board](https://github.com/AKJama/sekiro-collab/blob/main/WORK.md) for work the maintainers have claimed.
A draft PR with no new commits for 7 days frees the issue again.
For something with no issue yet, have your agent file one first (the repo's `issues` skill), then open the draft PR.
Mark the PR ready for review when it's done.
PRs that don't link an open issue are closed.

## Set up with your agent

Fork `AKJama/sekiro-rs`, clone your fork, and open the folder in Claude Code desktop or CLI, Codex, or your agent.
The repo carries its own agent rules and skills in [AGENTS.md](AGENTS.md), [pr-flow](.claude/skills/pr-flow/SKILL.md), [issues](.claude/skills/issues/SKILL.md), and [live-test](.claude/skills/live-test/SKILL.md) for your agent to read and follow.
Paste this prompt, replacing `N` with the issue number:

```text
Read AGENTS.md, CONTRIBUTING.md and .claude/skills/pr-flow/SKILL.md.
I want to work on issue #N in AKJama/sekiro-rs.
Check it has no linked PR, make a branch and open a draft PR against AKJama/sekiro-rs main with "Fixes #N" to claim it.
Then fix it following the pr-flow skill, run its checks, and tell me what to test in the game.
```

## The flow

One PR fixes exactly one issue, and the `issue-link` check fails a PR that closes more than one.
Keep each PR to one feature or fix that can be played and judged on its own.
A big area (a whole menu system, all of parkour) becomes several issues first, for example "equip menu for prosthetic tools" and "ledge grab", and then one PR each.
Use one branch per PR, named `<area>/<short-kebab-name>`, made from the latest `main`.
Follow the [pr-flow skill](.claude/skills/pr-flow/SKILL.md), including code review, simplify, and the full step 4 checks:

```powershell
$env:CARGO_TARGET_DIR = "target/agent"
cargo fmt --all --check
cargo clippy --workspace --all-targets --release -- -D warnings
cargo test --release --workspace
```

For rendering changes, also build the release binary and run both render gates listed in step 4.
Play the game yourself following the live-test skill and describe what you saw.
Use the PR template and include `Fixes #N` in the PR body.

Before marking the PR ready for review, bring it up to date with `main` and rerun the checks:

```powershell
git remote add upstream https://github.com/AKJama/sekiro-rs.git  # once
git fetch upstream
git rebase upstream/main
git push --force-with-lease
```

Resolve any conflicts on your side; `main` only accepts branches that are up to date with it.
Small PRs rarely conflict, which is another reason to keep them to one issue.

## Review and merge

You do not need to be a collaborator: push the branch to your fork and open the PR against `AKJama/sekiro-rs` `main`.
Only the maintainer, AKJama, merges into `main`; branch rules enforce this, and every PR needs the maintainer's approval.
The maintainer reviews the code and plays the game to compare before and after.
Expect change requests.
PRs are squash-merged.

## Hard limits

- Never commit game files, anything from `cache/` or `re/`, extracted data, decompiled code, or analysis databases.
- Do not include personal paths.
- Attach screenshots and videos in the PR conversation, not in the repo.
- Single-player and offline only; nothing touching online play or anti-cheat.
- Do not commit `.env` files or keys.

## AI use

AI agents are welcome, but you are responsible for the result.
You must have run the checks and played the change yourself.
Say which agent and model you used in the PR.

## Licence

Contributions are licensed under the repo's [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) dual licence, same as the rest of the code.

## Coordination

The maintainers maintain the shared board, [AKJama/sekiro-collab](https://github.com/AKJama/sekiro-collab) (`WORK.md`).
Contributors coordinate on the issue itself, not the board.
