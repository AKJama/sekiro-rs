# Group d: Mibu and Fountainhead arenas

The reviewed rows use the original local MSB placements and `cache/raw/event/<map>.emevd` scripts.
No cache files, research files, game files, saves, git state or existing logs were changed.
These are staging changes, with boss attack fidelity outside this task.
Graphical completion is blocked by the missing character model extracts listed below, so none of these arenas is claimed fully DONE.

## Mibu bridge: `mibu-bridge`

Map is `m15_00_00_00`, boss entity is 1500800, and boss part is `c5000_0000`, NpcParam 50001000.
Wolf starts at the authored player part `c0000_0004`, and collision is cropped within 60 m of the boss unless full-map loading is requested.
The placement is the ghost Monk encounter named by entity 1500800, even though the bridge name also refers to other Mibu encounters.
Event 11505849 starts common event 20005820 for fog asset 1501801, effect 12, start flag 11505801 and defeat flag 9306, with no paired 20005825 collision.
Start flag 11505801 is seeded for a fight already under way, and victory clears the gate through 9306.
No compulsory earlier time boss or SetLighting override exists in this fight's entry event, so this row stages the valid early-story morning visit.
Later story visits can occur at other times as earlier boss defeats advance the shared story clock.
The selected events are 11505810 from Label0, 11505800, 11505812 and 11505814, covering entry, boss bar, immortality, defeat, finishing request and illusion-effect cleanup.
There is no reviewed alternate map floor or door state to hide for this encounter.
The headless test checks MSB placement, supporting collision, the script's boss bar and the defeat-controlled fog lifecycle, retaining 153787 cropped collision triangles.
Visible staging is blocked by missing `cache/models/c5000.glb`.
Boss attacks and the finishing HKS response are not claimed verified by the staging smoke.
The numeric snap-seed counter in 11505814 still uses unsupported event bitfield instructions, so its illusion-dispel threshold is not claimed implemented.

## Fountainhead bridge: `fountainhead-bridge`

Map is `m25_00_00_00`, boss entity is 2500850, and boss part is `c5000_0000`, NpcParam 50000000.
Wolf starts at player part `c0000_0002`, within the fight's region 2502850 and both fog planes, with a 70 m collision radius.
Player part `c0000_0001` lies outside the entrance fog and would strand Wolf outside an already-started fight.
Event 12505999 starts two common 20005820 walls on assets 2501850 and 2501851, effects 10 and 11, start flag 12505951 and defeat flag 9309, with no paired 20005825 boundary.
The row seeds defeat prerequisites 9303 through 9306 and 9308, cumulative story-time flags 9800 through 9802, and the active-fight flag 12505951.
Story time is night, while the map retains its authored Fountainhead sky.
Events 12505990 and 12505960 from Label0 initialise and show the boss bar, and 12505961 and 12505962 set the second and final phase flags.
Events 12505963 through 12505965 control the finishing sequence, and 12505950 controls defeat and fog removal.
Events 12505970, 12505980 through 12505984 retain the phantom placement, visibility and presentation requests, including all three slots of 12505970 and 12505984.
Supporting actors 2500851, 2500852, 2500853 and 2500854 are parts `c5000_0001` through `c5000_0004`, and enabling them does not replace the real Monk.
The renderer retains their MSB placements and applies event enable, animation, mask and region-warp state separately from the duel opponent.
The headless test checks placement, collision, named boss bar, both fog lifecycles and the phase-two phantom enables without an opponent hand-over, retaining 115491 cropped collision triangles.
Visible staging is blocked by missing `cache/models/c5000.glb`.
Simultaneous phantom combat, AI-command-driven phantom attacks and the finishing attachment to dummy 260 remain unsupported by the single-opponent simulation.
Those gaps prevent claiming full illusion fidelity or a complete fight-through.
The map instruction 2003[59] in 12505990 and the special camera requests remain unimplemented.

## Fountainhead sanctuary: `fountainhead-sanctuary`

Map is `m25_00_00_00`, and the opening opponent is the Old Dragons boss-bar entity 2500810 at part `c5300_0025`, NpcParam 53000000.
Divine Dragon is entity 2500800 at part `c5200_0000`, NpcParam 52000000.
The visible Old Dragons cast includes entities 2500811 through 2500835 at parts `c5300_0000` through `c5300_0024`, with entity 2500840 at `c5320_0000` serving the script's aggregate character.
Wolf's story start is the original MSB warp region 2502821, used by event 12505801, because none of the exported m25 player parts is at the sanctuary elevation.
The transition uses the original MSB region 2502824 from event 12505885, and collision is cropped within 150 m of the opening boss unless full-map loading is requested.
Event 12505802 starts common 20005830 for the encounter, but neither it nor transition event 12505886 places a 20005820 or 20005825 fog wall.
The row seeds prerequisites 9303 through 9306, 9308 and 9309, story flags 9800 through 9802 and sanctuary state 9200, keeping defeat 9310 off.
Event 12505801 sets lighting 2 for the Old Dragons, and event 12505885 sets lighting 1 for Divine Dragon, overriding the preceding story night.
Event 12505801 also swaps sky groups 8907070 and 8907090, so the renderer selects the extracted cloud-sea sky `m900090` and hides the ordinary Fountainhead cloud layer.
The opening event set is 12505801 from Label0, 12505803, 12505834, 12505837, 12505840, 12505841, 12505850, 12505851, 12505870, 12505871, 12505879, 12505880 and 12505885.
All placed slots of 12505803, 12505834, 12505841 and 12505851 are instantiated.
The Divine Dragon row joins 12505885 at Label20 and runs 12505880, 12505887 through 12505890, 12505895, 12505901 and 12505906, including every placed tree-prop slot.
The sanctuary's `o258*` and `o259*` set pieces load automatically without the general objects option, and map events control their visibility and placed effects.
Supporting Old Dragons, aggregate characters and tree helpers retain their own MSB transforms instead of being sequentially substituted for the primary opponent.
The transition's Divine Dragon boss-bar command provides the sequential opponent hand-over, with Wolf's cinematic warp and morning lighting retained.
The headless test checks the elevated hit floor, region-based start, absence of invented fog and both authored lighting choices, retaining 745557 cropped collision triangles.
Its transition check uses the script's explicit retry flag 12505882 instead of pretending to defeat a simultaneous cast through one Duel.
Visible staging is blocked by missing `cache/models/c5200.glb`, `cache/models/c5300.glb`, `cache/models/c5310.glb` and `cache/models/c5320.glb`.
The map geometry, sanctuary object GLBs, original MSB, sky and animation extracts are present.
Aggregate Old Dragons HP and death counting, simultaneous combat, attached animated tree props, attack-driven lightning and grapple interactions, and native cutscene playback remain unsupported.
The cinematic boundary uses the existing timed fade placeholder, and the staging tests do not claim natural Old Dragons defeat or Dragon combat fidelity.

## Verification

All build and test Cargo commands use `CARGO_TARGET_DIR=target/alt-d` and four build jobs.
`cargo test --release -p sekiro-sim --test it -j 4 arena_d_ -- --nocapture` passed all three tests in 6.98 seconds after the final scene-retention changes, with no skips or ignored cases.
Two earlier attempts were blocked by another arena agent's stale `scripts.scene` reference in `arenas_a.rs`, which that agent subsequently corrected.
`cargo clippy --workspace --all-targets -j 4 -- -D warnings` passed, with its final invocation finishing in 18.41 seconds.
`cargo build --release -p sekiro-game -j 4` passed, with the final invocation finishing in 2 minutes 15 seconds.
Both checks were retried after concurrent compile errors and Clippy diagnostics were corrected.
The transition regression also proves that the cloud-sea selection survives the opponent replacement and the Old Dragons backread group is removed.
The final `cargo fmt --all --check` reported only the intentionally appended module order in shared `tests/it/main.rs`, so the module table was not reordered.
Only the new group-d Rust files were formatted directly with `rustfmt --edition 2024`.
The read-only collision audit used `uv run --no-project --offline --no-sync -m probe` in `target/alt-d` with `UV_CACHE_DIR` inside that directory, and confirmed floors at all six MSB starts.
The initial uv attempt could not access the default cache, so the retry used the workspace-local cache without downloads or installs.
The read-only region audit used `uv run --no-project --offline -m regions_probe` with the same workspace-local uv cache.
Event research used `cargo run --release -p sekiro-formats --example emevd_dump -j 4 -- cache/raw/event/m15_00_00_00.emevd --find 1500800`, then the resulting `target/alt-d/release/examples/emevd_dump.exe` against the two local map scripts and the event IDs documented above.
No graphical run was attempted because the required character GLBs are absent, and no screenshot was taken.

## Files changed

Group-d arena definitions and warp resolution are in `crates/sim/src/arenas/depths_palace.rs`, with additive registration and loading hooks in `crates/sim/src/arena.rs`.
Presentation directives are in `crates/sim/src/arenas/scene.rs`, with additive state, event-slot and completion-flag hooks in `crates/sim/src/events.rs`.
`crates/sim/src/duel.rs` retains supporting actors, applies authored primary region warps and performs the Old Dragons to Divine Dragon boss-bar hand-over.
`crates/game/src/arena_d_scene.rs`, its registration in `crates/game/src/play.rs` and the event sky override in `crates/game/src/sky.rs` provide the rendering hooks.
Other arena agents extended the shared scene hooks during this session, and their additions were preserved.
The tests are `crates/sim/tests/it/arena_d.rs`, `mibu_bridge.rs`, `fountainhead_bridge.rs` and `fountainhead_sanctuary.rs`, registered additively in `main.rs`.
Documentation changes are this file and the group-d session entry in `STATUS.md`.
