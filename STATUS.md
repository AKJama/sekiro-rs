# Status

Last updated 9 October 2026.
Read this first in a new session, then `AGENTS.md` and `docs/DESIGN.md`.

## Where things stand

| Area | State | Docs |
|---|---|---|
| Archives, formats | Whole install unpacks to `cache/raw` in under a minute | `docs/FORMATS.md` |
| Models | Wolf (c0000 + parts) and Ashina soldier (c1010) export as textured, skinned GLB; enemy weapons via draw masks | `docs/FORMATS.md` |
| Animation | All c0000/c1010 clips decode; TAE parsed and decoded; clip aliases resolved | `docs/FORMATS.md` |
| Params | All 186 tables load from Paramdex defs; typed combat tables | `docs/PARAMS.md` |
| Player behaviour | Real `c0000.hks` runs in our HKS VM against the real behaviour graph; TAE cancel windows, blending, lock-on | `docs/HKS-INTERFACE.md`, `docs/HKS-COMBAT.md` |
| Movement | Run, sprint, jump (running jump fixed), step, crouch; root motion; walks the real m11 map with floor/wall collision | `docs/FORMATS.md` (Walking on the map) |
| Combat | Engine rules for hit/block/deflect, posture both ways, breaks, recovery; HP/posture HUD | `docs/COMBAT-RULES.md` |
| Deathblows | Real ThrowParam pairs; posture break -> deathblow -> death; Wolf death and respawn | `docs/DEATHBLOW.md` |
| Enemy | Soldier runs its own HKS script, behaviour graph and real Lua AI (Lua 5.0 VM + goal runtime) | `docs/AI.md` |
| Map | m11_00_00_00 (Ashina Outskirts and Castle): pieces, textures, normal maps, collision | `docs/FORMATS.md` (Maps) |

## How to run

```powershell
./tools/fetch-refs.ps1
cargo run --release -p sekiro-extract -- unpack
cargo run --release -p sekiro-extract -- models
cargo run --release -p sekiro-extract -- anims c0000 c1010
cargo run --release -p sekiro-extract -- params
cargo run --release -p sekiro-extract -- map m11_00_00_00
cargo run --release -p sekiro-game -- --play --map m11_00_00_00 --start 3
```

Useful flags: `--play` alone uses a flat floor; `--lock-on`, `--no-enemy`, `--simple-ai`, `--script FILE --shots step:path --exit` for scripted runs with screenshots; `--arena --deathblow` for the deathblow demo; `--viewer <glb>` and `--map <id>` for inspection.

## In progress when the session paused

- Grappling hook, ledge hang, Mikiri counter and perilous attacks (player/enemy behaviour).
- Fall death and respawn, FLVER tangents to cut the 14 s map load, map lighting from the game's draw params, foliage alpha.
- Audio: FSB/FEV decoding, TAE sound cues, hit/guard/deflect sounds, footsteps.

Uncommitted files in the working tree belong to these tasks; check `git status` and build before committing.

### Leads for grapple, ledges and Mikiri (found, not built)

- Ledges: `map/<id>/<id>.edgebhd/.edgebdt` hold one `.edge` file per collision part (16 files, 255 edges on m11). Header `u32 version=4, u32 count, u32 collision number, u32 0`, then 64-byte records: two vec4 endpoints, a zero vec4, `u32 chain id`, `u32 flags` (low byte 2 or 3, second byte 0..3 or 10, meaning unknown), 8 bytes padding. Endpoints are in the collision part's local space; apply its layout transform. `.wallbhd/.wallbdt` is likely wall-hug data, undecoded.
- Grapple: map pieces have no dummies; object `o000100` (placed 67 times on m11) carries dummy 90 and is very likely the grapple point. Search rules: WirePointSearchParam row 0 (act 3 to 15 m, find to 25 m, filter and soft-lock boxes); WireSetParam picks search rows per area; WireVariationParam (angles, marker type, homing offset) is probably set per placed object in MSB fields the parser still skips.
- Mikiri: find the soldier's perilous thrusts (AtkParam rows that disable guarding) and the warning effect; the counter likely uses an attack-cancel window plus a ThrowParam row, wired like deathblows in `crates/sim/src/duel.rs`.
- Study tools: `crates/sim/examples/bxf_list.rs`, `dummy_census.rs`.

## Decision needed from the user

- 62 of 174 sound banks (`sm*`, `smain`, `vm*`, `xm*`, `rm*`: footsteps, most hit sounds, voices, music) are encrypted FSB. Reading them means decrypting with a key from the game. Ask before touching them; without them, decode the FEV event-record index next (see `docs/AUDIO.md`).

## Known issues

- Wolf's max HP/posture come from CalcCorrectGraph at progression level 1; the real progression input is not yet traced.
- Player HP damage formula is inferred (weapon attack power x correction), not traced.
- Single-clip rendering: layered upper/lower-body states show one clip.
- Lock-on range (15 m) and parry-timing input to the AI are approximations.
- Shading is plain PBR; map lighting is hand-picked; 3 map textures from other areas are missing.
- Deathblow needs lock-on to meet the angle limit.

## Next steps

1. Finish the three in-progress tasks above.
2. Validation against the real game: record posture/HP/animation traces in a real fight (read-only) and compare with `duel-sim` output.
3. More enemies from the m11 layout (43 soldiers placed in the MSB), then a boss-style enemy with perilous attacks.
4. Combat arts, prosthetics and items only after the core loop is solid.

## Publishing

The repo is public at github.com/AKJama/sekiro-rs.
Local git hooks (`.git/hooks/pre-commit`, `commit-msg`) reject personal paths, emails, co-author lines and anything under `cache/` or `re/`; they are not part of the repo, so re-create them on a new clone.
Write commands in docs as plain `cargo`, never a full user path.
