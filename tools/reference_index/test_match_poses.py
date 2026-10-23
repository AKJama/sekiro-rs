"""Behaviour checks for pose scoring without game assets or generated fixtures."""
import unittest
import os
import threading
import time
import uuid
import json
import tempfile
from unittest.mock import Mock, patch
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageEnhance

from tools.reference_index import collection
from tools.reference_index.match_poses import camera, coarse_similarity, features, remove_candidate, similarity
from tools.reference_index.pose_collision import CameraCollision
from tools.reference_index.report_parity import report


class PoseScoring(unittest.TestCase):
    def test_reference_report_rejects_historical_story_by_default(self):
        output = Mock()
        with patch.object(Path, 'read_text', return_value=json.dumps([
            {'render_settings': {'story': 'night'}}
        ])):
            with self.assertRaisesRegex(ValueError, 'regenerate pairs'):
                report(Path('historical-captures'), output)
        output.write_text.assert_not_called()

    def test_collection_declares_its_own_story_and_image_folder(self):
        with tempfile.TemporaryDirectory() as directory:
            original, opening = Path(directory) / 'reference', Path(directory) / 'reference-opening'
            original.mkdir()
            opening.mkdir()
            (opening / 'collection.json').write_text(json.dumps({'story': 'morning', 'images': '../reference/m11_00'}))
            for root, override, story, images in [
                (original, None, 'genichiro-defeated', original / 'm11_00'),
                (opening, None, 'morning', original / 'm11_00'),
                (opening, 'night', 'night', original / 'm11_00'),
            ]:
                self.assertEqual(collection.story(root, override), story)
                self.assertEqual(collection.images(root).resolve(), images.resolve())
            # A report on the morning collection expects morning captures without being told.
            (opening / 'poses.json').write_text(json.dumps([{'render_settings': {'story': 'genichiro-defeated'}}]))
            with self.assertRaisesRegex(ValueError, 'Expected morning captures'):
                report(opening, opening / 'summary.md')

    def test_camera_orbit_pulls_in_at_a_wall_and_ignores_other_side(self):
        wall = np.array([[[-3, -3, 2], [3, -3, 2], [3, 3, 2]],
                         [[-3, -3, 2], [3, 3, 2], [-3, 3, 2]]])
        collision = CameraCollision(wall)
        self.assertAlmostEqual(collision.distance([0, 0, 0], 0, 0, 4.5, 1.5, 4.5), 1.7)
        self.assertAlmostEqual(collision.distance([0, 0, 0], 180, 0, 6., 1.5, 4.5), 4.5)

    def test_structure_beats_brightness_and_colour(self):
        reference = Image.new('RGB', (256, 144), (50, 60, 70))
        draw = ImageDraw.Draw(reference)
        for x, y, w, h in [(75, 28, 26, 37), (160, 33, 53, 35), (205, 90, 35, 20)]:
            draw.rectangle((x, y, x+w, y+h), fill=(170, 160, 130))
        aligned = ImageEnhance.Brightness(reference).enhance(.45)
        shifted = Image.fromarray(np.roll(np.asarray(reference), 24, axis=1))
        target = features(reference)
        self.assertLess(similarity(target, features(aligned)), similarity(target, features(shifted)))

    def test_pose_uses_bevy_back_vector_and_focus_height(self):
        np.testing.assert_allclose(camera([10, 20, 30], 90, 0, 4.5, 1.5),
                                   [14.5, 21.5, 30, 90, 0], atol=1e-6)
        view = camera([0, 0, 0], 0, -30, 4, 1.5)
        self.assertAlmostEqual(view[1], 3.5)
        self.assertAlmostEqual(view[2], np.sqrt(12))

    def test_coarse_search_recovers_shift_under_changed_exposure(self):
        reference = Image.new('RGB', (128, 72), (50, 60, 70))
        draw = ImageDraw.Draw(reference)
        for box in [(40, 15, 55, 30), (85, 20, 105, 45), (35, 46, 47, 58)]:
            draw.rectangle(box, fill=(180, 160, 130))
        shifted = Image.fromarray(np.roll(np.roll(np.asarray(reference), 12, axis=1), -7, axis=0))
        shifted = ImageEnhance.Brightness(shifted).enhance(.5)
        score, dx, dy = coarse_similarity(features(reference), features(shifted))
        self.assertLess(score, .4)
        self.assertAlmostEqual(dx, -12, delta=2)
        self.assertAlmostEqual(dy, 7, delta=2)

    @unittest.skipUnless(os.name == 'nt', 'Windows file sharing regression')
    def test_candidate_cleanup_survives_a_brief_windows_reader(self):
        artifact_root = Path(__file__).resolve().parents[2] / 'target' / 'alt-pt'
        artifact_root.mkdir(parents=True, exist_ok=True)
        path = artifact_root / f'candidate-test-{uuid.uuid4().hex}.bin'
        self.assertTrue(path.resolve().is_relative_to(artifact_root.resolve()))
        reader = None
        thread = None
        try:
            path.write_bytes(b'generated candidate')
            reader = path.open('rb')
            def close_reader():
                time.sleep(.08)
                reader.close()
            thread = threading.Thread(target=close_reader)
            thread.start()
            remove_candidate(path)
            thread.join()
            self.assertFalse(path.exists())
        finally:
            if thread is not None:
                thread.join()
            if reader is not None:
                reader.close()
            if path.exists():
                path.unlink()


if __name__ == '__main__':
    unittest.main()
