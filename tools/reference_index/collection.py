"""Per-collection settings: the story a reference collection was captured in and where its images live.

A collection root may hold `collection.json`, for example `{"story": "morning", "images": "../reference/m11_00"}`.
Without it, a collection is the original post-Genichiro set with its screenshots in `m11_00/`.
"""
from __future__ import annotations

import json
from pathlib import Path

DEFAULT_STORY = 'genichiro-defeated'
DEFAULT_IMAGES = 'm11_00'


def settings(root):
    path = Path(root) / 'collection.json'
    return json.loads(path.read_text(encoding='utf-8')) if path.exists() else {}


def story(root, override=None):
    """The explicit override, else the collection's declared story, else the original default."""
    return override or settings(root).get('story', DEFAULT_STORY)


def images(root):
    """Folder holding the collection's screenshots; a relative path is relative to the root."""
    return Path(root) / settings(root).get('images', DEFAULT_IMAGES)
