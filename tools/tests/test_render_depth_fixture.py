"""Check that comparison walls resolve to visible retail art and clear units."""

import base64
import configparser
import struct
import unittest

from tools.render_depth_fixture import build_fixture, decode_fixture_lcw


# Retail expandmd01.mix/rulesmd.ini, lines 1736..1740. Native 0x00668BF0
# inserts values in declaration order; numeric key text is not the array ID.
RETAIL_OVERLAY_PREFIX = """[OverlayTypes]
1=GASAND
2=CYCL
3=GAWALL
4=BARB
"""


def parse_ini(text):
    ini = configparser.ConfigParser(interpolation=None)
    ini.optionxform = str
    ini.read_string(text)
    return ini


def decode_overlay_section(section):
    packed = base64.b64decode(''.join(section.values()), validate=True)
    result = bytearray()
    cursor = 0
    while cursor < len(packed):
        source_length, output_length = struct.unpack_from('<HH', packed, cursor)
        cursor += 4
        block = decode_fixture_lcw(packed[cursor:cursor + source_length])
        if len(block) != output_length:
            raise ValueError('Decoded overlay chunk length differs from its header')
        result.extend(block)
        cursor += source_length
    if cursor != len(packed):
        raise ValueError('Overlay chunk extends past packed section')
    return result


class RenderDepthFixtureTests(unittest.TestCase):
    def test_ring_resolves_to_retail_allied_wall_and_does_not_overlap_units(self):
        retail_names = list(parse_ini(RETAIL_OVERLAY_PREFIX)['OverlayTypes'].values())
        expected_cells = {
            (x, y) for x in range(53, 57) for y in range(50, 54)
            if x in (53, 56) or y in (50, 53)
        }
        for walls_only in (False, True):
            with self.subTest(walls_only=walls_only):
                ini = parse_ini(build_fixture(walls_only=walls_only))
                identities = decode_overlay_section(ini['OverlayPack'])
                frames = decode_overlay_section(ini['OverlayDataPack'])
                self.assertEqual(len(identities), 512 * 512)
                self.assertEqual(len(frames), 512 * 512)
                wall_cells = set()
                for index, identity in enumerate(identities):
                    if identity == 255:
                        continue
                    self.assertLess(identity, len(retail_names))
                    self.assertEqual(retail_names[identity], 'GAWALL')
                    self.assertEqual(frames[index].bit_count(), 2)
                    wall_cells.add((index % 512, index // 512))
                self.assertEqual(wall_cells, expected_cells)
                for section in ('Units', 'Infantry'):
                    for value in ini[section].values():
                        fields = value.split(',')
                        cell = (int(fields[3]), int(fields[4]))
                        self.assertNotIn(cell, wall_cells, f'{section}: {value}')


if __name__ == '__main__':
    unittest.main()
