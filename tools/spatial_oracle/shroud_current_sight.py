"""Original ordinary-cell reveal and selected hostile-gap transitions.
Run python -B -m tools.spatial_oracle.shroud_current_sight --check / --write.
"""
import json,struct
from pathlib import Path
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32
from unicorn.x86_const import UC_X86_REG_ECX,UC_X86_REG_ESP,UC_X86_REG_EAX,UC_X86_REG_EDI
from tools.native_oracle import load_image,run_checked,finish_vectors,provenance,STACK_BASE,STACK_SIZE,RET_MAGIC
from tools.spatial_oracle.map_queries import packed,dwords
MAP,TABLE,CELL,QUERY,WORLD,PLAYER=0x87F7E8,0xC00000,0xB00000,0xB90000,0xB90020,0xBA0000

def execute(name,ops):
 u=Uc(UC_ARCH_X86,UC_MODE_32);load_image(u);u.mem_map(STACK_BASE,STACK_SIZE);u.mem_map(RET_MAGIC,4096)
 sp=STACK_BASE+STACK_SIZE-0x1000
 u.mem_write(MAP+0x13C,dwords(TABLE,0x40000));u.mem_write(TABLE,bytes(0x100000))
 u.mem_write(MAP+0xF4,dwords(12,12))
 n=0
 for ay in range(33):
  for ax in range(33):
   if not (12<ax+ay<=36 and ax-ay<12 and ay-ax<12):continue
   p=0xB10000+n*0x200;n+=1
   u.mem_write(p,bytes(0x200));u.mem_write(p,dwords(0x7E4EEC));u.mem_write(p+0x24,packed(ax,ay));u.mem_write(p+0x120,bytes([254,254]));u.mem_write(p+0x130,dwords(1,0,0,0));u.mem_write(TABLE+(ay*512+ax)*4,dwords(p))
 u.mem_write(sp,dwords(RET_MAGIC));u.reg_write(UC_X86_REG_ESP,sp);run_checked(u,0x49F2F0,RET_MAGIC,count=1000)
 for y in range(9,12):
  for x in range(9,12):
   p=CELL+((y-9)*3+x-9)*0x200
   u.mem_write(p,bytes(0x200));u.mem_write(p,dwords(0x7E4EEC));u.mem_write(p+0x24,packed(x,y));u.mem_write(p+0x120,bytes([254,254]));u.mem_write(p+0x130,dwords(1,0,0,0))
   u.mem_write(TABLE+(y*512+x)*4,dwords(p))
 center=CELL+4*0x200
 u.mem_write(QUERY,packed(10,10));u.mem_write(WORLD,dwords(2560,2560,0));u.mem_write(0xABDE88,dwords(104));u.mem_write(0xA83D4C,dwords(PLAYER));u.mem_write(0xA8B230,dwords(0xBB0000));u.mem_write(0x887324,dwords(0xBC0000))
 def observe(op):
  u.mem_write(sp,dwords(RET_MAGIC,WORLD));u.reg_write(UC_X86_REG_ESP,sp);run_checked(u,0x586360,RET_MAGIC,count=1000)
  return dict(op=op,counter=struct.unpack('<i',u.mem_read(center+0x130,4))[0],gap=struct.unpack('<i',u.mem_read(center+0x134,4))[0],alt=struct.unpack('<I',u.mem_read(center+0x12C,4))[0],flags=struct.unpack('<I',u.mem_read(center+0x140,4))[0],cache=list(struct.unpack('<bb',u.mem_read(center+0x120,2))),shrouded=u.reg_read(UC_X86_REG_EAX)&0xFF)
 out=[observe('constructed')]
 for op in ops:
  u.reg_write(UC_X86_REG_ESP,sp)
  if op in ('reveal','leave'):
   u.mem_write(sp,dwords(RET_MAGIC,QUERY,PLAYER,int(op=='leave')));u.reg_write(UC_X86_REG_ECX,MAP)
   run_checked(u,0x4A9CA0,RET_MAGIC,count=10000)
  elif op=='unshroud':
   u.mem_write(sp,dwords(RET_MAGIC));u.reg_write(UC_X86_REG_ECX,center);run_checked(u,0x4876F0,RET_MAGIC,count=100)
  elif op.startswith('frame'):
   frame=int(op[5:]);u.mem_write(0xA8ED84,dwords(frame));u.reg_write(UC_X86_REG_EDI,frame);run_checked(u,0x55B29A,0x55B2C4,count=100000)
  elif op=='gap':
   u.mem_write(sp+0x10,packed(10,10));run_checked(u,0x6FB2F7,0x6FB3C1,count=1000)
  elif op.startswith('remove'):
   u.mem_write(PLAYER+0x577A,bytes([int(op=='remove_mapclear')]));u.mem_write(sp+0x10,packed(10,10));run_checked(u,0x6FB5E1,0x6FB69E,count=1000)
  else:raise ValueError(op)
  out.append(observe(op))
 return dict(name=name,observations=out)
scenarios=[('never_seen',['gap','remove']),('current_sight',['reveal','gap','remove']),('past_sight',['reveal','leave','gap','remove']),('current_sight_overlap',['reveal','reveal','gap','leave','leave','remove']),('enter_gap',['gap','reveal','leave','remove']),('spysat_remove_gate',['reveal','leave','gap','remove_mapclear']),('fire_only',['unshroud','gap']),('psychic_only',['reveal','leave','gap']),('departure_boundary',['reveal','gap','leave','frame119','frame120']),('return_cancels_pending',['reveal','gap','leave','frame119','reveal','frame120']),('removal_preserves_pending',['gap','reveal','leave','remove','frame120']),('departure_after_boundary',['reveal','gap','frame120','leave','frame121','frame239','frame240']),('second_gap_consumes_pending',['reveal','gap','leave','gap']),('fire_under_gap',['gap','unshroud','frame119','frame120']),('psychic_under_gap',['gap','reveal','leave','frame119','frame120'])]
if __name__=='__main__':
 finish_vectors(lambda:{'cases':[execute(*s) for s in scenarios]},Path(__file__).with_suffix('.json'),provenance=lambda:provenance(
  scope='Original ordinary-cell current-sight versus hostile gap shroud observations',
  assumptions=['Constructor-derived Cell fields supplied:130=1,134=0,120/121=-2; complete Size12x12 allocated diamond, selected3x3 real-cell neighborhood',
   'Original MapCell4A9CA0 executes toreturn with real neighbor-cache and notify leaves; frame0,emptyobjectlists,scenarioFogOfWarclear',
   'Hostile gap apply/remove begin after owner/power/radius admission and stop before second owner classification; original fixedlookup executes',
   'IsShrouded586360 executes on ordinary worldZ0; boolean result isAL, upperEAX ignored',
   'FireUnshroud leaf corresponds5673A0 final0; PsychicReveal sequence corresponds6CD773 final0 then6CD79C final1',
   'Original Logic55B29A modulo gate and complete578100 two-pass sweep execute at supplied signed native frames; selected shroud observations do not certify all edge-cache outputs',
   'No renderer pixels, complete reveal traversal, optionalfogrecords or fullGapGenerator lifecycle comparison'],
  substitutions=[],entry_points={'map_cell':0x4A9CA0,'unshroud':0x4876F0,'hostile_gap_cell':0x6FB2F7,'hostile_remove_cell':0x6FB5E1,'is_shrouded':0x586360,'periodic_logic':0x55B29A,'recalc_shroud':0x578100}))
