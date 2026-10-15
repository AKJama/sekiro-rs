# /// script
# requires-python = ">=3.12"
# dependencies = ["pillow>=12", "numpy>=2", "scipy>=1.16"]
# ///
"""Contact sheets and projected part inventories for human visual-parity review.

Generated inventories identify candidates, not pixel ownership or proven defects.
"""
from __future__ import annotations

import argparse
import itertools
import io
import json
import math
import struct
import time
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFont
from scipy.spatial.transform import Rotation

from tools.reference_index.match_poses import write_json


def sheets(root, poses, per_sheet=6, output=None):
    output = output or root / 'pairs' / 'review'
    output.mkdir(parents=True, exist_ok=True)
    for page, start in enumerate(range(0, len(poses), per_sheet), 1):
        subset = poses[start:start+per_sheet]
        sheet_path = output / f'sheet-{page:02}.jpg'
        # Pose insertion/reordering can change page membership without changing
        # any image timestamp. Rebuild rather than reuse a stale labelled page.
        sheet = Image.new('RGB', (1920, len(subset)*408), '#181818')
        for i, pose in enumerate(subset):
            with Image.open(root / pose['pair']) as image:
                sheet.paste(image.resize((1920, 384), Image.Resampling.LANCZOS), (0, i*408+24))
            cost_label = 'saved cost' if pose.get('source_pose_renderer_sha256') else 'cost'
            ImageDraw.Draw(sheet).text((5, i*408+3),
                f'{pose["spot"]} | shot {pose["shot"]} | yaw {pose["yaw"]:.1f} pitch {pose["pitch"]:.1f} distance {pose["distance"]:.1f} | {cost_label} {pose["match_score"]:.3f}',
                fill='white', font=ImageFont.load_default(size=19))
        sheet.save(sheet_path, quality=92)


def watch_coarse(root):
    """Archive contact sheets while the controller consumes candidate rasters.

    Watch atomic batch completions; never modify the queue or source images.
    Missing already-consumed images leave labelled empty cells for later review.
    """
    spots = json.loads((root / 'spots.json').read_text())
    output = root / 'pairs' / 'review' / 'coarse'
    output.mkdir(parents=True, exist_ok=True)
    cursors = {block: 1 for block in ('m11_00_00_00', 'm11_01_00_00')}
    sessions = {}
    while not (root / 'pose-search' / 'archive-stop').exists():
        advanced = False
        for block, cursor in list(cursors.items()):
            directory = root / 'pose-search' / block
            session = directory / 'session.json'
            if session.exists():
                token = json.loads(session.read_text())['id']
                if sessions.get(block) != token:
                    sessions[block] = token
                    cursors[block] = cursor = 1
            done = directory / f'batch-{cursor:05}.done.json'
            try:
                if session.exists() and done.stat().st_mtime_ns < session.stat().st_mtime_ns:
                    continue
                rows = json.loads(done.read_text())
            except FileNotFoundError:
                # A resumed controller removes stale results before publishing
                # the next atomic completion with the same batch name.
                continue
            cursors[block] += 1
            advanced = True
            meta_path = directory / f'batch-{cursor:05}.meta.json'
            meta = json.loads(meta_path.read_text()) if meta_path.exists() else None
            if not ((meta and meta.get('kind') == 'coarse') or (meta is None and len(rows) == 48)):
                continue
            first = rows[0]['view']
            if meta:
                spot = next((s for s in spots if s['spot_id'] == meta.get('spot')), None)
                if spot is None:
                    player = meta['requests'][0]['player']
                    spot = min(spots, key=lambda s: np.linalg.norm(np.array(s['position'])-player))
            else:
                # Older batches predate explicit player metadata and had a fixed
                # coarse distance. Their contact sheets remain useful evidence.
                rot = Rotation.from_euler('YXZ', [first[3], first[4], 0], degrees=True).as_matrix()
                player = np.array(first[:3]) - rot[:, 2]*4.5 - [0, 1.5, 0]
                spot = min(spots, key=lambda s: np.linalg.norm(np.array(s['position'])-player))
                if np.linalg.norm(np.array(spot['position'])-player) > .01:
                    continue
            requests = {r['id']: r for r in meta['requests']} if meta else {}
            sheets_out = [Image.new('RGB', (1920, 816), '#101215') for _ in range(4 if len(rows) > 48 else 2)]
            present = 0
            for i, row in enumerate(rows):
                request = requests.get(row['id'], {})
                column = int(round(row['view'][3]/30)) % 12
                pitch_row = [-60, -30, 0, 30].index(int(round(row['view'][4])))
                page, column = divmod(column, 6)
                if request.get('obstruction_variant'):
                    page += 2
                image_path = directory / f'{row["id"]:07}.png'
                draw = ImageDraw.Draw(sheets_out[page])
                draw.text((column*320+4, pitch_row*204+4),
                          f'yaw {row["view"][3]:.0f} pitch {row["view"][4]:.0f} d {request.get("actual_distance", 4.5):.1f}', fill='white')
                try:
                    # Release the OS file before PNG decoding; the matcher owns
                    # deletion and must not wait for this optional review sheet.
                    with Image.open(io.BytesIO(image_path.read_bytes())) as image:
                        tile = image.convert('RGB').resize((320, 180))
                    sheets_out[page].paste(tile, (column*320, pitch_row*204+24))
                    present += 1
                except FileNotFoundError:
                    pass
            if present:
                for page, image in enumerate(sheets_out, 1):
                    image.save(output / f'{spot["spot_id"]}-{page}.jpg', quality=92)
                print(f'archived {spot["spot_id"]}: {present}/48 coarse views', flush=True)
        if not advanced:
            time.sleep(.01)


def glb_materials(path):
    with path.open('rb') as f:
        header = f.read(20)
        length = struct.unpack_from('<I', header, 12)[0]
        document = json.loads(f.read(length))
    return [{'name': m.get('name'), 'extras': m.get('extras')}
            for m in document.get('materials', [])]


def inventory(root, maps, poses):
    layouts = {}
    models = {}
    material_cache = {}
    for pose in poses:
        manifest_path = root / pose['visibility']
        inventory_path = root / 'pairs' / f'{pose["spot"]}-shot-{pose["shot"]:03}.inventory.json'
        summary_path = inventory_path.with_name(inventory_path.name.replace('.inventory.', '.summary.'))
        if (inventory_path.exists() and summary_path.exists()
                and min(inventory_path.stat().st_mtime, summary_path.stat().st_mtime) > manifest_path.stat().st_mtime):
            continue
        manifest = json.loads(manifest_path.read_text())['visibility']
        write_json(summary_path, {
            'shot': pose['shot'], 'selected_collision': manifest['selected_collision'],
            'parts': len(manifest['parts']),
            'visible': sum(p['visible'] for p in manifest['parts']),
            'event_enabled': sum(p['event_enabled'] for p in manifest['parts']),
            'group_visible': sum(p['group_visible'] for p in manifest['parts']),
            'ready': sum(p['ready'] for p in manifest['parts']),
            'resident': sum(p['resident'] for p in manifest['parts']),
        })
        rotation = Rotation.from_euler('YXZ', [pose['yaw'], pose['pitch'], 0], degrees=True).as_matrix()
        eye = np.asarray(pose['camera_position'])
        fy = math.tan(math.radians(pose['fov']/2))
        fx = fy * 16/9
        rows = []
        for decision in manifest['parts']:
            block = decision['map']
            if block not in layouts:
                layout = json.loads((maps / block / 'layout.json').read_text())
                layouts[block] = {r['name']: r for r in layout['parts']}
                models[block] = layout['models']
            part = layouts[block].get(decision['part'])
            if not part or part.get('model') not in models[block]:
                continue
            model = models[block][part['model']]
            bounds = np.array(list(itertools.product(*zip(model['min'], model['max']))))
            part_rotation = Rotation.from_quat(part['rotation']).as_matrix()
            world = (bounds * part['scale']) @ part_rotation.T + part['translation']
            view = (world-eye) @ rotation
            if np.all(view[:, 2] >= -.05):
                continue
            # Clip the box's twelve edges at the eye plane before projection.
            # A box far to the side can straddle that plane without covering
            # the image, so assigning every such box full-frame is misleading.
            if np.any(view[:, 2] >= -.05):
                clipped = list(view[view[:, 2] < -.05])
                for i in range(8):
                    for bit in (1, 2, 4):
                        j = i ^ bit
                        if i < j and ((view[i, 2] < -.05) != (view[j, 2] < -.05)):
                            t = (-.05-view[i, 2]) / (view[j, 2]-view[i, 2])
                            clipped.append(view[i] + t*(view[j]-view[i]))
                view = np.array(clipped)
            xy = view[:, :2] / -view[:, 2:3] / [fx, -fy] * .5 + .5
            lo, hi = xy.min(axis=0), xy.max(axis=0)
            if (hi < 0).any() or (lo > 1).any():
                continue
            box = [*np.maximum(lo, 0).tolist(), *np.minimum(hi, 1).tolist()]
            area = (box[2]-box[0])*(box[3]-box[1])
            if area < .001:
                continue
            rows.append({'map': block, 'part': part['name'], 'model': part['model'],
                         'kind': part['kind'], 'screen_bounds': box,
                         'bounds_screen_fraction': area,
                         'distance_to_origin': float(np.linalg.norm(np.array(part['translation'])-eye)),
                         'visible': decision['visible'], 'event_enabled': decision['event_enabled'],
                         'group_visible': decision['group_visible'],
                         'ready': decision['ready'], 'resident': decision['resident'],
                         'entity_id': part.get('entity_id'),
                         'entity_groups': part.get('entity_group_ids'),
                         'asset': model['glb']})
        rows.sort(key=lambda r: (not r['visible'], r['distance_to_origin']))
        # Read native material provenance only for in-frustum candidate models.
        for row in rows:
            key = row['map'], row['model']
            if key not in material_cache:
                material_cache[key] = glb_materials(maps / row['map'] / row['asset'])
            row['materials'] = material_cache[key]
        write_json(inventory_path, rows)
        print(f'inventory shot {pose["shot"]}: {len(rows)} bounding-box candidates', flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--reference', type=Path, required=True)
    parser.add_argument('--maps', type=Path)
    parser.add_argument('--sheets', action='store_true')
    parser.add_argument('--sheet-output', type=Path, help='Writable contact-sheet folder for a read-only capture root')
    parser.add_argument('--inventory', action='store_true')
    parser.add_argument('--watch-coarse', action='store_true')
    args = parser.parse_args()
    if args.watch_coarse:
        watch_coarse(args.reference)
        return
    poses = json.loads((args.reference / 'poses.json').read_text())
    if args.sheets:
        sheets(args.reference, poses, output=args.sheet_output)
    if args.inventory:
        if not args.maps:
            parser.error('--inventory requires --maps')
        inventory(args.reference, args.maps, poses)


if __name__ == '__main__':
    main()
