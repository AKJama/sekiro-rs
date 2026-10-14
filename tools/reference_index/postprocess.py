# /// script
# requires-python = ">=3.12"
# dependencies = ["pillow>=12", "numpy>=2"]
# ///
"""Add HUD labels, close-capture spots, and independent Windows OCR results."""
from __future__ import annotations

import argparse
from datetime import datetime
import json
from pathlib import Path
import re
import subprocess
import numpy as np
from PIL import Image


def hud_state(image: Image.Image) -> tuple[str, dict[str, int]]:
    """Use distinctive lower-left vitality red and upper-center posture orange."""
    pixels = np.asarray(image.convert("RGB")).astype(np.int16)
    red = pixels[1190:1430, 25:520]
    r, g, b = red.transpose(2, 0, 1)
    red_pixels = int(((r > 105) & (r > g * 1.65) & (r > b * 1.35)).sum())
    red_row_peak = int(((r > 105) & (r > g * 1.65) & (r > b * 1.35)).sum(axis=1).max(initial=0))
    orange = pixels[20:170, 800:1760]
    r, g, b = orange.transpose(2, 0, 1)
    orange_pixels = int(((r > 110) & (g > 45) & (r > g * 1.3) & (g > b * 1.35)).sum())
    evidence = {"vitality_red_pixels": red_pixels, "vitality_red_row_peak": red_row_peak,
                "posture_or_boss_bar_pixels": orange_pixels}
    visible = red_row_peak >= 24 or orange_pixels >= 90
    return ("visible" if visible else "hidden"), evidence


def normalize_ocr(text: str | None) -> str:
    value = re.sub(r"\s+", "", text or "").casefold()
    value = re.sub(r"\.{2,}", ".", value)
    return re.sub(r"\.(?!\d)", "", value)


def expected_reading(entry: dict) -> str | None:
    if not entry.get("igt") or not entry.get("raw_numbers"):
        return None
    coords = " ".join(f"{value:.3f}" for value in entry["raw_numbers"])
    return f"IGT {entry['igt']}\n{coords}"


def assign_spots(entries: list[dict], maximum_gap: float, tolerance: float) -> list[dict]:
    spots = []
    active = None
    for entry in entries:
        entry["spot_id"] = None
        position = entry.get("position")
        if position is None:
            active = None
            continue
        captured = datetime.fromisoformat(entry["capture_time"])
        same_place = active is not None and np.linalg.norm(np.subtract(position, active["position"])) <= tolerance
        close_in_time = same_place and (captured - active["last_capture"]).total_seconds() <= maximum_gap
        if not close_in_time:
            active = {"spot_id": f"spot-{len(spots) + 1:03}", "position": position,
                      "map_block": entry.get("map_block"), "nearest_landmark": entry.get("nearest_landmark"),
                      "files": [], "last_capture": captured}
            spots.append(active)
        active["files"].append(entry["file"])
        active["last_capture"] = captured
        entry["spot_id"] = active["spot_id"]
    for spot in spots:
        del spot["last_capture"]
    return spots


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--folder", type=Path, required=True, help="Read-only screenshot folder")
    parser.add_argument("--output", type=Path, required=True, help="Reference output folder")
    parser.add_argument("--review", type=Path, required=True, help="Existing manual review JSON")
    parser.add_argument("--ocr-script", type=Path, default=Path(__file__).with_name("windows_ocr.ps1"))
    parser.add_argument("--max-gap-seconds", type=float, default=30)
    parser.add_argument("--position-tolerance", type=float, default=0.002)
    args = parser.parse_args()
    if args.output.resolve() == args.folder.resolve():
        raise ValueError("Output must be separate from the source screenshots")

    entries = json.loads((args.output / "index.json").read_text(encoding="utf-8"))
    manual_review = json.loads(args.review.read_text(encoding="utf-8"))
    for entry in entries:
        with Image.open(args.folder / entry["file"]) as image:
            entry["hud"], entry["hud_evidence"] = hud_state(image)

    command = ["powershell.exe", "-NoProfile", "-STA", "-File", str(args.ocr_script), str(args.folder)]
    completed = subprocess.run(command, check=True, capture_output=True, text=True, encoding="utf-8")
    ocr_by_file = {item["file"]: item for item in json.loads(completed.stdout)}
    disagreements = []
    for entry in entries:
        result = ocr_by_file.get(entry["file"], {"text": None, "error": "No Windows OCR result"})
        entry["windows_ocr_reading"] = result.get("text")
        entry["windows_ocr_error"] = result.get("error")
        expected = expected_reading(entry)
        actual = result.get("text")
        if expected and actual and normalize_ocr(expected) != normalize_ocr(actual):
            disagreements.append({"index": entry["index"], "file": entry["file"],
                                 "template_reading": expected, "windows_ocr_reading": actual,
                                 "note": "Independent OCR disagreement; neither reading was changed."})
    spots = assign_spots(entries, args.max_gap_seconds, args.position_tolerance)
    (args.output / "index.json").write_text(json.dumps(entries, indent=2) + "\n", encoding="utf-8")
    (args.output / "spots.json").write_text(json.dumps(spots, indent=2) + "\n", encoding="utf-8")
    manual_review["ocr_disagreements"] = disagreements
    manual_review["ocr_engine"] = "Windows.Media.Ocr on the 430x70 practice-overlay crop at (45,255)"
    (args.review).write_text(json.dumps(manual_review, indent=2) + "\n", encoding="utf-8")
    print(f"Indexed {len(entries)} shots into {len(spots)} spots; HUD visible on "
          f"{sum(entry['hud'] == 'visible' for entry in entries)} shots; "
          f"Windows OCR disagreements: {len(disagreements)}")


if __name__ == "__main__":
    main()
