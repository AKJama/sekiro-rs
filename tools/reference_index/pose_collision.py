"""Read-only camera obstruction queries matching Play's exported-hit ray rule."""
from __future__ import annotations

import math
import numpy as np


def read_collision(path):
    data = path.read_bytes()
    offset = 0
    def varint():
        nonlocal offset
        value = shift = 0
        while True:
            byte = data[offset]
            offset += 1
            value |= (byte & 127) << shift
            if byte < 128:
                return value
            shift += 7
            if shift > 35:
                raise ValueError('Invalid collision postcard varint')
    count = varint()
    vertices = np.frombuffer(data, dtype='<f4', count=count*3, offset=offset).reshape(-1, 3)
    offset += count*12
    count = varint()
    indices = np.fromiter((varint() for _ in range(count)), dtype=np.uint32, count=count).reshape(-1, 3)
    count = varint()
    materials = np.fromiter((varint() for _ in range(count)), dtype=np.uint32, count=count)
    if offset != len(data) or len(materials) != len(indices):
        raise ValueError('Unexpected collision serialization')
    valid = ~np.isin(materials, [22, 36, 38])
    return vertices[indices[valid]]


class CameraCollision:
    def __init__(self, triangles):
        self.triangles = np.asarray(triangles, dtype=np.float32)
        self.minimum = self.triangles.min(axis=1)
        self.maximum = self.triangles.max(axis=1)
        self.key = None
        self.local = None

    @classmethod
    def load(cls, maps, block):
        # The primary block contains the player's indexed support; include the
        # other Ashina block at their shared seam without altering either cache.
        blocks = [block]
        other = 'm11_01_00_00' if block == 'm11_00_00_00' else 'm11_00_00_00'
        if (maps / other / 'collision.bin').exists():
            blocks.append(other)
        return cls(np.concatenate([read_collision(maps / b / 'collision.bin') for b in blocks]))

    def raycast(self, origin, direction, reach):
        origin = np.asarray(origin, dtype=np.float64)
        direction = np.asarray(direction, dtype=np.float64)
        # Every candidate ray is short and shares a player anchor. A conservative
        # cube keeps intersecting large triangles as well as small nearby walls.
        cube_reach = max(8., float(reach))
        key = tuple(origin), cube_reach
        if self.key != key:
            near = ((self.maximum >= origin-cube_reach) & (self.minimum <= origin+cube_reach)).all(axis=1)
            triangles = self.triangles[near].astype(np.float64)
            a = triangles[:, 0]
            self.local = a, triangles[:, 1]-a, triangles[:, 2]-a
            self.key = key
        a, edge1, edge2 = self.local
        p = np.cross(direction, edge2)
        det = np.einsum('ij,ij->i', edge1, p)
        valid = np.abs(det) >= 1e-9
        inverse = np.divide(1., det, out=np.zeros_like(det), where=valid)
        start = origin-a
        u = np.einsum('ij,ij->i', start, p)*inverse
        q = np.cross(start, edge1)
        v = np.einsum('j,ij->i', direction, q)*inverse
        distance = np.einsum('ij,ij->i', edge2, q)*inverse
        valid &= (u >= 0) & (u <= 1) & (v >= 0) & (u+v <= 1) & (distance >= 0) & (distance <= reach)
        return float(distance[valid].min()) if valid.any() else None

    def distance(self, player, yaw, pitch, requested, height, normal_distance):
        distance = min(float(requested), float(normal_distance))
        y, p = math.radians(yaw), math.radians(pitch)
        back = [math.sin(y)*math.cos(p), -math.sin(p), math.cos(y)*math.cos(p)]
        target = np.asarray(player, dtype=float) + [0, height, 0]
        # As play.rs: query 0.3 m past the target orbit, leave 0.3 m clearance,
        # and preserve its 0.4 m minimum instead of inventing a floor plane.
        hit = self.raycast(target, back, distance+.3)
        return float(np.clip(hit-.3, .4, distance)) if hit is not None else distance
