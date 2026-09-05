"""Native ordinary directed heading, actual Unit +308, and Dropping reset."""
from pathlib import Path
import json

src = Path(__file__).with_name('fireat_launch.py').read_text(encoding='utf-8').split('\nrows=')[0]
src = src.replace('floater=False):', 'floater=False, dropping=False, rot=-1, turret=False, hull=0, barrel=0):')
src = src.replace('(VT+0x48,0x5F65A0)', '(VT+0x48,0x5F65A0),(VT+0x308,0x740F80),(VT+0x2A8,0x746E30),(SOURCE+0x6C4,STYPE),(SOURCE+0x388,hull),(SOURCE+0x3A0,barrel),(PTYPE+0x2DC,rot)')
src = src.replace('u.mem_write(PTYPE+0x29b,bytes([arcing]));', 'u.mem_write(PTYPE+0x29C,bytes([dropping]));u.mem_write(STYPE+0xCA1,bytes([turret]));u.mem_write(PTYPE+0x29b,bytes([arcing]));')
src = src.replace("u.mem_write(SP+0x44,struct.pack('<iii',1280,1280,0))", "u.mem_write(SP+0x44,struct.pack('<iii',1390,1320,80))")
src = src.replace('floater=floater,velocity=', "floater=floater,dropping=dropping,rot=rot,turret=turret,hull=hull,barrel=barrel,origin=list(struct.unpack('<iii',u.mem_read(BULLET+0x9C,12))),velocity=")
exec(compile(src, '<original-directed-launch>', 'exec'))
rows = [run(500, 200, dz, arcing=arc, dropping=drop, rot=rot, turret=turret, hull=hull, barrel=barrel)
        for dz in (0, -200)
        for arc in (False, True)
        for drop, rot in ((True, 0), (False, -1))
        for turret in (False, True)
        for hull, barrel in ((0, 16384), (16384, 49152), (65535, 0x1234), (0x8123, 0xCDEF))]
Path(__file__).with_suffix('.json').write_text(json.dumps(rows, indent=2), encoding='utf-8')
assert all(r['origin'] == ([1280,1280,0] if r['dropping'] else [1390,1320,80]) for r in rows if r['success'])
print('Original directed launch + Unit heading receivers + Fire:', len(rows), 'cases,', sum(bool(r['success']) for r in rows), 'persistent vectors; Dropping origin reset asserted')
