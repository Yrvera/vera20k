# DestroyBridge_{High,Low}_MapInit — Body Decode

**Slot 2 of /re-swarm bridges --area (2026-05-18)**
**Scope:** Decompile and document the hut-death dispatchers
`MapClass::DestroyBridge_High_MapInit @ 0x00574000` and
`MapClass::DestroyBridge_Low_MapInit @ 0x00574C20`.
**Mode:** READ-ONLY Ghidra. No annotations applied.

---

## 0. TL;DR

- **The two are structural twins.** Identical control flow; differ in three constants:
  inner-scan overlay band, sub-dispatcher target, and bridge tile-base global.
- **`_MapInit` suffix is misleading and pre-confirmed wrong.** Both are
  runtime-called from `BombClass::Detonate @ 0x00438720` (demo-truck on a
  BridgeRepairHut) and `BuildingClass::Update @ 0x0043FB20` (C4-timer expiry on
  a BridgeRepairHut). Xref verification matches the documented call sites.
- **Cell-walk pattern is a 5×5 inner scan + an 8-direction fallback walk + a
  ramp-search forward walk.** Not a simple linear scan.
- **No RNG, no direct +0x11E / +0x11A / +0x11B / +0x44 / +0x140 writes** in
  these two functions. All per-cell state mutation is delegated to
  `ApplyDamageToCell @ 0x00587180` (which calls the lower-level
  `DestroyBridge_Low/High` tile primitives and the state machine), or to
  `DestroyBridgeFromCell_{High,Low}` if the inner scan finds a match.
- **Global side effects (always, on the slow path):**
  `MapClass::UpdateAdjacentBridges_High @ 0x00576770`, then
  `byte [g_Tactical + 0xD7C] = 1` (deferred-rebuild flag), then
  unconditional `MapClass::UpdateBridgeZonesHelper @ 0x0056C510` at the tail.
- **No animation spawn, no audio cue, no EVA call** is emitted directly by these
  two functions. Anim/sound for collapse happens inside the
  `DestroyBridgeFromCell_*` / `CollapseBridge_*_*` / `BlowUpBridge` callees, or
  inside `ApplyDamageToCell`'s per-cell handlers.

**Active in YR:** Yes, both functions, all branches. Verified by live xrefs
into `BombClass::Detonate` and `BuildingClass::Update`. No TS-gated branches
observed in the two bodies themselves.

---

## 1. Caller xrefs (confirms runtime-live, not map-init-only)

Verified via `get_function_xrefs` and `get_function_callers`:

| Function | Caller | Xref site | Type |
|----------|--------|-----------|------|
| `DestroyBridge_High_MapInit` @ `0x00574000` | `BombClass::Detonate` @ `0x00438720` | `0x00438982` | UNCONDITIONAL_CALL |
| `DestroyBridge_High_MapInit` @ `0x00574000` | `BuildingClass::Update` @ `0x0043FB20` | `0x0044031B` | UNCONDITIONAL_CALL |
| `DestroyBridge_Low_MapInit`  @ `0x00574C20` | `BombClass::Detonate` @ `0x00438720` | `0x0043896A` | UNCONDITIONAL_CALL |
| `DestroyBridge_Low_MapInit`  @ `0x00574C20` | `BuildingClass::Update` @ `0x0043FB20` | `0x00440301` | UNCONDITIONAL_CALL |

Both functions have **exactly two callers**, both runtime-live. No map-load
caller exists in the binary (verified — total callers = 2). The `_MapInit` Ghidra
label is a misnomer.

Caller sites match those documented in
`BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md §3` exactly.

---

## 2. High variant — body decode

### Signature

```
void __thiscall MapClass::DestroyBridge_High_MapInit(int param_1, short *param_2)
```

- `param_1` (`this` / `MapClass*`, ECX) — used at offsets `+0x124`, `+0x128`,
  `+0x12C`, `+0x130`, `+0x13C` (these are the **map bounds / cell-pointer
  array**, matching the standard `MapClass` layout; param_1 is `int` so these
  are **direct byte offsets**, no ×4 multiply).
- `param_2` (`Cell::Coord*`, packed `short[2]` X/Y) — input cell coord of the
  destroyed hut.

### Phase 1 — 5×5 inner overlay scan (column-major)

`0x00574009..0x0057409A`

```
for (dx = -2; dx < 3; dx++)
  for (dy = -2; dy < 3; dy++)
    cell = MapClass::Get_CellClass(X+dx, Y+dy)
    if (0xCC < cell->OverlayTypeIndex(+0x44) && cell->OverlayTypeIndex < 0xE9)
      return DestroyBridgeFromCell_High(cell.Coord)
```

- Scan order: **dx outer, dy inner** (verified at `0x0057405A INC ESI` =
  dy-inner, `0x00574060 INC EDI` = dx-outer).
- Overlay band tested: `[0xCD .. 0xE8]` inclusive (high-bridge body overlays).
  Verified at `0x0057404C CMP EAX, 0xCD` and `0x00574053 CMP EAX, 0xE8`.
- On first match: dispatches to `MapClass::DestroyBridgeFromCell_High`
  (`0x005749C0`, verified via `0x0057408E CALL 0x005749C0`) and returns
  **immediately**. The slow path below does not run.

### Phase 2 — Fallback flag walk (if no overlay match in 5×5)

`0x0057409D..0x0057422D`

1. Re-fetch the input cell pointer at `(X,Y)` (with the standard bounds-check
   fallback to a sentinel cell `DAT_00ABDC50` and `DAT_00ABDC74` last-coord
   slot).
2. If `(cell.Flags+0x140 & 0x500) != 0` → skip the walk, jump straight to the
   anchor-handling phase. (`0x500` = `0x100 | 0x400` = "bridge structural cell"
   | "bridgehead cell".)
3. Otherwise walk **8 directions** indexed by a rotating `dir_idx` from 0..7.
   For each direction, step 1, 2, and 3 cells out via `g_DirectionOffsets` table
   pair (X offsets at `0x0089F688`, Y offsets at `0x0089F68A` — interleaved 4-byte
   stride). Break on the first cell whose `flags & 0x500 != 0`.
4. After 8 directions × 3 steps without a hit → terminate (return). (Loop bound
   verified at `0x0057421C CMP EAX, 0x8`.)

### Phase 3 — Anchor resolution

`0x00574231..0x00574346`

Branches on `(found_cell.Flags+0x140 & 0x100)` and `& 0x400`:

| Flag state                      | Anchor source                                         |
|---------------------------------|-------------------------------------------------------|
| `& 0x100 == 0 && & 0x400 == 0`  | **Return early** (`0x00574244 JZ 0x005745F2`).        |
| `& 0x100` set, `& 0x80` set     | `anchor = *(short*)(cell + 0x24)` (this cell's coord).|
| `& 0x100` set, `& 0x80` unset   | `anchor = *(short*)(*(cell+0x2C) + 0x24)` (peer cell's coord; `+0x2C` is the bridge-peer pointer per BRIDGE_SYSTEM.md). |
| `& 0x100 == 0 && & 0x400` set   | Walk perpendicular for up to 4 cells looking for another `& 0x400` cell; if not found → return; else step 2 more cells in the same dir to land on anchor. Initial dir = `(cell.flags & 0x800) ? 4 : 2`. Verified at `0x00574265` via `read_memory(0x00574265, 30)` = `25 00 08 00 00 ... F7 D8 1B C0 ... 83 E0 02 ... 83 C0 02` decoding to `AND EAX,0x800; NEG; SBB EAX,EAX; AND EAX,0x2; ADD EAX,0x2` — bit-set yields 4, bit-unset yields 2 (the `0` in the prior version of this doc was the intermediate uVar10 value before the final `+ 2`). |

### Phase 4 — Ramp-search forward walk

`0x00574350..0x005745AB`

1. `forward_dir = (cell.flags & 0x800) ? 6 : 0` (verified at
   `0x00574352 AND EDI, 0x800` → `0x0057435C SBB / AND 0x6`).
2. Construct a stack-local **DynamicVector accumulator** via
   `FUN_0042FCB0(0, 0)` (verified — it's a vector ctor; vftable
   `0x007E3890` is a vector class). The vector is **never appended to** by
   either *_MapInit body; it exists for the destructor to clean up if
   `ApplyDamageToCell` populates it (it doesn't, in this path).
3. Walk forward in `forward_dir` from `anchor`, stepping one cell per
   iteration, bounded by `param_1+0x124..param_1+0x130` (map bounds).
4. At each cell along the walk:
   - If `param_1+0x13C[cell_index] != 0` (some per-cell occupancy/index array
     entry) AND `MapClass::IsBridgeRampTile @ 0x005746C0` returns true
     → enter the "destroy ramp" sub-loop:
     - `reverse_dir = (forward_dir - 4) & 7`
     - Call `ApplyDamageToCell @ 0x00587180` on the ramp cell up to 3 times.
       (Each call returns a bool; loop breaks on the first true return.)
5. After the first ramp is destroyed, step forward in `reverse_dir` looking
   for a cell where `MapClass::IsLowBridgeEndpointTile @ 0x00574600` returns
   true. (Yes — even on the High dispatcher, the loop tail probe is named
   `IsLowBridgeEndpointTile`; verified at `0x00574544 CALL 0x00574600`. The
   distinction between High and Low at this point is encoded only by the
   tile-base global `DAT_00AA0E28` (High) vs `DAT_00ABAD1C` (Low) subtracted
   from `cell+0x38`.)
6. If `cell+0x38 - DAT_00AA0E28 != -2`, call `ApplyDamageToCell` up to 3 times
   on this cell as well.

### Phase 5 — Tail (global side effects)

`0x005745AD..0x005745CC`

Always reached when any phase 3+/4 work was done (LAB_005745AD):

```
MapClass::UpdateAdjacentBridges_High(&anchor)            // 0x00576770
*(byte*)(g_Tactical + 0xD7C) = 1                         // 0x008880A0 (deferred-rebuild flag)
MapClass::UpdateBridgeZonesHelper()                      // 0x0056C510 (unconditional)
```

The unconditional `UpdateBridgeZonesHelper` call runs even when the function
returns early via `LAB_005745CA` (early bounds-rejection at the ramp walk).
The early-out at `LAB_005745F2` (no-bridge case after phase 3) does **NOT**
reach the tail — it skips both the dirty flag and the zone rebuild.

### Per-cell write map (writes inside this function only)

**None.** This function writes only to:

- `DAT_00ABDC74` — sentinel "last-coord" global, written each time a cell
  fetch falls back to the sentinel cell `DAT_00ABDC50`. Not a state mutation;
  diagnostic only.
- `byte [g_Tactical + 0xD7C] = 1` — deferred terrain-rebuild flag (one byte).

All bridge-cell mutations (writes to `+0x11A`, `+0x11B`, `+0x11C`, `+0x11E`,
`+0x44`, `+0x140`) are produced **downstream** inside
`DestroyBridgeFromCell_High`, `ApplyDamageToCell` (→
`DestroyBridge_High`/`DestroyBridge_Low` tile primitives and the state
machines), or `UpdateAdjacentBridges_High` / `UpdateBridgeZonesHelper`.

---

## 3. Low variant — body decode + cell walk + writes

Structurally identical to §2 — only the three constants differ:

| Element                           | High (0x00574000)         | Low (0x00574C20)          |
|-----------------------------------|---------------------------|---------------------------|
| Inner 5×5 overlay band            | `[0xCD .. 0xE8]`          | `[0x4A .. 0x65]`          |
| Inner-scan dispatch target        | `DestroyBridgeFromCell_High` @ `0x005749C0` | `DestroyBridgeFromCell_Low` @ `0x00574780` |
| Ramp-walk tile-base subtracted from `cell+0x38` | `DAT_00AA0E28` | `DAT_00ABAD1C` |

Verified by side-by-side disassembly diff of `0x00574000`/`0x00574C20`:

- `0x00574C6C CMP EAX, 0x4A` and `0x00574C71 CMP EAX, 0x65` (vs High's
  `0xCD..0xE8`).
- `0x00574CAA CALL 0x00574780` (DestroyBridgeFromCell_Low, vs High's
  `0x005749C0`).
- `0x00575028 MOV EBX, dword ptr [0x00ABAD1C]` (vs High's
  `MOV EBX, dword ptr [0x00AA0E28]` at `0x0057440C`).

Every other instruction (including the `UpdateAdjacentBridges_High` call,
the `Tactical+0xD7C = 1` write, the `UpdateBridgeZonesHelper` tail, the
`IsLowBridgeEndpointTile` and `IsBridgeRampTile` probe addresses) is byte-for-byte
identical between the two functions. The compiler emitted them as two
independent copies — no shared helper, no inlining.

**Notably:** even the Low dispatcher calls `UpdateAdjacentBridges_High`
(`0x00576770`), not a hypothetical `_Low` sibling. This is consistent with
HIGH_BRIDGE_DAMAGE §11.14, which reports the same global write from the High
path. There is no separate "UpdateAdjacentBridges_Low" called from these
functions.

---

## 4. High vs Low diff — structural twin? (per HIGH_BRIDGE §12.14)

**Confirmed: structural twin.** HIGH_BRIDGE_DAMAGE §12.14's claim holds. The
two functions are textbook hand-duplicated / template-instantiated code with
only the three numeric constants differing (overlay band low bound + high
bound, sub-dispatcher target, tile-base global).

Implications for the Rust port:
- A single shared implementation parameterized by `(overlay_lo, overlay_hi,
  destroy_from_cell_fn, tile_base)` is correct and not a parity risk — the
  binary is just unfolded for performance / template instantiation.
- The "Low dispatcher calls High's UpdateAdjacentBridges" is **not** a bug
  in the original; it is the same call in both functions and produces the
  identical observable effect (the helper handles both kinds of adjacent
  bridges internally).

---

## 5. Helper graph

Helpers invoked directly by `DestroyBridge_{High,Low}_MapInit`:

| Address     | Function                                    | Role                                                            | Called by High | Called by Low |
|-------------|---------------------------------------------|-----------------------------------------------------------------|----------------|---------------|
| `0x005657A0` | `MapClass::Get_CellClass`                  | 5×5 inner-scan cell lookup                                       | yes            | yes           |
| `0x005749C0` | `MapClass::DestroyBridgeFromCell_High`     | Inner-scan dispatch target (High only)                           | yes            | no            |
| `0x00574780` | `MapClass::DestroyBridgeFromCell_Low`      | Inner-scan dispatch target (Low only)                            | no             | yes           |
| `0x0042FCB0` | `DynamicVectorClass<XCell*>::ctor`         | Stack-local accumulator vector (vftable `0x007E3890`)            | yes            | yes           |
| `0x005746C0` | `MapClass::IsBridgeRampTile`               | Ramp-search probe during forward walk                            | yes            | yes           |
| `0x00574600` | `MapClass::IsLowBridgeEndpointTile`        | Endpoint probe in the reverse-walk loop                          | yes            | yes           |
| `0x00587180` | `ApplyDamageToCell`                        | Per-cell destruction primitive (calls `DestroyBridge_Low/High` tile primitives, state machines, and `FUN_00487720` splash-damage) | yes (≤3×) | yes (≤3×) |
| `0x00576770` | `MapClass::UpdateAdjacentBridges_High`     | Re-evaluate ramp edges in neighborhood; dirties screen rect      | yes            | yes (sic — High helper)|
| `0x0056C510` | `MapClass::UpdateBridgeZonesHelper`        | Full zone-graph rebuild (unconditional at tail)                  | yes            | yes           |
| `0x007C8B3D` | `operator delete` (vector accumulator)     | DynamicVector dtor cleanup                                       | yes (conditional) | yes (conditional) |

**Not called** (despite the brief asking us to watch for them):

- `ToggleBridgePavement @ 0x0056E990` — not called by either body. (May be
  called downstream by `DestroyBridgeFromCell_*` or `CollapseBridge_*_*`;
  out of scope for this slot.)
- `SetBridgeDirection_NESW @ 0x0047E040` / `_NWSE @ 0x0047E470` — not called.
- `BlowUpBridge @ 0x0047DD70` — not called directly. (Downstream via the
  CollapseBridge walkers reachable from `DestroyBridgeFromCell_*`.)
- `FUN_00569760` (pavement walker, slot 3) — not called directly.
- `FUN_00586990` (cell-list dispatch, slot 3) — not called directly.
  (`ApplyDamageToCell @ 0x00587180` is the cell-list dispatcher these
  functions actually use.)
- Any `AnimClass` / `XSurface` constructor — none in the bodies themselves.

**Indirectly reachable via the call graph** (one level down via
`ApplyDamageToCell`):
- `DestroyBridge_High` / `DestroyBridge_Low` (tile primitives, distinct from
  the *_MapInit dispatchers; see ApplyDamageToCell at `0x005871E1` /
  `0x00587206` callsites).
- `ProcessBridgeDamageStateMachine_High` @ `0x00576BA0`,
  `ProcessBridgeDamageStateMachine_Low` @ `0x00571490` (anchor + body-cell
  state machines).
- `FUN_00487720` @ `0x00487720` — 5×5 splash-damage walker applied to the
  collapsed-cell list, uses warhead `*(RulesClass + 0xFA8)` (C4Warhead).
  This is the post-collapse occupant damage step.

---

## 6. Global side effects (Tactical, dirty-rect, deferred-redraw, accumulators)

Direct writes from the two bodies:

| Address          | Write                                | Site (High / Low)            | Effect                                                             |
|------------------|--------------------------------------|------------------------------|--------------------------------------------------------------------|
| `0x00ABDC74`     | sentinel "last bad coord"            | numerous (each bounds fallback) | Diagnostic; not a state mutation. Read by debug paths only.       |
| `[g_Tactical + 0xD7C]` (i.e. `0x008880A0`) | byte = 1                | `0x005745C3` / `0x005751DF`  | **Deferred terrain-rebuild flag.** Consumed by the tactical compose path next frame (full rebuild — see HIGH_BRIDGE §11.14). |

Indirect (via callees, in observable order):

1. `DestroyBridgeFromCell_{High,Low}` (inner-scan match path only) — mutates
   cell state, spawns debris animations, applies splash damage. Details out
   of scope for this slot.
2. `ApplyDamageToCell` (slow-path forward walk, up to 6 calls per dispatch:
   3 on the first ramp + 3 on the endpoint) — mutates per-cell `+0x11E` /
   `+0x44`, dispatches to state machines, walks attached-object lists with
   `FUN_00487720` to apply C4Warhead splash damage to occupants.
3. `UpdateAdjacentBridges_High` — re-evaluates ramp edges in the neighborhood
   and calls `TacticalClass::DirtyScreenRect` for the changed tiles
   (per BRIDGE_SYSTEM.md §"Helper Functions").
4. `UpdateBridgeZonesHelper` — full zone-graph rebuild for pathfinding.

No audio cue / EVA / `VocClass::Play` is emitted from either body directly.
Any sound for the collapse is generated downstream (anims and splash damage).

---

## 7. RNG usage + lockstep notes

**No call to `Random__RandomRanged @ 0x0065C7E0` in either body.** Verified by
full disassembly inspection of both `0x00574000..0x005745FB` and
`0x00574C20..0x00575217` — no CALL targets that address.

Any RNG-driven choices (e.g., `BridgeExplosions[rand_index]` debris animation
selection per HIGH_BRIDGE §11.4) happen inside
`DestroyBridgeFromCell_{High,Low}` and the `CollapseBridge_*_*` walkers, which
are reached via the inner-scan dispatch but are outside this slot's scope.

For lockstep correctness, this means the *_MapInit dispatchers themselves are
deterministic-by-construction; the RNG dependency lives one level down.

---

## 8. Active in YR — verdict per branch

| Branch                                  | Reachable in YR skirmish? | Evidence |
|-----------------------------------------|---------------------------|----------|
| Phase 1: 5×5 inner overlay match → `DestroyBridgeFromCell_*` | **Yes** | Both runtime callers pass a coord adjacent to a destroyed bridge hut; the hut sits next to body overlay cells. |
| Phase 2: 8-direction flag walk fallback | **Yes** | Fires when the hut's coord is not within 2 cells of body overlays (e.g., wide ramps); standard YR map layouts hit this. |
| Phase 3: Pure bridgehead branch (0x400 only) | **Yes** | Activates when the found cell is a bridgehead anchor without `0x100`. |
| Phase 3: Bridge-cell branch (`0x100` set) | **Yes** | Activates when found cell is a structural bridge cell; sub-branched on `0x80` for body-vs-peer anchor. |
| Phase 3: No-bridge return-early | **Yes** | Activates when the 8-dir walk found no `0x500` cells; observable as "hut destroyed but bridge unaffected." |
| Phase 4: Ramp-search forward walk | **Yes** | Standard path for in-bounds bridges; ramp tiles always present. |
| Phase 5: Global tail (`Tactical+0xD7C`, zone rebuild) | **Yes** | Always reaches via LAB_005745AD (slow path completes) or LAB_005745CA (early bounds-out from ramp walk). |

**No TS-only gating.** No `SpecialFlags`-conditional branches, no
`DestroyableBridges` check, no `Game::IsCampaign`-style guard appears in
either body. The hut-death dispatch fires unconditionally when its callers
fire (which themselves are gated by `BridgeRepairHut=yes` on the building
type — verified in BRIDGE_REPAIR_AND_HUT_DEATH §3).

---

## 9. Open Questions (deferred)

1. **`param_1+0x13C[cell_index]` read in Phase 4** (`0x005743D1`,
   `0x00574FED`) — what does this index? Likely the global `g_CellArray_Base`
   re-derived per `MapClass` instance, but the field name is uncited.
   Confirmed cited in HIGH_BRIDGE §"map bounds" indirectly via
   `param_1+0x124..+0x130`. Not parity-critical for this slot.
2. **Why does the slow path skip the dirty-flag write on the
   `LAB_005745CA` early-out?** This means the Phase-4 forward walk that
   exits "early but legitimately" (e.g., walked off the map edge) skips
   `Tactical+0xD7C = 1` but still calls `UpdateBridgeZonesHelper`. Observable
   effect: zones re-computed, but no full terrain rebuild scheduled. Is that
   a missing flag or intentional? Defer — likely intentional since
   `UpdateBridgeZonesHelper` triggers its own dirty as needed.
3. **`DynamicVectorClass` accumulator created by `FUN_0042FCB0(0,0)`** —
   constructed but never observably appended to by either body. Is it
   populated indirectly by callees through a pointer kept in `[ESP+0x10..]`?
   Not from the call signatures observed; appears to be a no-op leftover.
   Defer to slot 3's investigation of `FUN_00586990`.

---

## 10. Sources

**Verified via live Ghidra MCP this session (read-only):**

- `get_function_by_address`: `0x00574000`, `0x00574C20`, `0x005749C0`,
  `0x00574780`, `0x005746C0`, `0x00574600`, `0x00587180`, `0x00487720`,
  `0x00576770`, `0x0056C510`, `0x005657A0`, `0x0042FCB0`, `0x0065C7E0`.
- `get_function_callers`: `0x00574000`, `0x00574C20`.
- `get_function_xrefs`: `0x00574000`, `0x00574C20`.
- `decompile_function`: `0x00574000` (High body), `0x00574C20` (Low body),
  `0x00587180` (ApplyDamageToCell), `0x00487720` (per-cell splash walker),
  `0x0042FCB0` (DynamicVector ctor).
- `disassemble_function`: `0x00574000` (full, 510 bytes),
  `0x00574C20` (full, 510 bytes).
- `read_memory`: `0x00887324` (g_Tactical static base — uninit BSS),
  `0x0089F688` (g_DirectionOffsets — uninit BSS), `0x007E3890`,
  `0x007E38D0` (DynamicVector vftables).

**Cross-referenced docs:**
- `BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md` §3 (caller xrefs match),
  §6 (dispatcher table).
- `HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md` §11.14
  (`Tactical+0xD7C` deferred-redraw flag write — confirmed),
  §12.14 (structural twin claim — confirmed),
  §11.11 (BridgeRepairHut death → collapse semantics).
- `BRIDGE_SYSTEM.md` (cell field offsets `+0x140`, `+0x24`, `+0x2C`, `+0x44`).
