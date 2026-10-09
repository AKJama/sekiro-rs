# /// script
# requires-python = ">=3.10"
# dependencies = ["pyogg==0.6.14a1"]
# ///
"""Regenerates the Vorbis identification and setup headers the game's FSB5 banks leave out.

FSB5 Vorbis samples store raw audio packets plus a CRC-32 of the setup header the encoder used;
the headers themselves are not in the file. Stock libvorbis produces the same setup header for the
same channel count, sample rate and VBR quality, so this script encodes nothing but headers for a
grid of qualities and keeps the ones whose CRC matches a value found in the banks.

Usage: uv run tools/sound/vorbis_setups.py <cache dir>
Writes <cache>/sound/vorbis/<crc>.ident and <crc>.setup (raw header packets).
"""

import ctypes
import os
import struct
import sys
import zlib
from pathlib import Path

import pyogg

RATES = {1: 8000, 2: 11000, 3: 11025, 4: 16000, 5: 22050, 6: 24000, 7: 32000, 8: 44100, 9: 48000}


def wanted_setups(sound_dir: Path) -> dict[int, set[tuple[int, int]]]:
    """CRC -> {(channels, rate)} over every unencrypted FSB5 bank."""
    out: dict[int, set[tuple[int, int]]] = {}
    for path in sorted(sound_dir.glob("*.fsb")):
        data = path.read_bytes()
        if data[:4] != b"FSB5":
            continue
        version, count, headers_size = struct.unpack_from("<3I", data, 4)
        pos = 0x3C if version == 1 else 0x40
        for _ in range(count):
            (raw,) = struct.unpack_from("<Q", data, pos)
            pos += 8
            more = raw & 1
            rate = RATES.get((raw >> 1) & 0xF, 0)
            channels = 2 if (raw >> 5) & 1 else 1
            crc = None
            while more:
                (chunk,) = struct.unpack_from("<I", data, pos)
                pos += 4
                more = chunk & 1
                size = (chunk >> 1) & 0xFFFFFF
                kind = (chunk >> 25) & 0x7F
                if kind == 1:
                    channels = data[pos]
                elif kind == 2:
                    (rate,) = struct.unpack_from("<I", data, pos)
                elif kind == 11:
                    (crc,) = struct.unpack_from("<I", data, pos)
                pos += size
            if crc is not None:
                out.setdefault(crc, set()).add((channels, rate))
    return out


class Packet(ctypes.Structure):
    _fields_ = [
        ("packet", ctypes.POINTER(ctypes.c_ubyte)),
        ("bytes", ctypes.c_long),
        ("b_o_s", ctypes.c_long),
        ("e_o_s", ctypes.c_long),
        ("granulepos", ctypes.c_int64),
        ("packetno", ctypes.c_int64),
    ]


def load_vorbis():
    folder = os.path.dirname(pyogg.__file__)
    os.add_dll_directory(folder)
    ctypes.CDLL(os.path.join(folder, "libogg.dll"))
    lib = ctypes.CDLL(os.path.join(folder, "libvorbis.dll"))
    lib.vorbis_encode_init_vbr.argtypes = [ctypes.c_void_p, ctypes.c_long, ctypes.c_long, ctypes.c_float]
    return lib


def headers(lib, channels: int, rate: int, quality: float):
    # Opaque state blocks, generously sized for the 64-bit libvorbis structs.
    info, dsp, comment = (ctypes.c_byte * 1024)(), (ctypes.c_byte * 8192)(), (ctypes.c_byte * 1024)()
    lib.vorbis_info_init(info)
    if lib.vorbis_encode_init_vbr(info, channels, rate, ctypes.c_float(quality)) != 0:
        return None
    lib.vorbis_comment_init(comment)
    lib.vorbis_analysis_init(dsp, info)
    ident, comm, setup = Packet(), Packet(), Packet()
    lib.vorbis_analysis_headerout(dsp, comment, ctypes.byref(ident), ctypes.byref(comm), ctypes.byref(setup))

    def raw(p):
        return bytes(ctypes.cast(p.packet, ctypes.POINTER(ctypes.c_ubyte * p.bytes)).contents)

    return raw(ident), raw(setup)


def main() -> int:
    cache = Path(sys.argv[1] if len(sys.argv) > 1 else "cache")
    out = cache / "sound" / "vorbis"
    out.mkdir(parents=True, exist_ok=True)
    lib = load_vorbis()
    wanted = wanted_setups(cache / "raw" / "sound")
    missing = []
    for crc, variants in sorted(wanted.items()):
        if (out / f"{crc}.setup").exists():
            continue
        found = False
        for channels, rate in sorted(variants):
            for step in range(-10, 101):
                h = headers(lib, channels, rate, step / 100)
                if h and zlib.crc32(h[1]) == crc:
                    (out / f"{crc}.ident").write_bytes(h[0])
                    (out / f"{crc}.setup").write_bytes(h[1])
                    print(f"{crc}: {channels} ch {rate} Hz quality {step / 100}")
                    found = True
                    break
            if found:
                break
        if not found:
            missing.append((crc, sorted(variants)))
    for crc, variants in missing:
        print(f"{crc}: no match for {variants}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
