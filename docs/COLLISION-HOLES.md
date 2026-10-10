# Collision holes

Headless audit of all nine gameplay maps; m89 is excluded because it has no walking terrain.
Coordinates are Bevy world metres (source X mirrored).
Raw candidates are render/collision discrepancies, not automatically gameplay defects.
`confirmed` requires both a Body reproduction and a retained visual review; unchecked and inconclusive candidates remain unconfirmed.

## Totals

| Map | Found | Body reproduced | Visually confirmed | Rejected false positives | Body checked | Audit seconds |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| m10_00_00_00 | 27950 | 13 | 0 | 0 | 40 | 109.47 |
| m11_00_00_00 | 26279 | 13 | 2 | 2 | 40 | 108.61 |
| m11_01_00_00 | 18250 | 7 | 3 | 0 | 40 | 100.08 |
| m11_02_00_00 | 4998 | 15 | 2 | 0 | 40 | 167.14 |
| m13_00_00_00 | 4180 | 23 | 1 | 2 | 40 | 214.81 |
| m15_00_00_00 | 19695 | 19 | 2 | 0 | 40 | 96.98 |
| m17_00_00_00 | 29043 | 21 | 1 | 0 | 40 | 229.56 |
| m20_00_00_00 | 36367 | 3 | 0 | 0 | 40 | 142.11 |
| m25_00_00_00 | 41479 | 5 | 1 | 0 | 40 | 114.69 |
| Total | 208241 | 119 | 12 | 4 | 360 | 1283.46 |

Summed headless scan and validation time: 1283.46 seconds (21.39 minutes).
Parallel batch wall time is recorded in STATUS.md.

These timings include the final full scans and the latest cached validation pass per map, excluding exploratory scans, compilation and screenshot review.

## Method and thresholds

A conservative 4 m voxel mask around walkable collision removes distant background surfaces before raster storage; large triangles iterate only occupied horizontal tiles.
Render walls use a BVH to avoid duplicating large triangles into thousands of horizontal buckets.
The floor scan rasterizes upward-facing triangles from every placed map-piece GLB at 0.5 m cell centres, preserving separate height layers at 0.25 m quantization.
It accepts slopes up to 50 degrees and flags no collision within 0.3 m above or below the exact render height.
Water, sky, foliage, grass, leaf, bush, ivy, flowers, clouds, fog, decals, shadow/dummy/invisible helpers and blended materials are excluded by exported material/MTD names.
A candidate needs a collision floor within 2 m horizontally and 1 m vertically, with an unobstructed collision ray to it.
Four-neighbour components connect heights differing by at most 0.75 m; area is the union of their horizontal grid cells.
The reachability filter clips large unsupported surfaces to their nearby supported edges, so area is a sampled discrepancy area rather than the whole decorative mesh area.

Each map runs 2,048 deterministic probes, starting at walkable collision triangle centroids, with 64 random moves of 0.5 to 3 m.
Seed is 0x5e61726f XOR probe index times 0x9e3779b97f4a7c15.
Rayon runs independent probes in parallel; capsule radius is the Body's 0.4 m, and every slide call moves at most 0.1 m.
Probes flag absent floor or drops over 5 m under sampled render ground, and render-wall crossings with no collision hit on the movement ray extended by 0.3 m.
Odd render-wall intersection parity along three horizontal rays labels enclosure-entry candidates, but does not prove watertightness or distinguish a hollow room from solid rock.
Wall matching is a conservative ray test, not a full nearest-triangle distance calculation.

The top 30 ranked floor candidates and first 10 independent probe candidates use Body at 60 Hz, 0.04 m per tick, in six directions for floor gaps.
Starts need 1.6 m headroom and 30 stable standing ticks with less than 0.2 m displacement.
Floor reproduction requires a drop over 5 m and more than five ticks over 5 m below the witness render-height layer while within 1 m horizontally of that witness.
Wall reproduction requires an actual Body movement segment to cross a retained render triangle.
Ranking is area divided by (1 + nearest-start distance / 100); nearest-start distance is a geometric proxy, not a proven navigable route.
Centroid is descriptive; witness is an actual sampled cell used for checks.

This audit uses default viewer map pieces without event-controlled objects, rendering, animation or player HKS.
It cannot certify collision completeness, detect sub-grid floor gaps, or prove every candidate is accessible using jumping, grappling or swimming.
Unreproduced and unchecked candidates are preserved for investigation.

## Ranked candidates

### m10_00_00_00

| ID | Kind | Area m² | Centroid x, y, z | Nearest start (distance m) | Status | Screenshot |
| ---: | --- | ---: | --- | --- | --- | --- |
| 1 | floor_gap | 3141.25 | 100.06, -759.42, 503.66 | 6 (21.3) | not reproduced | pending |
| 2 | floor_gap | 797.25 | 63.83, -759.35, 457.39 | 6 (78.2) | Body reproduced, visual pending/inconclusive | cache/screens/holes/m10_00_00_00_2.png |
| 3 | floor_gap | 475.25 | -34.03, -759.99, 411.34 | 4 (15.8) | Body reproduced, visual pending/inconclusive | cache/screens/holes/m10_00_00_00_3-alt.png |
| 4 | floor_gap | 544.75 | 28.03, -759.34, 404.83 | 4 (50.1) | Body reproduced, visual pending/inconclusive | cache/screens/holes/m10_00_00_00_4.png |
| 5 | floor_gap | 679.50 | 9.15, -766.64, 498.55 | 4 (99.0) | Body reproduced, visual pending/inconclusive | pending |
| 6 | floor_gap | 416.00 | -35.26, -754.61, 443.71 | 4 (43.4) | not reproduced | pending |
| 7 | floor_gap | 299.50 | -1.36, -757.50, 389.39 | 4 (26.8) | not reproduced | pending |
| 8 | floor_gap | 274.50 | -20.32, -758.78, 386.65 | 4 (19.5) | Body reproduced, visual pending/inconclusive | pending |
| 9 | floor_gap | 339.50 | -37.31, -843.64, 39.99 | 1 (109.8) | not reproduced | pending |
| 10 | floor_gap | 271.00 | -212.53, -741.74, 401.99 | 8 (108.4) | not reproduced | pending |

Review 2: Body reproduced a 6.44 m fall. The high camera shows walkable roof surfaces and a courtyard at this layer, but no clear opening at the witness; the roof edge and nearby surfaces obscure the exact path..

Review 3: Body reproduced a 263.48 m fall. The view shows rendered rooftops and village surfaces, but the exact witness cannot be established as a standable roof or reachable surface from the screenshot..

Review 4: Body reproduced a 165.31 m fall. The view shows a rendered surface over a steep hillside and a large drop, but does not establish whether the witness lies on reachable ground or a decorative terrain sheet..

### m11_00_00_00

| ID | Kind | Area m² | Centroid x, y, z | Nearest start (distance m) | Status | Screenshot |
| ---: | --- | ---: | --- | --- | --- | --- |
| 1 | floor_gap | 562.75 | 157.46, -74.39, 44.88 | 6 (29.3) | not reproduced | pending |
| 2 | floor_gap | 784.50 | -157.83, -121.32, 86.81 | 3 (104.9) | not reproduced | pending |
| 3 | floor_gap | 658.00 | -124.80, -30.79, 140.38 | 3 (75.4) | not reproduced | pending |
| 4 | floor_gap | 653.25 | -166.01, -115.32, -32.06 | 2 (102.8) | not reproduced | pending |
| 5 | floor_gap | 393.25 | 46.25, -64.22, 7.22 | 5 (43.5) | not reproduced | pending |
| 6 | floor_gap | 707.75 | -106.53, -153.51, 258.32 | 4 (165.3) | not reproduced | pending |
| 7 | floor_gap | 511.50 | -148.93, -119.82, 50.05 | 2 (95.9) | not reproduced | pending |
| 8 | floor_gap | 260.25 | -183.84, -60.11, 141.38 | 3 (20.4) | not reproduced | pending |
| 9 | floor_gap | 271.50 | 35.12, -132.32, 28.66 | 5 (83.9) | not reproduced | pending |
| 10 | floor_gap | 131.00 | -123.33, -48.67, -189.45 | 0 (4.6) | not reproduced | pending |

Visually confirmed reproductions (ranked):

| ID | Area m² | Witness x, y, z | Screenshot |
| ---: | ---: | --- | --- |
| 15 | 155.25 | -69.25, -24.48, 125.25 | cache/screens/holes/m11_00_00_00_15.png |
| 22 | 118.00 | 102.75, -50.10, -2.75 | cache/screens/holes/m11_00_00_00_22.png |

Review 15: Body reproduces up to 201.7 m of falling from the witness layer while the screenshot shows continuous tiled roof beneath it. Sekiro roofs are walkable, so the missing collision is a plausible floor hole..

Review 17: The screenshot shows the witness area buried in dense tree canopy against a steep rock face, with no clear reachable floor surface. The body fall is consistent with an unreachable cliff-side or foliage surface rather than a walkable ground hole..

Review 22: The witness lies on a continuous rendered tiled roof. Body evidence falls 100.8 m through this layer, and the screenshot confirms a walkable roof surface rather than a cliff or decorative rock..

Review 29: The witness is obscured by tree canopy and sits against steep rock faces. The reproduced fall does not establish a traversable floor surface here; this reads as an unreachable cliff-side surface..

### m11_01_00_00

| ID | Kind | Area m² | Centroid x, y, z | Nearest start (distance m) | Status | Screenshot |
| ---: | --- | ---: | --- | --- | --- | --- |
| 1 | floor_gap | 798.50 | 113.87, 64.70, 235.65 | 3 (12.5) | not reproduced | pending |
| 2 | floor_gap | 626.50 | 111.94, 53.21, 237.45 | 8 (7.3) | not reproduced | pending |
| 3 | floor_gap | 554.75 | 130.87, 40.08, 222.47 | 9 (2.7) | not reproduced | pending |
| 4 | floor_gap | 567.50 | 110.74, 39.98, 238.52 | 8 (8.1) | not reproduced | pending |
| 5 | floor_gap | 572.25 | 111.81, 37.40, 236.97 | 8 (10.4) | not reproduced | pending |
| 6 | floor_gap | 757.25 | 265.17, -68.45, 200.45 | 1 (79.0) | not reproduced | pending |
| 7 | floor_gap | 515.25 | 80.16, 25.09, 201.33 | 4 (44.4) | not reproduced | pending |
| 8 | floor_gap | 551.50 | 264.44, -32.22, 141.96 | 1 (71.2) | not reproduced | pending |
| 9 | floor_gap | 296.00 | 337.14, -22.41, 259.09 | 6 (11.6) | not reproduced | pending |
| 10 | floor_gap | 379.25 | 61.84, -11.17, 246.37 | 2 (46.7) | not reproduced | pending |

Visually confirmed reproductions (ranked):

| ID | Area m² | Witness x, y, z | Screenshot |
| ---: | ---: | --- | --- |
| 12 | 249.00 | 50.75, 31.08, 214.25 | cache/screens/holes/m11_01_00_00_12_context3.png |
| 14 | 201.25 | 224.75, -14.59, 165.75 | cache/screens/holes/m11_01_00_00_14_context.png |
| 19 | 187.50 | 283.25, -22.34, 134.25 | cache/screens/holes/m11_01_00_00_19_context.png |

Review 12: The wider view shows a continuous tiled castle roof with a substantial traversable slope. Body falls through the roof at the witness height, consistent with a collision hole..

Review 14: The wider view shows the castle roof as a substantial walkable sloped surface. All six nearby probe starts fall through at the witness height instead of moving beyond a real roof edge, consistent with a collision hole..

Review 19: The witness lies in an enclosed courtyard with a rendered dirt/grass floor, and the body-confirmed route reaches it. The floor reads as a reachable ground surface, so the fall is consistent with a collision hole..

### m11_02_00_00

| ID | Kind | Area m² | Centroid x, y, z | Nearest start (distance m) | Status | Screenshot |
| ---: | --- | ---: | --- | --- | --- | --- |
| 1 | floor_gap | 434.25 | 226.68, -89.18, 354.71 | 0 (99.0) | not reproduced | pending |
| 2 | floor_gap | 205.50 | 302.99, -38.30, 302.21 | 0 (15.9) | not reproduced | pending |
| 3 | floor_gap | 156.75 | 282.44, -40.77, 273.26 | 0 (41.2) | not reproduced | pending |
| 4 | floor_gap | 164.50 | 366.22, -46.84, 210.74 | 1 (90.4) | not reproduced | pending |
| 5 | floor_gap | 156.50 | 227.23, -87.86, 363.87 | 0 (103.0) | not reproduced | pending |
| 6 | floor_gap | 79.25 | 320.09, -47.92, 328.19 | 0 (28.1) | not reproduced | pending |
| 7 | floor_gap | 134.75 | 173.83, -21.96, 360.19 | 2 (121.6) | not reproduced | pending |
| 8 | floor_gap | 98.25 | 173.99, -26.58, 360.05 | 2 (120.9) | not reproduced | pending |
| 9 | floor_gap | 48.25 | 314.02, -49.76, 311.26 | 0 (11.7) | not reproduced | pending |
| 10 | floor_gap | 93.50 | 173.44, -33.01, 360.83 | 2 (121.2) | not reproduced | pending |

Visually confirmed reproductions (ranked):

| ID | Area m² | Witness x, y, z | Screenshot |
| ---: | ---: | --- | --- |
| 11 | 41.00 | 310.75, -59.20, 321.75 | cache/screens/holes/m11_02_00_00_11_context.png |
| 12 | 39.75 | 256.75, -68.46, 201.25 | cache/screens/holes/m11_02_00_00_12_context.png |

Review 11: Body reaches the dirt path between stone and timber retaining structures. The witness is on a rendered ground surface along a traversable-looking corridor, and the body drops through it; retain as a likely collision hole..

Review 12: The witness is beside a rock wall and tree-rooted ground near a player start. The wider view shows a ground-like surface rather than a decorative backdrop, and Body confirms a deep fall through the witness layer; retain as a likely collision hole..

Review 15: The witness is obscured by a tree trunk at the edge of a cliff, with visible underside and open mesh faces. Current views do not establish a standable ground surface, so the review is inconclusive..

### m13_00_00_00

| ID | Kind | Area m² | Centroid x, y, z | Nearest start (distance m) | Status | Screenshot |
| ---: | --- | ---: | --- | --- | --- | --- |
| 2 | floor_gap | 89.75 | 432.39, -55.62, 333.81 | 1 (35.0) | confirmed | cache/screens/holes/m13_00_00_00_2.png |
| 4 | floor_gap | 77.50 | 395.81, -54.18, 308.82 | 0 (36.7) | Body reproduced, visual pending/inconclusive | pending |
| 5 | floor_gap | 51.50 | 466.13, -59.77, 348.61 | 1 (6.2) | not reproduced | pending |
| 6 | floor_gap | 75.50 | 436.73, -56.05, 296.58 | 1 (61.5) | not reproduced | pending |
| 7 | floor_gap | 48.25 | 376.35, -44.13, 303.37 | 0 (19.3) | Body reproduced, visual pending/inconclusive | pending |
| 8 | floor_gap | 46.25 | 385.11, -43.54, 288.46 | 0 (15.9) | not reproduced | pending |
| 9 | floor_gap | 53.50 | 429.72, -49.07, 329.71 | 1 (40.4) | Body reproduced, visual pending/inconclusive | pending |
| 10 | floor_gap | 52.00 | 379.23, -78.85, 416.31 | 2 (37.7) | not reproduced | pending |
| 11 | floor_gap | 47.50 | 485.52, -68.66, 348.52 | 1 (26.3) | Body reproduced, visual pending/inconclusive | pending |
| 12 | floor_gap | 67.75 | 512.55, -71.86, 277.96 | 1 (91.4) | Body reproduced, visual pending/inconclusive | pending |

Visually confirmed reproductions (ranked):

| ID | Area m² | Witness x, y, z | Screenshot |
| ---: | ---: | --- | --- |
| 2 | 89.75 | 432.25, -55.79, 333.25 | cache/screens/holes/m13_00_00_00_2.png |

Review 1: Body reproduced a 172.75 m fall, but the stable eye view places the witness in a tight rock crevice with overlapping mesh faces and no traversable opening. This looks like decorative or unreachable ground..

Review 2: Body reproduced a 185.01 m fall. The screenshot clearly shows a broad, rendered dirt floor beside a large cave boulder at the witness height. This is a standable-looking surface aligned with the strict body fall, so retain as a plausible collision hole..

Review 3: Body reproduced a 125.48 m fall, but the stable view shows the candidate path enclosed by overlapping rock faces with only a narrow crevice floor. This appears to be decorative or unreachable rock geometry..

### m15_00_00_00

| ID | Kind | Area m² | Centroid x, y, z | Nearest start (distance m) | Status | Screenshot |
| ---: | --- | ---: | --- | --- | --- | --- |
| 1 | floor_gap | 553.50 | 126.76, -390.62, 1445.37 | 4 (30.3) | not reproduced | pending |
| 2 | floor_gap | 212.75 | 136.16, -364.45, 768.76 | 1 (34.5) | not reproduced | pending |
| 3 | floor_gap | 221.25 | 96.35, -364.38, 796.67 | 1 (69.5) | not reproduced | pending |
| 4 | floor_gap | 163.00 | 209.27, -417.02, 1203.29 | 3 (35.7) | not reproduced | pending |
| 5 | floor_gap | 230.25 | 40.59, -400.30, 1444.32 | 4 (114.0) | confirmed | cache/screens/holes/m15_00_00_00_5.png |
| 6 | floor_gap | 193.50 | 124.14, -392.73, 1350.21 | 4 (86.6) | confirmed | cache/screens/holes/m15_00_00_00_6.png |
| 7 | floor_gap | 160.00 | 134.63, -369.42, 919.00 | 2 (137.0) | not reproduced | pending |
| 8 | floor_gap | 155.25 | 136.98, -375.94, 918.43 | 2 (132.7) | not reproduced | pending |
| 9 | floor_gap | 80.75 | 151.65, -381.96, 1407.35 | 4 (26.7) | Body reproduced, visual pending/inconclusive | cache/screens/holes/m15_00_00_00_9.png |
| 10 | floor_gap | 85.25 | 173.97, -408.39, 1240.41 | 3 (38.5) | Body reproduced, visual pending/inconclusive | pending |

Visually confirmed reproductions (ranked):

| ID | Area m² | Witness x, y, z | Screenshot |
| ---: | ---: | --- | --- |
| 5 | 230.25 | 36.25, -400.34, 1440.75 | cache/screens/holes/m15_00_00_00_5.png |
| 6 | 193.50 | 122.75, -392.93, 1349.25 | cache/screens/holes/m15_00_00_00_6.png |

Review 5: The Body sweep falls over 70 m beneath a visible dirt and rock floor near the witness. The camera is placed just above the Body start with Display groups and far helper pieces hidden; the exposed ground appears walkable and the loss is consistent with a real floor hole..

Review 6: The Body sweep falls over 52 m beneath the witness layer. The screenshot shows a leaf-covered dirt slope that appears standable, though a foreground rock edge partly obscures the exact spot; this is a plausible floor collision hole..

Review 9: The Body reproduces a deep fall, but the camera view shows a boulder, tree trunks, and open void without a clear reachable floor patch at the witness. The screenshot does not distinguish a walkable hole from a cliff-side surface..

### m17_00_00_00

| ID | Kind | Area m² | Centroid x, y, z | Nearest start (distance m) | Status | Screenshot |
| ---: | --- | ---: | --- | --- | --- | --- |
| 1 | floor_gap | 1153.50 | 418.73, -220.67, 576.16 | 2 (86.6) | not reproduced | pending |
| 2 | floor_gap | 773.00 | 575.24, -296.00, 734.27 | 9 (48.5) | not reproduced | pending |
| 3 | floor_gap | 523.00 | 403.47, -168.36, 616.22 | 2 (50.7) | not reproduced | pending |
| 4 | floor_gap | 181.75 | 395.98, -313.39, 804.40 | 3 (67.9) | Body reproduced, visual pending/inconclusive | cache/screens/holes/m17_00_00_00_4-alt.png |
| 5 | floor_gap | 98.25 | 657.29, -281.05, 709.07 | 4 (18.7) | Body reproduced, visual pending/inconclusive | cache/screens/holes/m17_00_00_00_5-alt.png |
| 6 | floor_gap | 116.25 | 411.38, -180.84, 621.89 | 2 (61.8) | not reproduced | pending |
| 7 | floor_gap | 90.75 | 591.81, -281.71, 710.19 | 9 (60.5) | not reproduced | pending |
| 8 | floor_gap | 73.25 | 557.30, -286.00, 759.29 | 9 (44.7) | confirmed | cache/screens/holes/m17_00_00_00_8-alt.png |
| 9 | floor_gap | 73.50 | 164.98, -376.20, 618.55 | 7 (46.1) | Body reproduced, visual pending/inconclusive | pending |
| 10 | floor_gap | 70.50 | 607.43, -319.44, 774.45 | 9 (48.2) | not reproduced | pending |

Visually confirmed reproductions (ranked):

| ID | Area m² | Witness x, y, z | Screenshot |
| ---: | ---: | --- | --- |
| 8 | 73.25 | 559.75, -285.53, 757.75 | cache/screens/holes/m17_00_00_00_8-alt.png |

Review 4: Body reproduced a 32.95 m fall. The view shows a rough rock outcrop and broken cliff mesh, but does not establish whether the upward-facing witness is a standable ledge or inaccessible decoration..

Review 5: Body reproduced a 12.30 m fall and the nearest player start is 18.66 m away. Screenshots show dark rock walls around a narrow crevice, but no clearly standable surface at the witness; the view cannot distinguish a reachable floor edge from inaccessible cliff geometry..

Review 8: Body reproduced a 60.44 m fall. The offset view shows broad, leaf-covered ground with scattered rocks at the witness height, a visibly standable surface consistent with a collision gap. No rendered opening is needed to explain the fall..

### m20_00_00_00

| ID | Kind | Area m² | Centroid x, y, z | Nearest start (distance m) | Status | Screenshot |
| ---: | --- | ---: | --- | --- | --- | --- |
| 1 | floor_gap | 3416.25 | 859.71, 131.32, 552.39 | 4 (18.3) | not reproduced | pending |
| 2 | floor_gap | 1769.50 | 604.92, 10.76, 479.02 | 2 (109.1) | not reproduced | pending |
| 3 | floor_gap | 910.50 | 823.98, 125.66, 643.84 | 5 (14.7) | not reproduced | pending |
| 4 | floor_gap | 679.25 | 831.87, 124.10, 561.17 | 4 (12.5) | not reproduced | pending |
| 5 | floor_gap | 857.50 | 816.43, 95.28, 444.22 | 3 (46.6) | not reproduced | pending |
| 6 | floor_gap | 664.50 | 651.73, 9.93, 460.41 | 2 (60.1) | not reproduced | pending |
| 7 | floor_gap | 893.75 | 709.00, 63.70, 561.51 | 2 (135.7) | not reproduced | pending |
| 8 | floor_gap | 716.25 | 745.45, 87.75, 575.08 | 5 (100.2) | not reproduced | pending |
| 9 | floor_gap | 448.50 | 857.11, 143.32, 572.82 | 4 (28.0) | not reproduced | pending |
| 10 | floor_gap | 488.50 | 649.56, 19.90, 460.23 | 2 (60.1) | not reproduced | pending |

Review 36026: The wider view shows dense tree canopy beside rooftops, while the witness area remains obscured by overlapping branches and foliage. The capture does not establish a standable floor or a definite solid wall crossing, so the result is inconclusive..

Review 36028: The wider view places the witness near a narrow rocky ravine, but foliage and the cliff face still obscure the exact crossing. The capture does not establish a standable surface or prove whether Body crossed a solid wall, so the result is inconclusive..

Review 36034: The close view shows a wooden plank underside and the wider capture is empty sky at this location. The witness surface cannot be identified as a walkable surface or a definite wall, so the result is inconclusive..

### m25_00_00_00

| ID | Kind | Area m² | Centroid x, y, z | Nearest start (distance m) | Status | Screenshot |
| ---: | --- | ---: | --- | --- | --- | --- |
| 1 | floor_gap | 1171.25 | 177.22, 82.40, 473.34 | 3 (65.9) | not reproduced | pending |
| 2 | floor_gap | 1069.25 | -55.34, 81.55, 74.25 | 2 (84.5) | not reproduced | pending |
| 3 | floor_gap | 980.25 | 163.56, 84.55, 488.52 | 9 (71.8) | not reproduced | pending |
| 4 | floor_gap | 894.00 | 66.09, 86.92, 343.16 | 7 (90.3) | not reproduced | pending |
| 5 | floor_gap | 737.25 | 40.46, 120.06, 546.04 | 5 (59.9) | not reproduced | pending |
| 6 | floor_gap | 677.50 | -89.16, 70.11, 540.67 | 8 (90.1) | not reproduced | pending |
| 7 | floor_gap | 673.25 | -104.00, 105.71, 431.34 | 8 (96.7) | not reproduced | pending |
| 8 | floor_gap | 404.00 | 12.46, 114.60, 460.56 | 4 (21.6) | not reproduced | pending |
| 9 | floor_gap | 576.50 | 98.91, 94.16, 362.99 | 3 (74.4) | not reproduced | pending |
| 10 | floor_gap | 755.75 | 15.93, 81.40, 321.72 | 7 (130.1) | not reproduced | pending |

Visually confirmed reproductions (ranked):

| ID | Area m² | Witness x, y, z | Screenshot |
| ---: | ---: | --- | --- |
| 41045 | 0.00 | 171.46, 81.44, 469.99 | cache/screens/holes/m25_00_00_00_41045.png |

Review 41039: Body reproduces a render-wall crossing beside a tiled roof and timber structure. The screenshot establishes solid-looking architecture but does not isolate the exact crossed face, so the wall defect remains visually inconclusive..

Review 41042: Body reproduces a render-wall crossing, but the view is occluded by roof and rock geometry and does not identify the exact solid face..

Review 41045: Body crosses a rendered boulder beside visible ground, and the screenshot shows a substantial solid-looking rock at the witness. Retain as a solid-geometry crossing, even though it causes no fall. The enclosure parity label does not prove that the entire mesh is watertight..

## Reproduce

```powershell
$env:CARGO_TARGET_DIR='target/alt'
$env:CARGO_BUILD_JOBS='6'
$env:RAYON_NUM_THREADS='6'
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --release -p sekiro-sim --example collision_holes -j 6
& "$env:USERPROFILE\.cargo\bin\cargo.exe" build --release -p sekiro-sim --example collision_holes -j 6
.\target\alt\release\examples\collision_holes.exe cache/maps --spacing 0.5 --probes 2048
.\target\alt\release\examples\collision_holes.exe --report
```

Pass map IDs after `cache/maps` to audit a subset.
Use `--recheck` with map IDs to repeat Body validation of cached candidates without rerunning the random probes or clustering; geometry is reloaded, and scan and validation times are recorded separately.
Recheck is valid only when geometry, spacing and probe count are unchanged.
`--report` merges ID-and-coordinate-matched reviews from `cache/holes/<map>.review.json`, annotates per-map JSON and regenerates this document.
Screenshot review is manual and must be repeated if geometry or thresholds change.
Viewer command: `target/alt/release/sekiro-rs.exe --map <id> --groups off --camera x,y,z,yaw,pitch --screenshot <absolute repo path> --at 1`.
Capture from the reproduction start plus 1.4 m eye height when a high camera is inside a ceiling.
Mibu review captures use `--groups display --hide m9,m8` to remove occluding far/helper pieces; the audit still loads all map pieces.
Every viewer must be capped at 90 seconds and killed by its exact process ID on timeout.

## Verification and changed files

Six tests pass: missing-patch detection, solid-floor detection, a 100 m thin-wall sweep, Body reproduction and unstable-start rejection, BVH ray equivalence, and bounded rasterization of a 20 km background floor.
The tool uses a dev-only workspace Rayon dependency in `crates/sim/Cargo.toml`; implementation and tests are in `crates/sim/examples/collision_holes.rs`.
Cargo.lock records the dev-only Rayon dependency automatically.
This report and STATUS.md are the tracked documentation changes.
No collision engine code, protected brain code, game install files, links, junctions or commits are changed by this task.
Final workspace check results are recorded in STATUS.md.
