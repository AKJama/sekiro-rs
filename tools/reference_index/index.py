# /// script
# requires-python = ">=3.12"
# dependencies = ["pillow>=12", "numpy>=2"]
# ///
"""Read a fixed Sekiro practice overlay without changing source screenshots."""
from __future__ import annotations
import argparse
from collections import defaultdict
from datetime import datetime
import json
from pathlib import Path
import re
import numpy as np
from PIL import Image, ImageDraw


def mask(image, channel):
    a = np.asarray(image.convert('RGB')).astype(np.int16)
    if channel == 3:
        a = a[263:290, 45:235]
        return (a.min(2) > 242) & (a.max(2) - a.min(2) < 8)
    a = a[292:316, 45:470]
    color = np.array([(179, 30, 44), (29, 136, 89), (36, 73, 145)][channel])
    return np.max(np.abs(a - color), axis=2) < 22



def glyphs(m):
    xs = np.where(m.any(0))[0]
    if not len(xs):
        return []
    result = []
    for run in np.split(xs, np.where(np.diff(xs) > 1)[0] + 1):
        g = m[:, run[0]:run[-1] + 1]
        ys = np.where(g.any(1))[0]
        if len(ys):
            g = g[ys[0]:ys[-1]+1]
            if g.sum() < 3:
                continue
            canvas = np.zeros((28, 18), dtype=bool)
            h, w = g.shape
            if h > 28 or w > 18:
                result.append(None)
                continue
            canvas[:h, :w] = g
            result.append(canvas)
    return result


def igt_glyphs(m):
    result = []
    for i in range(11):
        left = round(101 + i * 11.75) - 45
        right = round(101 + (i+1) * 11.75) - 45
        gs = glyphs(m[:, left:right])
        if len(gs) != 1:
            result.append(None)
        else:
            result.append(gs[0])
    return result


def templates(files, calibration):
    bank = [defaultdict(list) for _ in range(2)]
    for seed in calibration['seeds']:
        with Image.open(files[seed['index']-1]) as im:
            for k in range(4):
                label = (seed.get('igt') or seed.get('verified_igt')) if k == 3 else (seed.get('raw') or [None]*3)[k]
                if label is None:
                    continue
                gs = igt_glyphs(mask(im, k)) if k == 3 else glyphs(mask(im, k))
                if len(gs) != len(label):
                    raise ValueError(f"Calibration {seed['index']} channel {k}: {len(gs)} glyphs for {label}")
                for c, g in zip(label, gs):
                    if g is not None:
                        bank[k == 3][c].append(g)
    return bank


def read(m, bank, white=False):
    chars, errors, margins = [], [], []
    for g in (igt_glyphs(m) if white else glyphs(m)):
        if g is None:
            return None, 1., 0.
        scores = []
        for c, examples in bank.items():
            examples = np.asarray(examples)
            score = float((np.logical_xor(g, examples).sum((1, 2)) / np.maximum(np.logical_or(g, examples).sum((1, 2)), 1)).min())
            scores.append((score, c))
        scores.sort()
        chars.append(scores[0][1]); errors.append(float(scores[0][0])); margins.append(float(scores[1][0]-scores[0][0]))
    return ''.join(chars), max(errors, default=1.), min(margins, default=0.)


def main():
    p = argparse.ArgumentParser(__doc__)
    p.add_argument('folder', type=Path)
    p.add_argument('--output', type=Path)
    p.add_argument('--calibration', type=Path, required=True)
    p.add_argument('--maps', type=Path)
    p.add_argument('--review', type=Path)
    p.add_argument('--validation', type=Path)
    p.add_argument('--reuse-recognition', action='store_true')
    p.add_argument('--refresh-igt', action='store_true')
    args = p.parse_args()
    files = sorted(args.folder.glob('*.png'))
    out = args.output or args.folder.parent
    if out.resolve() == args.folder.resolve():
        raise ValueError('Output must be separate from the screenshot folder')
    out.mkdir(parents=True, exist_ok=True)
    if args.reuse_recognition:
        entries = json.loads((out/'index.json').read_text())
        finish(entries, args, out)
        return
    bank = templates(files, json.loads(args.calibration.read_text()))
    entries = json.loads((out/'index.json').read_text()) if args.refresh_igt else []
    for i, file in enumerate(files, 1):
        with Image.open(file) as im:
            if im.size != (2560, 1440):
                raise ValueError(f'Unsupported dimensions: {file.name} {im.size}')
            if args.refresh_igt:
                t,err,margin=read(mask(im,3),bank[1],True)
                e=entries[i-1]
                e['recognition'].update(igt_text=t)
                e['recognition']['errors'][3]=err
                e['recognition']['margins'][3]=margin
                e['igt']=t if re.fullmatch(r'\d{2}:[0-5]\d:[0-5]\d\.\d{2}',t or '') and err<.6 and margin>.04 else None
                if e['status'] not in ('menu','overlay missing'):
                    e['status']='ok' if e['raw_numbers'] and e['igt'] else 'unreadable'
                continue
            results = [read(mask(im, k), bank[k == 3], k == 3) for k in range(4)]
        raw_text = [r[0] for r in results[:3]]
        valid = all(re.fullmatch(r'-?\d+\.\d{3}', t or '') and e < .43 and margin > .035 for t, e, margin in results[:3])
        igt, err, margin = results[3]
        igt = (igt or '').removeprefix('IGT')
        igt_ok = bool(re.fullmatch(r'\d{2}:[0-5]\d:[0-5]\d\.\d{2}', igt)) and err < .6 and margin > .04
        match = re.search(r'_(\d{14})_', file.name)
        raw = list(map(float, raw_text)) if valid else None
        entry = dict(index=i, file=file.name, capture_time=datetime.strptime(match[1], '%Y%m%d%H%M%S').isoformat() if match else None,
                     igt=igt if igt_ok else None, raw_numbers=raw, position=[-raw[0], raw[1], raw[2]] if raw else None,
                     map_block=None, nearest_landmark=None, height_above_floor=None,
                     status='ok' if valid and igt_ok else ('overlay missing' if all(not t for t in raw_text) else 'unreadable'),
                     recognition=dict(raw_text=raw_text, igt_text=igt, errors=[r[1] for r in results], margins=[r[2] for r in results]))
        entry['automatic_status'] = entry['status']
        entries.append(entry)
        if i % 50 == 0:
            print(f'Read {i}/{len(files)}', flush=True)
    (out/'index.json').write_text(json.dumps(entries, indent=2)+'\n')
    print('Status:',dict((s,sum(e['status']==s for e in entries)) for s in {e['status'] for e in entries}))
    finish(entries, args, out)
    if args.refresh_igt:
        return
    # Page small enough to inspect every scene and identify exceptional menus.
    for start in range(0, len(files), 80):
        sheet=Image.new('RGB',(1600,10*118),'#111820'); draw=ImageDraw.Draw(sheet)
        for j,file in enumerate(files[start:start+80]):
            with Image.open(file) as im:
                im.thumbnail((200,113)); x=(j%8)*200;y=(j//8)*118;sheet.paste(im,(x,y))
            draw.rectangle((x,y,x+40,y+16),fill='black');draw.text((x+2,y+2),str(start+j+1),fill='white')
        sheet.save(out/f'overview-{start//80+1:02}.jpg',quality=92)

def finish(entries, args, out):
    if args.validation:
        checks = []
        for expected in json.loads(args.validation.read_text()):
            actual = entries[expected['index']-1]
            checks.append(dict(index=expected['index'],
                               coordinates_match=actual['raw_numbers'] == expected['raw_numbers'],
                               igt_match=actual['igt'] == expected['igt'],
                               actual_raw=actual['raw_numbers'], actual_igt=actual['igt']))
        (out/'validation-results.json').write_text(json.dumps(checks,indent=2)+'\n')
        print('Held-out validation:',sum(c['coordinates_match'] for c in checks),'coordinate matches,',sum(c['igt_match'] for c in checks),'IGT matches of',len(checks),flush=True)
    if args.review:
        review = json.loads(args.review.read_text())
        for item in review.get('entries', []):
            e = entries[item['index']-1]
            e.setdefault('automatic_status', e['status'])
            if item.get('automatic_status'):
                e['automatic_status'] = item['automatic_status']
            for field in ['igt', 'raw_numbers', 'status', 'menu']:
                if field in item:
                    e[field] = item[field]
            if e['raw_numbers']:
                x,y,z=e['raw_numbers'];e['position']=[-x,y,z]
            e['manual_review'] = item.get('note', 'Visual transcription')
    (out/'index.json').write_text(json.dumps(entries,indent=2)+'\n')
    if args.maps:
        from spatial import enrich
        enrich(entries,args.maps,out,args.folder)


if __name__ == '__main__':
    main()

