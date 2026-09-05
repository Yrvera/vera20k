# Native executable oracle. Run with python -m tools.projectile_oracle.arc_second_probe
# Controlled structs supply upstream world/target/type state. FireAt probes execute
# original scalar bodies and BulletFire; their hooks supply listed virtual/world
# leaves, not native numeric results. Vertical and arc-domain math have no hooks.
from tools.rmg_oracle.harness import GAMEMD
import hashlib
assert hashlib.sha256(GAMEMD.read_bytes()).hexdigest() == '1cdd1180e49024fbda8ad568caac2e86e856063ff67ab38f62b7d2c7bb84298c'
import json,struct
from pathlib import Path
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32,UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EAX,UC_X86_REG_EBP,UC_X86_REG_EBX,UC_X86_REG_ECX,UC_X86_REG_EDI,UC_X86_REG_EIP,UC_X86_REG_ESI,UC_X86_REG_ESP,UC_X86_REG_FPCW
from tools.rmg_oracle.harness import _load_image,NATIVE_FPCW
BASE=0x20000000
SOURCE,STYPE,WEAPON,PTYPE,BULLET,TARGET,RULES,VT,REF,HOOK= [BASE+i*0x2000 for i in range(10)]
SP=BASE+0x40000
def w32(u,a,n):u.mem_write(a,struct.pack('<I',n&0xffffffff))
def r32(u,a):return struct.unpack('<I',u.mem_read(a,4))[0]
def run(dx,dy,dz,speed=100,arcing=True,lobber=False,floater=False,seed=0):
    u=Uc(UC_ARCH_X86,UC_MODE_32);_load_image(u);u.mem_map(BASE,0x50000)
    for reg,n in [(UC_X86_REG_ESP,SP),(UC_X86_REG_EBP,SP+0x1000),(UC_X86_REG_EBX,WEAPON),(UC_X86_REG_ECX,PTYPE),(UC_X86_REG_ESI,SOURCE),(UC_X86_REG_EDI,dz&0xffffffff),(UC_X86_REG_EAX,dy&0xffffffff),(UC_X86_REG_FPCW,NATIVE_FPCW)]:u.reg_write(reg,n)
    for a,n in [(SOURCE,VT),(SOURCE+0x2b4,TARGET),(VT+0x84,HOOK),(VT+0x3f8,HOOK+16),(VT+0x48,0x5F65A0),(TARGET,VT),(REF,WEAPON),(WEAPON+0xA0,PTYPE),(WEAPON+0xA8,speed),(BULLET+0xAC,PTYPE),(0x8871E0,RULES),(RULES+0x16B8,6),(SP+0x28,speed),(SP+0x3c,BULLET),(SP+0x40,WEAPON),(SP+0x68,PTYPE),(SP+0x94,dx),(SP+0x98,dy),(SP+0x9c,dz),(SP+0x1000+12,0)]:w32(u,a,n)
    u.mem_write(SOURCE+0x9c,struct.pack('<iii',1280,1280,0));u.mem_write(TARGET+0x9c,struct.pack('<iii',1280+dx,1280+dy,dz));u.mem_write(SP+0x44,struct.pack('<iii',1280,1280,0))
    u.mem_write(PTYPE+0x29b,bytes([arcing]));u.mem_write(PTYPE+0x295,bytes([floater]));u.mem_write(WEAPON+0x12e,bytes([lobber]))
    w32(u,VT+0x2c,HOOK+32)
    w32(u,VT+0x58,0x5F65A0)
    w32(u,BULLET,0x7E46E4)
    w32(u,BULLET+0x10c,TARGET)
    w32(u,SP-32,seed);w32(u,SP-28,seed)
    seen=[]; second=[]
    def hook(u,a,size,_):
        if a in (HOOK,HOOK+16,HOOK+32):
            sp=u.reg_read(UC_X86_REG_ESP);u.reg_write(UC_X86_REG_EAX,{HOOK:STYPE,HOOK+16:REF,HOOK+32:1}[a]);u.reg_write(UC_X86_REG_EIP,r32(u,sp));u.reg_write(UC_X86_REG_ESP,sp+(8 if a==HOOK+16 else 4))
        elif a in (0x5F4EC0,0x4A9770,0x4A9720):
            sp=u.reg_read(UC_X86_REG_ESP)
            if a==0x5F4EC0:
                u.mem_write(BULLET+0x9c,bytes(u.mem_read(r32(u,sp+4),12)));u.mem_write(BULLET+0x90,b'\x01')
            u.reg_write(UC_X86_REG_EAX,1);u.reg_write(UC_X86_REG_EIP,r32(u,sp));u.reg_write(UC_X86_REG_ESP,sp+(12 if a==0x5F4EC0 else 8))
        elif a in (0x70D590,0x48A8D0,0x48A9D0,0x4CB3D0):seen.append(hex(a))
        elif a==0x48A954:
            sp=u.reg_read(UC_X86_REG_ESP);second.append(dict(al=u.reg_read(UC_X86_REG_EAX)&255,output=bytes(u.mem_read(sp+0x10,8)).hex(),first=bytes(u.mem_read(sp+0x24,8)).hex()))
        elif a==0x6FF93C:u.emu_stop()
    u.hook_add(UC_HOOK_CODE,hook)
    try:u.emu_start(0x6FE8EE,0x6FF01A,count=200000)
    except Exception:
        print('failure',dx,dy,dz,arcing,lobber,'eip',hex(u.reg_read(UC_X86_REG_EIP)),'esp',hex(u.reg_read(UC_X86_REG_ESP)),'calls',seen)
        raise
    assert u.reg_read(UC_X86_REG_EIP) in (0x6FF01A,0x6FF93C),hex(u.reg_read(UC_X86_REG_EIP))
    raw=bytes(u.mem_read(SP+0x50,24))
    if u.reg_read(UC_X86_REG_EIP)==0x6FF01A:
        assert raw==bytes(u.mem_read(BULLET+0xe8,24)), 'ordinary persistent bits differ'
    return dict(delta=[dx,dy,dz],speed=speed,seed=seed,velocity=list(struct.unpack('<ddd',raw)),bits=[f'{b:016x}' for b in struct.unpack('<QQQ',raw)],pitch=r32(u,SP+0x80)&65535,success=u.mem_read(SP+0x27,1)[0],second=second)
rows=[run(r,0,h,seed=seed) for r in range(1660,1676) for h in [0,-1] for seed in [0,0xffffffff,0x12345678]]
Path(__file__).with_suffix('.json').write_text(json.dumps(rows,indent=2), encoding='utf-8')
fail=[r for r in rows if r['second'] and r['second'][0]['al']==0]
print('cases',len(rows),'second solver failures',len(fail));print(json.dumps(fail[:12],indent=2))
