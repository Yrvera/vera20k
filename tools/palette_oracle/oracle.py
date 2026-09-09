"""Read-only original gamemd x86 execution for palette table research.

No Ghidra/process mutation. Only isolated Unicorn memory is initialized. The
executable hash is checked; output goldens are bytes executed from that image.
"""
from pathlib import Path
import hashlib
import json
import struct
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32
from unicorn.x86_const import *

import argparse
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--exe', required=True, type=Path)
parser.add_argument('--output', required=True, type=Path)
args = parser.parse_args()
EXE = args.exe
EXPECTED = '1cdd1180e49024fbda8ad568caac2e86e856063ff67ab38f62b7d2c7bb84298c'
OUT = args.output
OUT.mkdir(parents=True, exist_ok=True)
BASE = 0x400000
HEAP = 0x1000000
STACK = 0x2000000
STOP = 0x2100000


def load_image():
    raw = EXE.read_bytes()
    assert hashlib.sha256(raw).hexdigest() == EXPECTED
    pe = struct.unpack_from('<I', raw, 0x3c)[0]
    n, optsz = struct.unpack_from('<H', raw, pe + 6)[0], struct.unpack_from('<H', raw, pe + 20)[0]
    opt = pe + 24
    base, size, hdr = struct.unpack_from('<I', raw, opt + 28)[0], struct.unpack_from('<I', raw, opt + 56)[0], struct.unpack_from('<I', raw, opt + 60)[0]
    assert base == BASE
    img = bytearray(size)
    img[:hdr] = raw[:hdr]
    for i in range(n):
        off = opt + optsz + i * 40
        vs, va, rs, rp = struct.unpack_from('<IIII', raw, off + 8)
        img[va:va + rs] = raw[rp:rp + rs]
    return bytes(img)


IMAGE = load_image()


def machine():
    u = Uc(UC_ARCH_X86, UC_MODE_32)
    u.mem_map(BASE, (len(IMAGE) + 4095) & ~4095)
    u.mem_write(BASE, IMAGE)
    u.mem_map(HEAP, 0x100000)
    u.mem_map(STACK, 0x100000)
    u.mem_map(STOP, 0x1000)
    # Normal game x87 policy. Math__ftol retains this word after forcing it.
    # Also record the cached word in the report; a live scene read must check it.
    u.reg_write(UC_X86_REG_FPCW, 0x0e7f)
    return u


def put32(u, a, v):
    u.mem_write(a, struct.pack('<I', v & 0xffffffff))


def call(u, address, args, ecx=0):
    sp = STACK + 0x80000
    u.mem_write(sp, struct.pack('<' + 'I' * (len(args) + 1), STOP, *[a & 0xffffffff for a in args]))
    u.reg_write(UC_X86_REG_ESP, sp)
    u.reg_write(UC_X86_REG_ECX, ecx)
    u.emu_start(address, STOP, count=15000000)
    assert u.reg_read(UC_X86_REG_EIP) == STOP, hex(u.reg_read(UC_X86_REG_EIP))


def intensity_table(n):
    u = machine()
    # Exact generator loop, no allocator/cache scaffolding: EDI=N-1,
    # ESI=0, EBX=output. Stop before native cache append.
    u.reg_write(UC_X86_REG_EDI, n - 1)
    u.reg_write(UC_X86_REG_ESI, 0)
    u.reg_write(UC_X86_REG_EBX, HEAP)
    u.emu_start(0x420196, 0x4201d5, count=2000000)
    assert u.reg_read(UC_X86_REG_EIP) == 0x4201d5
    result = bytes(u.mem_read(HEAP, 0x20000))
    assert all(v == min((i & 255) * (i >> 8) * (n - 1) // 32258, n - 1) << 8
               for i, (v,) in enumerate(struct.iter_unpack('<H', result)))
    return result


def base_scales():
    u = machine()
    out = []
    num, den = struct.unpack_from('<d', IMAGE, 0x7ed0b0-BASE)[0].as_integer_ratio()
    for light in range(2001):
        # Native LEA/SHL, FILD, FMUL, original Math__ftol call.
        u.reg_write(UC_X86_REG_ESP, STACK + 0x80000)
        u.reg_write(UC_X86_REG_EAX, light)
        u.emu_start(0x556192, 0x5561b1, count=200)
        actual = u.reg_read(UC_X86_REG_EAX)
        assert actual == light * 1000 * num // den, (light, actual)
        assert actual == max(light * 65536 - 1, 0) // 1000, (light, actual)
        out.append(actual)
    return out


def palette_table(n, rgb, palette, mode, mask=None):
    u = machine()
    obj, pal, dest = HEAP, HEAP + 0x1000, HEAP + 0x10000
    put32(u, obj + 0x16c, n)
    put32(u, obj + 0x170, dest)
    put32(u, obj + 0x188, pal)
    if mask is not None:
        put32(u, obj + 0x190, HEAP + 0x2000)
        u.mem_write(HEAP + 0x2000, mask)
    u.mem_write(pal, palette)
    put32(u, 0x829d20, 2)  # RGB565 format dispatch, conditional native mode.
    u.mem_write(0x84e860, bytes((mode == 'cmov', 0, mode == 'mmx')))
    call(u, 0x556090, rgb + (0,), obj)
    return bytes(u.mem_read(dest, n * 512))


def plain_palette_table(n, palette):
    # ConvertClass constructor 0048E740 uses 004BBB00 for RGB565. This route
    # does not dispatch to MMX and has an exact 65536 neutral scale.
    u = machine()
    dest, pal = HEAP, HEAP + 0x10000
    u.mem_write(pal, palette)
    for address, value in ((0x8a0dd0,11),(0x8a0dd4,3),(0x8a0dd8,0),(0x8a0ddc,3),(0x8a0de0,5),(0x8a0de4,2)):
        put32(u,address,value)
    u.reg_write(UC_X86_REG_EDX,n)
    call(u,0x4bbb00,(pal,),dest)
    return bytes(u.mem_read(dest,n*512))


def scanline(n, lut, palette, brightness, abuf):
    u = machine()
    obj, pal, lookup, remap_ptr, remap, dst, src, aa, circ = [HEAP + x for x in (0, 0x1000, 0x10000, 0x31000, 0x32000, 0x33000, 0x34000, 0x35000, 0x36000)]
    for off, value in ((4, remap_ptr), (8, pal), (12, lookup)):
        put32(u, obj + off, value)
    put32(u, remap_ptr, remap)
    u.mem_write(remap, bytes(range(256)))
    u.mem_write(pal, palette)
    u.mem_write(lookup, lut)
    u.mem_write(src, bytes(range(256)))
    u.mem_write(aa, struct.pack('<256H', *([abuf] * 256)))
    u.mem_write(dst, bytes.fromhex('efbe') * 256)
    put32(u, 0x87e8a4, circ)
    put32(u, circ + 0x1c, aa + 512)
    put32(u, circ + 0x20, 512)
    call(u, 0x493f30, (dst, src, 256, 0, 0, aa, brightness, 0), obj)
    return bytes(u.mem_read(dst, 512))


def direct_scanline(lut, palette, brightness, abuf, mode):
    u = machine()
    obj, pal, lookup, dst, src, aa, acirc, zz, zcirc, zsrc = [HEAP + x for x in (0, 0x1000, 0x10000, 0x33000, 0x34000, 0x35000, 0x36000, 0x37000, 0x38000, 0x39000)]
    put32(u, obj + 4, pal)
    put32(u, obj + 8, lookup)
    u.mem_write(pal, palette)
    u.mem_write(lookup, lut)
    u.mem_write(src, bytes(range(1, 256)))
    u.mem_write(aa, struct.pack('<255H', *([abuf] * 255)))
    u.mem_write(zz, b'\xff\xff' * 255)
    for global_ptr, circ, data in ((0x87e8a4, acirc, aa), (0x887644, zcirc, zz)):
        put32(u, global_ptr, circ)
        put32(u, circ + 0x1c, data + 510)
        put32(u, circ + 0x20, 510)
    if mode == 'voxel_cached':
        call(u, 0x497fd0, (dst, src, 255, 0, 0, zz, aa, brightness, 0, zsrc), obj)
    else:
        address = 0x493df0 if mode == 'shp_opaque' else 0x494b60
        call(u, address, (dst, src, 255, 0, zz, aa, brightness, 0), obj)
    return bytes(u.mem_read(dst, 510))


def cell_light_samples(extra_cases=None):
    # Original call setup establishes scale/additive/top/common/bottom/R/G/B
    # ownership. Execute finalization with supplied post-gather inputs, then
    # original Cell stores; no gather, Convert allocation or call substitution.
    u = machine()
    frame, cell, output = STACK + 0x1000, HEAP + 0x1000, HEAP + 0x100
    u.reg_write(UC_X86_REG_ESP, frame)
    u.reg_write(UC_X86_REG_ESI, cell)
    u.emu_start(0x483eb0, 0x483eda, count=100)
    assert u.reg_read(UC_X86_REG_EIP) == 0x483eda
    sp = u.reg_read(UC_X86_REG_ESP)
    offsets = [v - frame for v in struct.unpack('<8I', u.mem_read(sp, 32))]
    assert offsets == [0x20, 0x24, 0x28, 0x2c, 0x30, 0x1c, 4, 8]
    cases = [
        ((1000, 1000, 1000), 0, 1000, 1000, 2),
        ((1000, 1000, 1000), 0, 600, 600, 2),
        ((1000, 1000, 1000), 0, 900, 900, 2),
        ((290, 390, 740), 0, 863, 871, 2),
        ((1050, 1050, 1010), 200, 1150, 1182, 2),
        ((800, 600, 500), 200, 1000, 1080, 2),
        ((2000, 1000, 1500), 600, 900, 932, 2),
        ((0, 0, 0), 200, 1000, 1080, 2),
        ((240, 480, 800), 0, 1000, 1000, 2),
        ((-100, 0, -1), -200, -100, -50, 2),
        ((3000, 2000, 1999), 3000, 2500, 2600, 2),
    ]
    for maximum in range(2001):
        rgb = (maximum, maximum // 2, maximum * 3 // 4)
        top = (-50, 0, 1, 600, 1000, 1999, 2000, 2500)[maximum % 8]
        for rotation in range(3):
            cases.append((rgb[rotation:] + rgb[:rotation], 200, top, top + 32, maximum % 3))
    if extra_cases is not None:
        cases = extra_cases
    result = bytearray()
    for rgb, additive, top, bottom, detail in cases:
        sp = frame + 0x100
        u.mem_write(output, struct.pack('<8i', 0, additive, top, 0, bottom, *rgb))
        for off, value in ((0x40, output), (0x4c, output + 12), (0x54, output + 20), (0x58, output + 24), (0x5c, output + 28)):
            put32(u, sp + off, value)
        for register, value in ((UC_X86_REG_ESP, sp), (UC_X86_REG_EBX, output + 8), (UC_X86_REG_ESI, output + 16), (UC_X86_REG_FPCW, 0x0e7f)):
            u.reg_write(register, value)
        u.emu_start(0x4845a2, 0x484617, count=10000)
        assert u.reg_read(UC_X86_REG_EIP) == 0x484617
        actual = list(struct.unpack('<8i', u.mem_read(output, 32)))
        for off, value in zip(offsets, actual):
            put32(u, frame + off, value)
        u.reg_write(UC_X86_REG_ESP, frame)
        u.reg_write(UC_X86_REG_ESI, cell)
        u.emu_start(0x483f7a, 0x483fd8, count=100)
        assert u.reg_read(UC_X86_REG_EIP) == 0x483fd8
        stored = [struct.unpack('<I', u.mem_read(cell + 0x104, 4))[0], *struct.unpack('<7H', u.mem_read(cell + 0x108, 14))]
        assert stored == [actual[0], *[v & 0xffff for v in actual[1:]]]
        # 544E70's neutral fast path retains the identity key. Other keys pass
        # through original 555AC0, whose detail owner is A8EB78.
        if actual[5:] != [1000, 1000, 1000]:
            put32(u, 0xa8eb78, detail)
            u.reg_write(UC_X86_REG_EDX, output + 24)
            call(u, 0x555ac0, (output + 28,), output + 20)
            actual[5:] = struct.unpack('<3i', u.mem_read(output + 20, 12))
        result.extend(struct.pack('<15i', *rgb, additive, top, bottom, detail, *actual))
    return bytes(result), len(cases)


def ground_level_samples():
    # Prepared ReadDouble returns (native float scan widened to double).
    # Execute original FMUL/FADD/full Math__ftol for all four active fields.
    # CCINI allocation/token scanning is outside this isolated fixture.
    u = machine()
    assert struct.unpack_from('<d', IMAGE, 0x7e4658-BASE)[0] == 1000.0
    assert IMAGE[0x7f0e78-BASE:0x7f0e7c-BASE].hex() == '6f12833a'
    u.reg_write(UC_X86_REG_EBP, HEAP)
    u.emu_start(0x6838f7, 0x68390b, count=10)
    assert u.reg_read(UC_X86_REG_EIP) == 0x68390b
    defaults = struct.unpack('<2i', u.mem_read(HEAP + 0x3540, 8))
    assert defaults == (50, 8)

    def set_st0(value):
        bits = struct.unpack('<Q', struct.pack('<d', value))[0]
        exponent, fraction = (bits >> 52) & 2047, bits & ((1 << 52) - 1)
        significand = (fraction | ((1 << 52) if exponent else 0)) << 11
        extended_exponent = ((bits >> 63) << 15) | (exponent - 1023 + 16383 if exponent else 0)
        u.reg_write(UC_X86_REG_FPSW, 0)
        u.reg_write(UC_X86_REG_FPTAG, 0xfffc)
        u.reg_write(UC_X86_REG_FP0, (significand, extended_exponent))

    def quantize(value, start, end):
        set_st0(value)
        u.reg_write(UC_X86_REG_ESP, STACK + 0x80000)
        u.emu_start(start, end, count=200)
        assert u.reg_read(UC_X86_REG_EIP) == end
        return struct.unpack('<i', struct.pack('<I', u.reg_read(UC_X86_REG_EAX)))[0]

    sites = ((0x68a92e, 0x68a93f), (0x68a968, 0x68a979),
             (0x68aa8a, 0x68aa9b), (0x68aac4, 0x68aad5))
    tokens = ['0', '.008', '.05', '.20', '.032', '.40', '.5', '.0039', '.0319',
              '.0079', '.0119', '-.032', '-.20', '1.99999', '.00099', '.000989']
    records = []
    for token in tokens:
        value = struct.unpack('<f', struct.pack('<f', float(token)))[0]
        output = [quantize(value, *site) for site in sites]
        assert len(set(output)) == 1
        records.append({'token': token, 'f32_widened': value, 'units': output[0]})
    roundtrips = []
    for offset, initial, start, end, site in (
        (0x3540, defaults[0], 0x68a905, 0x68a91f, sites[0]),
        (0x3544, defaults[1], 0x68a93f, 0x68a959, sites[1]),
    ):
        put32(u, HEAP + offset, initial)
        u.reg_write(UC_X86_REG_ESI, HEAP)
        u.reg_write(UC_X86_REG_ESP, STACK + 0x80000)
        u.reg_write(UC_X86_REG_FPSW, 0)
        u.reg_write(UC_X86_REG_FPTAG, 0xffff)
        u.emu_start(start, end, count=20)
        assert u.reg_read(UC_X86_REG_EIP) == end
        value = struct.unpack('<d', u.mem_read(u.reg_read(UC_X86_REG_ESP), 8))[0]
        actual = quantize(value, *site)
        assert actual == initial
        roundtrips.append({'offset': hex(offset), 'internal': initial,
                           'original_default_double': value, 'units': actual})
    # Scenario arithmetic into the finalizer is separately traced in 484180;
    # these prepared cases cover production grid expectations after parsing.
    cases = [((1000, 1000, 1000), 0, top, bottom, 2) for top, bottom in
             ((950, 982), (982, 1014), (800, 928), (928, 1056), (600, 600), (1000, 1128))]
    cases.append(((290, 390, 740), 0, 875, 919, 2))
    cells, count = cell_light_samples(cases)
    return {'sites': [[hex(a), hex(b)] for a, b in sites], 'authored': records,
            'missing_key_roundtrips': roundtrips,
            'cell_finalizations': [list(row) for row in struct.iter_unpack('<15i', cells)],
            'limits': 'prepared ReadDouble returns and post-gather Cell values; original reset/default inverse/quantization/finalization instructions; excludes CCINI token scanning and map gather'}


def main():
    # Every palette channel takes all 256 byte values, with decorrelated RGB.
    pal = bytes(c for i in range(256) for c in (i, (i * 73) % 256, 255 - i))
    report = {'exe': EXE.name, 'sha256': EXPECTED, 'unicorn': __import__('unicorn').__version__, 'x87_control_word': '0x0e7f', 'cached_x87_control_word': hex(struct.unpack_from('<I', IMAGE, 0x822d80-BASE)[0]), 'scope': 'isolated original x86 table/scanline execution; RGB565 only; no retail scene capture', 'tables': [], 'scanlines': []}
    scales = base_scales()
    ground = ground_level_samples()
    (OUT / 'ground-level.json').write_text(json.dumps(ground, indent=2) + '\n')
    report['ground_level'] = ground
    cells, cell_count = cell_light_samples()
    (OUT / 'cell-light-finalization.bin').write_bytes(cells)
    report['cell_light_finalization'] = {'cases': cell_count, 'bytes_sha256': hashlib.sha256(cells).hexdigest(), 'record': '15 little-endian i32: input RGB/additive/top/bottom/detail; output scale/additive/top/common/bottom/quantized RGB', 'limits': 'prepared post-gather values; original call setup/finalization/Cell stores and key quantizer; excludes gather/Convert allocation/scene'}
    (OUT / 'base-scales-0-2000.bin').write_bytes(struct.pack('<2001I', *scales))
    report['base_scale_samples'] = {str(i): scales[i] for i in (1, 32, 128, 512, 768, 992, 999, 1000, 1200, 1999, 2000)}
    for n, rgb in ((53, (1000, 1000, 1000)), (53, (992, 768, 512)), (27, (512, 640, 768)), (53, (2000, 1, 1999))):
        lut = intensity_table(n)
        (OUT / f'intensity-{n}.bin').write_bytes(lut)
        tables = {mode: palette_table(n, rgb, pal, mode) for mode in ('scalar', 'cmov', 'mmx')}
        for mode, data in tables.items():
            name = f'palette-{n}-' + '-'.join(map(str, rgb)) + f'-{mode}.bin'
            (OUT / name).write_bytes(data)
            words, ref = list(struct.iter_unpack('<H', data)), list(struct.iter_unpack('<H', tables['scalar']))
            diffs = [i for i, pair in enumerate(zip(words, ref)) if pair[0] != pair[1]]
            report['tables'].append({'file': name, 'n': n, 'rgb': rgb, 'mode': mode, 'bytes_sha256': hashlib.sha256(data).hexdigest(), 'scalar_difference_count': len(diffs), 'first_differences': [{'row': i // 256, 'index': i % 256, 'value': words[i][0], 'scalar': ref[i][0]} for i in diffs[:12]]})
        if rgb in ((1000, 1000, 1000), (512, 640, 768)):
            for b in (-1, 0, 1, 992, 999, 1000, 1001, 1199, 1200, 1201, 1999, 2000, 32767):
                for a in (0, 1, 63, 126, 127, 128, 254, 255):
                    data = scanline(n, lut, tables['mmx'], b, a)
                    q = min(max(b, 0) * 261 >> 11, 254)
                    row = min(a * q * (n - 1) // 32258, n - 1)
                    wanted = b'\xef\xbe' + tables['mmx'][row * 512 + 2:(row + 1) * 512]
                    assert data == wanted, (n, b, a, row)
                    for mode in ('shp_opaque', 'voxel_uncached', 'voxel_cached'):
                        actual = direct_scanline(lut, tables['mmx'], b, a, mode)
                        assert actual == wanted[2:], (n, b, a, row, mode)
                    report['scanlines'].append({'n': n, 'brightness': b, 'abuf': a, 'q': q, 'row': row, 'bytes_sha256': hashlib.sha256(data).hexdigest(), 'shp_00493f30': 'match', 'shp_opaque_00493df0': 'match', 'voxel_uncached_00494b60': 'match', 'voxel_cached_00497fd0': 'match'})
    mask = IMAGE[0x83e1ac-BASE:0x83e1ac-BASE+256]
    assert [i for i, v in enumerate(mask) if v == 0] == list(range(240, 255))
    report['house_mask_zero_indices'] = list(range(240, 255))
    for rgb in ((1000, 1000, 1000), (992, 768, 512)):
        for mode in ('scalar', 'cmov', 'mmx'):
            data = palette_table(53, rgb, pal, mode, mask)
            name = 'palette-53-' + '-'.join(map(str, rgb)) + f'-{mode}-house.bin'
            (OUT / name).write_bytes(data)
            # Verify decoded formula independently against native-executed bytes.
            for row in range(53):
                for i in range(1, 256):
                    channels = pal[i*3:i*3+3]
                    row_scales = [scales[v] * 2 * row // 52 for v in rgb]
                    if not mask[i]:
                        row_scales = [min(row * 65536 // 6, 65536)] * 3
                    if mode == 'mmx':
                        lit = [min(c * (s >> 4) >> 12, 255) for c, s in zip(channels, row_scales)]
                    else:
                        lit = [min(c * s >> 16, 255) for c, s in zip(channels, row_scales)]
                    wanted = (lit[0] >> 3) << 11 | (lit[1] >> 2) << 5 | (lit[2] >> 3)
                    actual = struct.unpack_from('<H', data, (row*256+i)*2)[0]
                    assert actual == wanted, (mode, rgb, row, i, actual, wanted)
            report['tables'].append({'file': name, 'n': 53, 'rgb': rgb, 'mode': mode, 'mask': 'retail0x0083e1ac', 'decoded_formula_all_entries': 'match', 'bytes_sha256': hashlib.sha256(data).hexdigest()})
    # First ColorScheme is N=1 (AltPalette), with the same retail mask.
    data = palette_table(1, (1000,1000,1000), pal, 'mmx', mask)
    name = 'palette-1-1000-1000-1000-mmx-house.bin'
    (OUT/name).write_bytes(data)
    report['tables'].append({'file':name,'n':1,'rgb':[1000]*3,'mode':'mmx','mask':'retail0x0083e1ac','bytes_sha256':hashlib.sha256(data).hexdigest()})
    for n in (1,53):
        data = plain_palette_table(n,pal)
        name = f'palette-{n}-plain.bin'
        (OUT/name).write_bytes(data)
        for row in range(n):
            scale = 65536 if n==1 else row*131072//(n-1)
            for i in range(256):
                lit = [min(c*scale>>16,255) for c in pal[i*3:i*3+3]]
                wanted = (lit[0]>>3)<<11 | (lit[1]>>2)<<5 | lit[2]>>3
                assert struct.unpack_from('<H',data,(row*256+i)*2)[0] == wanted
        report['tables'].append({'file':name,'n':n,'rgb':[1000]*3,'mode':'plain','bytes_sha256':hashlib.sha256(data).hexdigest()})
    (OUT / 'oracle-results.json').write_text(json.dumps(report, indent=2))
    print(f"Native original-byte oracle: {len(report['tables'])} complete palettes, {len(report['scanlines']) * 4} scanlines, 2001 base scales, {cell_count} cell finalizations passed")


if __name__ == '__main__':
    main()
