//! The real play path in a window: Explore Ashina Outskirts as the menu launches it, with
//! `--live-check` measuring loading, frame times and drawn ground (see
//! `docs/MAP-COMPLETENESS-SWEEP.md`, "Live play check"). Needs a GPU, a desktop session and
//! the extracted cache; it opens one game window at a time.
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Seconds from launch until the loading screen lifts. HEAD e7bebb6 took about 125 s; the
/// streamed, GPU-checked load takes 50 to 60 s on the reference machine (Ryzen 7 5700X3D,
/// RTX 5070 Ti, warm file cache).
const READY_SECONDS: f64 = 90.0;
/// The longest frame while loading. HEAD e7bebb6 stalled 8.7 s (10.2 s for the first frame).
/// Streaming now leaves 2 to 7 s frames where Bevy prepares and allocates thousands of GPU
/// meshes at once (`prepare_assets<RenderMesh>`, `allocate_and_free_meshes`); Windows marks
/// a window "Not Responding" after 5 s, so the target is 4 s and this bound only stops the
/// old stalls returning (open item in docs/MAP-COMPLETENESS-SWEEP.md).
const LOADING_FRAME_MS: f64 = 8000.0;
const FRAME_MEDIAN_MS: f64 = 20.0;
const FRAME_P95_MS: f64 = 40.0;

/// Explore mode's arguments for the Outskirts, exactly as `menu.rs` builds them.
const EXPLORE: [&str; 16] = [
    "--play",
    "--enemy",
    "soldier",
    "--map",
    "m11_00_00_00",
    "--start",
    "0",
    "--brain",
    "lua",
    "--progression",
    "castle-roof",
    "--player",
    "default",
    "--objects",
    "--simple-ai",
    "--no-enemy",
];

/// Run out of the start hall in four directions, then sprint.
const WALK: &str = "\
60 0 0 -
240 0 1 -
240 1 0 -
240 0 -1 -
240 -1 0 -
240 0 1 dodge
60 0 0 -
";

fn run(root: &Path, out: &Path, name: &str, extra: &[&str]) -> Value {
    let summary = out.join(format!("{name}.json"));
    let status = Command::new(env!("CARGO_BIN_EXE_sekiro-rs"))
        .current_dir(root)
        .args(EXPLORE)
        .arg("--cache")
        .arg(root.join("cache"))
        .arg("--live-check")
        .arg(&summary)
        .args(extra)
        .status()
        .expect("start the game");
    assert!(status.success(), "{name}: the game exited with {status}");
    serde_json::from_slice(&std::fs::read(&summary).expect("live check summary")).unwrap()
}

fn number(summary: &Value, key: &str) -> f64 {
    summary[key]
        .as_f64()
        .unwrap_or_else(|| panic!("summary has no {key}: {summary:#}"))
}

#[test]
#[ignore = "slow: opens a game window and loads Ashina Outskirts twice, about 3 minutes; needs a GPU and the extracted cache"]
fn outskirts_loads_then_plays_smoothly_on_drawn_ground() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    if !root.join("cache/maps/m11_00_00_00/layout.json").is_file() {
        eprintln!("skipped: Ashina Outskirts is not extracted");
        return;
    }
    let out = std::env::temp_dir().join(format!("sekiro-live-play-{}", std::process::id()));
    std::fs::create_dir_all(&out).unwrap();
    let walk = out.join("walk.txt");
    std::fs::write(&walk, WALK).unwrap();

    // A scripted walk from the menu's start, timed from launch.
    let s = run(&root, &out, "walk", &["--script", walk.to_str().unwrap()]);
    println!(
        "walk: ready {:.1} s, longest loading frame {:.0} ms, frames median {:.1} ms p95 {:.1} ms, holes {}/{}",
        number(&s, "play_ready_s"),
        number(&s, "longest_loading_frame_ms"),
        number(&s, "frame_ms_median"),
        number(&s, "frame_ms_p95"),
        number(&s, "ground_holes"),
        number(&s, "grounded_samples")
    );
    assert!(number(&s, "play_ready_s") <= READY_SECONDS, "slow load");
    assert!(
        number(&s, "longest_loading_frame_ms") <= LOADING_FRAME_MS,
        "the window stalls while loading"
    );
    assert!(
        number(&s, "frame_ms_median") <= FRAME_MEDIAN_MS,
        "low frame rate"
    );
    assert!(
        number(&s, "frame_ms_p95") <= FRAME_P95_MS,
        "frame spikes in play"
    );
    assert!(number(&s, "grounded_samples") >= 20.0, "Wolf never walked");
    assert_eq!(number(&s, "ground_holes"), 0.0, "walked on undrawn ground");
    assert!(
        number(&s, "floor_coverage_min") >= 0.9,
        "floor around Wolf undrawn"
    );
    let start = s["samples"][0]["position"].clone();
    let end = &s["final_position"];
    let moved = (0..3)
        .map(|i| (end[i].as_f64().unwrap() - start[i].as_f64().unwrap()).powi(2))
        .sum::<f64>()
        .sqrt();
    assert!(moved > 3.0, "the walk did not move Wolf ({moved:.1} m)");

    // Every authored player start: drawn ground under Wolf after a teleport.
    let s = run(&root, &out, "starts", &["--live-tour", "starts"]);
    let stops = s["stops"].as_array().expect("tour stops");
    assert!(stops.len() >= 9, "player starts missing from the tour");
    for stop in stops {
        assert!(stop["floor"].is_number(), "no collision floor at {stop}");
        assert_eq!(
            stop["ground_drawn"].as_bool(),
            Some(true),
            "undrawn ground at {stop}"
        );
    }
}
