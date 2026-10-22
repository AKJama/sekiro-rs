# Performance notes

## Castle main-thread cost (#70, 2026-10-22)

The castle (m11_01 start 0) ran at 48 to 55 fps with the GPU 32 to 48% busy, because the CPU set the frame time.
A `profile-spans` build found four game systems doing whole-world work every frame for nothing:

- `capture_ready::watch_meshes` (about 2.9 ms) rescanned about 17,000 loaded meshes on every mesh asset event.
  Cloth, effect batches and the sky rewrite about 21 meshes every frame, so it ran every frame.
  It also held the late-mesh lock, which made the render world's `check_late_meshes` cost 1.3 ms.
  It now prunes only when a mesh asset is removed or the set of referenced meshes changes.
- `WireframePlugin` (about 1.8 to 2.2 ms) checked every mesh for specialization in play, where the collision wireframe can never be shown.
  Only the viewer installs it now.
- `map_world::update_detail` (about 2.2 ms) walked about 17,000 parts on one thread.
  The distance and level-of-detail pass now runs in parallel, and only changed parts are written, in entity order.
- `mesh_residency::extract` (about 1.1 ms, inside extraction, which blocks both worlds) reconciled the suspended set on rewritten meshes too.
  Only added, removed or unused meshes trigger that pass now.

Span profiles before and after: main schedule 19.0 to 14.0 ms, render schedule 18.9 to 16.6 ms.
On a quiet machine the castle start went from 55.5 fps idle and 49.9 fps walking (main thread 13.2 and 14.6 ms) to the 60 Hz cap (main thread 7.1 and 7.3 ms).
Screenshots at the same steps differ from main no more than main differs from itself (falling snow).

## Outskirts fence corner (#121, 2026-10-22)

Wolf pushed into the wooden fence corner by the snowy firewood in the samurai general's courtyard (m11_00, about (-221.6, -38.0, 11.5), standing on log o114541_1809) dropped the game to 3-10 fps.
The cause was the fixed simulation step, not rendering, and #159 and #161 already fixed it.
- The build before #159 (44c372f), on the issue's route: at the corner each frame ran 15 catch-up steps (the old 250 ms cap), the fixed steps took a median 257 ms a frame and `sim character` 16.6 ms a step, so frames took a median 286 ms.
  The render world stayed at a median 17.4 ms and the GPU under 45% busy.
  Walking more than 5 m away the same build stepped at 5.4 ms a step and 32 ms a frame.
- Main (6bfbc9c) on the same route, pushing for 40 s: median frame 16.5 ms (p95 20.0 ms), one step a frame at 1.1 ms, main thread 11.1 ms, render world 15.6 ms, GPU 40 to 46% busy; the same as walking more than 5 m away (17.1 ms).
  Spawned at the corner and pushing for 60 s: median frame 16.6 ms, p95 20.4 ms.
- In the sim alone (map and object collision of m11_00 with its hook points), a pushing step costs a median 4.9 ms before #159 and 0.27 ms on main.

The GPU-bound reading from #70's run (GPU 99% busy, 67 ms swapchain waits) came while another game shared the GPU; alone the corner never passed 46% GPU.
`step_cost::fence_corner_push_is_bounded` walks Wolf into the corner with the placed objects and the hook search and bounds each step's collision work (1,282 grid entries now, under 10,000).

## Wall-jump contact probe (#167, 2026-10-22)

`docking::wall_contact` slides eight 0.8 m probes from the chest every airborne step in the wall-jump window.
Each probe read and filtered the same grid cells again, and its push-outs tested every triangle the whole move could reach on every pass.
Now `Ground::slide_fan` reads the cells once for the eight probes, and a slide's push-outs test only the triangles near the body (`Nearby`) and skip those whose plane is out of reach.
Every probe and slide ends bit for bit where it did: 2.96 million slides at four m11_00 spots matched the old code when it landed, and `step_cost::wall_contact_fan_matches_lone_slides` keeps a compact check.
At the air-kick spot (-256.25, -42.2, 11.43), sim only: grid entries read per probe set 2,387 to 462 (on a 9 by 9 grid of chests, eight facings, three heights), and time per probe set 241 to 187 us (interleaved, minimum of five, on a busy machine).
What is left is real contact work: about 900 triangle contacts per probe set there, since the body is pressed into many small triangles.

## Measuring

`--perf` with `PLAY_PERF_DIR=<folder>` writes the per-frame CSV outside `cache/perf`.
On the development PC frames stop at about 16.6 ms even with `--uncapped` and Immediate presentation, so compare main-thread and render milliseconds, not fps above 60.
Other agents' builds on the same machine move medians by several milliseconds, so compare alternating runs.

## Open

- #106 (castle roof, spawn `83.307,30.6,243.649,133.216,0`): about 46 fps after #70, render-bound (render world about 20 ms, of which queue submit about 6.4 ms).
- Render world: queue submit 4.5 to 6.5 ms, `prepare_preprocess_bind_groups` about 2 ms, shadow `write_binned_instance_buffers` about 1.9 ms.
- Effects (`fxr::render`, 2 to 3 ms) and cloth simulation (about 1.5 ms) on the main thread.
- Bevy's per-material specialization checks and `calculate_bounds` over about 180,000 entities; static batching (#118) would cut the entity count.
