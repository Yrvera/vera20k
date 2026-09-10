"""Bounded original BounceClass terrain transactions; no native code replacements."""
import struct
from pathlib import Path
from functools import lru_cache
from tools.projectile_oracle.ordinary_collision import slope_matrices
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32, UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EAX, UC_X86_REG_ECX, UC_X86_REG_ESP, UC_X86_REG_FPCW
from tools.native_oracle import load_image, run_checked, finish_vectors, provenance, NATIVE_FPCW

MEM=0x21000000
TABLE=MEM
CELLS=MEM+0x100000
BODY=MEM+0x120000
SP=MEM+0x1FF000
STOP=0x30000000
DUMMY=0xABDC50

@lru_cache(maxsize=1)
def matrices(): return slope_matrices()

def words(*v): return struct.pack('<'+'I'*len(v),*(x&0xFFFFFFFF for x in v))
def cases():
    def case(name,position=(128,128,420),velocity=(0,0,-8),cells=None,**kwargs):
        return dict(name=name,position=position,velocity=velocity,gravity=1.4,elasticity=0.8,
                    cells=cells if cells is not None else [(0,0,0,0,256,-1)],
                    dummy_level=0,dummy_slope=0,dummy_flags=0,**kwargs)
    result = [case('deck_fall'),case('deck_rest',velocity=(0,0,0)),case('deck_rise',position=(128,128,410),velocity=(0,0,12)),
            case('ordinary_floor',position=(128,128,4),cells=[(0,0,0,0,0,-1)]),
            case('negative_fraction',position=(-1,128,420)),
            case('dummy_ground',position=(-300,128,420),cells=[]),
            case('wall',position=(128,128,40),cells=[(0,0,0,0,0,2)]),
            case('sandbag',position=(128,128,40),cells=[(0,0,0,0,0,0)]),
            case('both_missing_retained',position=(-510,128,40),velocity=(-8,0,-4),cells=[]),
            case('old_only_structural',position=(255,128,420),velocity=(8,0,-8)),
            case('new_only_structural',position=(255,128,420),velocity=(8,0,-8),cells=[(1,0,0,0,256,-1)]),
            case('low_wood_no_deck',cells=[(0,0,0,0,0,237)])]
    for level in (-128,-1,0,1,127):
        c=case(f'signed_level_{level}',position=(128,128,level*104+2),cells=[(0,0,level,0,0,-1)])
        result.append(c)
    for slope in range(21):
        result.append(case(f'slope_{slope}',position=(128,128,1000),velocity=(3,-4,-8),
                           cells=[(0,0,0,slope,0,-1)]))
    for flag in (0,256):
        c=case(f'dummy_live_height_{flag}',position=(-510,128,630),velocity=(-8,0,-8),cells=[])
        c.update(dummy_level=2,dummy_flags=flag);result.append(c)
    for overlay in (26,243):
        result.append(case(f'wall_{overlay}',position=(128,128,40),cells=[(0,0,0,0,0,overlay)]))
    for objects in (['building'],['undeploy'],['large_undeploy'],['nonbuilding'],['undeploy','building'],['building','undeploy']):
        result.append(case('objects_'+'_'.join(objects),position=(128,128,40),
                           cells=[(0,0,0,0,0,-1)],objects=objects))
    return result


def execute(case):
    uc=Uc(UC_ARCH_X86,UC_MODE_32);load_image(uc)
    uc.mem_map(MEM,0x200000);uc.mem_map(STOP,0x1000)
    uc.reg_write(UC_X86_REG_FPCW,NATIVE_FPCW)
    uc.mem_write(0x822D80,struct.pack('<H',NATIVE_FPCW))
    uc.mem_write(0x87F924,words(TABLE,0x40000));uc.mem_write(0x89E7C0,words(104))
    uc.mem_write(0x89C778,words(104))
    uc.mem_write(SP,words(STOP));uc.reg_write(UC_X86_REG_ESP,SP)
    run_checked(uc,0x439610,STOP)
    assert struct.unpack('<i',uc.mem_read(0x89C76C,4))[0]==416
    for slope,matrix in enumerate(matrices()):
        uc.mem_write(0xB45188+48*slope,struct.pack('<12I',*matrix))
    uc.mem_write(DUMMY,bytes(0x200));uc.mem_write(DUMMY+0x24,struct.pack('<hh',1234,-2345))
    uc.mem_write(DUMMY+0x44,words(-1));uc.mem_write(DUMMY+0x11B,bytes([case['dummy_level']&255,case['dummy_slope']]))
    uc.mem_write(DUMMY+0x140,words(case['dummy_flags']))
    for i,(x,y,level,slope,flags,overlay) in enumerate(case['cells']):
        ptr=CELLS+i*0x200
        uc.mem_write(ptr+0x24,struct.pack('<hh',x,y));uc.mem_write(ptr+0x44,words(overlay))
        uc.mem_write(ptr+0x11B,bytes([level&255,slope]));uc.mem_write(ptr+0x140,words(flags))
        uc.mem_write(TABLE+(y*512+x)*4,words(ptr))
    uc.mem_write(0xA8E9A0,b'\x01') # running game:47C520 list admission
    objects=case.get('objects',[])
    for i,kind in enumerate(objects):
        ptr=BODY+0x2000+i*0x2000;typ=ptr+0x800
        uc.mem_write(ptr,words(0x7F5C70 if kind=='nonbuilding' else 0x7E3EBC))
        uc.mem_write(ptr+0x30,words(ptr+0x2000 if i+1<len(objects) else 0))
        uc.mem_write(ptr+0x520,words(typ))
        uc.mem_write(typ+0x408,words(typ if 'undeploy' in kind else 0))
        uc.mem_write(typ+0xEF0,words(3 if kind=='large_undeploy' else 0))
    if objects:uc.mem_write(CELLS+0xE4,words(BODY+0x2000))
    uc.mem_write(BODY,struct.pack('<ddd6f8f',case['elasticity'],case['gravity'],0,
        *case['position'],*case['velocity'],0,0,0,1,0,0,0,1))
    events=[];sampled_ground=[]
    def observe(u,address,_size,_data):
        if address==0x439BF8: sampled_ground.append(struct.unpack("<i",words(u.reg_read(UC_X86_REG_EAX)))[0])
        if address in (0x578080,0x565730):
            sp=u.reg_read(UC_X86_REG_ESP);arg=struct.unpack('<I',u.mem_read(sp+4,4))[0]
            events.append([hex(address),*struct.unpack('<iii',u.mem_read(arg,12))])
        elif address in (0x47C520,0x480510):
            receiver=u.reg_read(UC_X86_REG_ECX)
            events.append([hex(address),*struct.unpack('<hh',u.mem_read(receiver+0x24,4)),receiver==DUMMY])
    uc.hook_add(UC_HOOK_CODE,observe)
    uc.mem_write(SP,words(STOP));uc.reg_write(UC_X86_REG_ESP,SP);uc.reg_write(UC_X86_REG_ECX,BODY)
    run_checked(uc,0x439B00,STOP,count=1000000,required_addresses=(0x578080,0x439A10))
    return dict(**case,ground_z=sampled_ground[0],outcome=uc.reg_read(UC_X86_REG_EAX),
        position_bits=struct.unpack('<3I',uc.mem_read(BODY+0x18,12)),
        velocity_bits=struct.unpack('<3I',uc.mem_read(BODY+0x24,12)),events=events,
        dummy_coord=struct.unpack('<hh',uc.mem_read(DUMMY+0x24,4)))

if __name__=='__main__':
    finish_vectors(lambda:[execute(c) for c in cases()],Path(__file__).with_suffix('.json'),
        provenance=lambda:provenance(scope='Original439B00 full function with bounded terrain fixtures',
            assumptions=['Supplied initialized Cell scalar104; original439610 derives Bounce deck416',
                'Flat contacts and airborne slope queries only; zero quaternion spin; no cliff rollback fixtures',
                'No OS, scheduler, debris constructor, renderer or host effects emulated'],substitutions=[],
            entry_points={'update':0x439B00,'deck_init':0x439610}))
