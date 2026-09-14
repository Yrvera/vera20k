"""Original accepted Infantry Cell destination through Walk first-Process path request.
No eager queue/head production is supplied. Pathfinding core remains outside this caller-timing corpus.
"""
from pathlib import Path
import struct,json,sys
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32,UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EBP,UC_X86_REG_EBX,UC_X86_REG_EAX,UC_X86_REG_ECX,UC_X86_REG_EDX,UC_X86_REG_ESI,UC_X86_REG_EIP,UC_X86_REG_ESP,UC_X86_REG_FPCW
from tools.native_oracle import load_image,run_checked,STACK_BASE,STACK_SIZE,SCRATCH,RET_MAGIC,finish_vectors,provenance
from tools.spatial_oracle.map_queries import dwords,packed
from tools.spatial_oracle import walk_head_occupation as head_native

ACTOR,TARGET,TYPE,HOUSE,LOCO,VT,TVT,CELL,CVT,WEAPON,WEAPON_SLOT,RULES=[SCRATCH+i*0x2000 for i in range(12)]
MAP,TABLE,DUMMY=0x87F7E8,0xC00000,0xABDC50

def query(row):
 u=Uc(UC_ARCH_X86,UC_MODE_32);load_image(u)
 u.mem_map(STACK_BASE,STACK_SIZE);u.mem_map(SCRATCH,0x20000);u.mem_map(RET_MAGIC,0x1000)
 u.reg_write(UC_X86_REG_FPCW,0x0E7F)
 def read32(p):return struct.unpack('<I',u.mem_read(p,4))[0]
 def ret(cleanup,result):
  sp=u.reg_read(UC_X86_REG_ESP);u.reg_write(UC_X86_REG_EAX,result&0xffffffff);u.reg_write(UC_X86_REG_EIP,read32(sp));u.reg_write(UC_X86_REG_ESP,sp+4+cleanup)
 def call(entry,this,args):
  sp=STACK_BASE+STACK_SIZE-0x1000;u.mem_write(sp,dwords(RET_MAGIC,*args));u.reg_write(UC_X86_REG_ESP,sp);u.reg_write(UC_X86_REG_ECX,this)
  run_checked(u,entry,RET_MAGIC,count=100000,required_addresses=[entry]);assert u.reg_read(UC_X86_REG_ESP)==sp+4*(len(args)+1)
 u.mem_write(VT,bytes(u.mem_read(0x7EB058,0x600)));u.mem_write(TVT,bytes(u.mem_read(0x7F5C70,0x600)))
 u.mem_write(ACTOR,dwords(VT));u.mem_write(TARGET,dwords(TVT));u.mem_write(ACTOR+0x21C,dwords(HOUSE));u.mem_write(HOUSE+0x1EC,bytes([int(row.get('human',True))]))
 u.mem_write(ACTOR+0x6C0,dwords(TYPE));u.mem_write(ACTOR+0x6C4,dwords(row.get('doing',0)))
 u.mem_write(ACTOR+0xAC,dwords(row.get('mission',1)));u.mem_write(ACTOR+0xB4,dwords(row.get('queued_mission',-1)))
 u.mem_write(ACTOR+0x90,b'\x01');u.mem_write(ACTOR+0x9C,dwords(10*256+64,10*256+64,0))
 u.mem_write(ACTOR+0x2B4,dwords(TARGET if row.get('target',True) else 0));u.mem_write(TARGET+0x14,dwords(4 if row.get('foot',True) else 0))
 u.mem_write(TARGET+0x9C,dwords(*row.get('target_xyz',[11*256+220,10*256+220,123])))
 u.mem_write(ACTOR+0x5A0,dwords(123));u.mem_write(ACTOR+0x5A4,dwords(TARGET))
 u.mem_write(ACTOR+0x558,packed(9,8));u.mem_write(ACTOR+0x5E0,dwords(2,3,4,5));u.mem_write(ACTOR+0x598,dwords(row.get('nav_queue',0)))
 u.mem_write(ACTOR+0x58C,dwords(SCRATCH+0x1A000));u.mem_write(SCRATCH+0x1A000,dwords(TARGET,CELL))
 if row.get('contact'):
  u.mem_write(ACTOR+0xE4,dwords(SCRATCH+0x1A100));u.mem_write(ACTOR+0xE8,dwords(1));u.mem_write(SCRATCH+0x1A100,dwords(TARGET))
 u.mem_write(ACTOR+0x6B7,b'\x01');u.mem_write(ACTOR+0x640,dwords(50,0,5));u.mem_write(ACTOR+0x668,dwords(40,0,6))
 u.mem_write(0xA8ED84,dwords(100));u.mem_write(0x8871E0,dwords(RULES));u.mem_write(RULES+0x1768,dwords(22))
 u.mem_write(RULES+0x1760,struct.pack('<d',row.get('retry_delay',0.0)))
 u.mem_write(DUMMY+0x24,packed(99,98));u.mem_write(0xB45BE8,dwords(0,0,0));u.mem_write(0xB45C28,dwords(104))
 call(0x75AA90,LOCO,[]);u.mem_write(LOCO+0xC,dwords(ACTOR));u.mem_write(ACTOR+0x674,dwords(LOCO+4))
 u.mem_write(LOCO+0x1C,dwords(30*256+128,10*256+128,0));u.mem_write(LOCO+0x34,b'\x01')
 if row.get('head') or row.get('process_motion'):u.mem_write(LOCO+0x28,dwords(11*256+64,10*256+64,0))
 produced_motion=None
 if row.get('process_motion'):
  # Execute original entry/head gate and +36 producer, stopping before numeric
  # motion. A later cleared head is a declared retirement seam, not Process.
  u.reg_write(UC_X86_REG_ECX,LOCO);u.reg_write(UC_X86_REG_ESP,STACK_BASE+STACK_SIZE-0x1000)
  run_checked(u,0x75AEC0,0x75BD29,count=100,required_addresses=[0x75AEC0,0x75BD25])
  produced_motion=u.mem_read(LOCO+0x36,1)[0]
  if not row.get('head'):u.mem_write(LOCO+0x28,dwords(0,0,0))
 table=bytearray(0x100000)
 for i,(x,y) in enumerate([(10,10),(11,10)]):
  cell=CELL+i*0x200;struct.pack_into('<I',table,(y*512+x)*4,cell);u.mem_write(cell,dwords(CVT));u.mem_write(cell+0x24,packed(x,y));u.mem_write(cell+0x44,dwords(0xffffffff))
 u.mem_write(CVT,bytes(u.mem_read(0x7E4EEC,0x100)));u.mem_write(CVT+0x48,dwords(0x486840));u.mem_write(TABLE,bytes(table));u.mem_write(MAP+0x13C,dwords(TABLE,0x40000));u.mem_write(0x87F924,dwords(TABLE))
 u.mem_write(WEAPON_SLOT,dwords(WEAPON));u.mem_write(WEAPON+0x134,bytes([int(row.get('cell_rangefinding',False))]))
 events=[]
 recent=[]
 def observer(_u,address,_size,_data):
  sp=u.reg_read(UC_X86_REG_ESP)
  recent.append(hex(address))
  if SCRATCH<=address<SCRATCH+0x20000:raise RuntimeError(recent[-20:])
  if address in [read32(0x7E11C8),read32(0x7E11CC)]:
   p=read32(sp+4);value=read32(p)+(1 if address==read32(0x7E11C8) else -1);u.mem_write(p,dwords(value));ret(4,value)
  elif address in [0x4D9FF0,0x41BDD0,0x5F65A0,0x6F7970,0x565730,0x6F77B0,0x5B3040,0x51AA40,0x51AD11,0x4D94B0,0x75ADA0,0x75ACB0,0x4D3920,0x521B40,0x4D896E]:events.append(hex(address))
 u.hook_add(UC_HOOK_CODE,observer)
 u.mem_write(ACTOR+0x5A4,dwords(0));u.mem_write(LOCO+0x14,dwords(1));u.mem_write(LOCO+0x34,b'\0');u.mem_write(LOCO+0x1C,dwords(0,0,0))
 initial_timer=dict(start_frame=read32(ACTOR+0x640),duration=read32(ACTOR+0x648))
 call(0x51AA40,ACTOR,[CELL+0x200,1])
 def state():return dict(movement_timer=dict(start_frame=read32(ACTOR+0x640),duration=read32(ACTOR+0x648)),reference=list(struct.unpack('<hh',u.mem_read(ACTOR+0x558,4))),nav_queue_count=read32(ACTOR+0x598),nav_queue_entries=list(struct.unpack('<II',u.mem_read(SCRATCH+0x1A000,8))),queue=list(struct.unpack('<iiii',u.mem_read(ACTOR+0x5E0,16))),head=list(struct.unpack('<iii',u.mem_read(LOCO+0x28,12))),destination=list(struct.unpack('<iii',u.mem_read(LOCO+0x1C,12))),nav=read32(ACTOR+0x5A4),moving=u.mem_read(LOCO+0x34,1)[0],motion=u.mem_read(LOCO+0x36,1)[0])
 before=state()
 endpoint=0x75BD29 if row.get('head') else 0x4D3920
 if not any(before['destination']):
  call(0x75AEC0,LOCO,[0])
  return dict(input=row,initial_movement_timer=initial_timer,setter=before,process=state(),find_args=None,events=events)
 sp=STACK_BASE+STACK_SIZE-0x1000;u.mem_write(sp,dwords(RET_MAGIC,0));u.reg_write(UC_X86_REG_ESP,sp);u.reg_write(UC_X86_REG_ECX,LOCO)
 run_checked(u,0x75AEC0,endpoint,count=100000,required_addresses=[0x75AEC0,0x75BD25 if row.get('head') else 0x75AFC5])
 sp=u.reg_read(UC_X86_REG_ESP)
 return dict(input=row,initial_movement_timer=initial_timer,setter=before,process=state(),find_args=None if row.get('head') else list(struct.unpack('<III',u.mem_read(sp+4,12))),events=events)

class ChaseHead(head_native.Original):
 def __init__(self,row):
  self.row=row
  super().__init__()
  self.uc.reg_write(UC_X86_REG_FPCW,0x0E7F)
 def observe(self,u,address,size,data):
  if address==head_native.MISSION_GET:
   self.events.append(['mission',self.row['mission']]);self.ret(0,self.row['mission'])
  else:super().observe(u,address,size,data)
 def call(self,entry,this,args):
  if entry==0x75C240:
   self.uc.mem_write(head_native.OWNER+0x5A4,dwords(head_native.CELL if self.row['nav'] else 0))
   self.uc.mem_write(head_native.OWNER+0x2B4,dwords(head_native.BUILDING if self.row['target'] else 0))
   self.before_rng=bytes(self.uc.mem_read(head_native.SCENARIO+0x218,0x3F4)).hex()
  super().call(entry,this,args)

def head_query(row):
 n=ChaseHead(row)
 output=n.producer({'input':[2752,2624,260],'ground':0,'deck':0,'owner':41,'current':[2496,2624,260],'seed':31})
 return dict(input=row,output=output,rng_before=n.before_rng,rng_after=bytes(n.uc.mem_read(head_native.SCENARIO+0x218,0x3F4)).hex())

def generate():
 return dict(setter=[query(row) for row in [{},{'mission':5},{'head':True},{'doing':27},{'doing':27,'human':False},{'nav_queue':1},{'retry_delay':7.75}]],
             head_producer=[head_query(row) for row in [{'mission':0,'nav':False,'target':False},{'mission':1,'nav':True,'target':True},{'mission':1,'nav':False,'target':True}]])

if __name__=='__main__':
 finish_vectors(generate,Path(__file__).with_suffix('.json'),provenance=lambda:provenance(
  scope='Original Infantry51AA40 nonnull Cell setter, Foot4D94B0, Walk75ACB0, then original WalkProcess through first FindPath4D3920 entry or already-paid-head motion-byte producer. Separate actual75C240 head rows contrast Mission1/TarCom/CellNavCom with the existing no-target producer. No pathfinder-core/whole Process parity.',
  entry_points={'infantry_set_destination':0x51AA40,'foot_set_destination':0x4D94B0,'walk_constructor':0x75AA90,'walk_move_to':0x75ACB0,'walk_process':0x75AEC0,'first_find_path':0x4D3920,'head_producer':0x75C240,'head_choice':0x481180,'raw_mark':0x5217C0,'raw_clear':0x521850},
  assumptions=['Supplied original Infantry/Unit/Cell tables, valid ordinary non-Jumpjet owner/type/House, no links/radio/bunker/transport, currentXYZ2624,2624,0, destination Cell11,10. Human Doing27 refusal contrast and nonhuman acceptance execute original gate.', 'Existing path backing2,3,4,5 and null ownerNavCom are supplied; actual setter clears one path head and publishes Cell plus Walk destination. Optional paid head is supplied and remains independently owned.', 'Actual Walk constructor executes; reference count1 models the already-owned ILoco interface. Original setter obtains/releases a temporary interface reference. OS InterlockedIncrement/Decrement imports emulate their one-integer operations, no gameplay callable substituted.', 'First no-head Process is stopped at original FindPath entry after observing arguments and unchanged path suffix/reference; it does not execute the AStar owner, path writes, Mark or movement. Paid-head row stops after actual+36 producer.', 'Frame100, initial logical Foot timer start+640=50/duration+648=5 (+644 supplied padding0), Rules+1768=22 and Rules+1760 double0.0 (retry_delay7.75 contrast) are supplied. Actual accepted Foot setter resets this timer before first Process; the later Process writes its retry delay before FindPath. Scope is the first request after an accepted setter, not arbitrary Process with a live retry timer. Flat Cell levels, startup control0E7F and known104 level constant are supplied.', 'All three supplied free-head rows preserve the full RandomClass byte-for-byte (zero draws); this is not a general head-choice RNG equivalence claim. Separate head rows use the imported walk_head_occupation fixture: supplied current2496,2624,260 and requested2752,2624,260, level2/slope1, subcell-offset table, no gate/crate/slave. Original75C240/481180/5217C0/521850 and seededScenarioRandom execute; full0x3F4 RandomClass state retained before/after. These are separate kernel rows, not execution through the FindPath core.'],
  substitutions=['Setter rows only: OS InterlockedIncrement/Decrement imports update the pointed count and return it with original stdcall cleanup. No SetDestination, MoveTo, GetCoords, House, Mission or Process callable replaced.', 'Separate head rows inherit explicit owner-index/no-building/gate/false-owner virtual seams from walk_head_occupation; Mission+184 returns supplied0 or1, TarCom and CellNavCom pointers are supplied. The actual75C240 caller decides whether to consult them.']))
