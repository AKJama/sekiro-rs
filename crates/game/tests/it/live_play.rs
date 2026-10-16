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
/// The longest frame while loading, after the first. HEAD e7bebb6 stalled 8.7 s; streaming
/// measures about 0.4 s. Windows marks a window "Not Responding" after 5 s.
const LOADING_FRAME_MS: f64 = 2000.0;
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

/// Ashina Castle player starts 1, 2, 3, 5, 6 and 7, reached from the Outskirts load, then a drop
/// into the Outskirts ravine north of start 0, whose only collision is the filter-15 event wall
/// `h900000` 30 m down (`sekiro_sim::hit_filter`): Wolf falls through it and the fall return
/// puts him back on the last stop.
const CASTLE: &str = "202.6,-26.4,176.7;82.9,29.8,254.3;120.3,54.1,234.2;-60.4,-31.2,360.4;336.9,-32.1,252.7;47.1,-2.6,188.6;drop:-137.6,-44.3,-124.5";

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

/// Grapple from the Outskirts ledge east of the ravine to the branch over it, then, perched,
/// turn the camera a full circle from step 400 (`--live-orbit`). The branch stands on no map
/// collision (only the event walls h900000 and h900001 are below), so the map must stay on
/// the ledge's collision part, whichever way the camera looks.
const GRAPPLE_SPAWN: &str = "-132.3,-42.0,-123.9,90,0";
const GRAPPLE: &str = "120 0 0 -
1 0 0 grapple
240 0 0 -
720 0 0 -
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
#[ignore = "slow: opens a game window and loads Ashina Outskirts four times, about 5 minutes; needs a GPU and the extracted cache"]
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
    let grapple = out.join("grapple.txt");
    std::fs::write(&grapple, GRAPPLE).unwrap();

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

    // Far into Ashina Castle (block m11_01), 300 to 600 m from the start: the connected blocks
    // draw where Wolf stands after loading from the Outskirts.
    let s = run(&root, &out, "castle", &["--live-tour", CASTLE]);
    let stops = s["stops"].as_array().expect("tour stops");
    assert_eq!(stops.len(), 7, "tour stops missing");
    for stop in stops {
        assert_eq!(
            stop["ground_drawn"].as_bool(),
            Some(true),
            "undrawn ground at {stop}"
        );
    }
    // The ravine drop: through h900000 (y -74.2), never standing on it, then back on the
    // last stop's ground.
    let drop = &stops[6];
    let events = drop["helper_events"].as_array().expect("helper events");
    let wall = events
        .iter()
        .find(|e| e["part"] == "h900000")
        .unwrap_or_else(|| panic!("the drop never met h900000: {drop}"));
    assert_eq!(wall["hit_filter"].as_u64(), Some(15), "{drop}");
    let back: Vec<f64> = wall["returned_to"]
        .as_array()
        .expect("no fall return")
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect();
    let last = stops[5]["position"].as_array().unwrap();
    for i in 0..3 {
        let (a, b) = (back[i], last[i].as_f64().unwrap());
        assert!(
            (a - b).abs() < 0.5,
            "returned to {back:?}, last stop {last:?}"
        );
    }
    assert!(
        number(drop, "lowest_y") < -75.0,
        "Wolf stopped on the helper: {drop}"
    );
    assert_eq!(drop["grounded"].as_bool(), Some(true), "{drop}");
    println!(
        "ravine drop: fell to {:.1}, contacts {}, back at {back:.2?}",
        number(drop, "lowest_y"),
        events.len()
    );

    // The grapple to the branch and a camera turn there: the Outskirts block keeps its display
    // groups and the map around Wolf stays drawn at every camera yaw.
    let s = run(
        &root,
        &out,
        "grapple",
        &[
            "--spawn",
            GRAPPLE_SPAWN,
            "--script",
            grapple.to_str().unwrap(),
            "--live-orbit",
            "400,8",
        ],
    );
    assert_eq!(
        s["orbit_turned"].as_bool(),
        Some(true),
        "the camera never turned"
    );
    let end = &s["final_position"];
    let perched = [[-137.58, -44.30, -124.49], [-141.58, -44.26, -123.86]]
        .iter()
        .any(|p| (0..3).all(|i| (end[i].as_f64().unwrap() - p[i]).abs() < 0.5));
    assert!(perched, "Wolf did not reach the branch: {end}");
    let changes = s["block_changes"].as_array().expect("block changes");
    for change in changes {
        if change["block"] == "m11_00_00_00" {
            assert_eq!(
                change["state"], "Ready",
                "the Outskirts block lost its groups: {change}"
            );
        }
    }
    let samples = s["samples"].as_array().expect("samples");
    let near: Vec<u64> = samples
        .iter()
        .map(|v| v["drawn_meshes_near"].as_u64().unwrap())
        .collect();
    assert!(near.len() >= 40, "too few samples: {near:?}");
    assert!(
        near.iter().all(|&n| n > 0),
        "the map around Wolf vanished: drawn meshes near {near:?}"
    );
    println!(
        "grapple: perched at {end}, {} samples, fewest drawn meshes near {}",
        near.len(),
        near.iter().min().unwrap()
    );
}
