# Native executable oracle. Run with python -m tools.projectile_oracle.arc_domain
# Controlled structs supply upstream world/target/type state. FireAt probes execute
# original scalar bodies and BulletFire; their hooks supply listed virtual/world
# leaves, not native numeric results. Vertical and arc-domain math have no hooks.
from tools.rmg_oracle.harness import GAMEMD
import hashlib
assert hashlib.sha256(GAMEMD.read_bytes()).hexdigest() == '1cdd1180e49024fbda8ad568caac2e86e856063ff67ab38f62b7d2c7bb84298c'
import json,struct,itertools
from pathlib import Path
from tools.rmg_oracle.harness import call,SCRATCH
def words(v):return list(struct.unpack('<II',struct.pack('<d',v)))
rows=[]
for r,h,s,g,mode in itertools.product([0,1,500],[-200,-1,0,1],[0,1,100],[-6.,0.,1.,6.],[0,1]):
    row=dict(range=r,height=h,speed=s,gravity=g,mode=mode)
    for address,label,size in [(0x48A9D0,'angle',8),(0x48A8D0,'word',4)]:
        try:
            out=call(address,ecx=mode,edx=s,stack_args=[r,h&0xffffffff,*words(g),SCRATCH],writes={SCRATCH:b'\xcd'*8},dumps={'out':(SCRATCH,size)})
            row[label+'_ok']=out['eax']&255;row[label+'_raw']=out['dumps']['out']
            if out['eax']&255:row[label]=struct.unpack('<d' if size==8 else '<I',bytes.fromhex(out['dumps']['out']))[0]
        except Exception as e:row[label+'_error']=str(e)
    rows.append(row)
Path(__file__).with_suffix('.json').write_text(json.dumps(rows,indent=2), encoding='utf-8')
print('cases',len(rows),'errors',sum('angle_error'in x or'word_error'in x for x in rows))
for x in rows:
    if x['range']==0 and x['speed']==100 and x['gravity'] in [0.,6.]:print(x)
