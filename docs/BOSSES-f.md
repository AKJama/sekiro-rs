# Hirata Father combat - group f, 12 October 2026

Both Hirata encounters now host the real Father and a separately simulated bird, including native phase changes and finishing executions.
Neither encounter is claimed fully DONE under the requested every-attack fidelity standard.

## Identity and sources

Story entity 1000900 uses c5060, NpcParam 50601010, think 50600000 and battle goal 506000 from the Hirata luabnd.
Inner entity 1000990 uses the same character and NPC row with think 50630000 and battle goal 506300.
Their shared NPC row supplies 2552 HP, 750 posture, two marks and behavior variation 50600, rather than castle Owl's 50601.
Both use the extracted c5060 skeleton, animations, TAE and model.
Story bird 1000901 uses c5070 and think 50709000, selecting battle goal 507000.
Inner bird 1000991 uses c5070 and think 50709300, selecting battle goal 507300.
The bird's own model, skeleton and animation extract are present, resolving ARENAS-a.md's earlier missing-model warning.
The map scripts are m10_00_00_00 for story Father and m10_00_50_61 for Inner Father, with the physical Hirata map shared.
The HUD resolves the map's FMG names 905061 and 905063 instead of the castle name associated with c5060.
The staged arena and registry wiring is reused, preserving each placed think identity.
The ordinary scripted CLI option retains these native Lua goals because the external c5060 move sheet describes castle Owl.
Read-only FwLo notes enemies.md cross-check the Father NPC row, and deathblow_groups.md cross-checks the shared a221 deathblow family.
All gameplay code is original; no reference code or compiled game script is copied into tracked source.

## Story Father, phase one

The native Lua/HKS arena replay selects thirteen distinct attacks and records seventeen contacts over thirty simulated seconds.
It requires animated attack clips, contacts, the shown bar and two marks, with no HKS script errors or Lua errors.
The first sword fixture spends a mark through ThrowParam pair 15060000/15060001 and plays a000_012000.
Its authored message 20 reaches event 11005912, sets effect 200051 and leads to the bird's appearance.
The get-up restores 2552 HP, with one mark remaining.
The isolated thrust fixture verifies native Mikiri pair 15060190 and ninety posture damage without Wolf losing HP.
Complete guard/deflect, shuriken, firecracker, grab, thrust and sweep coverage against variation 50600 is not independently verified here.
Those attacks reuse the existing Owl hit, bullet, grab and perilous systems and their installed parameter rows.

## Story Father, phase two

The live c5070 runs its own shipped Lua and HKS against Wolf's already advanced pose and combat state.
It receives AI requests, replans, effects and forced animations, while the map reads its messages, effects and region membership.
The replay observes appearance/disappearance clips, native fire-pass action 3004, flight actions and the map's return request.
Events 11005930/31 freeze and hide Father, warp him through the posed bird position and play the falling attack a000_003035.
The final thirty-second combat segment records six contacts, with no unknown bird AI calls or HKS errors.
The final sword fixture spends the remaining mark through pair 15060180/15060181 and observes a000_013700, a000_020200 and Wolf's a000_710201 continuation.
The real map script reaches defeat flag 9317, disables the bird and releases the fog through its existing flag wiring.
Fire-pass damage and its jump counter, complete teleport/grab branches and a continuous unassisted victory are not verified by this fixture.

## Inner Father, phase one

The native 506300 goal selects twelve distinct attacks and records eighteen contacts over thirty simulated seconds, including act 3048.
The normal fixture verifies the shown bar and two marks and rejects Lua or HKS errors.
The first paired deathblow plays the native get-up, restores 2552 HP and enters the one-mark bird phase through the rematch event script.
The isolated thrust fixture verifies pair 15060190, ninety posture damage and no vitality loss to Wolf.
Every Mist Raven, combo, guard, projectile, grab and perilous branch is not covered by these tests.

## Inner Father, phase two

The bird preserves its placed 50709300 think row and runs goal 507300 rather than story bird's goal 507000.
The replay observes native flight actions, appearance/disappearance and Father's a000_003035 return attack.
The thirty-second combat segment records seven contacts and no unknown bird AI calls or HKS errors.
The final sword fixture plays the native finishing pair and continuation and reaches rematch defeat flag 9346.
The bird is disabled after finishing without replacing Father as the primary opponent.
The Inner bird's fire attacks were not selected in this sample, so their damage, horizontal/vertical distinction and counters remain unverified.
Complete Mist Raven coverage and a continuous unassisted victory remain open.

## Fixture limits and approximations

The transition fixture keeps Wolf alive with the existing god option and lowers remaining boss posture to one before real sword contacts.
Both native AIs run for the phase-two combat segment, then their decisions stop and current clips settle before the finishing sword pair is isolated.
The fixture never assigns a defeat flag, reduces boss HP directly or invents a phase timeline.
Its two encounters finish in 18.99 seconds and are ignored by the normal tier because the combined fixture exceeds ten seconds.
An assertion that every short native sample must select the fire pass was removed when Inner Father's legitimate sample selected other actions.
Finite native samples prove the observed branches, not exhaustive attack coverage.
Re-enabling the bird resumes its animation module rather than preserving the dormant setup's freeze from 11005900.
This lifecycle interpretation is inferred from 11005912 enabling the bird and forcing appearance without another animation-enable command; retaining the dormant freeze stalled the entire phase.
Dummy warps use the current posed dummy position and copy source yaw; the native copy-floor field is not hosted.
Native hit-mask toggles for dummy 220 are not hosted, so complete ghost collision/immunity and lock-target fidelity are not claimed.
Effect 3506030 is rendered as a hidden model, consistent with the existing Owl fade fallback.
Map warp-effect shots and production fire/bird effects are not added in this pass.
The final story fixture reports exhausted bounded throw clearance and retains its authored alignment, so collision-perfect finishing placement is not guaranteed.
The sword blocker was reproduced at real arena placement with HKS input traces: reference 110005 cancelled sword actions on shallow Hirata collision despite Wolf not swimming.
That synthetic reference now follows the existing swimming-water predicate; material/depth classification and the step-height depth threshold remain approximations.
No original-game comparison or graphical phase-two/finishing capture is claimed.

## Commands and results

Every Cargo command below uses target/alt-f, offline dependencies and three jobs.
Read-only event dumps reviewed 11005900/10/12/20/21/22/23/30/31/35/36 and the Inner return event.

```powershell
$env:CARGO_TARGET_DIR='target/alt-f'
$env:CARGO_NET_OFFLINE='true'
cargo build --release -p sekiro-formats --example emevd_dump -j 3
cargo run --release -p sekiro-sim --example owl_ai_inspect -j 3 -- cache/raw/script/m10_00_00_00.luabnd.d/507000_battle.lua
cargo test --release -p sekiro-sim --test it -j 3 owl_father -- --nocapture
cargo test --release -p sekiro-sim --test it -j 3 owl_father -- --include-ignored --nocapture
cargo test --release -p sekiro-sim --test it -j 3 owl_father_both -- --include-ignored --nocapture
foreach ($filter in 'owl','genichiro','brain_tactics','attack_power','posture_recovery','swim') {
    cargo test --release -p sekiro-sim --test it -j 3 $filter
}
cargo clippy --workspace --all-targets -j 3 -- -D warnings
cargo build --release -p sekiro-game -j 3
cargo fmt --all --check
rustfmt --edition 2024 --check crates/sim/src/owl_father.rs crates/sim/src/father_bird.rs crates/sim/tests/it/owl_father.rs crates/sim/examples/owl_ai_inspect.rs
```

The formats and Lua-inspection examples build and run successfully without extract writes.
The normal Father run passes two tests with one slow fixture ignored in 8.42 seconds.
The final slow fixture passes both encounter identities and native defeat events in 18.99 seconds.
Earlier attempts exposed the waterside attack blocker, inherited bird freeze and active-vanish interference in the final-sword setup.
The combined include-ignored attempt passed the two normal tests and failed only the overly strict Inner fire-selection assertion, corrected before the final two-encounter run.
Regression filters pass: owl 62 with one ignored, genichiro 15, brain_tactics 19 with one ignored, attack_power 3, posture_recovery 6 and swim 6.
Workspace Clippy fails on both attempts at another agent's concurrent fight.rs:921 hp_damage argument-count lint, and that file is left untouched.
The first release game build passes after 22 minutes 18 seconds of fresh dependency compilation, and the final build passes in 2 minutes 39 seconds with exit zero.
Owned Rust files pass direct formatting checks.
Workspace formatting returns one with shared-file/module-order differences, left untouched under the shared-tree rule.
The preliminary story arena launch exits zero and produces target/alt-f/father-phase1.png.
Both CLI forms exit zero for both encounters with native Lua retained, the correct named bar and two displayed marks.
Inspected representative captures are target/alt-f/hirata-father-arena.png and target/alt-f/hirata-inner-father-play.png.
The other successful captures are target/alt-f/hirata-father-play.png and target/alt-f/hirata-inner-father-arena.png.
These captures and their logs verify first-phase presentation and contacts, not full graphical phase-two or finishing fidelity.
The staged cave/fog presentation and production effects remain outside this pass.
The preliminary scripted launch generated a new timestamped brain-session report; no existing log, memory or dataset file was edited.

```powershell
foreach ($arenaName in 'hirata-father','hirata-inner-father') {
    foreach ($launchMode in 'arena','play') {
        $launchArgs=@('--arena',$arenaName,'--brain-memory','off','--brain','scripted','--script','builtin','--god','--shots',"180:target/alt-f/$arenaName-$launchMode.png",'--exit')
        if ($launchMode -eq 'play') { $launchArgs=@('--play') + $launchArgs }
        & .\target\alt-f\release\sekiro-rs.exe @launchArgs
    }
}
```

## Files changed

New simulation code is crates/sim/src/owl_father.rs and crates/sim/src/father_bird.rs.
The three focused tests share crates/sim/tests/it/owl_father.rs, registered additively in tests/it/main.rs.
The original read-only inspection utility is crates/sim/examples/owl_ai_inspect.rs.
Small shared hooks are in duel.rs, arena.rs, enemy.rs, lib.rs and character.rs.
Presentation hooks are in crates/game/src/arena_d_scene.rs and play.rs.
Documentation changes are this file and STATUS.md.
No cache/re, original install/save, git-state, memory/dataset or existing-log writes are made by this task.
