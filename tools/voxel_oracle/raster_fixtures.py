"""Native-generated synthetic raster goldens, including difficult branches.

python -m tools.voxel_oracle.raster_fixtures --write / --check
The input files are generated test art, not extracted commercial assets.
"""
from pathlib import Path
import struct

from tools.native_oracle import finish_vectors, provenance
from tools.voxel_oracle.lighting import native_camera, native_facing, native_multiply
from tools.voxel_oracle.raster import render_native

IDENTITY = [1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0.]


def test_files(reverse=False, no_hva=False, long_hull=False):
    body, headers, tailers, matrices = bytearray(), bytearray(), bytearray(), []
    count, nx, ny, nz = 4, 6, 5, 8
    for section in range(count):
        name = f'fixture{section}'.encode().ljust(16, b'\0')
        headers += name + struct.pack('<3I', section, 1, 0)
        stream, starts, ends = bytearray(), [], []
        for column in range(nx * ny):
            if section == 3 or column in (5, 9, 20):
                starts.append(-1)
                ends.append(-1)
                continue
            starts.append(len(stream))
            if no_hva and section == 2:
                # Native consumes these valid empty runs without imposing a
                # voxel-height-based group limit. Each still advances bytes.
                stream += bytes(30)
            for run, (skip, run_count) in enumerate(((0, 2), (2, 1), (2, 1))):
                stream += bytes((skip, run_count))
                for voxel in range(run_count):
                    color = 17 if (column + run + section) % 4 == 0 else 32 + section * 20 + run
                    stream += bytes((color, (column * 7 + run * 19 + voxel) % 245))
                stream.append(run_count)
            ends.append(len(stream) - 1)
        start_offset, end_offset = len(body), len(body) + nx * ny * 4
        data_offset = len(body) + nx * ny * 8
        body += struct.pack('<' + 'i' * len(starts), *starts)
        body += struct.pack('<' + 'i' * len(ends), *ends)
        body += stream
        scale = (0.5, 0.25, 2., 1.)[section]
        # The fourth section is empty but still changes native union/crop.
        bounds = [-2.37, -1.91, -0.43, 3.78, 3.04, 7.79]
        if long_hull:
            bounds[0], bounds[3] = -89.37, 88.78
        tailer_matrix = IDENTITY.copy()
        if no_hva:
            tailer_matrix[3], tailer_matrix[7], tailer_matrix[11] = 41.25, -19.5, 77.75
        tailers += struct.pack('<3If12f6f4B', start_offset, end_offset, data_offset,
                               scale, *tailer_matrix, *bounds, nx, ny, nz, 4)
        matrix = IDENTITY.copy()
        if reverse:
            matrix[5], matrix[6], matrix[9], matrix[10] = -.6, -.8, .8, -.6
        if section == 2:
            matrix[11] = -3.
        elif section == 3:
            matrix[3], matrix[7], matrix[11] = 9.375, -1.125, 2.625
        matrices.append(matrix)
    vxl = b'Voxel Animation\0' + struct.pack('<4I', 1, count, count, len(body))
    vxl += bytes(770) + headers + body + tailers
    hva = b'NativeFixture\0'.ljust(16, b'\0') + struct.pack('<2I', 1, count)
    hva += b''.join(f'fixture{i}'.encode().ljust(16, b'\0') for i in range(count))
    hva += b''.join(struct.pack('<12f', *matrix) for matrix in matrices)
    vpl = struct.pack('<4I', 16, 31, 32, 0) + bytes(768)
    vpl += bytes(0 if color == 17 else color for _page in range(32) for color in range(256))
    return vxl, None if no_hva else hva, vpl


def generate():
    camera, models = native_camera(), []
    seen_reverse = False
    seen_unstable = False
    seen_zero_erase = False
    for name, reverse, no_hva, long_hull in (
            ('ordinary_sections', False, False, False),
            ('tilted_reverse', True, False, False),
            ('no_hva_ignores_tailer_transform', False, True, False),
            ('long_sections', False, False, True)):
        vxl, hva, vpl = test_files(reverse, no_hva, long_hull)
        cases = []
        for step in (0, 3, 8, 12, 16, 20, 24, 28):
            result = render_native(vxl, hva, vpl, native_multiply(camera, native_facing(step)))
            seen_reverse |= any(p['entry'] == 0x7dfae0 for p in result['raster_params'])
            seen_zero_erase |= result['write_counts']['zero_erases_nonzero'] > 0
            assert len(result['raster_params']) == 4, 'empty section must still submit and raster-dispatch'
            # Sections 0/1 have identical bounds and HVA but different colors;
            # the pairwise exchange sort may reverse their submitted tie order.
            order = result['paint_order']
            seen_unstable |= order.index(1) < order.index(0)
            cases.append({'step': step, **result})
        models.append({'name': name, 'vxl': vxl.hex(),
                       'hva': hva.hex() if hva is not None else None,
                       'vpl': vpl.hex(), 'cases': cases})
    assert seen_reverse, 'fixture must exercise native backward encoded traversal'
    assert seen_unstable, 'fixture must exercise indirectly reversed equal-depth section ties'
    assert seen_zero_erase, 'fixture must exercise zero result erasing an earlier visible pixel'
    return {'source': 'unicorn/gamemd.exe', 'models': models}


if __name__ == '__main__':
    finish_vectors(generate, Path(__file__).with_suffix('.json'), provenance=lambda: provenance(
        scope='Four synthetic VXL models at eight facings: ordinary/reverse-Z, no HVA and long bounds; full native buffers and metadata',
        assumptions=[
            'original x87 0E7F and original startup camera/light/viewer initializers',
            'immutable generated VXL/HVA/VPL files, normal mode 4, frame zero, unmirrored opaque Render core',
            'all sections submitted including empty; original HVA scaling uses limb zero',
            'native loader, normal loop, bounding corners, section exchange sort and both 7DF9C0/7DFAE0 execute',
            'zero VPL output, raw run/skip/duplicate grouping, unequal limb scales and empty section are intentional test inputs',
            'no-HVA model carries nonidentity tailer transforms and ten empty prefix runs in one section',
        ],
        substitutions=['file vtable open/read/seek/close over bounded immutable input bytes',
                       'operator new returns distinct mapped emulator heap allocations'],
        entry_points={'vxl_loader': 0x755DB0, 'hva_loader': 0x5BD5C0, 'hva_scale': 0x5BD730,
                      'submit_box': 0x7540F0, 'sort_rasterize': 0x754510,
                      'section_prepare': 0x756590, 'forward': 0x7DF9C0, 'backward': 0x7DFAE0}))
