#!/usr/bin/env python3
"""Generate a shared retail YR / VERA depth-comparison map, using only stdlib.

Usage:
    python tools/render_depth_fixture.py target/depth-comparison/depth-walls-cliff.map
    python tools/render_depth_fixture.py target/depth-comparison/depth-walls.map --walls-only

This writes only the requested map. It neither edits game configuration nor
launches either engine. The cliff fixture deliberately focuses on one cliff
stamp; distant plateau edges are not a fully landscaped map.
"""

import argparse
import base64
import struct
from pathlib import Path


def decode_literal_lzo(encoded: bytes) -> bytes:
    """Check this fixture's literal-only LZO subset, not arbitrary LZO files."""
    if not encoded:
        raise ValueError('Empty LZO block')
    index = 1
    if encoded[0] > 17:
        count = encoded[0] - 17
    elif encoded[0] == 0:
        count = 18
        while index < len(encoded) and encoded[index] == 0:
            count += 255
            index += 1
        if index >= len(encoded):
            raise ValueError('Truncated LZO literal length')
        count += encoded[index]
        index += 1
    else:
        raise ValueError('LZO command outside the literal-only fixture subset')
    end = index + count
    if encoded[end:] != b'\x11\0\0':
        raise ValueError('Invalid LZO literal length or terminator')
    return encoded[index:end]


def decode_fixture_lcw(encoded: bytes) -> bytes:
    """Check emitted Format80 literal/RLE/end commands without writer state."""
    decoded = bytearray()
    index = 0
    while index < len(encoded):
        command = encoded[index]
        index += 1
        if command == 0x80:
            if index != len(encoded):
                raise ValueError('Trailing bytes after LCW terminator')
            return bytes(decoded)
        if command == 0xFE:
            if index + 3 > len(encoded):
                raise ValueError('Truncated LCW repeat command')
            count = struct.unpack_from('<H', encoded, index)[0]
            decoded.extend(bytes([encoded[index + 2]]) * count)
            index += 3
        elif 0x81 <= command <= 0xBF:
            count = command & 0x3F
            if index + count > len(encoded):
                raise ValueError('Truncated LCW literal command')
            decoded.extend(encoded[index:index + count])
            index += count
        else:
            raise ValueError('LCW command outside the fixture subset')
    raise ValueError('Missing LCW terminator')


def pack_lzo(data: bytes) -> str:
    packed = bytearray()
    for start in range(0, len(data), 8192):
        block = data[start:start + 8192]
        count = len(block)
        if count <= 238:
            prefix = bytes([17 + count])
        else:
            zeros, tail = divmod(count - 19, 255)
            prefix = b'\0' + b'\0' * zeros + bytes([tail + 1])
        encoded = prefix + block + b'\x11\0\0'
        if decode_literal_lzo(encoded) != block:
            raise ValueError('LZO block did not round-trip')
        packed += struct.pack('<HH', len(encoded), count) + encoded
    text = base64.b64encode(packed).decode('ascii')
    return '\n'.join(f'{i // 70 + 1}={text[i:i + 70]}' for i in range(0, len(text), 70))


def pack_lcw(data: bytes) -> str:
    """Native OverlayPack uses Format80, not the LZO IsoMapPack5 codec.

    Emit only Format80 literal-copy (0x81..0xBF), repeat (0xFE), and end
    (0x80) commands; independently expand every chunk before writing it.
    No external dependencies.
    """
    packed = bytearray()
    for start in range(0, len(data), 8192):
        block = data[start:start + 8192]
        encoded = bytearray()
        index = 0
        while index < len(block):
            end = index + 1
            while end < len(block) and block[end] == block[index]:
                end += 1
            if end - index >= 4:
                encoded += b'\xfe' + struct.pack('<H', end - index) + block[index:index + 1]
                index = end
            else:
                end = min(index + 63, len(block))
                encoded.append(0x80 | (end - index))
                encoded.extend(block[index:end])
                index = end
        encoded.append(0x80)
        if decode_fixture_lcw(encoded) != block:
            raise ValueError('LCW block did not round-trip')
        packed += struct.pack('<HH', len(encoded), len(block)) + encoded
    text = base64.b64encode(packed).decode('ascii')
    return '\n'.join(f'{i // 70 + 1}={text[i:i + 70]}' for i in range(0, len(text), 70))


CLEAR_TILE = 0
CLIFF_TILE = 49  # temperatmd.ini TileSet0010 starts at Cliff01.tem (2x3).
GAWALL = 3  # rulesmd.ini [OverlayTypes] 3=GAWALL.
C = 48


def build_fixture(*, walls_only: bool = False) -> str:
    """Keep the same buildings and front/back units in both comparison scenes."""
    cliff_x0, cliff_y0 = C - 8, C - 2   # 2 wide, 3 tall

    def high(x, y):
        return y <= x + 6  # Raised ground north-west of the cliff line.

    cells = bytearray()
    for y in range(1, 97):
        for x in range(1, 97):
            if 48 < x + y <= 144 and abs(x - y) < 48:
                tile, sub, level = CLEAR_TILE, 0, 0
                if walls_only:
                    pass
                elif cliff_x0 <= x < cliff_x0 + 2 and cliff_y0 <= y < cliff_y0 + 3:
                    sub = (y - cliff_y0) * 2 + (x - cliff_x0)
                    if sub in (0, 2, 3, 5):
                        tile = CLIFF_TILE
                        level = 4 if sub in (0, 3) else 0
                    else:
                        sub = 0
                        level = 4 if high(x, y) else 0
                elif high(x, y):
                    level = 4
                cells += struct.pack('<hHiBBB', x, y, tile, sub, level, 0)

    # Overlay planes: 512x512 bytes, 0xFF = none; walls ring the pillbox at (C+6, C+3).
    overlay = bytearray(b'\xff' * 262144)
    overlay_data = bytearray(262144)
    px, py = C + 6, C + 3
    for dx in range(-1, 3):
        for dy in range(-1, 3):
            if dx in (-1, 2) or dy in (-1, 2):
                overlay[(py + dy) * 512 + (px + dx)] = GAWALL

    # Final OverlayDataPack overwrites placement-derived connectivity natively.
    # Author the nibble explicitly so these are connected walls, not isolated posts.
    for y in range(512):
        for x in range(512):
            if overlay[y * 512 + x] == GAWALL:
                bits = 0
                for bit, (dx, dy) in enumerate(((0, -1), (1, 0), (0, 1), (-1, 0))):
                    if (0 <= x + dx < 512 and 0 <= y + dy < 512
                            and overlay[(y + dy) * 512 + x + dx] == GAWALL):
                        bits |= 1 << bit
                overlay_data[y * 512 + x] = bits

    structures = [
        f'1=Americans,GACNST,256,{C},{C},0,None,0,0,1,0,0,None,None,None,0,0',
        f'2=Americans,GAPOWR,256,{C+8},{C-6},0,None,0,0,1,0,0,None,None,None,0,0',
        f'3=Americans,GAWEAP,256,{C-3},{C+5},0,None,0,0,1,0,0,None,None,None,0,0',
        f'4=Americans,GAPILL,256,{px},{py},0,None,0,0,1,0,0,None,None,None,0,0',
    ]
    if not walls_only:
        structures.append(f'5=Americans,NAPOWR,256,{C-8},{C+1},0,None,0,0,1,0,0,None,None,None,0,0')

    tanks = [
        (C - 1, C - 1, 64), (C, C - 2, 64), (C - 2, C, 64), (C + 1, C - 2, 64),
        (C + 3, C + 3, 192), (C + 2, C + 4, 192),
        (C - 4, C + 4, 64), (C - 2, C + 3, 64), (C - 1, C + 8, 192),
        # walls: one tank just behind the north wall, one just in front of the south wall
        (px, py - 2, 64), (px + 1, py + 3, 192),
        # cliff: raised-ground and foot units, clear of the power-plant foundation
        (C - 10, C - 1, 64), (C - 6, C - 1, 192), (C - 5, C + 2, 192),
    ]
    units = [
        f'{i}=Americans,MTNK,256,{x},{y},{f},Guard,None,0,-1,0,-1,1,1'
        for i, (x, y, f) in enumerate(tanks)
    ]
    gis = [
        (C - 1, C, 2, 64), (C + 1, C - 1, 3, 64), (C + 4, C + 2, 2, 192),
        (C - 1, C + 5, 2, 64), (px - 1, py + 1, 2, 64),
    ]
    infantry = [
        f'{i}=Americans,E1,256,{x},{y},{s},Guard,{f},None,0,-1,0,1,1'
        for i, (x, y, s, f) in enumerate(gis)
    ]

    return f'''[Basic]
Name=Depth Continuation - Walls and Cliff
NewINIFormat=4
Player=Americans
HomeCell={(C + 1) * 1000 + (C - 5)}
AltHomeCell={(C + 1) * 1000 + (C - 5)}
MultiplayerOnly=no
Official=no
IgnoreGlobalAITriggers=yes
Intro=<none>
Brief=<none>
Win=<none>
Lose=<none>
Action=<none>
FreeRadar=yes
EndOfGame=yes

[Map]
Theater=TEMPERATE
Size=0,0,48,48
LocalSize=2,4,44,40

[SpecialFlags]
FogOfWar=no
Inert=no

[Lighting]
Ambient=1.0
Red=1.0
Green=1.0
Blue=1.0
Ground=0.0
Level=0.032

[Houses]
0=Americans
1=Neutral

[Americans]
Country=Americans
PlayerControl=yes
IQ=0
TechLevel=10
Credits=50000
Edge=North
Allies=Americans,Neutral
Color=DarkBlue

[Neutral]
Country=Neutral
PlayerControl=no
IQ=0
TechLevel=0
Credits=0
Edge=North
Allies=Americans,Neutral
Color=Grey

[Waypoints]
0={(C - 15) * 1000 + (C + 11)}
1={(C + 14) * 1000 + (C + 14)}

[Structures]
{chr(10).join(structures)}

[Units]
{chr(10).join(units)}

[Infantry]
{chr(10).join(infantry)}

[OverlayPack]
{pack_lcw(bytes(overlay))}

[OverlayDataPack]
{pack_lcw(bytes(overlay_data))}

[IsoMapPack5]
{pack_lzo(bytes(cells))}
'''


def main() -> None:
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument('output', type=Path, help='Map path to write')
    parser.add_argument(
        '--walls-only', action='store_true',
        help='Use flat ground and omit the cliff-foot power plant',
    )
    args = parser.parse_args()
    mission = build_fixture(walls_only=args.walls_only)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(mission, encoding='ascii')
    print(f'Wrote {args.output} ({len(mission)} characters)')


if __name__ == '__main__':
    main()
