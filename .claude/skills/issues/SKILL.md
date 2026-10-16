---
name: issues
description: Capture problems in sekiro-rs as prioritised GitHub issues - from the user's playtest notes, a review or an investigation. Use whenever the user reports something they saw in the game, or a job turns up a problem outside its scope.
---

# Issues

The backlog lives in GitHub issues on `AKJama/sekiro-rs`, one issue per job.
Work only starts from an issue; the [`pr-flow`](../pr-flow/SKILL.md) skill takes it from there.

## Every issue, every time

Every issue gets exactly two labels when it is filed: `bug` or `enhancement`, and one of `priority: high`, `priority: medium`, `priority: low`.
This holds for side issues found during other work as well as for the user's reports.
File with both labels in the same command, never as a later step:

```powershell
gh issue create --title "<what is wrong, in player terms>" --label bug --label "priority: medium" --body-file <body.md>
```

After filing, check that no open issue is missing a label: `gh issue list --limit 200 --json number,labels --jq '.[] | select((.labels | length) < 2) | .number'` must print nothing.

## When the user reports problems

The user plays the game and describes what they saw, often several things at once.
1. Split the report into separate problems, one per issue.
2. Check each against the open issues (`gh issue list`) and add to an existing one instead of duplicating it.
3. Look before filing: find the likely cause in the code, data or docs when it is quick to do, and say plainly when it is a guess.
4. Set a priority from what the user said and your own judgement, and tell the user which you chose and why:
   - `priority: high`: blocks play or progress (falling through the world, invisible world, crashes, broken controls), or is obvious on screen everywhere.
   - `priority: medium`: clearly wrong but play goes on (a wrong-looking material, a slow transition).
   - `priority: low`: noticeable on a close look, or likely to be hidden by unfinished work (distant pop-in, occasional frame drops).
   If the user calls something low priority or "later", follow them.
5. File the issues with both labels (see above) and give the user the links.

## What an issue holds

- Title: what is wrong, in player terms.
- Labels: `bug` or `enhancement`, plus one priority label.
- Body sections:
  - Problem: what the player sees, where (map, coordinates, how to get there), and since when if known.
  - Likely cause: the evidence so far (file:line, data, exe addresses), or "unknown".
  - To do: the steps a worker should take, starting with reproducing it in the live game.
  - Done when: how a fix is checked, including the live game and a test that fails on regression.
- No game assets, screenshots of game content or personal paths; describe images in words and keep the files off the repo.

## Picking and claiming

- Take the highest-priority open issue that nobody has claimed.
- High-priority issues go to the most capable model available (Claude Opus 5.5 or newer); medium and low ones can go to any capable worker.
- An issue is taken when it already has a linked PR (draft or not), or a row in Claimed on the collab board; skip those.
- Maintainers claim it on the collab board (`sekiro-collab/WORK.md`) with a link to the issue before starting.
- Outside contributors claim it by opening a draft PR with `Fixes #N` before writing code (see `CONTRIBUTING.md`); a draft with no new commits for 7 days frees the issue.
- `STATUS.md` stays a short snapshot of what works and what is in progress; it is not the to-do list.
