"""Execute original Anim AI boundary region on supplied runtime/type fields.
No full-AI/constructor/session claim. No branch result hooks or Rust model.
"""
from pathlib import Path
from itertools import product
from collections import Counter
import hashlib,json,struct
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32,UC_HOOK_CODE
from unicorn.x86_const import *
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
import os


def load_image(uc, executable):
 """Map this PE32 image without any project harness or behavioral hooks."""
 data = executable.read_bytes()
 pe = struct.unpack_from('<I', data, 0x3c)[0]
 assert data[pe:pe+4] == b'PE\0\0'
 section_count, optional_size = struct.unpack_from('<H', data, pe+6)[0], struct.unpack_from('<H', data, pe+20)[0]
 optional = pe + 24
 assert struct.unpack_from('<H', data, optional)[0] == 0x10b
 base = struct.unpack_from('<I', data, optional+28)[0]
 size = struct.unpack_from('<I', data, optional+56)[0]
 headers = struct.unpack_from('<I', data, optional+60)[0]
 uc.mem_map(base, (size+0xfff)&~0xfff)
 uc.mem_write(base, data[:headers])
 for index in range(section_count):
  section = optional + optional_size + index*40
  rva, raw_size, raw_offset = struct.unpack_from('<III', data, section+12)
  if raw_size:
   uc.mem_write(base+rva, data[raw_offset:raw_offset+raw_size])


def configured_executable():
 if explicit := os.environ.get('VERA20K_GAMEMD_EXE'):
  return Path(explicit).expanduser().resolve()
 if retail := os.environ.get('RA2_DIR'):
  return (Path(retail)/'gamemd.exe').expanduser().resolve()
 raise RuntimeError('Set VERA20K_GAMEMD_EXE or RA2_DIR to the retail executable')


HASH='1cdd1180e49024fbda8ad568caac2e86e856063ff67ab38f62b7d2c7bb84298c'
MEM=0x21000000; ANIM=MEM; TYPE=MEM+0x1000

def signed(v):return ((v+0x80000000)&0xffffffff)-0x80000000

def run():
 executable = configured_executable()
 assert hashlib.sha256(executable.read_bytes()).hexdigest()==HASH
 u=Uc(UC_ARCH_X86,UC_MODE_32);load_image(u, executable);u.mem_map(MEM,0x10000)
 def put(a,v):u.mem_write(a,struct.pack('<I',v&0xffffffff))
 def get(a):return struct.unpack('<i',u.mem_read(a,4))[0]
 put(ANIM+0xc8,TYPE)
 endpoint=None
 def observe(u,a,n,_):
  nonlocal endpoint
  if a in (0x4246dc,0x4247b1,0x4247f3,0x424b42):
   endpoint={0x4246dc:'bounce',0x4247b1:'boundary_loop',0x4247f3:'boundary_terminal',0x424b42:'continue'}[a];u.emu_stop()
 for a in (0x4246dc,0x4247b1,0x4247f3,0x424b42):u.hook_add(UC_HOOK_CODE,observe,begin=a,end=a)
 rows=[];counts=Counter()
 bounds=[('stock_FH',0,0,32,15),('stock_FDHD',16,0,32,31),('stock_EG',0,0,1,2),('negative_start',-3,-2,8,10),('wrapped_loop_difference',-2147483648,0,2147483647,2147483647),('negative_end',3,-4,-2,-1)]
 for name,start,loop_start,loop_end,end in bounds:
  d=signed(loop_end-start)
  stages=sorted({signed(x+y) for x in (0,start,end,d,loop_start) for y in (-1,0,1)})
  for loop,stage,shadow,reverse,ctor_reverse,ping,step in product((0,1,2,255),stages,(0,1),(0,1),(0,1),(0,1),(-2147483648,-1,0,1)):
   for off,v in ((0x2b4,start),(0x2b8,loop_start),(0x2bc,loop_end),(0x2c0,end)):put(TYPE+off,v)
   for off,v in ((0x370,ping),(0x371,reverse),(0x372,shadow)):u.mem_write(TYPE+off,bytes([v]))
   u.mem_write(ANIM+0x120,bytes([ctor_reverse]));u.mem_write(ANIM+0x195,bytes([loop]));put(ANIM+0xc4,step);put(ANIM+0xac,stage)
   for reg,v in ((UC_X86_REG_ESI,ANIM),(UC_X86_REG_EAX,TYPE),(UC_X86_REG_EBX,stage&0xffffffff),(UC_X86_REG_EDI,0),(UC_X86_REG_ESP,MEM+0xff00),(UC_X86_REG_EFLAGS,2)):u.reg_write(reg,v)
   endpoint=None;u.emu_start(0x42468c,0x424b43,count=100)
   assert endpoint is not None,hex(u.reg_read(UC_X86_REG_EIP))
   result=[endpoint,get(ANIM+0xc4),u.mem_read(ANIM+0x195,1)[0],get(ANIM+0xac)]
   rows.append([name,loop,stage,shadow,reverse,ctor_reverse,ping,step,*result]);counts[endpoint]+=1
 for name,stage in [('stock_FH',15),('stock_FDHD',16)]:
  r=next(r for r in rows if r[:8]==[name,1,stage,1,0,0,0,1]);assert r[8:]==['boundary_terminal',1,0,stage]
 raw=bytes(u.mem_read(0x42468c,0x4247b7-0x42468c))
 native=[f'{i.address:08X}: {i.mnemonic} {i.op_str}' for i in Cs(CS_ARCH_X86,CS_MODE_32).disasm(raw,0x42468c)]
 out=dict(native_sha256=HASH,script_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),region_sha256=hashlib.sha256(raw).hexdigest(),entry='0042468C',stops={'bounce':'004246DC after original neg/store','boundary_loop':'004247B1 after original loop decrement and stage reset','boundary_terminal':'004247F3 after original loop decrement; before Next/terminal consumers','continue':'00424B42 before epilogue'},native=native,bounds_columns=['name','start','loop_start','loop_end','end'],bounds=bounds,columns=['bounds','loop','stage','shadow','reverse','constructor_reverse','ping_pong','step','decision','step_after','loop_after','stage_after'],counts=dict(counts),rows=rows)
 Path(__file__).with_suffix('.json').write_text(json.dumps(out,separators=(',',':'))+'\n')
 print(f'PASS: {len(rows)} original Anim boundary cases {dict(counts)}')
if __name__=='__main__':run()
