# /// script
# requires-python = ">=3.12"
# dependencies = ["pillow>=12", "numpy>=2", "scipy>=1.16"]
# ///
"""Replay reviewed cameras exactly into a separate story comparison collection."""
from __future__ import annotations

import argparse
import hashlib
import json
import shutil
import uuid
from pathlib import Path

if __package__:
    from . import collection
    from .match_poses import Renderer, make_pair, write_json
else:
    import collection
    from match_poses import Renderer, make_pair, write_json


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--reference', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--game', type=Path, required=True)
    parser.add_argument('--annotations', type=Path, required=True)
    parser.add_argument('--poses', type=Path)
    parser.add_argument('--maps', type=Path, default=Path('cache/maps'))
    parser.add_argument('--story', help='Defaults to the collection.json story of --reference (genichiro-defeated without one)')
    parser.add_argument('--only-spots', help='Comma-separated spot IDs for diagnostic subsets')
    args = parser.parse_args()
    args.render_arg = []
    args.story = collection.story(args.reference, args.story)
    source = collection.images(args.reference).resolve()
    args.output = args.output.resolve()
    repo = Path(__file__).resolve().parents[2]
    if args.output == repo or repo in args.output.parents or args.output == args.reference.resolve():
        parser.error('output must be a separate external collection')
    args.output.mkdir(parents=True, exist_ok=True)
    if (args.output / 'poses.json').exists():
        parser.error('output already contains a completed collection; choose a fresh directory')
    reviews = json.loads(args.annotations.read_text())
    accepted = {r['shot'] for r in reviews['pairs'] if r['camera_review'] == 'accepted'}
    poses = json.loads((args.poses or args.reference / 'poses.json').read_text())
    poses = [p for p in poses if p['shot'] in accepted]
    if args.only_spots:
        poses = [p for p in poses if p['spot'] in args.only_spots.split(',')]
    digest = hashlib.sha256(args.game.read_bytes()).hexdigest()
    completed = []
    for map_id in sorted({p['map'] for p in poses}):
        group = [p for p in poses if p['map'] == map_id]
        selected = [(dict(position=p['player_position']), []) for p in group]
        renderer = Renderer(args, map_id, selected)
        try:
            batch = 'replay-' + uuid.uuid4().hex
            requests = [dict(id=i, player=p['player_position'],
                             view=p['camera_position'] + [p['yaw'], p['pitch']],
                             fov=p['fov'], width=2560, height=1440, manifest=True)
                        for i, p in enumerate(group, 1)]
            write_json(renderer.directory / 'request.json', dict(batch=batch, requests=requests))
            done = renderer.directory / f'{batch}.done.json'
            renderer.wait(done, max(660, len(group)*30))
            records = {r['id']: r for r in json.loads(done.read_text())}
            for request, pose in zip(requests, group):
                stem = f'{pose["spot"]}-shot-{pose["shot"]:03}'
                image = renderer.directory / f'{request["id"]:07}.png'
                shutil.copyfile(image, args.output / f'{stem}-ours.png')
                make_pair(source / pose['file'], image,
                          args.output / f'{stem}.png', f'{stem} | {args.story} | exact saved camera')
                write_json(args.output / f'{stem}.visibility.json', records[request['id']])
                updated = dict(pose, pair=f'{stem}.png', visibility=f'{stem}.visibility.json',
                               source_pose_renderer_sha256=pose.get('renderer_sha256'),
                               score_kind='Saved original camera-fit score; exact replay does not rescore or search',
                               renderer_sha256=digest, render_settings=dict(story=args.story,
                               final_size=[2560, 1440], extra_args=[]))
                completed.append(updated)
                print(args.story, stem, flush=True)
        finally:
            renderer.close()
    write_json(args.output / 'poses.json', completed)
    write_json(args.output / 'collection.json', dict(story=args.story, images=str(source)))
    write_json(args.output / 'triage.json', reviews)
    write_json(args.output / 'replay.json', dict(story=args.story, renderer_sha256=digest,
               poses=len(completed), camera_policy='Exact saved view, player and FOV; no rematching',
               annotations='Inherited camera acceptance; differences require renewed visual review'))


if __name__ == '__main__':
    main()
