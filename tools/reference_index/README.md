# Reference screenshot index

A local, fixed-region reader for 2560x1440 Sekiro practice-tool screenshots.
Source images are opened read-only; output must be a separate folder.
Only Pillow and NumPy are required, and PEP 723 declares them for `uv`.

```powershell
uv run tools/reference_index/index.py <screenshots-folder> --output <reference-folder> --calibration <calibration.json> --maps cache/maps --review <review.json> --validation <validation.json>
```

The input folder, calibration, reviews, extracted layouts, image-derived glyphs and generated artifacts stay outside tracked source.
Supply folder paths as arguments; the tool has no machine-specific paths.
If the default uv cache is unavailable, set `UV_CACHE_DIR` to a writable local directory.
An offline environment can use already installed Pillow and NumPy with `uv run --no-project python tools/reference_index/index.py ...`.

## Reference story and exact replay

The original collection is matched with story `genichiro-defeated` (see Collections below), using boss progression flags rather than an hour override.
Gyoubu alone (`gyoubu-defeated`, flag 9301) retains morning at 06:00.
Genichiro (`genichiro-defeated`, flags 9301 and 9303) raises 9800 through the event-820 rule and selects afternoon at 12:00 in both Ashina areas.
The reference's Castle Tower Lookout travel destination, Attack Power 3 and overcast sky support the latter candidate; playtime alone cannot identify flags.
Unknown quest, pickup and interaction flags remain unset.

## Collections and their story

A reference root may hold `collection.json` declaring the story its shots were taken in and where its screenshots live, for example `{"story": "morning", "images": "../reference/m11_00"}`.
A relative `images` path is relative to the root.
Without the file a root is the original post-Genichiro collection, with story `genichiro-defeated` and screenshots in `m11_00/`, so that collection behaves as before.
`match_poses`, `regenerate_pairs` and `report_parity` take the story from the collection and treat `--story` only as an explicit override.
Capture roots written by `match_poses` and `regenerate_pairs` get their own `collection.json` with the story used and the absolute screenshot folder, so a later report reads the same story back.
A second collection can therefore reuse the read-only screenshot folder with its own `index.json` and `spots.json`.
Screenshots taken without the practice-tool overlay have no coordinates; give such entries an estimated `position` with a `position_source` note, and treat their pairs as rough until reviewed.

Replay accepted cameras without searching again into a fresh external collection:

```powershell
uv run --no-project --with pillow --with numpy --with scipy python -m tools.reference_index.regenerate_pairs --reference <reference-root> --output <reference-root>/pairs-genichiro-defeated --game target/alt-story/release/sekiro-rs.exe --annotations <review-ledger.json>
uv run --no-project --with pillow --with numpy --with scipy python -m tools.reference_index.triage_pairs --reference <reference-root>/pairs-genichiro-defeated --sheets --sheet-output <external-review-folder>
uv run --no-project python -m tools.reference_index.report_parity --reference <reference-root>/pairs-genichiro-defeated --annotations <renewed-review-ledger.json> --output <external-summary.md>
```

Replay writes full-resolution pairs, individual ours images, visibility records and its own `poses.json` directly in that collection.
It preserves saved camera/player/FOV fields exactly, records the executable digest and story, and refuses to overwrite a completed collection.
Replay retains the original camera-fit score as a saved score and records its source executable separately; it does not present that score as a new camera search.
Camera acceptance can carry forward from the original ledger, but each visible defect needs renewed review before its count is treated as current.
The original `pairs/` and `poses.json` remain untouched.
Reports reject a mismatched or mixed capture state by default; use an explicit historical `--story` only when reporting an older collection.
The matcher also rejects resuming a collection captured under a different story, so old night poses cannot silently be treated as finished daytime poses or overwritten by a state change.

## Recognition and review

Colour-distance masks isolate the red X, green Y and blue Z text in native pixels `(45,292)-(470,316)`.
The white IGT line uses eleven fixed monospace cells beginning at X 101, with a 11.75-pixel pitch.
Glyph templates are built in memory from hand-transcribed calibration images on every run.
No screenshot pixels or extracted templates are embedded in the repository.
Character mismatch and runner-up margins, numeric syntax and clock ranges gate acceptance.
An uncertain value is null and the entry is `unreadable`; the candidate text remains in `recognition` for review.
Menus require visual classification in the review file.
Capture times come from the filename and have no inferred time zone.

A calibration object contains `seeds`, each with a one-based sorted `index`, optional `raw` array of three decimal strings, and optional `igt` string.
`verified_igt` is an equivalent label for an additional independently transcribed calibration clock.
For example, a seed has the shape `{"index": 1, "raw": ["1.234", "-5.678", "9.012"], "igt": "01:23:45.67"}`.
These example numbers illustrate the schema and are not reference measurements.
Every digit, decimal point, sign and clock separator used by the collection needs calibration coverage.
Review input contains `entries` with `index`, explicit corrected fields and a `note` explaining visual verification.
Supported review fields are `raw_numbers`, `igt`, `status` and `menu`.
Do not promote a candidate merely because it resembles neighbouring screenshots.
Validation input is an array of independently transcribed `index`, `raw_numbers` and `igt` objects, kept separate from calibration.
`validation-results.json` records exact comparisons before review corrections.

Use `--reuse-recognition` to reapply reviews and regenerate spatial artifacts without reading the overlay again.
Use `--refresh-igt` to rerun only clock recognition while retaining coordinate results.
The primary `index.json` is a JSON array with one entry per source image.
Overview sheets cover capture order; area contact sheets retain the same global index numbers.

## Coordinate and floor rules

The verified overlay frame agrees with the source MSB placed coordinates across both blocks.
Convert `(X,Y,Z)` to Bevy `(-X,Y,Z)` without another translation or rotation.
Do not reapply Outskirts MapOffset; exported part placement already includes it, as documented in `docs/MAP-COMPLETENESS.md`.
Compare visible idols against authored part translations and collision support at several locations in both blocks before applying this rule to another collection or overlay configuration.
The reference output report records the measured evidence and alternative-frame checks for this collection.

Spatial enrichment reads canonical Object placements and the exported postcard `CollisionMesh` arrays without modifying the cache.
Nearest idol distance is Euclidean distance in metres, including height, across authored idol assets in both blocks.
The inventory includes authored variants that may be disabled by live story events; entity ID and part name remain explicit.
Map ownership is inferred from the highest collision support within 0.15 m above the player, breaking shared-floor ties with the nearest idol's block.
It describes the matching exported block, not an observation of the game's active streaming state.
`support_height_difference` is the signed distance to that tolerant support surface.
`height_above_floor` uses a separate strict vertical query to the highest triangle at or below the position; `floor_y` records that surface.
These queries include exported geometry without live story filtering, and swimming positions measure the submerged collision floor.
Null positions have null spatial fields and no invented map dot.
Coincident shots overlap on the map; all retain separate indexed entries and thumbnails.
`top-down-transform.json` supplies the X/Z projection in pixels per metre.

No Rust crate is changed by this tool.
Check Python syntax with `uv run --no-project python -m compileall -q tools/reference_index` and verify it on held-out screenshot transcriptions.

## HUD labels, spots and Windows OCR

After creating `index.json`, run `uv run tools/reference_index/postprocess.py --folder <screenshots-folder> --output <reference-folder> --review <review.json>` on Windows.
The Windows OCR pass uses the built-in `Windows.Media.Ocr` API through Windows PowerShell and needs an installed OCR language; it downloads no packages.
It reads the `(45,255)` crop at `430x70` pixels and stores the independent text in each entry as `windows_ocr_reading`.
Every normalized difference from the template reading is appended to `review.json` under `ocr_disagreements`; the script never changes either reading.
Normalization removes whitespace and stray/repeated periods while preserving digits, signs, colons and meaningful decimal points.
HUD visibility uses red vitality-bar and orange posture/boss-bar pixel signals outside the practice-overlay region.
Each entry stores `hud` as `visible` or `hidden` plus the measured pixel counts in `hud_evidence`.
The evidence counts remain available for checking ambiguous backgrounds.
Positioned shots in consecutive capture order share a spot when their positions are within 0.002 m and their filename capture times are at most 30 seconds apart.
Both grouping thresholds can be changed with `--position-tolerance` and `--max-gap-seconds`.
Every positioned entry receives a `spot_id`, including a spot containing only one shot; unpositioned menus have a null spot ID.
`spots.json` records each spot's representative position, block, nearest landmark and member filenames.

## Camera matching against our renderer

`match_poses.py` finds, for each indexed screenshot, the camera in our windowless renderer that best matches it, and writes side-by-side pair images for visual review.
It needs a release build of `sekiro-rs.exe` whose `--sweep` mode accepts `--pose-server` (implemented in `crates/game/src/map_pose.rs`, included from `map_sweep.rs`).
The renderer is started once per map block, headless, and serves pose requests through a file queue in `<output-root>/pose-search/<map>/`.

```powershell
$env:CARGO_TARGET_DIR = 'target/alt-pt'
cargo build --release -p sekiro-game -j 6
uv run --no-project --with pillow --with numpy --with scipy python tools/reference_index/match_poses.py --reference <reference-root> --output <output-root> --game target/alt-pt/release/sekiro-rs.exe --map m11_01 --limit-spots 20
```

`--reference` is the folder holding `index.json`, `spots.json` and the `m11_00/` screenshot folder; `--output` must be outside the repository.
Use a separate writable output folder when the reference collection is read-only; the matcher reads source screenshots from `--reference` and keeps the queue, poses and generated pairs under `--output`.
Spots are taken in `spots.json` order; each spot contributes its first HUD-hidden shot, plus a second shot when the spot is within 4 m of an idol.
Shots already present in `poses.json` are skipped unless `--force` is given; `--only-shots 311,319` restricts a run to listed shot numbers and `--map m11_00|m11_01` to one block.
Each finished shot writes `pairs/<spot>-shot-<n>.png` (real | ours | edge difference), `.search.json` (ranked candidates) and `.visibility.json` (the per-part visibility decision for that render), and appends a row to `poses.json`.
The camera uses the extracted default parameters (43 degree vertical FOV, 4.5 m distance, 1.5 m focus height) and Play's wall pull-in rule through `pose_collision.py`, reading `cache/maps/<map>/collision.bin`.
Yaw and pitch are Bevy YXZ degrees with positive pitch looking up; positions are already in the Bevy frame.
A spot takes about 45 seconds; the renderer stays hidden and no real-game process is involved.

The match score is a structural edge cost and is not proof of a correct camera: review every pair by eye and accept it only when the same landmarks line up.
`uv run --no-project --with pillow --with numpy --with scipy python -m tools.reference_index.triage_pairs --reference <capture-root> --sheets --sheet-output <review-folder>` writes review contact sheets to a separate writable folder.
Here `<capture-root>` is the root holding `poses.json` and `pairs/`, which may differ from the source screenshot root.
Without `--sheet-output`, contact sheets go under the capture root's `pairs/review/`; `--inventory --maps cache/maps` adds projected part bounds beside each pair and requires a writable capture root.
`report_parity.py` turns a hand-written `pairs/triage.json` annotation file into a Markdown summary; `--annotations <external-ledger.json>` selects a separate reviewed ledger.
Write that summary outside the repository; the tracked `docs/VISUAL-PARITY.md` contains the separately reviewed work list and should not be overwritten by the summary generator.
The completed review ledger is `2026-10-15/parity-review.json` under the external progress root and supersedes the incomplete inherited triage file.
Known weaknesses: when the coarse grid misses the heading the score prefers nearby dense texture (ground, walls) over distant structure, and the collision choice can land on a zero-demand helper plane (`h9*`) and render an empty frame.

Tests: `uv run --no-project --with pillow --with numpy --with scipy python -m unittest tools.reference_index.test_match_poses` from the repository root.
Add `--offline` after `uv run` when the dependencies are already available locally and networking is blocked.
For this workspace's restricted cache, the existing gitignored `target/alt-pt/run-python.ps1` wrapper runs uv with locally cached wheels and a writable cache directory.
Build and check with `CARGO_TARGET_DIR=target/alt-pt`, `cargo build --release -p sekiro-game -j 6`, `cargo clippy --release -p sekiro-game --all-targets -j 6 -- -D warnings` and `cargo fmt --all --check`.
