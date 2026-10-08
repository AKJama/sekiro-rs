# Guard, deflect and posture in the player script

A summary, in our own words, of how the player's HKS scripts (`c0000*.hks`) drive guarding, deflecting, guard break and posture-related reactions.
Ids refer to `docs/HKS-INTERFACE.md`.
Behaviour reference ids are the numbers the scripts pass to env 3036 ("SpEffect with this behaviour reference id is active").
The decompiled source this was read from lives under `re/hks/` and is not tracked.

## What the script decides and what it does not

The script only chooses animations and behaviour-graph transitions.
It never computes posture, damage or whether a hit was deflected.
The engine has already classified each incoming hit before the script runs, and the script reads that result through env 202 (damage type), env 236 (damage level), env 285 (element) and a handful of SpEffect checks.
Posture numbers, the deflect timing check and posture-break thresholds therefore belong to the exe and the params, not to this layer.

## How a frame flows

Every frame the active behaviour state's update hook calls `UpdateState`, which runs `Control`, then `Validate`, then `FireStateEndEvent`.
`Validate` looks up the current state's style (one of 16, such as stand, crouch, ground guard, free fall, swim) and state type (standby, action, reaction, guard variants, ...) in a table keyed by behaviour state id.
For that style it walks one fixed priority list of candidate behaviours, reactions first (throw death, death, special damage, hit damage, break damage, guard damage, fall, land, ...) and actions after (item use, jump, step, combat art, attack, deflect guard continue, start and end, grapple, crouch, movement, ...).
Each candidate has a validity function; the first that returns true wins.
`_ActivateBehavior` then fires one or more behaviour-graph events (named `W_...`) and usually calls act 9101 to reset input acceptance.
A per-style enable mask decides which candidates are even considered: deflect guard start is allowed in the stand, crouch, cover, cover-look and sprint styles, while continue and end only exist in the ground-guard style.
A second, additive pass runs the same way for reactions that play on an additive layer without leaving the current state (minimum-level guard hits, being deflected in the air, fire).

## Input

Input is never read directly.
env 1106 with an action-arm id is true on the frame the engine has a request for that action (guard is arm 2, attack 0, step 5, jump 4, combat art 32).
env 1108 with the same id is the hold duration; greater than zero means the button is still held.
Buffering and the hold time unit live in the engine.
Most actions are additionally gated by env 3033 (action unlocked by progression) and, for katana actions, by env 225 reporting weapon category 50 in the right hand.
env 207 equal to 0 (weapon sheathed) or behaviour reference 600 (non-combat area) puts the player in non-combat mode, which turns most combat behaviours into a plain reset request.

## Guard (deflect guard) states

Sekiro has one guard stance, entered through "deflect guard" states; there is no separate plain block.

Start.
The guard-start candidate is valid when guard is held (1108 greater than zero) and either requested this frame (1106) or reference 221 allows starting from a hold, the katana is usable, the player is not in a waterside area (reference 110005) and not force-crouched.
A TAE window can also accept a buffered guard press during another action (references 411 and 412).
The chosen event depends on context: an additive "hard deflect guard" when reference 228 is active, guarding straight out of hit stun when reference 503 is active, a sprint variant, a reversed variant when reference 227 is active, a prosthetic (umbrella) guard when the left hand holds category 76 and its TAE window is open, and otherwise the ordinary stand-to-guard event.
Running the shipped script in our VM with guard requested and held fires exactly this ordinary event.

Repeated presses.
While already guarding (ground-guard style), a fresh press is a "continue": with reference 212 (guard combo window) active the script steps through guard variants 2, 3 and 4 depending on which one is current; otherwise it restarts the first variant (or its reversed form).
These variants are the repeated-deflect animations; their individual deflect windows come from their TAE, not from the script.

End.
The guard ends when the state allows it (env 1105 standby, or reference 207 cancel window) and guard is released, the player is in non-combat mode, or in a waterside area.
The end event picks an idle or moving variant from the locomotion state, with an alternate set when reference 220 is active.

## Guard hits: block versus perfect deflect

When a hit lands while guarding, the engine reports damage type 3 (guard) or 1028 (named for the attacker's stamina running out against a guard).
The script splits the reaction three ways.

- Minimum-level guard hits, or small to large hits while reference 202 ("treat guard level as minimum") is active, play as additive reactions without leaving the current state.
- Otherwise the full guard reaction runs when env 237 (guard level action) is above zero.
- In both cases, whether the reaction is a block ("easy deflect") or a perfect deflect ("hard deflect") depends only on reference 203, the just-deflect window that TAE events open early in the guard animations.

Perfect deflect.
If one of the four special guard reaction identification values is set (env 334, values 20 to 23, used by specific bosses), a matching special deflect event plays.
If the damage type is 1028, the player plays a deflect stagger instead of a normal deflect.
This matches the known rule that a perfect deflect never breaks the player's posture even when the bar is full, and instead staggers briefly; the posture side of that rule is in the exe.
Otherwise the damage level picks a small, middle, large, minimum or extra-large perfect-deflect animation, and act 141 records the matching guard damage flag.

Block.
The same special identification values select boss-specific block reactions; otherwise the damage level picks small, middle, large, minimum or extra-large block animations.
One of the block animations has two variants chosen with `math.random`, the only use of randomness in the combat branches.

Direction.
For air deflects the script reads env 3056 (block) or env 3057 (perfect deflect) to choose a left or right animation.

Elements.
Lightning and blue lightning have dedicated air deflect reactions (lightning reversal setup), selected by env 285.

## Guard break and posture break

Guard break, the player's posture filling up while guarding, arrives as damage type 1001 or its blast and fling variants 1004 and 1005.
The script plays the stand guard-break event (or the prosthetic-guard variant), records the guard-break damage flag with act 141, and has separate events for air, swim and dive.
Damage type 1027 ("damage break") is a heavy-hit stagger that breaks the player out of whatever they were doing; its animation depends on damage level and direction (small blow, large blow, extra-large blast, launch, pound) and can be suppressed by env 349.
Wall impacts are damage types 1006 and 1007.
Whether a hit is a guard break, a damage break or a normal hit is decided by the engine before the script sees it.

## Being deflected (the player's own attacks)

When the target blocks or deflects the player's attack, the player receives a "guarded" damage type.
Types 1000, 1003 and 1033 together with env 3019 (just-guard-repelled behaviour) select the "hard deflected" recoil, meaning the enemy perfectly deflected.
Types 1017 and 1018 together with env 3018 (guard-repelled behaviour) select the lighter "easy deflected" recoil.
env 3018 and 3019 also give the recoil direction (left, right, or an additive variant), and a TAE window (reference 232) can disable the recoil.
Types 5, 1029 and 1031 are parries and contact parries and go to the same break-reaction branch.

## Mikiri Counter

The player scripts never mention Mikiri Counter by name, and no step, guard or damage branch selects a dedicated counter animation.
The most likely path is the native throw system: the counter is a paired animation like a deathblow, started by the engine (throw request env 1112, throw animation id env 273) and played through the many throw-attack states, whose hooks only do throw bookkeeping.
This is unverified and needs the Ghidra and ThrowParam passes.

## What a simulation must supply for this to run

- env 202, 236, 237, 285 and 334 at the moment a hit is resolved, so the reaction selector sees the hit.
- env 3036 answers for at least references 202, 203, 207, 212, 220, 221, 227, 228, 232, 411, 412, 503, 600 and 110005, driven by the TAE events of the current animation.
- env 1106 and 1108 from input, env 3033 progression unlocks, env 225 equipped weapon categories and env 207 arm style.
- A behaviour graph that turns the fired `W_...` events into state changes and calls the new state's hooks; without it the script keeps re-firing the same event each frame, which is what the stub host shows today.

## Determinism notes

Script numbers are 32-bit floats, so arithmetic must stay in f32 to match.
`math.random` has three call sites in the player set: the block variant above, a random index helper and a speaking-blend index; the VM's generator is seedable but is not the game's generator.
`pairs` iteration order differs between our tables (insertion order) and the game's hash order.
The only `pairs` loop in the player set is in a validation helper that nothing calls, so this does not affect behaviour today.
