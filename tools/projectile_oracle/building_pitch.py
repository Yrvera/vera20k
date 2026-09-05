# Native executable oracle. Run with python -m tools.projectile_oracle.building_pitch
# Controlled structs supply upstream world/target/type state. FireAt probes execute
# original scalar bodies and BulletFire; their hooks supply listed virtual/world
# leaves, not native numeric results. Vertical and arc-domain math have no hooks.
from tools.rmg_oracle.harness import GAMEMD
import hashlib
assert hashlib.sha256(GAMEMD.read_bytes()).hexdigest() == '1cdd1180e49024fbda8ad568caac2e86e856063ff67ab38f62b7d2c7bb84298c'
from pathlib import Path
import json

# Execute the existing bounded FireAt->Fire harness with the actual base
# Techno +300 coordinate getter and +AC redispatch. Only type/weapon/facing
# leaves and world insertion are supplied. The source has no locomotor.
src = Path(__file__).with_name('fireat_launch.py').read_text(encoding='utf-8').split('\nrows=')[0]
src = src.replace('floater=False):', 'floater=False, source_z=0, building_height=2):')
src = src.replace('(VT+0x48,0x5F65A0)', '(VT+0x48,0x5F65A0),(VT+0xAC,0x41BE00),(VT+0x300,0x6F3D60),(VT+0x2A8,HOOK+48),(TARGET+0x520,STYPE),(STYPE+0xEF4,building_height)')
src = src.replace('1280,1280,0', '1280,1280,source_z')
src = src.replace('1280+dx,1280+dy,dz', '1280+dx,1280+dy,source_z+dz')
src = src.replace('(HOOK,HOOK+16,HOOK+32):', '(HOOK,HOOK+16,HOOK+32,HOOK+48):')
src = src.replace('HOOK+32:1}', 'HOOK+32:6,HOOK+48:SOURCE+0x388}')
src = src.replace('a==HOOK+16 else 4', 'a in (HOOK+16,HOOK+48) else 4')
src = src.replace('floater=floater,velocity=', 'floater=floater,source_z=source_z,building_height=building_height,velocity=')
exec(compile(src, '<bounded-native-fireat-building-pitch>', 'exec'))
rows = [run(500, 0, dz, arcing=False, source_z=z, building_height=h)
        for dz in (-300, 300)
        for h in (0, 1, 2, 5)
        for z in (0, h*200-21, h*200-20, h*200-19, h*200, h*200+19, h*200+20, h*200+21)]
Path(__file__).with_suffix('.json').write_text(json.dumps(rows, indent=2), encoding='utf-8')
print('Native FireAt + base Techno pivot getter + BulletFire:', len(rows), 'vectors;', sum(bool(r['success']) for r in rows), 'successful')
