# Adversarial Verdicts — neighbor-classifiers facet

Auditor stance: refute each finder claim until the live binary proves it. Finder reported
NO DRIFT (P1-P14 PARITY-CONFIRMED + U1 UNCHECKED). I re-decompiled all 8 cited functions
live and re-read the current Rust. Verdicts below treat each P/U as a disparity claim
(here, a "no-disparity" claim that must survive scrutiny).

Live re-verification this session:
- `decompile_function 0x0057CBE0` → `MapClass__CheckBridgeNeighbors_NS_High` (identity reconfirmed via `get_function_by_address 0x0057CBE0`).
- `decompile_function 0x0057CAB0` → `..._EW_High` (id reconfirmed `0x0057CAB0`).
- `decompile_function 0x0057B990` → `..._NS_Low` (id reconfirmed `0x0057B990`).
- `decompile_function 0x0057B870` → `..._EW_Low` (id reconfirmed `0x0057B870`).
- `decompile_function 0x0057E7A0 / 0x0057ED00 / 0x0057DD50 / 0x0057E2A0` → 4 ApplyBridgeDestruction tables read byte-for-byte.
- `read_memory 0x00abdc50 len=80` → all zero (sentinel +0x44 = 0).
- `decompile_function 0x0057cf60` → `DestroyBridgeWalker_NS_High` (cross-check of cascade geometry / case values).

---

P1: REFUTED (no drift) — NS_High classifier. Live `0x0057CBE0`: puVar2=(X,Y-1)=NORTH first
switch `{0xDA,0xDC,0xDE,0xE4}→1`, `{0xDD,0xE8}→2`; puVar3=(X,Y+1)=SOUTH second switch
`{0xDB,0xDC,0xDD,0xE6}→return|4`, `{0xDE,0xE8}→|8`. Rust walker.rs:678-687 byte-identical.

P2: REFUTED (no drift) — EW_High classifier. Live `0x0057CAB0`: puVar2=(X-1,Y)=WEST,
puVar3=(X+1,Y)=EAST; first switch is on puVar3 (EAST) `{0xD1,0xD3,0xD5,0xE0}→1`,
`{0xD4,0xE7}→2`; second on puVar2 (WEST) `{0xD2,0xD3,0xD4,0xE2}→return|4`, `{0xD5,0xE7}→|8`.
Rust walker.rs:647-656 reads east first then west, identical bit map. Finder's east-first
detail confirmed.

P3: REFUTED (no drift) — NS_Low. Live `0x0057B990`: NORTH `{0x57,0x59,0x5B,0x61}→1`,
`{0x5A,0x65}→2`; SOUTH `{0x58,0x59,0x5A,99=0x63}→return|4`, `{0x5B,0x65}→|8`. Rust
walker.rs:1065-1074 identical (0x63 = decimal 99 confirmed).

P4: REFUTED (no drift) — EW_Low. Live `0x0057B870`: EAST(puVar3) `{0x4E,0x50,0x52,0x5D}→1`,
`{0x51,100=0x64}→2`; WEST(puVar2) `{0x4F,0x50,0x51,0x5F}→return|4`, `{0x52,100}→|8`. Rust
walker.rs:1034-1043 identical (0x64 = decimal 100 confirmed).

P5: REFUTED (no drift) — HIGH NS table. Live `0x0057E7A0` `local_70`:
`[-1,0xD2,0xD5,FF,0xD1,0xD3,0xD5,FF,0xD4,0xD4,0xE7,FF,FF,FF,FF,FF]`. Rust bridge_specs.rs:420
`DESTRUCTION_OVERLAY_HIGH_NS` identical (−1 ⇒ 0xFF ⇒ None).

P6: REFUTED (no drift) — HIGH EW table. Live `0x0057ED00`:
`[-1,0xDB,0xDE,FF,0xDA,0xDC,0xDE,FF,0xDD,0xDD,0xE8,FF×5]`. Rust bridge_specs.rs:426 identical.

P7: REFUTED (no drift) — LOW NS table. Live `0x0057DD50`:
`[-1,0x4F,0x52,FF,0x4E,0x50,0x52,FF,0x51,0x51,100=0x64,FF×5]`. Rust bridge_specs.rs:436
identical.

P8: REFUTED (no drift) — LOW EW table. Live `0x0057E2A0`:
`[-1,0x58,0x5B,FF,0x57,0x59,0x5B,FF,0x5A,0x5A,0x65,FF×5]`. Rust bridge_specs.rs:444 identical.

P9: REFUTED (no drift) — dispatch + idx-0 guard. Live `0x0057E7A0`: `if (0 < iVar2)` skips
idx 0; reads up to idx 15. Rust pick_destruction_overlay guards `>=16 → None`; idx0 ⇒
table[0]=0xFF ⇒ None. Callers gate `if idx==0 return` (walker.rs:730,786,1098,1150).
Output-identical.

P10: REFUTED (no drift) — no-op-on-equal. Live: `iVar2=local_70[iVar]; if (iVar8==iVar2)
return;`. Rust `Some(n) if n != cur => n, _ => return` (walker.rs:738,791,1103,1155). Identical.

P11: REFUTED (no drift) — second-switch early-return vs |8. Live confirms each second
switch `return uVar4|4` on its bit-4 case (suppresses the bit-8 arm of that switch). Verified
bit-4 ∩ bit-8 sets are disjoint in all 4 functions (re-derived from the live case lists, not
the finder's restatement): NS_H SOUTH {DB,DC,DD,E6}∩{DE,E8}=∅; EW_H WEST {D2,D3,D4,E2}∩{D5,E7}=∅;
NS_L SOUTH {58,59,5A,63}∩{5B,65}=∅; EW_L WEST {4F,50,51,5F}∩{52,64}=∅. A single C switch byte
takes one case only; the early-return cannot suppress a bit-8 that would otherwise be set.
Rust independent `match` arms produce identical result. Proven over full 256-byte input space.

P12: REFUTED (no drift) — off-map sentinel. `read_memory 0x00abdc50 len=80` = all zero ⇒
sentinel overlay (+0x44) = 0. Byte 0 is not in any classifier bit-set, contributes no bit,
identical to Rust treating off-map/missing as 0. Holds for all off-map cases EXCEPT the
in-bounds wrap edges noted in U1/MISS below (those substitute a real cell, NOT the sentinel).

P13: REFUTED (no drift) — triple write geometry. Live NS writes overlay to local_c4=(X,Y),
local_b8=(X,Y-1)=north, local_cc=(X,Y+1)=south; EW writes this=(X,Y), local_bc=(X-1,Y)=west,
local_c8=(X+1,Y)=east. Cross-axis pairing confirmed: ApplyBridgeDestruction_NS_* calls
CheckBridgeNeighbors_EW_*; _EW_* calls _NS_* (perpendicular). Rust ns_triple/ew_triple
(walker.rs:693-703) + cross-axis classifier calls (1097,1149) identical.

P14: REFUTED (no drift) — progressive intermediate gates. Live: NS_H `<0xDF→table`,
`0xDF→0xE0`, `0xE1→0xE2`, else return; EW_H `<0xE3→table`, `0xE3→0xE4`, `0xE5→0xE6`; NS_L
`<0x5C→table`, `0x5C→0x5D`, `0x5E→0x5F`; EW_L `<0x60→table`, `0x60→0x61`, `0x62→99=0x63`.
Rust walker.rs:736-748 / 790-801 / 1102-1113 / 1154-1165 identical.

U1: UNCERTAIN — column-0 EW west-probe row-wrap. The finder's BINARY reading is CORRECT and
reconfirmed live (`0x0057CAB0` / `0x0057B870`): west index `param_1[1]*0x200 +
(int)(short)(*param_1 - 1)`. At X=0 with Y>=1 this is `Y*512 - 1` — a valid in-bounds linear
index pointing to cell `(X=511, Y-1)` (last column of previous row), whose overlay byte is
read. Rust treats `rx==0` west as 0 (walker.rs:641-645, 1028-1032). So this IS a genuine
output difference at X=0 — NOT proven equivalent. The NS-axis analogue at Y=0 does NOT wrap
(`(short)(-1)*0x200 = -512` ⇒ negative ⇒ sentinel(0), matching Rust). I keep this UNCERTAIN
rather than REAL only because reachability (can a DestroyableBridge body/±1 sibling sit at
X=0?) was NOT independently verified against the map format this session; the finder asserts
"unreachable" from fixture inspection but did not prove it from the map authoring constraints.
Burden of proof: if a bridge body or perpendicular cascade cell can reach X=0, this is
PROVEN-DRIFT (binary reads `(511, Y-1)` overlay; Rust reads 0). Corrected delta if REAL:
Rust west-at-X=0 returns 0 -> gamemd reads overlay byte of linear cell `Y*512-1` = `(511,Y-1)`.

---

MISS: Symmetric EW EAST-edge wrap at X=511 (max column) — finder noted only the X=0 west
wrap. Live `0x0057CAB0`/`0x0057B870` east index `param_1[1]*0x200 +
(int)(short)(*param_1 + 1)`: at X=511, `(short)(512)=512`, index `= Y*512 + 512 = (Y+1)*512`
⇒ cell `(X=0, Y+1)` (first column of NEXT row). Binary reads that real cell's overlay. Rust
`cell(rx.saturating_add(1)=512, ry)` returns None ⇒ 0 (walker.rs:638-640, 1024-1027). Same
class of in-bounds wrap-vs-zero divergence as U1, on the opposite edge. Also UNCERTAIN on
the same reachability question (bridge body/sibling at X=511).

MISS: Symmetric NS SOUTH-edge behavior — for completeness, NS south probe `(short)(Y+1)*0x200
+ X`; at the max row Y=511, `(short)(512)*0x200 = 512*512 = 0x40000 = 262144 > 0x3FFFF` ⇒
fails the `0x3ffff < iVar1` guard ⇒ sentinel(0), matching Rust. NS north at Y=0 ⇒ negative ⇒
sentinel(0), matching Rust. So the NS axis has NO wrap divergence on either edge; only the
EW axis wraps (both X=0 and X=511). This asymmetry (EW wraps, NS does not) is a property of
the 0x200 row stride and is correctly absent from Rust for NS but divergent for EW edges.

MISS (cross-check, not in facet but confirms no contamination): `DestroyBridgeWalker_NS_High
@ 0x0057cf60` cascade siblings are `(X-1,Y)` then `(X+1,Y)` (perpendicular EW) with case
values 0xDF→0xE0(west cascade), 0xE1→0xE2(east cascade), `<0xD3`→0xD3(both), `0xD3..=0xD5`→
0xE7 final(both), `>0xD5` no-op — matching Rust destroy_bridge_walker_ns_high
(walker.rs:852-870). No drift found in the adjacent walker.
