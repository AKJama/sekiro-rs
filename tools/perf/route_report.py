"""Report settled frame times and the measured scopes in each traversal spike."""
import argparse
import csv
import json
import math
from pathlib import Path
import re
import statistics as stats


def report(folder, first):
    path = next(folder.glob('[0-9]*.csv'))
    rows = list(csv.DictReader(path.open(encoding='utf-8')))
    game = [r for r in rows[:-1] if r.get('sim_step') and int(r['sim_step']) >= first]
    ms = [float(r['wall_ms']) for r in game]
    data = (folder / 'run.log').read_bytes()
    log = data.decode('utf-16' if data.startswith(b'\xff\xfe') else 'utf-8-sig', errors='replace')
    log = re.sub(r'\s+', ' ', log)
    scopes = {}
    for name, mean in re.findall(r'profile system (.*?): mean ([\d.]+) ms', log):
        scopes.setdefault(name, []).append(float(mean))
    result = {
        'frames': len(game),
        'median_fps': 1000 / stats.median(ms),
        'low_1pct_fps': 1000 / stats.mean(sorted(ms, reverse=True)[:math.ceil(len(ms) * .01)]),
        'worst_ms': max(ms),
        'over_33ms': sum(t > 33 for t in ms),
        'main_ms': stats.median(float(r['x_main_ms']) for r in game),
        'render_ms': stats.median(float(r['x_render_max_ms']) for r in game),
        'pipeline_waiting_max': max(int(r['x_pipelines_waiting']) for r in game),
        'clip_loads': sum(int(r['x_clip_loads']) for r in game),
        'counts': re.findall(r'render counts: (.*?)20\d\d-\d\d-\d\dT', log)[-1:],
        'casters': re.findall(r'render shadow casters: (.*?)20\d\d-\d\d-\d\dT', log)[-1:],
        'tour': re.findall(r'perf tour: (.*?)20\d\d-\d\d-\d\dT', log),
        'scopes_last15_ms': {k: stats.median(v[-15:]) for k, v in scopes.items()},
        'spikes': [
            {k: r.get(k) for k in ('frame', 'wall_ms', 'sim_step', 'x_main_ms', 'x_fixed_ms',
                'x_clip_loads', 'x_clip_ms', 'x_render_max_ms', 'x_pipelines_ok',
                'x_pipelines_waiting', 'x_render_sets', 'x_systems', 'sim_x', 'sim_y', 'sim_z')}
            for r in sorted(game, key=lambda r: float(r['wall_ms']), reverse=True)[:15]
        ],
    }
    (folder / 'route-report.json').write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8')
    print(folder, json.dumps({k: v for k, v in result.items() if k not in ('spikes', 'casters', 'scopes_last15_ms')}))
    for row in result['spikes'][:5]:
        print(json.dumps(row))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('folders', nargs='+', type=Path)
    parser.add_argument('--from-step', type=int, default=300)
    args = parser.parse_args()
    for folder in args.folders:
        report(folder, args.from_step)
