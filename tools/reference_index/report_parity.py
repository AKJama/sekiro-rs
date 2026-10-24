"""Aggregate external human pair annotations into a separate Markdown summary.

No defect is inferred from a score or from an inventory bounding box.
"""
from __future__ import annotations

import argparse
from collections import Counter
import json
from pathlib import Path

if __package__:
    from . import collection
else:
    import collection


def require_story(poses, story):
    captured_stories = {p.get('render_settings', {}).get('story', 'unrecorded') for p in poses}
    if poses and captured_stories != {story}:
        raise ValueError(f'Expected {story} captures, found {sorted(captured_stories)}; regenerate pairs in a fresh output directory or explicitly select their historical --story')


def report(root, output, annotation_path=None, story=None):
    poses = json.loads((root / 'poses.json').read_text())
    story = collection.story(root, story)
    require_story(poses, story)
    annotation_path = annotation_path or (root / 'triage.json' if (root / 'triage.json').exists()
                                          else root / 'pairs' / 'triage.json')
    annotations = json.loads(annotation_path.read_text())
    by_shot = {row['shot']: row for row in poses}
    reviews = {row['shot']: row for row in annotations['pairs']}
    accepted = {shot for shot, row in reviews.items() if row['camera_review'] == 'accepted'}
    rejected = {shot for shot, row in reviews.items() if row['camera_review'] == 'rejected'}
    pending = sorted(set(by_shot)-accepted-rejected)
    excluded = sorted(set(by_shot)-accepted)
    occurrences = {}
    for shot, row in reviews.items():
        if shot not in accepted or shot not in by_shot:
            continue
        for difference in row['differences']:
            occurrences.setdefault(difference['cause'], []).append((by_shot[shot], difference))
    def priority(cause):
        rows = occurrences[cause]
        spots = {pose['spot'] for pose, _ in rows}
        severity = annotations['causes'][cause]['severity']
        return len(spots)*severity, len(spots), severity
    ranked = sorted(occurrences, key=priority, reverse=True)
    lines = [
        '# Visual parity work list: Ashina Outskirts and Castle', '',
        'This is a visual-difference triage, not a renderer fix or a claim of exact retail parity.',
        'Reference paths below are relative to the external reference root supplied to the tools.',
        'Source screenshots, poses, pair images, ranked searches, visibility diagnostics and human annotations remain outside the repository.',
        '', '## Coverage and evidence', '',
        f'The current capture set contains {len(poses)} shots at {len({r["spot"] for r in poses})} gameplay spots.',
        f'{len(accepted & set(by_shot))} pairs have passed visual camera review; {len(rejected & set(by_shot))} are rejected and {len(pending)} remain unreviewed.',
        'The two menu-only spots are excluded rather than assigned a camera.',
        'The selected gameplay images prefer hidden HUDs and the settled default view, with second views at nearby idol anchors.',
        'The reference travel destinations and overcast daylight support the post-Genichiro progression preset.',
        'A fixture difference is not automatically a missing model or an incorrect render group.',
        'Wolf, other characters and HUD differences are ignored.',
        '',
        'One windowless process per area loads the bounded connected neighbourhood and renders every selected shot at a spot in that process.',
        'Search images are 384 x 216; final real | ours | structural-difference PNGs retain 2560 x 1440 pixels per panel.',
        'The default camera parameters are loaded from the extracted tables: 43-degree vertical FOV, 4.5 m target distance and 1.5 m focus height.',
        'The final matcher supplements its coarse grid with Play-style wall-pulled candidates, using 0.3 m clearance and a 0.4 m minimum.',
        'The ray reads both Ashina collision caches and excludes hit materials 22, 36 and 38 as the simulation does.',
        'Those candidates supplement the image fit rather than constrain it, because exported helper/live-state collision and the retail camera filters are not known to be identical.',
        'Yaw and pitch use Bevy YXZ degrees, with positive pitch looking upward; camera positions use the already converted Bevy frame.',
        'The search uses a 30-degree coarse grid, masked local-contrast correlation, two yaw basins, exact-render refinement and verified subdegree fits.',
        'Practice-tool backing/text, HUD bars/corners, crosshair and a conservative Wolf silhouette envelope are excluded from scoring.',
        'The exact-render score combines local contrast structure, unsigned gradient orientation and symmetric thin-edge alignment; lower is better.',
        'A low score alone does not establish a correct camera, and a poor pose contributes no geometry-defect count.',
        f'Captured story presets: {", ".join(sorted({p.get("render_settings", {}).get("story", "unrecorded") for p in poses}))}; native exposure, fixed shader phase and normal CPU lighting/LOD settling.',
        '',
        'Lighting, fog and sky are being changed by another agent during this job.',
        'Each pose records its capture executable SHA-256; the visible lighting differences may shift after that other work is integrated.',
        'The capture job does not modify the lighting, fog, sky or material shader implementations.',
        '',
        '`poses.json` records each selected shot, camera position, yaw, pitch, distance, vertical FOV and match score.',
        '`pairs/*.search.json` retains ranked coarse/refined candidates.',
        '`pairs/*.visibility.json` calls the same per-part diagnostic used by --visibility-part, including source/effective masks, request sources, event state, readiness and residency.',
        '`pairs/*.inventory.json` projects exported part bounds and carries model/material provenance where generated.',
        'These bounds are conservative candidates, not pixel ownership or proof that an occluded part contributes to the image.',
        '`pairs/triage.json` lists the visible differences and camera review for each reviewed pair.',
        '`pairs/review/coarse/` retains coarse camera contact sheets for manual pose review.',
        '', '## Ranked work list', '',
        'Counts are distinct reviewed spot IDs, not percentages of the area or independent pixels.',
        'Priority is observed spot count multiplied by a 1-5 visual-prominence grade.',
        'This keeps widespread, prominent causes ahead of isolated defects without converting a similarity score into a defect metric.',
        '', '| Rank | Cause | Kind | Spots / pairs | Prominence | Owner |',
        '|---:|---|---|---:|---:|---|',
    ]
    for i, cause in enumerate(ranked, 1):
        item = annotations['causes'][cause]
        rows = occurrences[cause]
        count = len({pose['spot'] for pose, _ in rows})
        lines.append(f'| {i} | {cause}: {item["title"]} | {item["kind"]} | {count} / {len(rows)} | {item["severity"]} | {item["track"]} |')
    for cause in ranked:
        item = annotations['causes'][cause]
        rows = occurrences[cause]
        lines += ['', f'### {cause}: {item["title"]}', '',
                  f'Kind: {item["kind"]}.', f'Owner: **{item["track"]}**.',
                  f'Observed in {len({pose["spot"] for pose, _ in rows})} reviewed spots and {len(rows)} pairs.',
                  item['likely_cause'], item['confidence']]
        if item.get('provenance'):
            provenance = item['provenance']
            lines.extend(provenance if isinstance(provenance, list) else [provenance])
        examples = []
        seen = set()
        # Prefer both areas before filling the remaining examples in capture order.
        for map_id in sorted({pose['map'] for pose, _ in rows}):
            pair = next((r for r in rows if r[0]['map'] == map_id), None)
            if pair:
                examples.append(pair)
                seen.add(pair[0]['shot'])
        for pair in rows:
            if len(examples) >= 4:
                break
            if pair[0]['shot'] not in seen:
                examples.append(pair)
                seen.add(pair[0]['shot'])
        lines += ['', 'Examples:', '']
        for pose, difference in examples:
            path = Path(pose['pair']).as_posix()
            text = difference.get('note') or 'See the real and rewrite panels for this occurrence.'
            lines.append(f'- {pose["spot"]}, shot {pose["shot"]}: `{path}`.')
            lines.append(f'  {text}')
    lines += ['', '## Camera exceptions and unclassified differences', '']
    if excluded:
        lines.append('The following captures remain excluded from accepted geometry counts:')
        lines.append('')
        for shot in excluded:
            pose = by_shot[shot]
            note = reviews.get(shot, {}).get('notes') or 'Visual review pending.'
            lines.append(f'- {pose["spot"]}, shot {shot}: {note}')
    else:
        lines.append('Every selected pair has a visual camera review and a per-pair difference list in the external triage ledger.')
    lines += ['', '## Reproduce and verify', '',
              'All Cargo commands use CARGO_TARGET_DIR=target/alt-story and six build jobs.',
              'The reference root is an external path supplied at runtime; no personal path is embedded in tracked source.',
              '', '```powershell',
              "$env:CARGO_TARGET_DIR = 'target/alt-story'",
              "$referenceRoot = '<external-reference-root>'",
              "$captureRoot = '<fresh-external-story-capture-root>'",
              'cargo build --release --offline -p sekiro-game --bin sekiro-rs -j 6',
              'uv run --no-project --with pillow --with numpy --with scipy python -m tools.reference_index.regenerate_pairs --reference $referenceRoot --output $captureRoot --game target/alt-story/release/sekiro-rs.exe --annotations <review-ledger.json>',
              'uv run --no-project --with pillow --with numpy --with scipy python -m tools.reference_index.triage_pairs --reference $captureRoot --maps cache/maps --inventory --sheets',
              'uv run --no-project python -m tools.reference_index.report_parity --reference $captureRoot --annotations <renewed-review-ledger.json> --output <external-summary.md>',
              'cargo test --release --offline -p sekiro-game -j 6',
              'cargo clippy --release --offline -p sekiro-game --all-targets -j 6 -- -D warnings',
              'cargo fmt --all --check', '```', '',
              'Use external --seeds and --only-shots with --force for a reviewed camera rerun; completed poses are replaced only after a successful capture.',
              'No game install, retail process, save, live memory, cache export, commit or publication is required by this workflow.', '']
    output.write_text('\n'.join(lines), encoding='utf-8')
    print(json.dumps({'poses':len(poses), 'reviewed':len(accepted & set(by_shot)),
                      'rejected':len(rejected & set(by_shot)), 'pending':len(pending),
                      'causes':len(ranked), 'areas':dict(Counter(r['map'] for r in poses))}))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--reference', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--annotations', type=Path, help='External review ledger; defaults to pairs/triage.json')
    parser.add_argument('--story', help='Expected captured preset; defaults to the collection.json story (genichiro-defeated without one)')
    args = parser.parse_args()
    report(args.reference, args.output, args.annotations, args.story)


if __name__ == '__main__':
    main()
