# /// script
# requires-python = ">=3.12"
# dependencies = ["pillow>=12", "numpy>=2", "scipy>=1.16"]
# ///
"""Search external retail shots against one persistent windowless map renderer.

Run through uv with the declared dependencies, or an existing uv environment.
All image/JSON output is required to be outside the source repository.
"""
from __future__ import annotations

import argparse
import hashlib
import itertools
import json
import math
import re
import subprocess
import time
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw
from scipy import ndimage as ndi
from scipy.signal import fftconvolve

if __package__:
    from . import collection
    from .pose_collision import CameraCollision
    from .report_parity import require_story
else:
    import collection
    from pose_collision import CameraCollision
    from report_parity import require_story


class FeatureSet(list):
    def __init__(self, levels, local, oriented):
        super().__init__(levels)
        self.local = local
        self.oriented = oriented


def write_json(path, value):
    tmp = path.with_suffix('.tmp')
    tmp.write_text(json.dumps(value, indent=2), encoding='utf-8')
    tmp.replace(path)


def valid_mask(size):
    """Conservative fixed masks in normalized retail image coordinates.

    Exclude practice tool including its dark backing, HUD corners/bars,
    crosshair and the entire central lower Wolf/weapon silhouette envelope.
    """
    w, h = size
    image = Image.new('L', size, 255)
    draw = ImageDraw.Draw(image)
    for box in [(0, 0, .225, .27), (0, .79, .30, 1),
                (.73, .73, 1, 1), (.28, 0, .73, .12),
                (.46, .40, .54, .55)]:
        draw.rectangle(tuple(round(v * (w if i % 2 == 0 else h)) for i, v in enumerate(box)), fill=0)
    draw.polygon([(int(x*w), int(y*h)) for x, y in
                  [(.35, 1), (.37, .75), (.41, .47), (.59, .47), (.65, 1)]], fill=0)
    return np.array(image) > 0


def features(image, size=(256, 144)):
    gray = np.asarray(image.convert('L').resize(size, Image.Resampling.LANCZOS), dtype=np.float32)
    mask = valid_mask(size)
    levels = []
    for sigma in (1.1, 2.5):
        smooth = ndi.gaussian_filter(gray, sigma)
        gx, gy = ndi.sobel(smooth, axis=1), ndi.sobel(smooth, axis=0)
        magnitude = np.hypot(gx, gy)
        # Contrast-normalized edge selection tolerates exposure and colour shifts.
        threshold = max(3., float(np.percentile(magnitude[mask], 78)))
        yy, xx = np.indices(magnitude.shape, dtype=np.float32)
        nx, ny = gx / np.maximum(magnitude, 1e-6), gy / np.maximum(magnitude, 1e-6)
        before = ndi.map_coordinates(magnitude, [yy-ny, xx-nx], order=1, mode='nearest')
        after = ndi.map_coordinates(magnitude, [yy+ny, xx+nx], order=1, mode='nearest')
        # Thin gradient bands to ridges so unrelated textured images cannot win
        # merely by putting a dense edge within a pixel of every reference edge.
        edge = (magnitude >= threshold) & (magnitude >= before) & (magnitude >= after) & mask
        angle = np.mod(np.arctan2(gy, gx), np.pi)
        bins = np.floor(angle / np.pi * 4).astype(int).clip(0, 3)
        distances = [ndi.distance_transform_edt(~(edge & (bins == b))) for b in range(4)]
        levels.append((edge, bins, np.stack(distances)))
    low = np.asarray(image.convert('L').resize((128, 72), Image.Resampling.LANCZOS), dtype=np.float32)
    low = ndi.gaussian_filter(low, 1.)
    residual = low - ndi.gaussian_filter(low, 6.)
    local = residual / np.sqrt(ndi.gaussian_filter(residual**2, 6.) + 10.)
    gy, gx = np.gradient(low)
    mag = np.hypot(gx, gy)
    angle = np.mod(np.arctan2(gy, gx), np.pi)
    oriented = np.stack([ndi.gaussian_filter(mag * np.maximum(0, np.cos(2*(angle-b*np.pi/4)))**4, 1.5)
                         for b in range(4)])
    oriented /= np.sqrt((oriented**2).mean(axis=(1, 2), keepdims=True)) + 1.
    return FeatureSet(levels, local, oriented)


def coarse_similarity(real, ours, radius=28):
    """Local contrast correlation over overlapping masks, allowing coarse pose error."""
    mask = valid_mask((128, 72)).astype(np.float32)
    a, b = real.local * mask, ours.local * mask
    def correlate(x, y):
        return fftconvolve(x, y[::-1, ::-1], mode='full')
    product = correlate(a, b)
    norm = np.sqrt(np.maximum(correlate(a*a, mask) * correlate(mask, b*b), 1e-8))
    overlap = correlate(mask, mask)
    yy, xx = np.indices(product.shape)
    dx, dy = xx-127, yy-71
    allowed = (np.abs(dx) <= radius) & (np.abs(dy) <= radius) & (overlap > mask.sum()*.45)
    score = np.where(allowed, product/norm - .025*np.hypot(dx, dy)/radius, -2.)
    y, x = np.unravel_index(np.argmax(score), score.shape)
    return 1.-float(score[y, x]), float(dx[y, x]), float(dy[y, x])


def similarity(real, ours):
    """Local structure, unsigned gradient orientation and symmetric edge alignment.

    Adjacent orientation bins receive a small penalty rather than a hard veto.
    Masked regions never supply or consume edge evidence.
    """
    scores = []
    for (ae, ab, ad), (be, bb, bd) in zip(real, ours):
        if min(ae.sum(), be.sum()) < 40:
            return 1e3
        def directed(edge, bins, distance):
            yy, xx = np.nonzero(edge)
            b = bins[yy, xx]
            d = np.minimum(distance[b, yy, xx],
                           np.minimum(distance[(b+1) % 4, yy, xx], distance[(b-1) % 4, yy, xx]) + 1.5)
            return float(np.minimum(d, 12).mean())
        scores.append((directed(ae, ab, bd) + directed(be, bb, ad)) / 2)
    mask = valid_mask((128, 72))
    def correlation(a, b):
        return float(np.mean(a*b) / np.sqrt(max(1e-8, np.mean(a*a)*np.mean(b*b))))
    ncc = correlation(real.local[mask], ours.local[mask])
    oriented = correlation(real.oriented[:, mask], ours.oriented[:, mask])
    return .65*(1-ncc) + .2*(1-oriented) + .15*min(float(np.mean(scores))/12, 1.)


def camera(player, yaw, pitch, distance, height):
    y, p = math.radians(yaw), math.radians(pitch)
    # Bevy YXZ positive pitch looks upward; back vector is rotation * +Z.
    back = np.array([math.sin(y)*math.cos(p), -math.sin(p), math.cos(y)*math.cos(p)])
    eye = np.array(player) + [0, height, 0] + back * distance
    return [*map(float, eye), yaw % 360, pitch]


def select_shots(index, spots, folder):
    by_file = {r['file']: r for r in index}
    result = []
    for spot in spots:
        eligible = [by_file[f] for f in spot['files']
                    if by_file[f]['status'] == 'ok' and by_file[f].get('position')]
        hidden = [r for r in eligible if r.get('hud') == 'hidden']
        eligible = hidden or eligible
        if not eligible:
            continue
        # The first hidden gameplay image is the user's settled default view.
        # Dense foliage and extreme upward views won the old edge-count ranking
        # despite being poor geometric anchors, so preserve capture order here.
        chosen = [eligible[0]]
        landmark = spot.get('nearest_landmark') or {}
        if len(eligible) >= 2 and landmark.get('distance', 1e6) < 4.:
            chosen.append(eligible[1])
        result.append((spot, chosen))
    return result


class Renderer:
    def __init__(self, args, map_id, selected):
        self.directory = args.output / 'pose-search' / map_id
        self.directory.mkdir(parents=True, exist_ok=True)
        write_json(self.directory / 'session.json', {'id': time.time_ns()})
        self.counter = 0
        self.batch = 0
        self.spot = None
        self.review_directory = args.output / 'pairs' / 'review' / 'coarse'
        self.review_directory.mkdir(parents=True, exist_ok=True)
        self.collision = CameraCollision.load(args.maps, map_id)
        # These surround every player in every yaw; direct static capture culling
        # is conservative over this union, including nearby shadow casters.
        views = [camera(s['position'], y, p, 4.5, 1.5)
                 for s, _ in selected for y in range(0, 360, 30) for p in (-60, 0, 60)]
        write_json(self.directory / 'load-views.json', views)
        for name in ('stop', 'ready', 'error.txt', 'request.json'):
            (self.directory / name).unlink(missing_ok=True)
        self.log = (self.directory / 'renderer.log').open('w', encoding='utf-8')
        self.process = subprocess.Popen([
            str(args.game.resolve()), '--sweep', '--map', map_id, '--objects',
            '--sweep-views', str(self.directory / 'load-views.json'),
            '--sweep-out', str(self.directory / 'unused-sweep'),
            '--sweep-width', '384', '--sweep-height', '216',
            '--pose-server', str(self.directory), '--story', args.story, *args.render_arg,
        ], stdout=self.log, stderr=subprocess.STDOUT,
           creationflags=subprocess.CREATE_NO_WINDOW | subprocess.BELOW_NORMAL_PRIORITY_CLASS)
        try:
            self.wait(self.directory / 'ready', 660)
        except BaseException:
            self.close()
            raise
        self.params = json.loads((self.directory / 'camera-params.json').read_text())

    def wait(self, path, timeout):
        start = time.monotonic()
        while not path.exists():
            if self.process.poll() is not None:
                raise RuntimeError(f'renderer exited {self.process.returncode}: {self.directory / "renderer.log"}')
            if time.monotonic() - start > timeout:
                raise TimeoutError(str(path))
            time.sleep(.1)

    def render(self, candidates, final=False, coarse=False):
        self.batch += 1
        batch = f'batch-{self.batch:05}'
        requests = []
        for candidate in candidates:
            self.counter += 1
            candidate = dict(candidate, id=self.counter)
            candidate['actual_distance'] = (
                self.collision.distance(candidate['player'], candidate['yaw'], candidate['pitch'], candidate['distance'],
                                        self.params['focus_height'], self.params['distance'])
                if candidate.get('obstruction_variant') else candidate['distance'])
            candidate['view'] = camera(candidate['player'], candidate['yaw'], candidate['pitch'],
                                       candidate['actual_distance'], self.params['focus_height'])
            candidate['fov'] = self.params['fov']
            candidate['width'], candidate['height'] = (2560, 1440) if final else (384, 216)
            candidate['manifest'] = final
            requests.append(candidate)
        done = self.directory / f'{batch}.done.json'
        done.unlink(missing_ok=True)
        write_json(self.directory / f'{batch}.meta.json', {'spot': self.spot, 'kind': 'coarse' if coarse else 'final' if final else 'search', 'requests': requests})
        write_json(self.directory / 'request.json', {'batch': batch, 'requests': requests})
        self.wait(done, max(240, len(requests) * 15))
        records = {r['id']: r for r in json.loads(done.read_text())}
        return [(r, self.directory / f'{r["id"]:07}.png', records[r['id']]) for r in requests]

    def close(self):
        (self.directory / 'stop').write_text('stop', encoding='utf-8')
        try:
            self.process.wait(timeout=30)
        except subprocess.TimeoutExpired:
            self.process.terminate()
            self.process.wait(timeout=15)
        self.log.close()


def grid(player, yaws, pitches, distances):
    return [dict(player=player, yaw=float(y % 360), pitch=float(p), distance=float(d))
            for y, p, d in itertools.product(yaws, pitches, distances) if -80 <= p <= 45]


def coarse_grid(renderer, player):
    candidates = grid(player, range(0, 360, 30), [-60, -30, 0, 30], [renderer.params['distance']])
    alternatives = []
    for candidate in candidates:
        distance = renderer.collision.distance(player, candidate['yaw'], candidate['pitch'], candidate['distance'],
                                                renderer.params['focus_height'], renderer.params['distance'])
        if distance < candidate['distance']-.05:
            alternatives.append(dict(candidate, obstruction_variant=True))
    return candidates + alternatives


def remove_candidate(path):
    # Windows readers/antivirus can briefly deny deletion even after capture.
    # Retry only sharing violations on this tool's generated candidate path.
    for attempt in range(100):
        try:
            path.unlink()
            return
        except PermissionError:
            if attempt == 99:
                raise
            time.sleep(.02)


def ranked(renderer, candidates, reference_features, coarse=False):
    contact = [Image.new('RGB', (1920, 816), '#181818') for _ in range(2)] if coarse else []
    output = [[] for _ in reference_features]
    for candidate, path, _ in renderer.render(candidates, coarse=coarse):
        with Image.open(path) as image:
            feat = features(image)
            if coarse and not candidate.get('obstruction_variant'):
                column = int(candidate['yaw']//30) % 6
                page = int(candidate['yaw']//180)
                row = [-60, -30, 0, 30].index(candidate['pitch'])
                x, y = column*320, row*204
                contact[page].paste(image.resize((320, 180)), (x, y+24))
                ImageDraw.Draw(contact[page]).text((x+4, y+4),
                    f'yaw {candidate["yaw"]:g} pitch {candidate["pitch"]:g}', fill='white')
        for rows, reference in zip(output, reference_features):
            if coarse:
                score, dx, dy = coarse_similarity(reference, feat)
                seed = dict(candidate)
                fy = math.tan(math.radians(renderer.params['fov']/2))
                seed['estimated_yaw'] = (seed['yaw'] + math.degrees(math.atan(dx*2/128*fy*16/9))) % 360
                seed['estimated_pitch'] = min(45., max(-80., seed['pitch'] + math.degrees(math.atan(dy*2/72*fy))))
                seed['strict_score'] = similarity(reference, feat)
                rows.append((score, seed))
            else:
                rows.append((similarity(reference, feat), candidate))
        # Candidate rasters have no lasting evidential value; preserve final pairs.
        remove_candidate(path)
    for page, sheet in enumerate(contact, 1):
        sheet.save(renderer.review_directory / f'{renderer.spot}-{page}.jpg', quality=94)
    return [sorted(rows, key=lambda r: r[0]) for rows in output]


def subdegree_refine(renderer, reference, score, candidate):
    """Verify small axis stencils and a fitted joint minimum with real renders."""
    history = []
    for steps in ((2., 2., .5), (.5, .5, .15)):
        axes = ('yaw', 'pitch', 'distance')
        stencil = [candidate]
        for axis, step in zip(axes, steps):
            for sign in (-1, 1):
                point = dict(candidate)
                point[axis] += sign * step
                point['yaw'] %= 360
                point['pitch'] = min(45., max(-80., point['pitch']))
                point['distance'] = min(7., max(.4, point['distance']))
                stencil.append(point)
        measured = ranked(renderer, stencil, [reference])[0]
        def key(point):
            return tuple(round(point[a], 6) for a in axes)
        costs = {key(c): s for s, c in measured}
        centre = costs[key(candidate)]
        fitted = dict(candidate)
        for i, (axis, step) in enumerate(zip(axes, steps)):
            low, high = stencil[1+2*i:3+2*i]
            if low[axis] == candidate[axis] or high[axis] == candidate[axis]:
                continue
            a, b = costs[key(low)], costs[key(high)]
            curvature = a - 2*centre + b
            if curvature > 1e-5:
                fitted[axis] += float(np.clip(.5*(a-b)/curvature, -1, 1)) * step
        fitted['yaw'] %= 360
        fitted['pitch'] = min(45., max(-80., fitted['pitch']))
        fitted['distance'] = min(7., max(.4, fitted['distance']))
        measured += ranked(renderer, [fitted], [reference])[0]
        history.extend(measured)
        score, candidate = min([(score, candidate), *measured], key=lambda r: r[0])
    return score, candidate, history


def make_pair(source, rendered, output, title):
    size = (2560, 1440)
    with Image.open(source) as im:
        real = im.convert('RGB').resize(size, Image.Resampling.LANCZOS)
    with Image.open(rendered) as im:
        ours = im.convert('RGB').resize(size, Image.Resampling.LANCZOS)
    a, b = features(real, size=(640, 360)), features(ours, size=(640, 360))
    mask = valid_mask((640, 360))
    # Structural red = retail edges, cyan = rewrite edges; white = agreement.
    difference = np.zeros((360, 640, 3), np.uint8)
    difference[:, :, 0] = a[0][0] * 255
    difference[:, :, 1] = b[0][0] * 255
    difference[:, :, 2] = b[0][0] * 255
    difference[~mask] = 28
    diff = Image.fromarray(difference).resize(size)
    pair = Image.new('RGB', (size[0]*3, size[1]+48), '#16191e')
    draw = ImageDraw.Draw(pair)
    for i, (label, image) in enumerate([('REAL', real), ('OURS', ours), ('EDGE DIFFERENCE: red real / cyan ours / grey ignored', diff)]):
        pair.paste(image, (i*size[0], 48))
        draw.text((i*size[0]+10, 7), label, fill='white')
    draw.text((10, 27), title, fill='white')
    pair.save(output)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--reference', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--game', type=Path, required=True)
    parser.add_argument('--maps', type=Path, default=Path('cache/maps'))
    parser.add_argument('--limit-spots', type=int)
    parser.add_argument('--map', choices=['m11_00', 'm11_01'])
    parser.add_argument('--render-arg', action='append', default=[])
    parser.add_argument('--story', help='Event-flag preset; defaults to the collection.json story of --reference (genichiro-defeated without one)')
    parser.add_argument('--seeds', type=Path, help='External shot-number to yaw/pitch/distance review seeds')
    parser.add_argument('--only-shots', help='Comma-separated selected shot numbers for a review rerun')
    parser.add_argument('--force', action='store_true', help='Replace selected existing poses after successful capture')
    parser.add_argument('--exact-seeds', action='store_true', help='Capture visually selected seed poses and measure their structural scores')
    parser.add_argument('--seed-radius', type=float, default=10., help='Degrees around a visually reviewed seed to search')
    parser.add_argument('--wait-seeds', action='store_true', help='Publish review-pending.json after the coarse grid and await external review seeds')
    args = parser.parse_args()
    if args.exact_seeds and not args.seeds:
        parser.error('--exact-seeds requires an external --seeds file')
    if args.wait_seeds and not args.seeds:
        parser.error('--wait-seeds requires an external --seeds file')
    args.reference, args.output = args.reference.resolve(), args.output.resolve()
    args.story = collection.story(args.reference, args.story)
    source = collection.images(args.reference).resolve()
    repo = Path(__file__).resolve().parents[2]
    if args.output == repo or repo in args.output.parents:
        parser.error('generated images and JSON must stay outside the source repository')
    index = json.loads((args.reference / 'index.json').read_text(encoding='utf-8'))
    spots = json.loads((args.reference / 'spots.json').read_text(encoding='utf-8'))
    selected = select_shots(index, spots, source)
    manual_seeds = json.loads(args.seeds.read_text()) if args.seeds else {}
    if args.only_shots:
        wanted = {int(s) for s in re.split(r'[,\s]+', args.only_shots.strip())}
        selected = [(s, [r for r in rows if r['index'] in wanted]) for s, rows in selected]
        selected = [(s, rows) for s, rows in selected if rows]
    if args.map:
        selected = [(s, r) for s, r in selected if s['map_block'] == args.map]
    if args.limit_spots:
        selected = selected[:args.limit_spots]
    args.output.mkdir(parents=True, exist_ok=True)
    pairs = args.output / 'pairs'
    pairs.mkdir(exist_ok=True)
    if args.output != args.reference:
        # The capture root declares its story so reports and replays read it back.
        write_json(args.output / 'collection.json', {'story': args.story, 'images': str(source)})
    poses_path = args.output / 'poses.json'
    poses = json.loads(poses_path.read_text()) if poses_path.exists() else []
    poses = [r for r in poses if r.get('matcher_version') in (2, 3, 4)]
    try:
        require_story(poses, args.story)
    except ValueError as error:
        parser.error(str(error))
    finished = set() if args.force else {r['shot'] for r in poses}
    review_run = bool(args.only_shots or args.force or args.seeds)
    selection_name = 'pose-review-selection.json' if review_run else 'pose-selection.json'
    write_json(args.output / selection_name, [dict(spot=s['spot_id'], shots=[r['index'] for r in rows]) for s, rows in selected])
    digest = hashlib.sha256(args.game.read_bytes()).hexdigest()
    write_json(args.output / ('pose-review-run.json' if review_run else 'pose-run.json'), {
        'renderer_sha256': digest,
        'index_sha256': hashlib.sha256((args.reference / 'index.json').read_bytes()).hexdigest(),
        'spots_sha256': hashlib.sha256((args.reference / 'spots.json').read_bytes()).hexdigest(),
        'selected_spots': len(selected), 'selected_shots': sum(len(r) for _, r in selected),
        'exclusions': 'Menu/help frames and unpositioned frames; prefer HUD-hidden gameplay.',
        'search': '30-degree yaw / pitch coarse grid; two yaw basins; 10-degree then 4-degree refinement; verified subdegree axis/quadratic refinement; distance .4 to 7 metres',
        'score': 'coarse masked contrast correlation with bounded translation; exact-render local contrast, gradient orientation and thin-edge chamfer refinement',
        'mask': 'practice overlay/backing, HUD corners and bars, crosshair, conservative central lower Wolf silhouette',
        'lighting': f'{args.story} event preset; shader phase held at 1 second; native exposure reset each pose',
        'story_limit': 'Boss progression is replayed; unknown quest, pickup and interaction flags remain unset.',
        'review_limit': 'Automatic candidates require visual review; score is not proof of correct pose or missing geometry.',
    })
    print(f'Selected {len(selected)} spots / {sum(len(r) for _, r in selected)} shots; renderer {digest}', flush=True)
    for block in sorted({s['map_block'] for s, _ in selected}):
        area = [(s, [r for r in rows if r['index'] not in finished]) for s, rows in selected if s['map_block'] == block]
        area = [(s, rows) for s, rows in area if rows]
        if not area:
            continue
        renderer = Renderer(args, block + '_00_00', area)
        try:
            for spot, rows in area:
                renderer.spot = spot['spot_id']
                start = time.monotonic()
                references = []
                for row in rows:
                    with Image.open(source / row['file']) as image:
                        references.append(features(image))
                player = spot['position']
                coarse = None
                if any(str(row['index']) not in manual_seeds for row in rows):
                    coarse = ranked(renderer, coarse_grid(renderer, player), references, coarse=True)
                if args.wait_seeds:
                    write_json(args.output / 'review-pending.json', {
                        'spot': spot['spot_id'], 'shots': [r['index'] for r in rows],
                        'references': [r['file'] for r in rows],
                        'gallery': str(renderer.review_directory.relative_to(args.output)),
                    })
                    while True:
                        manual_seeds = json.loads(args.seeds.read_text())
                        if all(str(row['index']) in manual_seeds for row in rows):
                            break
                        if (args.output / 'review-stop').exists():
                            raise RuntimeError('review stopped by external marker')
                        if renderer.process.poll() is not None:
                            raise RuntimeError('renderer exited during review')
                        time.sleep(.5)
                best = []
                for row_index, (row, reference) in enumerate(zip(rows, references)):
                    manual = manual_seeds.get(str(row['index']))
                    ranking = coarse[row_index] if coarse is not None else []
                    if args.exact_seeds:
                        if not manual:
                            raise ValueError(f'Exact review seed missing for shot {row["index"]}')
                        candidate = grid(player, [manual['yaw']], [manual['pitch']],
                                         [manual.get('distance', renderer.params['distance'])])[0]
                        score, candidate = ranked(renderer, [candidate], [reference])[0][0]
                        write_json(pairs / f'{spot["spot_id"]}-shot-{row["index"]:03}.search.json',
                                   {'visually_selected_seed': manual, 'score': score})
                        best.append((row, score, candidate, None))
                        continue
                    # Keep two genuinely different yaw basins to reduce local-minimum failures.
                    if manual:
                        seeds = [dict(estimated_yaw=float(manual['yaw']), estimated_pitch=float(manual['pitch']))]
                    else:
                        seeds = [ranking[0][1]]
                        for _, candidate in ranking[1:]:
                            if abs((candidate['yaw']-seeds[0]['yaw']+180) % 360-180) >= 45:
                                seeds.append(candidate)
                                break
                    candidates = []
                    distances = [.5, 2., renderer.params['distance'], 6.]
                    if manual and 'distance' in manual:
                        distance = float(manual['distance'])
                        distances = [max(.4, distance-1), distance, min(7., distance+1)]
                    for seed in seeds:
                        radius = args.seed_radius if manual else 10.
                        candidates.extend(grid(player, [seed['estimated_yaw']+d for d in (-radius, 0, radius)],
                                               [seed['estimated_pitch']+d for d in (-radius, 0, radius)], distances))
                    refined = ranked(renderer, candidates, [reference])[0]
                    seed = refined[0][1]
                    fine = ranked(renderer, grid(player, [seed['yaw']+d for d in (-4, 0, 4)],
                                                 [seed['pitch']+d for d in (-4, 0, 4)],
                                                 [max(.5, seed['distance']-1), seed['distance'], seed['distance']+1]), [reference])[0]
                    score, candidate = min([refined[0], fine[0]], key=lambda r: r[0])
                    score, candidate, micro = subdegree_refine(renderer, reference, score, candidate)
                    write_json(pairs / f'{spot["spot_id"]}-shot-{row["index"]:03}.search.json', {
                        'coarse': [{'score': s, 'yaw': c['yaw'], 'pitch': c['pitch'], 'distance': c['distance'],
                                    'estimated_yaw': c.get('estimated_yaw'), 'estimated_pitch': c.get('estimated_pitch'),
                                    'strict_score': c.get('strict_score')} for s, c in ranking],
                        'refined_top10': [{'score': s, 'yaw': c['yaw'], 'pitch': c['pitch'], 'distance': c['distance']} for s, c in refined[:10]],
                        'fine_top10': [{'score': s, 'yaw': c['yaw'], 'pitch': c['pitch'], 'distance': c['distance']} for s, c in fine[:10]],
                        'subdegree': [{'score': s, 'yaw': c['yaw'], 'pitch': c['pitch'], 'distance': c['distance']} for s, c in micro],
                    })
                    best.append((row, score, candidate, ranking[0][1]['strict_score'] - score if ranking else None))
                final = renderer.render([c for _, _, c, _ in best], final=True)
                for (row, score, candidate, improvement), (request, image, manifest) in zip(best, final):
                    stem = f'{spot["spot_id"]}-shot-{row["index"]:03}'
                    pair_path = pairs / f'{stem}.png'
                    make_pair(source / row['file'], image, pair_path,
                              f'{stem} | yaw {candidate["yaw"]:.1f} pitch {candidate["pitch"]:.1f} distance {request["actual_distance"]:.2f} | structure cost {score:.3f}')
                    manifest_path = pairs / f'{stem}.visibility.json'
                    write_json(manifest_path, manifest)
                    pose = dict(matcher_version=4, shot=row['index'], file=row['file'], spot=spot['spot_id'], map=block+'_00_00',
                                player_position=player, camera_position=request['view'][:3], yaw=request['view'][3],
                                pitch=request['view'][4], distance=request['actual_distance'], orbit_distance=request['distance'], fov=request['fov'],
                                match_score=score, score_kind='local contrast structure, oriented gradients and edge chamfer; lower is better',
                                coarse_to_fine_improvement=improvement, pair=str(pair_path.relative_to(args.output)),
                                visibility=str(manifest_path.relative_to(args.output)), renderer_sha256=digest,
                                camera_status='automatic candidate; visual review required', hud=row.get('hud'))
                    pose['render_settings'] = {'story': args.story, 'extra_args': args.render_arg,
                                               'search_size': [384, 216], 'final_size': [2560, 1440]}
                    if str(row['index']) in manual_seeds:
                        pose['review_seed'] = manual_seeds[str(row['index'])]
                        pose['seed_selection'] = 'visually selected exact pose' if args.exact_seeds else 'local structural refinement'
                    poses = [r for r in poses if r['shot'] != row['index']]
                    poses.append(pose)
                    poses.sort(key=lambda r: r['shot'])
                    write_json(poses_path, poses)
                print(f'{spot["spot_id"]}: shots {[r["index"] for r in rows]} costs {[round(s,3) for _,s,_,_ in best]} ({time.monotonic()-start:.1f}s)', flush=True)
        finally:
            renderer.close()


if __name__ == '__main__':
    main()
