# RMG water-stage tail — `0x005A0160` ring dilation + the river-bridge builder `0x0059E740`

**Date:** 2026-07-25
**Binary:** `gamemd.exe`, image base `00400000` (`get_current_program_info`)
**Scope:** the last undecoded draw sites in the map-type-3/4 water stage — the *n*-ring
dilation both `CarveRiver` and the bridge builder run after `GrowMeanderArm`, and the
bridge-placement path itself.
**Status:** contracts below are VERIFIED from assembly this session unless they appear in
§9 (Unverified). Every address/offset/constant carries its proving MCP call inline.

**Relationship to existing docs.** `RMG_MODE34_WATER_BRIDGES_TECH_GHIDRA_REPORT.md` already
covers the surrounding carve/lake flow and was itself corrected earlier today; this document
does **not** edit it. Where the two overlap, findings agree — see §8 for the explicit
cross-check. What is new here: the transitive zero-draw *proof* for `0x005A0160`, the exact
uniform-draw constants and their x87 forms, the quantified `0x0057A0C0` shore-pass draw cost,
the endpoint (near/far) resolution generalised to all four headings, the runtime-initialised
`g_DirectionOffsets` values, and the `+0x11B` / `+0x11C` write census.

**Ghidra labels applied this session** (renames + plate comments; `save_program` deliberately
NOT called — the coordinator owns saving):

| Address | Was | Now |
|---|---|---|
| `0x005A0160` | `FUN_005a0160` | `RandomMapGenerator__DilateRegionRings` |
| `0x0059E740` | `FUN_0059e740` | `RandomMapGenerator__BuildRiverBridge` |
| `0x00598030` | `FUN_00598030` | `RandomMapGenerator__NextUniformRange` |
| `0x005A0090` | `FUN_005a0090` | `RmgGrid__SetRegionTag` |
| `0x005A00C0` | `FUN_005a00c0` | `RmgGrid__GetRegionTag` |

Plate comments added at `0x005A0160`, `0x0059E740`, `0x00598030`, `0x0049F300`,
`0x004865D0`, `0x00486380`.

---

## 1. `RandomMapGenerator__DilateRegionRings` @ `0x005A0160` — full contract

### 1.1 Signature and calling convention

`__stdcall` — **not** `__thiscall`. `RET 0x20` at `0x005A03DB` / `0x005A0403` = 32 bytes of
stack args = 8 dwords, and `ECX` is never read. *(verified via
`disassemble_function 0x005A0160`)*

```
u8 RandomMapGenerator__DilateRegionRings(
        int  regionTag,   // arg1  [ESP+0x30] after the 4 callee-saved pushes
        int  ringCount,   // arg2  [ESP+0x34]
        int  clampX,      // arg3  [ESP+0x38]
        int  clampY,      // arg4  [ESP+0x3C]
        int  clampW,      // arg5  [ESP+0x40]
        int  clampH,      // arg6  [ESP+0x44]
        char stampLevel,  // arg7  [ESP+0x48]   (byte)
        u8   levelByte)   // arg8  [ESP+0x4C]   (byte)
```

Slot mapping derived from the prologue: `SUB ESP,0x1C` at `0x005A0160`, then
`PUSH EBX/EBP/ESI/EDI` (`0x005A0167`–`0x005A016A`), so arg *n* sits at `[ESP + 0x2C + 4n]`.
The rect test at `0x005A0257`–`0x005A0285` reads `[ESP+0x38]`/`[ESP+0x40]` for x and
`[ESP+0x3C]`/`[ESP+0x44]` for y, confirming the `(x0, y0, w, h)` reading:
`clampX <= n.x < clampX+clampW` and `clampY <= n.y < clampY+clampH`.

Arg order independently confirmed at a call site: `CarveRiver` `0x0059E3CF`–`0x0059E3F2`
pushes arg8, arg7, writes args 3–6 into a reserved `SUB ESP,0x10` block, then pushes arg2
(`PUSH 6`) and arg1 (`PUSH [EBX+0x308]`). *(verified via
`disassemble_bytes 0x0059E3B0-0x0059E47F`)*

Return: `MOV AL,1` at `0x005A03D5` (all rings completed) or `XOR AL,AL` at `0x005A03FD`
(hard abort — see §1.3).

### 1.2 `param_7` / `param_8` — the two previously-undecoded parameters

* **`param_7` = `stampLevel` (char).** It does two independent things:
  1. It enables the *absorb* predicate at `0x005A02A6`–`0x005A02B8`:
     `isPrev = (stampLevel != 0) && (gridTag == regionTag - 1)`. `stampLevel == 0` forces
     `isPrev = false` (`XOR BL,BL` at `0x005A02BA`), which disables both the
     "claim cells of the previous region" path and the `HasBridgeOverlay` acceptance leg.
  2. It gates the level stamp at `0x005A033E`–`0x005A036B`.
* **`param_8` = `levelByte` (u8).** Written **raw**:
  `0x005A0367 MOV CL, byte ptr [ESP+0x4C]` / `0x005A036B MOV byte ptr [EAX+0x11B], CL`.
  This is a *plain byte store*, not `+= 4`, and there is no quantisation applied inside this
  function — the caller is responsible for passing a multiple of 4.

Only **one** call site in the whole binary passes `stampLevel != 0`: `CarveRiver`
`0x0059E4F5`, with `levelByte = MapSeed->+0x30C`. The other three sites
(`CarveRiver` ×2, `GrowLake`, `BuildRiverBridge`) pass `(0, 0)`.
*(verified via `get_function_callers 0x005A0160` → `{0x0059E740, 0x0059D510, 0x0059C920}`;
`decompile_function 0x0059D510`)*

### 1.3 Control flow

```
frontier = RandomMapGenerator__CollectRegionBorderCells(regionTag)      // 0x005A016C
for ring in 0 .. ringCount-1:                                          // 0x005A0175 / 0x005A03B8
    new = operator_new(0x18)                                           // 0x005A0189
    DynamicVectorClass ctor 0x0042FCB0(0,0); vtbl = 0x007E3890;         // 0x005A019B
        new.growthStep = 10 (0x005A01A6), new.count = 0
    new.growthStep = frontier.count * 3                                 // 0x005A01C2 (overwrite)
    for cell in frontier (ascending index):                             // 0x005A01D2
        for dir in 0..7:                                                // 0x005A0376
            n = cell + g_DirectionOffsets[dir]                          // 0x005A01FE
            if !(inside isometric diamond [0x00ABED04]/[0x00ABED08]):  continue   // 0x005A022B..0x005A0251
            if !(clampX <= n.x < clampX+clampW &&
                 clampY <= n.y < clampY+clampH):                 continue        // 0x005A0257..0x005A0285
            rec  = [0x00ABED10] + (n.y * g_PathfinderLinearMapWidth + n.x) * 0x50 // 0x005A028B..0x005A02A4
            tag  = rec[+0x38]                                           // 0x005A02A8
            isPrev = stampLevel != 0 && tag == regionTag - 1            // 0x005A02AB
            cc   = MapClass__Get_CellClass(&n)     // UNCONDITIONAL     // 0x005A02C6
            if (tag == 0 || isPrev) &&
               (CellClass__IsClearTile(cc) ||                           // 0x005A02D7
                (isPrev && 0x004865D0(cc))):                            // 0x005A02E6
                 append n to new                                        // 0x005A02FB..0x005A033B
                 rec[+0x38] = regionTag                                 // 0x005A0348
                 if stampLevel != 0 && CellClass__IsClearTile(cc):      // 0x005A0346 / 0x005A034F
                     MapClass__Get_CellClass(&n)[+0x11B] = levelByte    // 0x005A0362 / 0x005A036B
            else if tag != regionTag:
                 free(frontier); free(new); return 0                    // 0x005A02EF / 0x005A03DE
            // else (tag == regionTag): already ours, skip
    free(frontier); frontier = new                                      // 0x005A039B / 0x005A03AF
free(frontier); return 1                                                // 0x005A03C4 / 0x005A03D5
```

**Ring-count semantics.** `ringCount` is a *pass count*, not a radius in cells directly:
each pass expands the region by exactly one 8-connected ring, and the frontier for pass
*k+1* is the set of cells *newly claimed* in pass *k* (not the recomputed border). Observed
values: `1`, `6`, `2` from `CarveRiver`; `2` from `BuildRiverBridge`.

**The abort is order-sensitive and leaves partial writes.** When a neighbour carries a
foreign tag, the function returns 0 *immediately* — every `rec[+0x38] = regionTag` and
`cc[+0x11B] = levelByte` already performed in this and previous rings stays written. The
callers rely on their own rollback sweeps to undo them. A port must therefore replicate the
8-direction iteration order exactly (§1.5) and must not hoist the failure test.

**Vector growth failure is silent.** At `0x005A0305`–`0x005A032B`: if `count >= capacity`
and the grow (vtable slot 2 = `0x0042F860`) returns 0, the append is skipped but
`rec[+0x38] = regionTag` still executes. The cell is claimed but never becomes a frontier
cell for the next ring.

### 1.4 RNG draw ledger — **ZERO draws**, proven by CALL enumeration

Every `CALL` instruction in the function body (`disassemble_function 0x005A0160`):

| Address | Target | Identity |
|---|---|---|
| `0x005A016C` | `0x005A0700` | `RandomMapGenerator__CollectRegionBorderCells` |
| `0x005A0189` | `0x007C8E17` | `operator_new` |
| `0x005A019B` | `0x0042FCB0` | `DynamicVectorClass` ctor |
| `0x005A02C6`, `0x005A0362` | `0x005657A0` | `MapClass__Get_CellClass` |
| `0x005A02D7`, `0x005A034F` | `0x00486380` | `CellClass__IsClearTile` |
| `0x005A02E6` | `0x004865D0` | water/shore/bridge-tile predicate (§6) |
| `0x005A0324` | `[EDX+8]` → `0x0042F860` | vector grow, vtable slot 2 |
| `0x005A03A5`, `0x005A03D0`, `0x005A03EA`, `0x005A03F8` | `[EDX]` → `0x0042FBF0` | scalar deleting dtor, vtable slot 0 |

The two indirect targets are resolved by reading the vtable:
`read_memory 0x007E3890` len 24 → `{0x0042FBF0, 0x0042F620, 0x0042F860, 0x0042F420,
0x0042F930, 0x0042F450}`, so slot 0 = `0x0042FBF0` and slot 2 = `0x0042F860`.

Transitive closure of that set:
`0x005A0700` → `{0x0042F860, 0x0042FCB0, 0x007C8E17}` *(`get_function_callees 0x005A0700`)*;
`0x0042F860` → `{0x007C8B3D, 0x007C8E17}`; `0x0042FBF0` → `{0x007C8B3D}`;
`0x0042FCB0` → `{0x007C8E17}`; `0x007C8B3D` → `{0x007C93E8}`;
`0x005657A0`, `0x00486380`, `0x004865D0` are leaves with no `CALL` at all
*(`disassemble_function` on each)*.

**None of those addresses appears in the complete direct-caller set of
`Random__Next 0x0065C780`** *(`get_function_callers 0x0065C780`, 98 entries)*. That caller
set contains only game-class functions; no CRT/heap routine is in it. Therefore
`RandomMapGenerator__DilateRegionRings` and its entire subtree consume **0** RNG draws.

> This is a *reachability* proof over CALLs, not an operand scan. Per the standing tooling
> warning, `search_instructions` cannot match absolute memory operands and must never be
> used to argue absence of a reference.

### 1.5 `g_DirectionOffsets` @ `0x0089F688` — values (runtime-initialised)

`0x005A01FE LEA EDX,[EDX*0x4 + 0x89F688]`, then `word[+0]` = dx and `word[+2]` = dy — an
8-entry array of `{short dx; short dy}`, stride 4. (The decompiler renders the dx read as
`&g_DirectionOffsets + (i & 7)`; the assembly shows the `*4` scaling, so the decompiler form
is a pointer-arithmetic artefact.)

**`read_memory 0x0089F660` len 96 returns all zeros** — the table is *not* statically
initialised; a raw read of the image is not its value. Its only WRITE xref is `0x0049F394`
*(`get_xrefs_to 0x0089F6A4`)*, inside the routine Ghidra places at `0x0049F300`. That
function start is **wrong**: the real entry is `0x0049F2F0`
*(`read_memory 0x0049F2D0` len 54 → `51 33 D2 83 C9 FF 66 89 54 24 00 66 89 4C 24 02 …`)*:

```
0049F2F0  PUSH ECX                 ; reserve the packed 4-byte local
0049F2F1  XOR  EDX,EDX             ; DX = 0
0049F2F3  OR   ECX,0xFFFFFFFF      ; CX = -1
0049F2F6  MOV  word ptr [ESP],DX
0049F2FB  MOV  word ptr [ESP+2],CX
0049F300  MOV  EAX,[ESP]           ; (0, -1)
0049F30A  MOV  EAX,1               ; AX = 1 for the rest of the body
```

Substituting `DX = 0`, `CX = -1`, `AX = 1` into the stores at `0x0049F305`, `0x0049F322`,
`0x0049F336`, `0x0049F34A`, `0x0049F388`, `0x0049F375`, `0x0049F38E`, `0x0049F394`
*(`disassemble_function 0x0049F300`)*:

| idx | address | (dx, dy) | facing |
|---|---|---|---|
| 0 | `0x0089F688` | ( 0, −1) | N |
| 1 | `0x0089F68C` | ( 1, −1) | NE |
| 2 | `0x0089F690` | ( 1,  0) | E |
| 3 | `0x0089F694` | ( 1,  1) | SE |
| 4 | `0x0089F698` | ( 0,  1) | S |
| 5 | `0x0089F69C` | (−1,  1) | SW |
| 6 | `0x0089F6A0` | (−1,  0) | W |
| 7 | `0x0089F6A4` | (−1, −1) | NW |

(+X = east, +Y = south — the project's canonical cell frame.) The dilation therefore visits
neighbours in **N, NE, E, SE, S, SW, W, NW** order, which is the order a port must use for
the abort-path partial state to match.

**Label drift recorded:** the symbol at `0x0089F6A0` is named
`g_refinery_unload_adjacent_lookup_dx`. It is simply entry [6] (W) of this table.

### 1.6 Field writes

| Target | Offset | Form | Site |
|---|---|---|---|
| RMG scratch grid `[0x00ABED10]` record | `+0x38` | `= regionTag` | `0x005A0348` |
| `CellClass` | `+0x11B` | `= levelByte` (raw byte, no arithmetic) | `0x005A036B` |

Nothing else. It never touches `CellClass +0x38`, `+0x11A`, or **`+0x11C`**.

---

## 2. The bridge placement path — `RandomMapGenerator__BuildRiverBridge` @ `0x0059E740`

### 2.1 Identification by address, not by name

`get_function_callees 0x0059D510` (CarveRiver) lists `FUN_0059e740 @ 0059e740` and
`FUN_005a0160 @ 005a0160` among its callees. The single call to `0x0059E740` sits inside the
bridge gate at `0x0059DDF5` *(`disassemble_bytes 0x0059DDA3-0x0059DE1F`)*, immediately after
the heading-code selection at `0x0059DD1A`–`0x0059DD5E` and the `bridgeCount < 1` /
`MapSeed+0x310 != 0` / `stepCount > bridgeMinStep` gate at `0x0059DCA1`–`0x0059DCF6`
*(`disassemble_bytes 0x0059DB80-0x0059DCFB`, `0x0059DCFC-0x0059DDA7`)*.
`get_function_callers 0x005A0160` returns exactly `{0x0059E740, 0x0059D510, 0x0059C920}`,
so `0x0059E740` is the only other consumer of the dilation. **`0x0059E740` is the bridge
builder** — it is also the function the task described separately as item 3; they are the
same function.

### 2.2 Signature, verified from the call site's push sequence

`RET 0x1C` at `0x005A004A` = 28 bytes = 7 stack dwords, plus `ECX`. *(`disassemble_bytes
0x0059FFC0-0x005A004C`)*

```
char __thiscall RandomMapGenerator__BuildRiverBridge(
        MapSeedClass* this,     // ECX          (0x0059DDF3  MOV ECX,EBX)
        int      regionTag,     // arg1  = this->+0x308
        CellStruct* pCsEnd,     // arg2  = &[ESP+0xA0]
        CellStruct* pCsStart,   // arg3  = &[ESP+0xE0]
        int      heading,       // arg4  = ESI  in {0,2,4,6}
        char*    pOutPlaced,    // arg5  = &[ESP+0x10]
        double*  pRiverX,       // arg6  = &[ESP+0xB0]
        double*  pRiverY)       // arg7  = &[ESP+0xC0]
```

Push order at `0x0059DDD4`–`0x0059DDF2` (right-to-left): `EAX=&[ESP+0xC0]`,
`ECX=&[ESP+0xB0]`, `EDX=&[ESP+0x10]`, `ESI`, `EAX=&[ESP+0xE0]`, `ECX=&[ESP+0xA0]`,
`EDX=this->+0x308`. Stack offsets are pre-push values (each `LEA` was re-based for the
pushes already issued).

### 2.3 Which endpoint is near and which is far — resolved

`CarveRiver`'s frame base maps `local_X` ↔ `[ESP + 0x138 − X]`; four independent checks
confirm it (`local_88`↔`[ESP+0xB0]` = river x, `local_78`↔`[ESP+0xC0]` = river y,
`local_b8`↔`[ESP+0x80]` = sin h, `local_f8`↔`[ESP+0x40]` = cos h — the last two are
confirmed by the heading-selection compares at `0x0059DD22` and `0x0059DD44`).

The four `Math__ftol` calls that fill the two `CellStruct`s
*(`disassemble_bytes 0x0059DCFC-0x0059DDA7`, `0x0059DDA3-0x0059DE1F`)*:

| # | expression | stored to |
|---|---|---|
| 1 | `ftol([ESP+0x68] − [ESP+0x40])` = `ftol(walkX_final − cos h)` | `word [ESP+0xE0]` |
| 2 | `ftol([ESP+0x30] − [ESP+0x80])` = `ftol(walkY_final − sin h)` | `word [ESP+0xE2]` |
| 3 | `ftol([ESP+0xB0] − [ESP+0x100])` = `ftol(x − (w−1)·cos h/2)` | `word [ESP+0xA0]` |
| 4 | `ftol([ESP+0xC0] − [ESP+0x110])` = `ftol(y − (w−1)·sin h/2)` | `word [ESP+0xA2]` |

The cross-section walk starts at `(x − (w−1)cos/2, y − (w−1)sin/2)` and advances `(+cos, +sin)`
once per substep (`0x0059DC1B`–`0x0059DC6F`), so after `w` substeps
`walk_final = start + w·(cos, sin)` and `walk_final − (cos, sin)` is the **last** carved cell.

Therefore:

* **`arg3` (`pCsStart`, `&[ESP+0xE0]`) = the LAST cell of the width walk.**
* **`arg2` (`pCsEnd`, `&[ESP+0xA0]`) = the FIRST cell of the width walk.**

(The parameter *order* is reversed relative to the walk because of the push order; the two
labels above describe the geometry, which is what a port needs.)

**Neither pointer is universally "near".** The heading fixes which one the bridge anchors on,
and it is always the endpoint with the **smaller coordinate along the channel axis**, so the
span delta is always non-negative:

| heading | channel axis | anchor argument | span |
|---|---|---|---|
| 0 (N) | x | `arg2` (walk start) | `arg3.x − arg2.x` |
| 2 (E) | y | `arg2` (walk start) | `arg3.y − arg2.y` |
| 4 (S) | x | `arg3` (walk end) | `arg2.x − arg3.x` |
| 6 (W) | y | `arg3` (walk end) | `arg2.y − arg3.y` |

Proof: for heading 0, `cos h > 0` so the walk advances toward +x and the start has the
smaller x; for heading 4, `cos h <= 0` so the walk advances toward −x and the *end* has the
smaller x. Same argument on y with `sin h` for headings 2/6. The heading itself is selected
from the sign of `sin h` / `cos h` at `0x0059DD22`–`0x0059DD59`, and the case setup blocks
(§2.4) read exactly the argument this table names.

### 2.4 The rect/heading table — **re-verified against assembly**

Dispatch is a jump table at `0x005A0050`, indexed by `heading`
*(`0x0059E7A1 CMP EAX,6 / JA default`; `0x0059E7A7 JMP dword ptr [EAX*4 + 0x5A0050]`;
`read_memory 0x005A0050` len 28)*:

| index | target | case |
|---|---|---|
| 0 | `0x0059E7AE` | 0 = N |
| 1 | `0x0059E9FA` | default (no-op) |
| 2 | `0x0059E8CE` | 2 = E |
| 3 | `0x0059E9FA` | default |
| 4 | `0x0059E842` | 4 = S |
| 5 | `0x0059E9FA` | default |
| 6 | `0x0059E962` | 6 = W |

**The decompiler's case labelling is correct** — but note the block order in memory is
0, 4, 2, 6, which is why an unverified transcription is easy to get wrong. Stack-slot ↔
decompiler-local mapping for this function is `local_X ↔ [ESP + 0x128 − X]` after the four
callee-saved pushes; it is confirmed by `local_c8[1] ↔ [ESP+0x64]`, `local_d8 ↔ [ESP+0x50]`,
`local_a0 ↔ [ESP+0x88]`, `local_ec ↔ [ESP+0x3C]`, `local_118 ↔ [ESP+0x10]`,
`local_110 ↔ [ESP+0x18]` (the last two from the merge point `0x0059E9FA`–`0x0059EA04`).

With `A` = anchor cell (per §2.3) and `S` = span:

| | heading 0 (N) | heading 2 (E) | heading 4 (S) | heading 6 (W) |
|---|---|---|---|---|
| anchor `A` | `arg2` | `arg2` | `arg3` | `arg3` |
| clearance rect (x0,y0,w,h) | `(A.x−2, A.y−12, S+5, 12)` | `(A.x+1, A.y−2, 12, S+5)` | `(A.x−2, A.y+1, S+5, 12)` | `(A.x−12, A.y−2, 12, S+5)` |
| water fill #1 (x0,y0,w,h) | `(A.x, A.y−4, S+1, 4)` | `(A.x+1, A.y, 4, S+1)` | `(A.x, A.y+1, S+1, 4)` | `(A.x−4, A.y, 4, S+1)` |
| water fill #2 (x0,y0,w,h) | `(A.x, A.y−12, S+1, 8)` | `(A.x+5, A.y, 8, S+1)` | `(A.x, A.y+5, S+1, 8)` | `(A.x−12, A.y, 8, S+1)` |
| clamp rect (x0,y0,w,h) | `(0, A.y−4, 0x200, 0x200−(A.y−4))` | `(0, 0, A.x+4, 0x200)` | `(0, 0, 0x200, A.y+4)` | `(A.x−4, 0, 0x200−(A.x−4), 0x200)` |
| meander/dilation seed cell | `(A.x, A.y−4)` | `(A.x+5, A.y)` | `(A.x, A.y+1)` | `(A.x−4, A.y)` |

Assembly citations for the clamp-rect row (the load-bearing one):
`0x0059E7FE`/`0x0059E807`/`0x0059E835`/`0x0059E839` (N);
`0x0059E921`/`0x0059E92F`/`0x0059E933`/`0x0059E94A`/`0x0059E951`/`0x0059E955` (E);
`0x0059E89B`/`0x0059E8AB`/`0x0059E8B0`/`0x0059E8B5`–`0x0059E8C1` (S);
`0x0059E9C0`/`0x0059E9BE`/`0x0059E9CF`/`0x0059E9EC`/`0x0059E9F0` (W).
*(`disassemble_bytes 0x0059E740-0x0059E8EF` and `0x0059E8EF-0x0059EA1F`)*

### 2.5 "Half-plane behind the bridge" — the reading is **CORRECT**

Substituting the clamp-rect row above and remembering the heading is the river's *travel*
direction:

| heading | clamp rect reduces to | relative to the bridge |
|---|---|---|
| 0 (N) | `y >= A.y − 4` | everything **south** = upstream/behind |
| 2 (E) | `x <  A.x + 4` | everything **west** = behind |
| 4 (S) | `y <  A.y + 4` | everything **north** = behind |
| 6 (W) | `x >= A.x − 4` | everything **east** = behind |

So yes — the rect passed to both `GrowMeanderArm` and `DilateRegionRings` is the half-plane
on the upstream side of the bridge landing, in all four headings, verified in assembly. The
`0x200` (512) constants are the fixed `CellClass` array dimension, not a map-size read.

Do **not** confuse that clamp rect with the *clearance* rect, which is a bounded
12-deep × (span+5)-wide box **ahead** of the cross-section (§2.4 row 2). An earlier
description that put the "half-plane" on the clearance scan would be wrong; the half-plane
belongs to the meander/dilation arena.

### 2.6 Complete draw ledger, in execution order

All draws go to `g_MapGenRng @ 0x00ABE890` (`MOV ECX,0xABE890` before each
`CALL 0x0065C780`).

| # | stage | draws |
|---|---|---|
| A | `*pOutPlaced = 0` (`0x0059E754`), heading switch, **clearance scan** (`0x0059EA0E`–`0x0059EACD`) | **0** |
| B | **water fill #1**, rect = §2.4 row 3 | 2 rejection loops **per in-bounds cell** (§2.7) |
| C | `RandomMapGenerator__GrowMeanderArm(this; regionTag, 0.003f, &clampRect, &seedCell, 0)` — `0x0059EC5F PUSH 0x3B449BA6` / `0x0059EC65 CALL 0x005A08D0` | its own seed + step draws (see `RMG_RIVER_MEANDER_005A08D0_005A0410_GHIDRA_REPORT.md`) |
| D | `0x0057A0C0(this=MapClass, regionTag, 0)` — `0x0059EC86` | **≥ 2 × (map cell count)** — see §5 |
| E | `RandomMapGenerator__DilateRegionRings(regionTag, 2, clampRect, 0, 0)` — `0x0059ECCE` | **0** (§1.4) |
| F | full-map sweep: cells with `gridTag == regionTag` get `CellClass+0x11B += 4` (`0x0059ED32 CMP ECX,EDI` / `JNZ` / `0x0059ED36 ADD byte [EAX+0x11B],4`) | **0** |
| G | **water fill #2**, rect = §2.4 row 4 | identical to B |
| H | bridge-tile stamping (placement cursor + `MapClass__ApplyBridgeTile`) | **0** (§4) |
| I | tail: advance the caller's river position, write `*pOutPlaced` | **0** |

Early exits: the clearance scan returns `1` with `*pOutPlaced` still `0` (a *quiet* abort —
the caller reads `*pOutPlaced`, never the return value, at `0x0059DDFA`). Failure of C, D or
E jumps to the tail with `local_ed = 0` (`0x0059EC6E`, `0x0059EC91`, `0x0059ECD9`), so the
fills and the `+0x11B` sweep are **not** rolled back inside this function.

### 2.7 The per-cell water-fill draws — exact form

Both fill loops are byte-identical in shape. Outer loop = rows (y), inner = columns (x)
(`0x0059EAD2` reads the y-origin `local_c8[1]`, `0x0059EAFA` the x-origin `local_c8[0]`).
Every cell is first tested against the isometric diamond (`0x0059EB0A`–`0x0059EB42`); cells
that fail jump to the next x **before** any RNG call, so **out-of-bounds cells consume zero
draws** — this is essential to reproduce the stream.

For each in-bounds cell, in this order
*(`disassemble_bytes 0x0059EACD-0x0059EC0F`)*:

```
; draw 1 — tile variant, U{0..5}
0059EB4A  MOV  ECX,0xABE890
0059EB4F  CALL 0x0065C780                    ; Random__Next  -> r (u32)
0059EB62  FILD qword [ESP+0x88]              ; (double)r, high dword forced to 0
0059EB69  FMUL double [0x007EDA40]           ; = 6/(2^32-1)
0059EB6F  CALL 0x007C5F00                    ; Math__ftol (truncate toward zero)
0059EB74  CMP  EAX,5 / JA 0x0059EB4A         ; unsigned reject if n > 5
          -> CellClass+0x38 = g_WaterSet_TileSetBase([0x00AA0738]) + n   (0x0059EBA0)

; draw 2 — sub-tile index, U{0..3}
0059EBA3  MOV  ECX,0xABE890
0059EBA8  CALL 0x0065C780
0059EBBF  FILD qword [ESP+0x80]
0059EBC6  FMUL double [0x007EDA38]           ; = 4/(2^32-1)
0059EBCC  CALL 0x007C5F00
0059EBD3  CMP  EBX,3 / JA 0x0059EBA3         ; unsigned reject if n > 3
          -> CellClass+0x11A = (u8)n                                     (0x0059EBF5)

; then, unconditionally for the same cell:
0059EBFB..                                   ; grid[y*W + x] + 0x38 = regionTag  (arg1)
```

Constants, read from memory:

| address | bytes (LE) | value |
|---|---|---|
| `0x007ED898` | `00 00 10 00 00 00 F0 3D` | `1/(2^32−1)` = `0x3DF0000000100000` |
| `0x007EDA38` | `00 00 10 00 00 00 10 3E` | `4/(2^32−1)` = `0x3E10000000100000` |
| `0x007EDA40` | `00 00 18 00 00 00 18 3E` | `6/(2^32−1)` = `0x3E18000000180000` |

*(`read_memory 0x007ED898` len 8, `read_memory 0x007EDA38` len 16)*

The rejection branch only ever fires when `r == 0xFFFFFFFF` exactly (that is the only input
for which `ftol(r · 6/(2^32−1))` reaches 6), but it must still be modelled — it costs a
whole extra draw when it happens, which shifts every subsequent value.

**x87 rounding form matters.** These inline draws use a *pre-divided* double and a **single**
`FMUL`. That is **not** the same rounding as `RandomMapGenerator__NextUniformRange`
(§5), which does `FILD r; FMUL span; FMUL (1/(2^32−1)); FADD lo` — three separately-rounded
operations. A port must implement the two forms separately; they are not algebraically
interchangeable at double precision.

### 2.8 The tail — 12-cell advance

`disassemble_bytes 0x0059FFC0-0x005A004C`:

```
[ESP+0x70..0x7C] = { 0, +12, 0, −12 }    ; X deltas  (0x0059FFE4..0x0059FFF2)
[ESP+0x60..0x6C] = { −12, 0, +12, 0 }    ; Y deltas  (0x0059FFF6..0x005A0002)
if (local_ed != 0):                       ; 0x005A0006
    idx = heading / 2                     ; CDQ / SUB EAX,EDX / SAR EAX,1  (0x005A000F..0x005A0019)
    *pRiverX += (double)X[idx]            ; 0x005A001E..0x005A0024
    *pRiverY += (double)Y[idx]            ; 0x005A0026..0x005A0033
*pOutPlaced = local_ed ; return local_ed  ; 0x005A003F / 0x005A0041
```

i.e. on success the caller's river head jumps 12 cells in its travel direction (N → `y−12`,
E → `x+12`, S → `y+12`, W → `x−12`), skipping the bridged span. The caller then does
`this->+0x308 += 1` and `bridgeCount += 1` (`0x0059DE02`–`0x0059DE14`).

---

## 3. `+0x11B` (level) and `+0x11C` (slope) write census

**No function in this subtree writes `CellClass +0x11C`.** Checked writers:

| Function | `+0x11B` | `+0x11C` | Other |
|---|---|---|---|
| `0x005A0160` DilateRegionRings | `= levelByte` **raw byte assign** (`0x005A036B`) | — | grid `+0x38` |
| `0x0059E740` BuildRiverBridge | `+= 4` on region cells (`0x0059ED36`) | — | `+0x38`, `+0x11A`, grid `+0x38` |
| `0x0059E740` per-heading ramp cells | `+= 4` (ramp-adjacent), `−= 4` (the N/W run) | — | `+0x38 = 0xFFFF`, `+0x11A = 0` |
| `0x0057B440` `MapClass__ApplyBridgeTile` | `= tileRecord[+0x28] + param_3` (the anchor cell's own `+0x11B`) | — | `+0x38 = tileType[0xA5]`, `+0x11A = sub-cell index` |
| `0x0057A430` `…UpdateBridgeTile_Low` | — | — | `+0x38 = g_WaterSet_TileSetBase`, `+0x11A = 0` |
| `0x0059D510` CarveRiver rollback | `= (char)MapSeed->+0x30C` | — | `+0x38 = 0`, `+0x11A = 0` |

*(`decompile_function 0x0057B440`, `0x0057A430`, `0x0059D510`; `disassemble_function 0x005A0160`)*

**Units.** `+0x11B` is the cell level and every arithmetic write in this path moves it by
exactly one quantum of **4** (`ADD byte [EAX+0x11B],0x4` / `+ '\xFC'`). The two *assignment*
writes (`0x005A036B` and the CarveRiver rollback) copy a caller-supplied byte verbatim —
in both cases sourced from `MapSeed->+0x30C`, which is itself maintained in steps of 4
(`+0x30C += 4` at `0x0059E502`-region on canyon success). `ApplyBridgeTile`'s
`tileRecord[+0x28] + baseLevel` is the only place a *tile-type record* contributes, and it
uses record offset **`+0x28`**, not `+0x2A`.

**Polarity warning — the two `+0x11B += 4` sweeps have OPPOSITE tests.** Both are full-map
`CellIterator` sweeps that add one level quantum, but:

* `CarveRiver`'s canyon sweep raises cells **NOT** in the region:
  `0x0059E452 CMP ECX,ESI` / `0x0059E454 JZ skip` / `0x0059E456 ADD byte [EAX+0x11B],0x4`.
* `BuildRiverBridge`'s sweep raises cells **IN** the region:
  `0x0059ED32 CMP ECX,EDI` / `0x0059ED34 JNZ skip` / `0x0059ED36 ADD byte [EAX+0x11B],0x4`.

*(`disassemble_bytes 0x0059E3B0-0x0059E47F` and `0x0059EC49-0x0059ED40`)* Both verified in
assembly; this is exactly the kind of inverted-predicate detail a decompiler-only reading
gets backwards.

**Two different strides.** `MapClass__Get_CellClass` @ `0x005657A0` indexes the real cell
array as `(y << 9) + x` with a hard `0x40000` bound, returning the scratch cell at
`0x00ABDC50` (and stashing the coord at `0x00ABDC74`) when out of range. The RMG scratch
grid at `[0x00ABED10]` uses `(y * g_PathfinderLinearMapWidth[0x0089C2DC] + x) * 0x50`.
A port must not conflate the two strides. *(`disassemble_function 0x005657A0`)*

---

## 4. The bridge-tile stamping path (stage H) — 0 draws

Per heading the code writes `g_UIModeLock` from
`g_IsometricTileTypeClass_Array + tilesetBase*4` (offsets `+0`, `+4`, `+8`, `+0xC` select the
ramp/1-cell/2-cell/far-ramp pieces), calls `0x004A91B0`, then `MapClass__ApplyBridgeTile`.

* **`g_UIModeLock` is a misnamed global.** In this path it holds the current
  `IsometricTileTypeClass*`: `ApplyBridgeTile` immediately does
  `(**(code**)(*g_UIModeLock + 0x2C))()` and compares the result to `0x12`, then reads
  `piVar1[0xB9]`/`[0xBA]` (tile width/height in cells) and `piVar1[0xA5]` (tile index base).
  *(`decompile_function 0x0057B440`)*
* **`0x004A91B0` is the placement-cursor setter**, not RMG-specific: its `param_1 + 0x1174`
  is `0x0087F7E8 + 0x1174 = 0x0088095C`, which is exactly the pair
  `_DAT_0088095C` / `DAT_0088095E` that `ApplyBridgeTile` reads as the stamp origin.
  *(`decompile_function 0x004A91B0`, `decompile_function 0x0057B440`)*
* Per-heading tileset bases: `0` → `[0x00AA10A0]`, `2` → `[0x00AA073C]`,
  `4` → `[0x00AA1050]`, `6` → `[0x00ABB110]`.

**Draws:** `get_function_callees 0x0057B440` = `{0x00486380, 0x004865B0, 0x004863D0,
0x005A0090, 0x005A00C0, 0x00578D80}` — none of these, and neither `0x0057B440` nor
`0x004A91B0`, appears in `get_function_callers 0x0065C780`. Stage H draws nothing.

---

## 5. Stage D — the shore finalization is the expensive draw site

`0x0057A0C0` (Ghidra name `MapClass__MarkBridgesForRepair_High` — **label drift**, its real
role is RMG water-region shore finalization; the plate comment on that address, written
earlier today by a parallel pass, documents the four sweeps).

The finding that matters for the stream: sweeps 3 and 4 call
`MapClass__SelectBridgeTileVariant_Low @ 0x0057ACF0` once **per map cell**, and that function
draws **unconditionally at entry, before any early-out**:

```
0057ACF9  MOV  EDX,0x5          ; hi = 5
0057ACFE  XOR  ECX,ECX          ; lo = 0
0057AD04  CALL 0x00598030       ; RandomMapGenerator__NextUniformRange(0, 5)
0057AD09  ...                   ; only THEN is ComputeBridgeSurfaceMask called
```
*(`disassemble_bytes 0x0057ACF0-0x0057AD2F`; `decompile_function 0x0057ACF0`)*

`RandomMapGenerator__NextUniformRange @ 0x00598030`, `__fastcall(ECX = lo, EDX = hi)`
*(`disassemble_function 0x00598030`)*:

```
span = (double)(hi - lo + 1)
do {
    r = Random__Next()                                      ; 0x00598063
    n = ftol( ((double)r * span) * [0x007ED898] + (double)lo )   ; 0x00598070..0x00598082
} while ((unsigned)n > (unsigned)hi)                        ; 0x00598087 CMP EAX,ESI / JA
```

So **stage D costs at least `2 × cellCount` draws**, dwarfing everything else in the bridge
path, and the drawn value is consumed as `n & 1` (corner/straight variant) or `n % 3`
(3-way variant) depending on which mask branch is taken. Any port that treats the shore pass
as "a few variant draws" will desync immediately.

Sibling: `RandomMapGenerator__NextUniform01 @ 0x00598000` — one draw, returns
`(double)r * [0x007ED898]` on ST0, no rejection *(`disassemble_function 0x00598000`)*.

---

## 6. Two predicates the dilation depends on

**`CellClass__IsClearTile @ 0x00486380`** *(`disassemble_function 0x00486380`)* — leaf, no
calls. Returns 1 iff `cell+0x38 == 0` **or** `cell+0x38 == 0xFFFF`. "Clear" means "default
green tile, or the `0xFFFF` sentinel the RMG stamps on bridge-deck cells" — it is **not** a
passability test.

**`0x004865D0`** *(`disassemble_function 0x004865D0`)* — leaf, no calls. The Ghidra name
`CellClass__HasBridgeOverlay` understates it. It returns 1 iff `cell+0x38` falls in any of:

| range | meaning |
|---|---|
| `[ [0x00ABAD28], +0x2A )` | 42-entry shore-piece set (`g_ShorePieces`) |
| `[ [0x00AA0738], +0x0E )` | `g_WaterSet_TileSetBase`, 14 water tiles |
| `[ [0x00AA073C], +4 )` | RMG bridge tileset, heading 2 (E) |
| `[ [0x00ABB110], +4 )` | heading 6 (W) |
| `[ [0x00AA1050], +4 )` | heading 4 (S) |
| `[ [0x00AA10A0], +4 )` | heading 0 (N) |

i.e. "the cell's tile is shore, water, or one of the four RMG river-bridge tilesets". Those
four bridge bases are precisely the ones `BuildRiverBridge` uses in its heading switch, which
is what lets a `stampLevel != 0` dilation ring cross the previous segment's bridge and water.

---

## 7. Tiberian Sun / dead-branch check

| Branch | Verdict | Gating evidence |
|---|---|---|
| Jump-table entries 1, 3, 5 (`0x0059E9FA`) | **Unreachable in practice**, harmless | `CarveRiver` only ever produces `0/2/4/6` (`0x0059DD36`, `0x0059DD3D`, `0x0059DD55`, `0x0059DD59`). With all rect locals still zero, the default path degenerates to empty loops. |
| `0x005A0160` `HasBridgeOverlay` acceptance leg | **LIVE in stock YR** | Requires `stampLevel != 0`, passed only by `CarveRiver 0x0059E4F5`, which is reached when `bridgeCount > 0` — i.e. whenever a river actually placed a bridge. Not TS-gated, no `SpecialFlags` involvement. |
| `0x005A0160` `operator_new` returning 0 | Dead in practice | `0x005A0195 CMP ESI,EDI / JZ` sets the vector pointer to 0 and then immediately dereferences it at `0x005A01CC`; an actual OOM would fault. Not a branch to port. |
| `0x004A91B0` map-editor branch | **The non-editor leg is the live one** | `if (g_IsMapEditor == '\0')` selects the `FUN_006DA360` path; `gamemd.exe` skirmish has `g_IsMapEditor == 0`. |
| `0x004A91B0` invalid-cell / radar-viewport block | Not entered from RMG | It is gated on `coord == DAT_008A03F8` (the invalid-cell sentinel); the bridge path always passes a real cell. |
| Whole bridge path | Gated, but stock-reachable | `CarveRiver`'s bridge gate additionally requires `MapSeed->+0x310 != 0` (byte read at `0x0059DCFC`) and `stepCount > bridgeMinStep` where `bridgeMinStep = U{0..125}` drawn once per river (`while (0x7D < uVar6)` in `CarveRiver`). No TS flag. |

No `SpecialFlags`-gated, fog-of-war, or subterranean code exists anywhere in this subtree.

---

## 8. Cross-check against `RMG_MODE34_WATER_BRIDGES_TECH_GHIDRA_REPORT.md`

No contradictions found. Agreements and additions:

* That doc's "`A` = cross-section start, `B` = end" (inferred from the dir-0 clearance rect)
  is **confirmed independently** here from the four `ftol` source expressions in `CarveRiver`
  (§2.3). This report adds the generalisation that the *anchor* flips to `B` for headings
  4 and 6.
* Its "two draws per water cell of both fills, `U[0,5]` tile + `U[0,3]` subtile with
  rejection" is confirmed; this report adds the constant addresses, the x87 form, and the
  out-of-bounds-consumes-nothing rule.
* Its `{0,+12,0,−12}` / `{−12,0,+12,0}` advance table is confirmed byte-for-byte.
* Its "`FUN_005A0160` … no RNG" (an operand-level statement) is **upgraded to a transitive
  proof** here (§1.4).
* Its §5.1(b) "plus the `0x0057A0C0` shore-pass variant draws" is **quantified** here as
  ≥ 2 × cellCount (§5).
* Its `+0x11B += 4` polarity statements (canyon = cells *not* in region; bridge = cells *in*
  region) are confirmed in assembly (§3).

---

## 9. Unverified (YELLOW) — do not treat as VERIFIED

1. **Per-heading bridge-tile stamping loop coordinates inside `0x0059E740` (stage H).**
   The four case tails (`FUN_004A91B0` + `ApplyBridgeTile` sequences, the
   `uVar & 0x80000001` parity alternation, the `+2` step for N/S vs `+1` for E/W in the odd
   branch, the `(span+2)`-cell `+0x11B −= 4` run that exists only in cases 0 and 6) are taken
   from `decompile_function 0x0059E740` and were **not** re-derived from assembly in this
   session. The earlier mode-34 report claims to have verified them; that claim is
   second-hand here.
2. **`0x004A91B0`'s early-out.** Whether `[0x00880964]` (`param_1 + 0x117C`, the pending
   placement-object pointer) is zero during map generation determines whether the redraw
   sub-calls `FUN_004A95A0` / `FUN_004A8EB0` / `FUN_004A9070` execute. Not determined
   statically. It does not affect the draw ledger (none of those is a `Random__Next` caller),
   but it does affect which cells are marked dirty.
3. **`GrowMeanderArm`'s own draw count** (stage C) is out of scope here; see
   `RMG_RIVER_MEANDER_005A08D0_005A0410_GHIDRA_REPORT.md`, whose contract that doc's own
   §12 lists as single-source.
4. **`0x0057A0C0` sweeps 1 and 2** (`0x0057A430`, `0x0057A320`) are asserted draw-free on the
   strength of `get_function_callees` + the `Random__Next` caller set for `0x0057A430`;
   `0x0057A320` was not opened this session.
5. **Exact map-cell count for the stage-D estimate.** "`2 × cellCount`" uses the
   `MapClass__CellIterator_Next` traversal extent, which this session did not enumerate.

---

## 10. Reproducibility — MCP calls used

`get_current_program_info`;
`decompile_function` 0x005A0160, 0x0059D510, 0x0059E740, 0x0057A0C0, 0x0057ACF0, 0x0057A430,
0x0057B440, 0x004A91B0, 0x005A0700, 0x005A0090, 0x005A00C0, 0x0049F300, 0x00598030;
`disassemble_function` 0x005A0160, 0x005657A0, 0x00486380, 0x004865D0, 0x00598000, 0x00598030,
0x0049F300;
`disassemble_bytes` 0x0059DB80-0x0059DCFB, 0x0059DCFC-0x0059DDA7, 0x0059DDA3-0x0059DE1F,
0x0059E3B0-0x0059E47F, 0x0059E740-0x0059E8EF, 0x0059E8EF-0x0059EA1F, 0x0059EACD-0x0059EC0F,
0x0059EC49-0x0059ED40, 0x0059FFC0-0x005A004C, 0x0057ACF0-0x0057AD2F;
`read_memory` 0x0089F660, 0x0089F688, 0x007E3890, 0x007ED898, 0x007EDA38, 0x005A0050,
0x0049F2D0;
`get_function_callees` 0x0059D510, 0x0059E740, 0x005A0700, 0x005657A0, 0x00486380, 0x0042F860,
0x0042FBF0, 0x0042FCB0, 0x007C8B3D, 0x004A91B0, 0x0057B440, 0x0057A0C0, 0x005A08D0, 0x0057ACF0,
0x0057A430;
`get_function_callers` 0x0065C780, 0x005A0160;
`get_xrefs_to` 0x0089F688, 0x0089F6A4, 0x0049F300;
`get_function_by_address` 0x005657A0, 0x0059E740;
`analyze_data_region` 0x0089F688;
`list_segments`.

---

# ADDENDUM — 2026-07-25 — `MapClass__CellIterator` extent: the `0x0057A0C0` draw multiplier is now closed-form

**Added by a later session.** This addendum closes §9 open item 5 of this document
("Exact map-cell count for the stage-D estimate … this session did not enumerate"). Nothing
above is edited; the symbolic `2 × cellCount` is now resolved. `save_program` deliberately
NOT called — the coordinator owns saving.

## A1. The answer

Let **N = `g_nMapRectWidth`** (`0x0087F8DC` = `MapClass+0xF4`) and
**M = `g_nMapRectHeight`** (`0x0087F8E0` = `MapClass+0xF8`) — the Width/Height of the rect
handed to `MapClass::Resize` (`0x00566200`: `*(this+0xF4) = param_2[2]`,
`*(this+0xF8) = param_2[3]`; the rect's X/Y are then forced to 0).

```
cells per full CellIterator sweep      =  M * (2N - 1)
RandomRangeInclusive(0,5) draws taken
by MapClass__MarkBridgesForRepair_High =  2 * M * (2N - 1)      (no early refusal)
0x0057A0C0                                (upper bound; see A5 for the refusal case)
```

`g_Map` is `0x0087F7E8` — proved by the inlined `CellIterator_Init` in `0x0057A0C0`
(`disassemble_bytes 0x0057A20A`: `MOV ECX, 0x87F7E8` immediately alongside
`MOV EAX,[0x0087F8DC]` / `MOV EDX,[0x0087F924]`, and `0x87F7E8 + 0xF4 = 0x87F8DC`,
`+0x10C = 0x87F8F4`, `+0x13C = 0x87F924` — all four match the four stores the function makes).

## A2. Exact cell extent — the allocated playable diamond

`MapClass__CellIterator_Init` `0x00578350` / `MapClass__CellIterator_Next` `0x00578290`
(`decompile_function`, both) walk the 512x512 `CellClass*` table at
`g_CellArray_Base` = `MapClass+0x13C` = `0x0087F924` (row stride `0x800`, index
`y*0x200 + x`) **one anti-diagonal at a time, up-and-to-the-right** — not a rectangle scan.

State quartet (now named in Ghidra):

| Field | Global | Name applied | Meaning |
|---|---|---|---|
| `+0x10C` | `0x0087F8F4` | `g_nCellIterCol` | cursor `x` |
| `+0x110` | `0x0087F8F8` | `g_nCellIterRow` | cursor `y` |
| `+0x114` | `0x0087F8FC` | `g_nCellIterRunRemaining` | cells left on this diagonal |
| `+0x118` | `0x0087F900` | `g_pCellIterCursor` | `&array[y*0x200 + x]` |

`Init`: `x=1, y=N, run=N-1, ptr = base + N*0x800 + 4` -> starts at cell **(1, N)**.

`Next()` reads `v = *ptr` **first**, then advances, then returns `v`:

```
run != 0 :  x+1 ; y-1 ; run-1 ; ptr += 4 - 0x800          (step along the diagonal)
run == 0 :  newx = y ; newy = x ; then
              ((y - N - 1 + x) & 1) == 0 -> run = N-2 ; newx = y+1
              else                       -> run = N-1 ; newy = x+1
            ptr = base + (newy*0x200 + newx)*4
```

Diagonals therefore come in the order `x+y = N+1, N+2, N+3, …`, holding **N** cells when
`(x+y) - N` is odd and **N-1** cells when it is even.

**The allocated set.** A slot holds a non-NULL pointer exactly while

```
N < x + y     &&     x - y < N     &&     y - x < N     &&     x + y <= N + 2M
```

That identical four-term predicate appears at four independent binary sites:
`MapClass__Resize 0x00566200` (three times — the `CellClass__Constructor` allocation loop, the
default-level pass, and the final "outside -> `*slot = 0` + release" cleanup),
`MapClass__StampPendingIsoTileBlock 0x0057B440` (per-sub-cell gate), and — via the cached
copies `g_nMapDiamondWidth 0x00ABED04` (= N) and `g_nMapDiamondMaxCoordSum 0x00ABED08`
(= N + 2M), both written by `Resize` — `MapClass__ClearBridgeCell_Low 0x0057A320` and
`MapClass__UpdateBridgeTile_Low 0x0057A430` neighbour gates. (`decompile_function` on all
five.) Max `x` = max `y` = `N + M - 1`.

**The diagonals coincide with the diamond rows exactly.** For `s = x+y = N+k` the predicate
reduces to `k/2 < x < N + k/2`, giving `x` in `[k/2+1, N+k/2-1]` (N-1 cells) for even `k` and
`x` in `[(k+1)/2, N+(k-1)/2]` (N cells) for odd `k` — term for term what the state machine
produces. Over `k = 1 … 2M` that is `M` diagonals of `N` plus `M` of `N-1`:

```
M*N + M*(N-1)  =  M * (2N - 1)
```

Verified mechanically: the state machine transcribed verbatim from `0x00578290`/`0x00578350`
was run against the predicate for every `N = 2…60` x `M = 1…60` — visited set equals the
allocated set, no repeats, and the count equals `M*(2N-1)` in all 3540 cases, **0 mismatches**.
(Confidence note: this is an *algebraic/transcription* proof, not gamemd emulation — the two
inputs are each read from the binary, the composition is machine-checked. A live-emulation
cross-check of `0x00578290` remains UNVERIFIED-pending-instrument, since the function is
stateful on `g_Map` globals plus a populated 512x512 pointer table.)

**Count depends on map dimensions only** — N and M. No per-cell state enters the extent.

## A3. No cell is skipped; the terminating cell never reaches a loop body

`Next()` applies **no flag test, no bounds test, no predicate** — it walks contiguously and
returns whatever pointer is in the slot. Its only stop signal is a NULL pointer, and each
caller's loop is `while (cell != 0)` with the null test *before* the body
(`disassemble_bytes 0x0057A1F0` / `0x0057A28D`: `CALL 0x00578290 ; TEST EAX,EAX ; JZ out`).

The first cell of diagonal `x+y = N+2M+1` is outside the diamond, hence NULL, hence the sweep
ends there — and that NULL is **never** passed to `SelectBridgeTileVariant_Low`. So the
"draw is taken at callee entry regardless of predicates" concern does **not** add a
`+1`: the callee is entered exactly `M*(2N-1)` times, once per allocated cell.

The one caveat worth writing into a port assertion: the stop condition is *first NULL along
the walk*, not *outside the diamond*. They coincide only while every diamond cell is
allocated — the normal post-`Resize` invariant (`Resize` allocates every diamond slot and
frees/NULLs every non-diamond slot). A port that models the diamond directly is exact; a port
that models "stop at first hole" must guarantee no holes.

**Do not conflate this with `MapClass__IsCellInPlayfield`.** That function uses a *different*
diamond built from `MapClass+0xFC/+0x100/+0x104/+0x108` (the LocalSize window) — see
`docs/research/coord-cell-conversions/fn-map-is-cell-in-playfield.md`. The iterator's extent
is the **allocated MapRect diamond** (`+0xF4`/`+0xF8`), which is larger. That doc's open item
"exact semantic labels for `MapClass+0xf4`, `+0xf8` … remain unknown" is resolved for those
two fields by this addendum.

## A4. `0x0057A0C0` runs FOUR sweeps, two of which draw — confirmed from assembly

All four re-inline `CellIterator_Init` against the same globals
(`0x0057A14C`, `0x0057A1B1`, `0x0057A20A`, `0x0057A267`), so **all four cover the identical
extent** (nothing in the sweep bodies allocates or frees cells — `UpdateBridgeTile_Low`,
`ClearBridgeCell_Low` and `StampPendingIsoTileBlock` only write `CellClass` fields, so the
NULL pattern is stable across sweeps).

| # | Callee | Loop shape (`disassemble_bytes`) | RNG draws |
|---|---|---|---|
| 1 | `0x0057A430` `UpdateBridgeTile_Low` | fail-fast on return | **0** |
| 2 | `0x0057A320` `ClearBridgeCell_Low` | `0x0057A1EE`: `TEST BL,BL` gates entry but `BL` is **never reassigned** — return discarded, runs the full extent | **0** |
| 3 | `0x0057ACF0` `SelectBridgeTileVariant_Low(cell, 1, …)` | `0x0057A243` `TEST EAX,EAX ; JZ` then `0x0057A247` `TEST BL,BL ; JZ` ; `MOV BL,AL` after the call -> fail-fast | **1 per cell** |
| 4 | `0x0057ACF0` `SelectBridgeTileVariant_Low(cell, 2, …)` | byte-identical shape at `0x0057A2A0`/`0x0057A2A4`/`0x0057A2B9`, `PUSH 0x2` at `0x0057A2AA` | **1 per cell** |

So the earlier "second sweep `0x0057A320` that was never opened" is now opened: it is the
water notch/strait clearer, it is **not** fail-fast, and it takes **no RNG draw** — it does not
enter the multiplier.

**The draw itself** (`disassemble_bytes 0x0057ACF0`, first 40 bytes) is the 6th instruction of
`SelectBridgeTileVariant_Low`, before any branch:

```
0057ACF9  MOV  EDX, 0x5
0057ACFE  XOR  ECX, ECX
0057AD04  CALL 0x00598030          ; RandomMapGenerator__RandomRangeInclusive(lo=0, hi=5)
```

Unconditional, ahead of `ComputeBridgeSurfaceMask` and ahead of every `return 1` early-out
(`mask == 0`, unmatched neighbour patterns, `iVar7 < 1`, `param_2` not in `{1,2}`). The value is
consumed as `r & 1` or `r % 3` on the shore-piece paths and **discarded on the rest** — but the
stream has already advanced either way. Per `0x00598030`'s own plate comment the call is
exactly one `Random__Next` on `g_MapGenRng 0x00ABE890`, with a `2^-32` chance of a second
(rejection only on `raw == 0xFFFFFFFF`).

**No other RNG on the path.** None of `SelectBridgeTileVariant_Low`'s callees
(`ComputeBridgeSurfaceMask 0x0057B210`, `StampPendingIsoTileBlock 0x0057B440`,
`MapCoord_StepByDir_GetCell 0x00481810`, `CellClass__IsBridgeDeckTile 0x00485060`,
`0x004A91B0` — `get_function_callees 0x0057ACF0`) appears in `get_xrefs_to 0x00598030` or in
`get_xrefs_to 0x00ABE890`. Sweeps 1 and 2 likewise appear in neither list.

## A5. Closed form for the port assertion

```
N = g_nMapRectWidth   (0x0087F8DC = Map+0xF4)
M = g_nMapRectHeight  (0x0087F8E0 = Map+0xF8)
cells = M * (2N - 1)

draws_by_0x0057A0C0 = pass1_calls + pass2_calls

  pass1_calls = cells                                   if sweep 1 succeeded, no pass-1 refusal
              = (index of first refusing cell) + 1       otherwise, then pass2_calls = 0
  pass2_calls = cells                                   if pass 1 completed with no refusal
              = (index of first refusing cell) + 1       if pass 2 refuses
              = 0                                       if pass 1 refused or sweep 1 failed

  =>  draws = 2 * M * (2N - 1)   in the no-refusal case, which is the assertion to write.
      Upper bound in all cases:  draws <= 2 * M * (2N - 1).
```

A "refusal" is `SelectBridgeTileVariant_Low` returning 0, which happens **only** through the
by-ref ok-flag that `MapClass__StampPendingIsoTileBlock 0x0057B440` clears on a shore-piece
compatibility conflict (`g_nShorePieceOrientTable` delta in `[3,5]`, or the
`g_nShorePieceGroupTable` mismatch branch) — see that function's plate comment. It is
data-dependent, so a port must reproduce the stamper's refusal decision to reproduce the
*truncated* stream; the multiplier itself is geometry-only.

Worked values: N=M=60 -> 7 140 cells -> **14 280** draws. N=M=120 -> 28 680 cells -> **57 360**
draws. This is the dominant `g_MapGenRng` consumer in the whole water stage, as §5 of this
document estimated — the estimate's shape was right, only the multiplier was missing.

## A6. Tiberian Sun / reachability check

The whole `0x0057A0C0` subtree is **RMG-only, never gameplay**. Its only callers are the five
RMG water routines (`0x0059A6C0`, `0x0059BBC0`, `GrowLake 0x0059C920`, `CarveRiver 0x0059D510`,
`BuildRiverBridge 0x0059E740` — `get_function_callers 0x0057A0C0`), and the genuine gameplay
sibling `MapClass__MarkBridgesForRepair_Low 0x00578E60` is a different function. Nothing here
runs during a match.

`RandomMapGenerator__Generate 0x00598960` has exactly two callers
(`get_function_callers 0x00598960`):

- `RandomMapSetupDialog__Proc 0x00596300` — the RMG setup dialog, reached from
  `ChooseMap__AcceptRandomMapSetup 0x005E8590` (`get_xrefs_to 0x00596300` -> the dialog is
  registered at `0x00595BD8` in `FUN_00595BC0`, whose only caller is that ChooseMap handler).
- `ScenarioClass__Read_Scenario` at `0x00684989` (`disassemble_bytes 0x00684950`), gated on
  the byte `ScenarioClass+0x34BD` (`g_ScenarioClass_Instance` = `0x00A8B230`) and on
  `MapSeedClass::ReadScenarioSection 0x00597A10` (`this = g_MapSeed 0x00ABDFD8`) returning
  nonzero — i.e. the scenario file carries a map-seed section.

So the path is live YR code, not a dead TS branch, and it is *not* `SpecialFlags`-gated.
**UNVERIFIED here:** whether stock YR's retail map-selection UI actually exposes the generator
button to a skirmish player was not determined this session — only that the code path is
reachable and unconditional once a scenario carries a map-seed section. That question does not
affect the formula.

**UNRESOLVED (does not affect the formula):** which code writes `Map+0xF4`/`+0xF8` on the RMG
path. `MapClass::Resize 0x00566200` is the writer, but its only caller is `FUN_00653F50`
(`get_function_callers 0x00566200`), which is not on the RMG path, and
`RandomMapGenerator__Generate`'s callee list contains no `Resize`. The only *absolute* write to
`0x0087F8DC`/`0x0087F8E0` is `Clear_Scenario 0x0068555A`, which zeroes them
(`get_xrefs_to 0x0087F8DC`, `decompile_function 0x006851F0`); live writes go through
`this+0xF4`, which absolute-operand xrefs cannot see. `MapSeedClass__ReadINI 0x005981F0` does
not set them (`decompile_function`).

## A7. Ghidra labels applied by this addendum

Plate comments: `MapClass__CellIterator_Next 0x00578290`, `MapClass__CellIterator_Init
0x00578350`, `MapClass__SelectBridgeTileVariant_Low 0x0057ACF0`.
Globals named + typed + commented (`set_global`): `0x0087F8DC` -> `g_nMapRectWidth`,
`0x0087F8E0` -> `g_nMapRectHeight`, `0x0087F8F4` -> `g_nCellIterCol`, `0x0087F8F8` ->
`g_nCellIterRow`, `0x0087F8FC` -> `g_nCellIterRunRemaining`, `0x0087F900` ->
`g_pCellIterCursor`, `0x00ABED04` -> `g_nMapDiamondWidth`, `0x00ABED08` ->
`g_nMapDiamondMaxCoordSum`. `save_program` NOT called.

## A8. Addendum reproducibility — MCP calls

`get_current_program_info`;
`search_functions` "CellIterator";
`decompile_function` 0x0057A0C0, 0x00578290, 0x00578350, 0x00566200, 0x0057A320, 0x0057A430,
0x0057ACF0, 0x0057B440, 0x00598030, 0x005981F0, 0x006851F0;
`disassemble_bytes` 0x0057ACF0 (40), 0x0057A1E6 (10), 0x0057A1F0 (160), 0x0057A28D (60),
0x00684950 (70);
`get_xrefs_to` 0x0087F8DC, 0x00ABE890, 0x00598960, 0x00596300;
`get_function_callees` 0x0057ACF0, 0x00598960;
`get_function_callers` 0x00598960, 0x00595BC0, 0x005E8590, 0x00566200;
`set_plate_comment` 0x00578290, 0x00578350, 0x0057ACF0;
`set_global` 0x0087F8DC, 0x0087F8E0, 0x0087F8F4, 0x0087F8F8, 0x0087F8FC, 0x0087F900,
0x00ABED04, 0x00ABED08.
