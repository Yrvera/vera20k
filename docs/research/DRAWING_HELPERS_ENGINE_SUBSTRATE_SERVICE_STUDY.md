# Drawing Helpers — Engine Substrate Service Study & Replacement-Boundary Design

**Status:** STUDY + DESIGN (not an approved implementation plan). Read-only research; no Rust written.
**Date:** 2026-06-04 (Pass 2 expansion 2026-06-04 — JOB A gates closed live)
**Rule:** Rust-native structure, gamemd-native **output** semantics. For this RENDER-LAYER family the contract is the **observable output** — two-pass draw ORDER, draw OFFSETS, layer/z-resolution, palette/remap RESULT, pip/bracket placement — **NOT** a port of the software blitter. gamemd uses a fixed software per-pixel z-buffer blitter; this engine uses a wgpu GPU depth pipeline (terrain writes depth, sprites passthrough, cliff redraw zdepth+Less). The behavior contract is therefore order + offset math + layer/z ordering; the boundary is a render-side draw-helper service (coord/offset computation + atlas/remap), explicitly not a blitter port.

**LAYERING INVARIANT RECONFIRMED (Pass 2).** `sim/` MUST NOT depend on `render/`. This draw-helper service is strictly downstream: it consumes a frozen sim snapshot and never feeds deterministic state. The contract is **OBSERVABLE order / offset / z parity**, NOT a software-blitter port — and crucially (newly proven live, see Pass 2 §P2.4) gamemd's normal SHP sprites do painter's-order draw with **NO per-pixel z-test**; sprite-vs-sprite occlusion is decided ENTIRELY by layer + X+Y insertion order, exactly the property a GPU painter's-order substitute reproduces. The GPU z-buffer in this engine is therefore only an internal mechanism for terrain/cliff occlusion; it must not be allowed to reorder sprites relative to the X+Y sort key.

**Bar:** active in a standard local skirmish. MP-only / SpecialFlags-gated / TS-legacy behavior is flagged DORMANT or LEGACY.

**Confidence posture / provenance.** Five load-bearing anchors were **re-decompiled / re-read live this session** and are cited inline as LIVE:
- Blitter z-loop body `FUN_00495A50` (LIVE: `decompile_function 0x00495A50`).
- Blitter vtable identity at `0x007E5618` (LIVE: `read_memory 0x007E5618` → bytes `60 a6 49 00` = `0x0049A660`; `decompile_function 0x0049A660` = `Blitter__Constructor` installing `vtable__Blitter` and `PTR_Blitter__Constructor_007e5618`). **LABEL DRIFT corrected** — see §0.
- `Tactical_ObjectRenderingLoop` `0x006D8DB0`, full two-pass body (LIVE: `decompile_function 0x006D8DB0`).
- `TechnoClass::GetFLH` `0x006F3AD0` (LIVE: `decompile_function 0x006F3AD0`).
- `BuildingClass::GetFLH` `0x00453840` (LIVE: `decompile_function 0x00453840`).

Everything else is **DOC-SOURCED** from the prior verified corpus and cited to the source doc inline (SELECTION_BRACKETS_PIPS_DRAW_ORDER, DRAW_ORDER_DEPTH_SYSTEM, TACTICAL_RENDER_PIPELINE, BRIDGE_RENDERING, FLH_TURRET_AND_VISUAL_OFFSETS, MINIMAP_GENERATED_PIXEL_COLOR_PIPELINE, BUILDING_DAMAGE_DESTRUCTION, LAYER_CLASS, PIXEL_FX_SPARKLES, TARGET_LINES). Default verdict for any unproven order/offset/remap difference is **DRIFT** — there is no internal-only escape hatch for render output.

**Companion:** the in-flight core-engine-substrate program (`docs/plans/2026-05-29-core-engine-substrate-todo.md`). This is a render-layer service and therefore sits ABOVE `sim/`; it consumes a sim snapshot and never feeds back into deterministic state. It slots into that program as the render-side consumer, not a parallel architecture.

---

## Table of Contents
- §0. Conflict resolutions / label drift (binary-adjudicated this session)
- §1. Verified active-YR responsibilities of the Drawing-helpers family
- §2. Full inventory (functions, vtable slots, globals, tables, legacy)
- §3. Active-YR vs inactive/legacy (TS) split
- §4. Comparison against the current Rust architecture
- §5. gamemd-native behavior contract (testable statements D1–D24)
- §6. Rust-native replacement boundary (the draw-helper service)
- §7. Old ad hoc Rust logic to retire/fold in
- §8. Migration slices (shadow-first) + acceptance tests
- §9. Sources & verification ledger

---

## 0. Conflict resolutions / label drift (binary-adjudicated this session)

| Contested claim | Inputs disagreed | Verdict (this session) | Evidence |
|---|---|---|---|
| Blitter at "vtable 0x7E5618" = `FUN_00495A50` | Task anchor framed `0x00495A50 @ vtable 0x7E5618` as a single fact | **TWO related facts — now bit-exactly bridged (Pass 2).** `0x007E5618` is the **base of the Blitter vtable object** (slot 0 = ctor `0x0049A660`). `0x00495A50` is **vtable slot +4** of that same vtable (`read_memory 0x007E5618` = `60a64900 505a4900` → DWORD[1] = `0x00495A50`). And the `Blitter_selector` dispatch-table member at `param_1 + 0xC0` is a heap blitter object whose vtable is `&PTR_Blitter__Constructor_007e5618` (LIVE `decompile_function 0x0048EBF0` Blitter_init: `*(param_1+0xc0) = obj with *obj = &PTR_..._007e5618`). So selecting `+0xC0` → an object whose blit-method (vtable+4) **is** `0x00495A50`. The "0xC0 blitter" and `FUN_00495A50` ARE the same z-tested intensity-remap path; they were never contradictory, only at different indirection levels. | LIVE `read_memory 0x007E5618` (slot0=`0x0049A660` ctor, slot+4=`0x00495A50`). LIVE `decompile_function 0x0049A660` (ctor installs `vtable__Blitter`). LIVE `decompile_function 0x00495A50` (z-loop). LIVE `decompile_function 0x0048EBF0` (`+0xC0` member ← vtable 0x007E5618). LIVE `get_xrefs_to 0x00495A50` → only `0x007E561C` (vtable slot, no direct callers — virtual-only). |
| **0xC0 remap-blitter selection rule** (GATE — was open) | Task: resolve 0xC0 (remap/translucent) vs plain z-blitter | **RESOLVED — VERIFIED.** `Blitter_selector 0x00490B90`: `+0xC0` returned iff `(flags & 0x10)` [z-enable] AND `(flags & 0x4000)` AND `(flags & 0x800)` [remap]. The plain z-blitter (`+0x14`) is `(0x10) && !(0x3000) && !(0x800)`. Flag source (LIVE `CC_Draw_Shape 0x004AED70`): `0x1`=shadow, `0x2/0x4/0x6`=25/50/75% trans, `0x10`=zbuffer, `0x200`=center, `0x800`=remap; **`if (param_7 != 0) flags |= 0x10`** — the z-depth arg forces the z-enable bit on. `_g_BlitterFlagMask_0x3000` masks `0x3000`. | LIVE `decompile_function 0x00490B90` (full dispatch tree). LIVE `decompile_function 0x004AED70` (flag bits + `param_7→0x10`). |
| **Building `GetRenderCoords -0x80` shift** (GATE — was open) | Task: confirm `-0x80` exact, which frame | **RESOLVED — VERIFIED. X AND Y each `-0x80` LEPTONS; Z untouched.** `BuildingClass__GetRenderCoords 0x00459EF0`: `out.X = coord(+0x9c) - 0x80; out.Y = coord(+0xa0) - 0x80; out.Z = coord(+0xa4)`. The +0x9c/0xa0/0xa4 are the object's world lepton coords, so `-0x80` = half a cell (256 leptons/cell) in leptons, NOT pixels. This feeds the sort end-to-end: `ObjectClass__GetYSort 0x005F6BD0` calls vtable `+0xAC` (GetRenderCoords) twice and sums `X+Y`, so the `-0x80,-0x80` shift contributes `-0x100` to every building's Layer-2 sort key. | LIVE `decompile_function 0x00459EF0`. LIVE `decompile_function 0x005F6BD0` (calls `+0xAC`, sums X+Y). |
| **Layer-2 / Air-layer insert tie-break** (GATE — was open) | Task: 2nd-pass sort/insert tie-break at equal depth | **RESOLVED — VERIFIED. (a) ONLY layer 2 sorts; layers 0/1/3/4 append unsorted.** `DisplayClass__Submit_Object 0x004A9720` calls `DynamicVector__Insert(obj, sorted = (InWhichLayer()==2))`. **(b) Equal-key tie-break = FIFO/insertion order.** `DynamicVector__SortedInsert 0x00551A90` (disassembled) walks from index 0, calls `ObjectClass__YSortComparator(this=existing[i], arg=new) 0x005F6220` = `GetYSort(new) < GetYSort(existing)` (strict `<`), breaks at the FIRST strictly-greater existing key → new inserts AFTER all equal-or-lower keys. **(c) The 2nd pass does NOT re-sort** — `0x006D8DB0` second `do`-loop replays each layer buffer by index in stored order. So overlapping ELEVATED (Air, layer 3) objects draw in pure submission order with NO depth tie-break at all. | LIVE `decompile_function 0x004A9720` (`sorted = layer==2`). LIVE `decompile_function 0x005519C0` (`param_3 ? SortedInsert : append`). LIVE `disassemble_function 0x00551A90` (insert loop, FIFO). LIVE `decompile_function 0x005F6220` (strict `<`). LIVE `decompile_function 0x006D8DB0` (2nd loop, no re-sort). |
| Blitter z-test compare op | (none) | **`<` (strict Less)** in the inner loop: `if (param_5 < zbuffer_pixel && sprite_pixel != 0) { write remap; write z }`. Matches TMP tile blitter Less compare per BRIDGE_RENDERING §17. | LIVE `0x00495A50`: `if ((param_5 < (int)(uint)*puVar4) && (... *param_3 != 0))`. |
| `ObjectRenderingLoop` address | DRAW_ORDER_DEPTH had stale `0x006d8d50`; corrected to `0x006d8db0` (2026-05-29 GHIDRA_ADDRESS_SHIFT note) | **`0x006D8DB0` confirmed live**; full body decompiles as the two-pass loop. | LIVE `decompile_function 0x006D8DB0`. |
| GetYSort uses X+Y only (elevation-independent) | DRAW_ORDER_DEPTH says GetYSort = `X+Y`; Rust sorts by **screen_y + z·HEIGHT_STEP** | **DRIFT (see §5 D8 / §7).** gamemd sort key is lepton `X+Y`, Z excluded. Rust folds elevation INTO the sort key. Not adjudicated to equivalent — flagged DRIFT. | DRAW_ORDER_DEPTH_SYSTEM §3/§9 (doc); Rust `src/app_instances/helpers.rs:compute_sprite_depth_params` (`iso_row = screen_y + z*HEIGHT_STEP`). |

---

## 1. Verified active-YR responsibilities of the Drawing-helpers family

This family is the **shared draw-primitive layer**: it owns *where* a sprite lands on screen, *in what order* sprites paint, *how depth resolves*, *how palette/remap is applied*, and *where decorations (pips/brackets/FLH-anchored effects) attach*. It is the common substrate beneath every per-class `DrawIt`.

| # | Responsibility (what it owns) | Active-YR | Evidence |
|---|---|---|---|
| R1 | **Two-pass object render** over 5 display layers: Loop 1 draws all sprites (clears `+0x99` WasDrawn, projects coords, clips, sets WasDrawn, calls pre-draw `+0x10C` then `+0x104` DrawIt, shadow `+0x110` for foot classes), Loop 2 draws all extras (`+0x110` DrawExtras) on top. | VERIFIED LIVE | `0x006D8DB0` (two `do` loops over `g_DisplayLayers`; `(*+0x10c)`, `(*+0x104)`, `(*+0x110)`). |
| R2 | **Layer + Y-sort resolution.** 5 layers (Underground/Surface/Ground/Air/Top); only Layer 2 (Ground) is sorted on insert by `GetYSort = renderX + renderY` (leptons); others append unsorted. | VERIFIED (doc) | DRAW_ORDER_DEPTH_SYSTEM §2/§3; LAYER_CLASS §7. |
| R3 | **Building turret / garrison-fire pass** after Layer 2: iterate `g_BuildingClass_Array` in registration order, draw turret/garrison overlay on top of all Layer-2 bodies. | VERIFIED LIVE | `0x006D8DB0` `if (local_d4 == 0x2) { ... BuildingClass__UpdateGarrisonFire }`. |
| R4 | **Per-pixel z-resolution blitter** (the SHP/TMP write loop): `if (z_depth < zbuffer && pixel!=0) { write remapped pixel; write z }`. The remap is a palette-index → 16-bit lookup through a per-blitter intensity table. | VERIFIED LIVE | `0x00495A50` inner loop. |
| R5 | **Screen-coord computation** for objects: `CoordsToClient` / `WorldToScreenSub` lepton→screen, minus viewport, with `AdjustForZ` height lift applied to screen-Y only; `168×180`-pixel clip padding (`-0x169..+0x168` X, `-0xB5..+0xB4` Y). | VERIFIED LIVE | `0x006D8DB0` (`CoordsToClient`, `Tactical__AdjustForZ`, clip constants `DAT_00b0ce30+0x168`, `DAT_00b0ce34+0xb4`). |
| R6 | **FLH / fire-origin / turret-offset computation**: 32-way facing-quantized matrix transform of the FLH triplet (leptons), added to `GetRenderCoords`, with `CurrentBurstIndex` lateral sign-flip; building override adds fixed `PrimaryFirePixelOffset` (iso-pixel→world) or `GetTurretDrawPosition`. | VERIFIED LIVE | `0x006F3AD0`, `0x00453840`. |
| R7 | **Decoration placement** (DrawExtras 9-step): Ivan-bomb clock, deploy-wrench, veterancy chevron, selection brackets, alliance hook, health pips, hover pips, talk bubble — with exact per-class offsets and z-orders. | VERIFIED (doc) | SELECTION_BRACKETS_PIPS_DRAW_ORDER §3. |
| R8 | **DamageFireAnims attach points**: per-BuildingType iso-pixel offset table (`Type+0x15D8..0x1618`, stride 8) drives where fire anims spawn on a damaged building. | VERIFIED (doc) | BUILDING_DAMAGE_DESTRUCTION §8. |
| R9 | **BounceClass quaternion → 3×4 matrix** (`FUN_004399E0`) used for tumbling debris / thrown-unit visual transforms. | DOC-NOTED (not re-read) | task anchor; not load-bearing for skirmish parity (see §3). |
| R10 | **Minimap generated-pixel pipeline**: raw RGB terrain buffer → 16-bit packed secondary surface → primary surface with object dots, shroud (writes literal pixel `0`), fog dim (channel `>>1`, conditional), viewport rect; dirty-rect driven. | VERIFIED (doc) | MINIMAP_GENERATED_PIXEL_COLOR_PIPELINE §1–§5. |
| R11 | **PixelFX sparkles**: per-frame 1-pixel twinkles over visible water/ore, drawn between the object pass and UI pass. | VERIFIED (doc) | PIXEL_FX_SPARKLES (`DrawPixelFXSparkles 0x006D7840`). |

---

## 2. Full inventory

### 2a. Core render-loop & blitter functions

| Name | Address | Role | Active-YR | Evidence |
|---|---|---|---|---|
| `TacticalClass::Draw` | `0x006D3D10` | 3× per frame (Pass 0 scroll/buffer, Pass 1 terrain, Pass 2 objects); sidebar/UI composited between Pass 1 and Pass 2 | YES | TACTICAL_RENDER_PIPELINE; DRAW_ORDER_DEPTH §1 |
| `RenderFrame_main` | `0x004F4480` | Frame driver; calls TacticalClass::Draw three times | YES | SELECTION_BRACKETS §1 |
| `Tactical_ObjectRenderingLoop` | `0x006D8DB0` | The two-pass 5-layer object renderer | YES | LIVE `0x006D8DB0` |
| Blitter z-loop method (vtable 0x7E5618 slot+4) | `0x00495A50` | Per-pixel `z<zbuf && px!=0` intensity-**remap**-write blitter; the blit-method of the `+0xC0` dispatch-table entry. Reached for terrain/bridge-body z paths, NOT normal SHP objects (which use `0x800` no-z blitters — see Pass 2 §P2.4). | YES (bridge/special) | LIVE `0x00495A50`; LIVE `get_xrefs_to 0x00495A50` (virtual-only) |
| `Blitter_Opaque_RLE_Remap` (vtable 0x7E5470, member +0x124) | `0x004978C0` | The blitter ALL normal opaque SHP objects use (`0x800` set, visual-state 0): RLE intensity-remap, reads `g_ABuffer`, **no z-read/no z-write**. Sprite occlusion is painter's-order only. | YES (all normal sprites) | LIVE `decompile_function 0x004978C0` (no g_ZBuffer touch); ZBUFFER_DEPTH_SYSTEM |
| `Blitter__Constructor` | `0x0049A660` | Installs `vtable__Blitter` (`PTR..._007e5618`); blitter family object ctor (vtable slot 0) | YES | LIVE `0x0049A660` |
| `Blitter_init` | `0x0048EBF0` | Lazily allocates the ~90 blitter-variant member objects into the dispatch table (`param_1+0x08..+0x168`) that `Blitter_selector` indexes; `+0xC0` ← vtable 0x007E5618 (= `0x00495A50` blit). Two init branches (`*(p+4)==1` reduced set vs full set). | YES | LIVE `decompile_function 0x0048EBF0` |
| `CC_Draw_Shape` | `0x004AED70` | Core SHP draw entry: centering (flag `0x200` → `-w/2,-h/2`), SHP frame-rect offset add, `param_7`=z-depth → **forces flag `0x10`** if nonzero; selects blitter via `Blitter_selector 0x00490B90`. Flag bits: `0x1`=shadow, `0x2/0x4/0x6`=25/50/75% trans, `0x10`=z, `0x200`=center, `0x800`=remap. | YES | LIVE `decompile_function 0x004AED70` |
| `Blitter_selector` | `0x00490B90` | Chooses a dispatch-table member by draw flags. **`+0xC0` (z+remap) iff `(0x10)&(0x4000)&(0x800)`; plain z `+0x14` iff `(0x10)&!(0x3000)&!(0x800)`.** Lazy-inits via `Blitter_init` when `param_1+8==0`. | YES | LIVE `decompile_function 0x00490B90` |
| `TMP_TileBlitter` | `0x00547CF0` | Per-pixel terrain tile blit with Z R+W (`pixel_z<=zbuf` write) | YES | BRIDGE_RENDERING §17 |
| `CoordsToClient` / `CoordsToClient2` | (via loop) | lepton→screen; `…2` is the bounds-culling variant | YES | LIVE `0x006D8DB0`; SELECTION_BRACKETS §3.1 |
| `Tactical__WorldToScreenSub` | (via loop) | lepton→screen sub used by building/turret path | YES | LIVE `0x006D8DB0` |
| `Tactical__AdjustForZ` | (via loop) | Z lepton → screen-Y lift (`≈ ftol(z*0.14348 + (z≥728?1:0) + 0.5)`); **screen position only, not sort** | YES | LIVE `0x006D8DB0`; FLH report Rust note; DRAW_ORDER_DEPTH §9 |
| `IsometricPixelToWorld` | `0x006D2070` | iso-pixel pair → world X/Y (Matrix3x4 transform + ftol; Z untouched) | YES | FLH report §3 |
| `Matrix3x4_TransformPoint` | (via loop/GetFLH) | core 3×4 matrix apply | YES | LIVE `0x006D8DB0`, `0x006F3AD0` |

### 2b. Per-object draw vtable slots (TechnoClass base; verified via 6 vtable memory reads in SELECTION_BRACKETS)

| Slot | Address (base) | Function | Role | Active-YR |
|---|---|---|---|---|
| `+0x2C` | — | `WhatAmI` (RTTI) | branch selector in loop | YES |
| `+0x48` | — | `GetCoords` | world coords | YES |
| `+0x4C` | — | building coord/footprint accessor (fog gate path) | YES |
| `+0x78` | — | `InWhichLayer` | returns 0..4 (buildings/foot=2) | YES |
| `+0xAC` | — | `GetRenderCoords` | render anchor (FLH adds to this) | YES |
| `+0xB8` | `0x005F6BD0` (base) | `GetYSort` → `renderY+renderX` (Z **never read**) | Layer-2 sort key | YES |
| `+0xB8` | `0x00449410` | **BuildingClass::GetYSort override** — base `X+Y` `+ (Type+0x16c5 ? +0x20 : 0) - (Type+0x16b7 ? +0x10 : 0)` | Layer-2 sort key (building bias) | YES |
| `+0xB8` | `0x00422BC0` | **AnimClass::GetYSortWithAdjust override** (Ghidra label drift: stored as `AnimClass__GetRenderColor`) — base `X+Y` `+ Anim+0x104 (YSortAdjust)` | Layer-2 sort key (anim bias) | YES (attached/scripted anims) |
| `+0xC8` | `0x0041C020` | `IsDisguised` | veterancy gate | YES |
| `+0x104` | `0x005F4B10` (base) | **`DrawIt`** | main sprite blit (overridden per subclass) | YES |
| `+0x10C` | `0x006F60D0` | **pre-draw**: `DrawBehind` (buildings, back bracket edges) / `SetDrawCoords` (foot) | YES |
| `+0x110` | `0x006F5190` | **`DrawExtras`** (buildings) / **`DrawShadow`** (foot) — shared slot, per-subclass semantics | YES |
| `+0x130` | `0x0041BE80` | radial-indicator hook (empty stub on base) | LEGACY/no-op |
| `+0x438` | `0x004DC060` | `DrawActionLines` (target line) | YES (separate system) |
| `+0x448` | `0x006F60C0` | alliance-pip hook (empty stub) | LEGACY/no-op |
| `+0x44C` | `0x006F64A0` | `DrawHealthBar` (all classes) | YES |
| `+0x450` | `0x00709A90` | `DrawPipScalePips` (cargo/ammo/tib/occupant/self-heal/group#) | YES |
| `+0x454` | `0x0070A990` | `DrawVeterancyPips` (rank chevron) | YES |
| `+0x458` | `0x0070AA60` | `DrawExtraInfo` (house-color text label) | YES |
| `+0x118` | `0x005F65D0` | DrawVeterancyPips stub-chain (→`+0x114`) | LEGACY (TS hook, dead) |
| `+0x1C8` | — | `GetHeight` (side-effect read in bracket path) | YES |

### 2c. FLH / fire-origin / turret functions & type fields

| Name | Address / offset | Role | Active-YR | Evidence |
|---|---|---|---|---|
| `TechnoClass::GetFLH` | `0x006F3AD0` | 32-way facing matrix transform of FLH triplet → world coord + GetRenderCoords; burst lateral sign flip | YES | LIVE `0x006F3AD0` |
| `BuildingClass::GetFLH` | `0x00453840` | override: garrison port path / `0xFFFF` sentinel / `GetTurretDrawPosition` / fixed pixel offset add | YES | LIVE `0x00453840` |
| `GetTurretDrawPosition` | `0x00453BF0` | building voxel turret/barrel fire origin via VXL matrix | YES | FLH report §3 |
| `TechnoClass::GetWeapon` | `0x0070E140` | elite/normal FLH slot selection | YES | FLH report §3 |
| `BuildingClass::GetRenderCoords` | `0x00459EF0` | world coords with **X,Y each `-0x80` LEPTONS** (= -0.5 cell; 256 lept/cell); **Z untouched**. Feeds Layer-2 sort (GetYSort sums shifted X+Y). VERIFIED live this run. | YES | LIVE `decompile_function 0x00459EF0` |
| FLH 32-way quant constant `_DAT_007e4408` | `0x007E4408` | double **-π/16 = -0.19634954…** (`read_memory` = `18 2d 44 54 fb 21 c9 bf`); the per-bucket angle of the 32-way (`& 0x1F`) facing quantization in GetFLH. VERIFIED live. | YES | LIVE `read_memory 0x007E4408` |
| Normal FLH slot | `Type+0x898 + idx*0x1C +4/8/0xC` | leptons | YES | FLH report §2 |
| Elite FLH slot | `Type+0xA94 + idx*0x1C +4/8/0xC` | leptons | YES | FLH report §2 |
| `PrimaryFirePixelOffset` | `Type+0xE44/+0xE48` (sentinel `0xFFFF`) | iso pixels | YES | LIVE `0x00453840` |
| Garrison port offsets | `Type+0x1588 + firePort*8` | iso pixels | YES | LIVE `0x00453840` |
| Building anim X/Y/ZAdjust/YSort | per-slot `Type+…0xF84/0xF88`→`Anim+0x100/0x104`; e.g. ActiveAnim slot `+0x1048..+0x1054` | screen px + draw metadata | YES | FLH report §3/§4 |
| DamageFireAnims points | `Type+0x15D8..0x1618`, stride 8 (X,Y iso px) | YES | BUILDING_DAMAGE_DESTRUCTION §8 |

### 2d. DrawExtras decoration assets / globals (DOC-sourced)

| Asset/global | Address | Role | Active-YR |
|---|---|---|---|
| `PIPBRD.SHP` ptr | `0x00AC1478` | health-bar bracket background | YES |
| `PIPS.SHP` ptr | `0x00AC147C` | pips (health, veterancy 14/15/19, occupant 6..12, self-heal 13/20) | YES |
| `PIPS2.SHP` ptr | `0x00AC1480` | tiberium/ammo pips | YES |
| `TALKBUBL.SHP` ptr | `0x00AC1484` | talk bubble (scripted only) | YES (scripted) |
| Foundation width/height tables | `0x008192B8` / `0x00819310` | bracket diamond geometry | YES |
| group-digit string | `0x0081B3D0` `"1234567890"` | group-number overlay | YES |
| talk-bubble target / counter | `0x00B0EB38` / `0x00B0EB3C` | single active bubble | YES (scripted) |

### 2e. Display-layer & draw-order globals (DOC-sourced)

| Global | Address | Role |
|---|---|---|
| `g_DisplayLayers` | `0x008A0360` | 5× DynamicVector, 0x18 stride |
| flat-anim layer | `0x008A0390` | terrain-pass flat anims |
| `g_BuildingClass_Array` | (in loop) | turret pass iteration order |
| `g_ZBuffer` | `0x00887644` | 16-bit depth CircBuf |
| `g_ABuffer` | `0x0087E8A4` | 16-bit shroud/fog alpha CircBuf |
| `DAT_00b0ce30/34` | viewport extents | clip-padding bounds |
| `g_ScenarioClass & 0x1000` | SpecialFlags | fog-of-war darkening gate (TS, off by default) |

### 2f. Legacy / dormant TS paths in this family

| Path | Address/flag | Why dormant in YR |
|---|---|---|
| Fog-of-war "previously seen, dimmed" object cull in Loop 1/Loop 2 | `(*g_ScenarioClass_Instance & 0x1000)` gate (LIVE in `0x006D8DB0`) + minimap fog channel `>>1` | `FogOfWar=no` default in YR; the `0x1000` SpecialFlag is normally clear → branch not taken. Implement only black shroud, not darkening. |
| `vtable+0x118` DrawVeterancyPips stub-chain | `0x005F65D0` → `+0x114` empty | dead TS veterancy slot; live call is `+0x454` |
| `vtable+0x130` radial-indicator dispatch | empty stub `0x0041BE80` | no stock YR override |
| `vtable+0x448` alliance-pip hook | empty stub `0x006F60C0` | no stock YR override |
| `FUN_004D1890` FoggedObject snapshot walker | `0x004D1890` | TS fogged-object cache; dormant (`FogOfWar=no`) |
| Subterranean Layer-0 (Underground) object draw | layer index 0 | tunnel/subterranean is TS-only; not in YR (`feedback_no_tunnel_subterranean`) |
| `BounceClass` quaternion→matrix | `FUN_004399E0` | thrown-unit tumble; rare/scripted; not skirmish-load-bearing |

---

## 3. Active-YR vs inactive/legacy split (explicit two-list)

**ACTIVE-YR (must reproduce to last detail):**
- Two-pass object loop over layers 0..4; clear `+0x99`, set on visible, Loop 2 only on `WasDrawn==1`.
- Loop-1 call order per visible object: pre-draw `+0x10C` → (foot only: shadow `+0x110`) → DrawIt `+0x104`.
- Building turret/garrison pass runs once, **after layer 2**, in `g_BuildingClass_Array` order (NOT y-sorted).
- Layer-2 Y-sort by lepton `renderX+renderY`; ties = insertion order; **Z excluded**. (VERIFIED live: `DisplayClass__Submit_Object 0x004A9720` sets sorted=`layer==2` ONLY.)
- **Layers 0/1/3/4 are NOT sorted — pure submission/append order, no depth key.** Overlapping elevated Air-layer (3) objects (aircraft, in-air projectiles, jumpjets) draw in insertion order with NO tie-break. A Rust port that depth-sorts the Air/particle layer DRIFTS. (VERIFIED `0x004A9720` + `0x005519C0`.)
- Layer paint order Underground→Surface→Ground→Air→Top. The second (DrawExtras) pass replays the SAME per-layer buffer order — it does not re-sort. (VERIFIED `0x006D8DB0` 2nd loop.)
- Normal opaque SHP objects draw via the `0x800`/no-z blitter (`Blitter_Opaque_RLE_Remap 0x004978C0`): **no per-pixel z-test** — occlusion is layer + X+Y order only. The z-tested remap blitter (`0xC0`/`0x00495A50`) is for terrain/bridge-body, not units. (VERIFIED `0x004978C0`.)
- DrawExtras 9-step intra-order (bomb→wrench→veterancy→brackets→alliance-hook→pips→hover-pips→talkbubble) with z-orders `0xE00` (high group) and `0x600` (pip group); same-z resolved by call order.
- Per-class pip layout: building NW-edge isometric pips (count `(fh×15)/2`), infantry 8 pips frame-1 PIPBRD, unit/aircraft 17 pips frame-0 PIPBRD; `PixelSelectionBracketDelta` Y shift.
- Pip color = **health ratio only** (ConditionYellow `0.5`, ConditionRed `0.25`), never armor.
- Building line brackets only (units/inf use PIPBRD); back edges drawn in DrawBehind (Loop 1, behind sprite), front edges in DrawExtras (Loop 2, on top); bracket color palette `0xF`, dim `0xC` when `GetHeight()<-4`.
- Blitter z-resolution `<` compare with palette-remap write; CC_Draw_Shape centering (`0x200`), frame-rect offset, `param_7` z-depth.
- FLH 32-way quantized world-coord source + burst lateral sign flip; building fixed iso-pixel offset → world; turret draw position for voxel-turret buildings.
- Minimap generated-pixel pipeline (RGB→16-bit pack, dot dirty-rects, shroud literal 0).
- PixelFX sparkles between object and UI pass.
- DamageFireAnims iso-pixel attach table.

**INACTIVE / LEGACY (do NOT design around):**
- Fog-of-war object darkening (`SpecialFlags 0x1000`), minimap fog channel `>>1`, FoggedObject walker — all off by default in YR; only black shroud is active.
- `vtable+0x118`/`+0x130`/`+0x448` empty stubs.
- Subterranean Layer-0 object draws (tunnel = TS-only).
- Talk bubble + group-number overlay are scripted/hotkey UI — present in YR but invisible in a default skirmish until used; implement, but not parity-blocking.
- BounceClass quaternion transform (rare tumble visual).

---

## 4. Comparison against the current Rust architecture

The Rust render path is **app-layer**, not a service: per-frame instance builders in `src/app_render/build_instances.rs` produce `Vec<SpriteInstance>` per draw class, then `src/app_render/draw_passes.rs` issues a hand-ordered list of ~40 draw calls, with `src/app_render/merge_passes.rs` doing a CPU multi-way Y-merge across atlas textures. Draw-helper math is scattered across `src/util/flh_transform.rs`, `src/app_instances/helpers.rs`, `src/app_selection_brackets.rs`, `src/render/selection_overlay.rs`, `src/app_ui_overlays.rs`.

| Area | gamemd | Rust today | Verdict |
|---|---|---|---|
| Two-pass object loop | Loop1 sprites → Loop2 extras, per object | **Approximated as phase buffers**: `dispatch_draw_passes` does back-bracket pass → `selection_brackets_front_first` → merged object pass → … → final `selection_brackets_front` in DrawExtras-equivalent UI block. Per-object interleave is replaced by per-class phase ordering. | PARTIAL / DRIFT-risk (§5 D1) |
| Layer-2 Y-sort key | lepton `renderX+renderY`, **Z excluded** | `compute_sprite_depth_params`: `iso_row = screen_y + z·HEIGHT_STEP` → **Z folded into key** | **DRIFT** (§0, §5 D8, §7) |
| Layers 0..4 paint order | 5 explicit layers | Implicit: bridge-occluded pass, then unified Ground merge, then turrets (Step 6), then particles (Step 7.6 as "Layer 3"). No Underground/Surface/Top separation. | PARTIAL |
| Turret pass after layer 2 | yes, building-array order | Step 6 `building_turret` after the Ground merge | MATCHES (order); registration-order tie-break UNCHECKED |
| Blitter / z-resolution | software per-pixel `z<zbuf` remap | GPU: terrain writes depth, sprites passthrough, cliff redraw zdepth+Less. **Sprite-vs-sprite has no z-test** — pure painter's order. | INTENTIONAL substitution; output equivalence depends on sort key (so D8 DRIFT propagates) |
| Palette remap | per-blitter intensity table + house ramp | `PaletteSet` house_ramp + voxel byte→RGB shader; SHP path RGBA-baked | Substituted; result-equivalence UNCHECKED for intensity/remap blitters (bridge body, shadow) |
| FLH source | **world coord**, 32-way, burst flip | `flh_transform.rs` produces **screen-space f32 offset**; `_32way` variant exists; world-offset variant exists but `app_fire_effects` keeps origin at entity pos | PARTIAL / DRIFT (§5 D6) |
| Building fixed fire pixel offset | iso-pixel→world add | **missing** (per FLH report §6) | MISSING |
| DrawExtras 9-step | full | brackets, building/unit health pips, occupant pips, cargo(tib) pips implemented; **veterancy chevron, wrench, ivan clock, talk bubble, ammo/PipWrap, self-heal flash, group-number, DrawExtraInfo missing** | PARTIAL (SELECTION_BRACKETS §12) |
| `PixelSelectionBracketDelta` | applied | parsed, **not applied** to non-building Y | DRIFT |
| Bracket dim color `<-4` | conditional | not implemented | MISSING (rare) |
| Minimap pixel pipeline | RGB→pack→dots/shroud/fog | `src/render/minimap.rs` generates dots + terrain; fog dim path UNCHECKED for `>>1` parity | PARTIAL |
| PixelFX sparkles | between object & UI | Step 5.5, `pixel_fx_sparkles.rs` | MATCHES (position) |
| AdjustForZ | screen-Y lift only | `flh_transform::adjust_for_z_leptons` matches formula; folded into depth too (see D8) | math MATCHES; misuse in sort = DRIFT |

**Where logic is scattered ad hoc:** draw ordering lives as a literal call sequence in `draw_passes.rs`; depth/sort math in `app_instances/helpers.rs`; bracket geometry split between `app_selection_brackets.rs` + `render/selection_overlay.rs`; pip placement in `app_ui_overlays.rs`; FLH in `util/flh_transform.rs`. There is **no single "draw-helper service"** owning coord/offset/order/remap — it is spread across the app layer with per-class duplication.

---

## 5. gamemd-native behavior contract (testable statements)

Any Rust render replacement must reproduce these **observable outputs**. Each is a parity assertion.

**Draw order / two-pass:**
- **D1.** Within one display layer, EVERY sprite body paints before ANY DrawExtras decoration of that same layer. (Brackets/pips of object A never sit under object B's body unless B is in a higher layer.) (LIVE `0x006D8DB0`: Loop2 strictly after Loop1.)
- **D2.** Loop-1 per-object call order is pre-draw `+0x10C` → DrawIt `+0x104`; for foot classes (flag bit0 set, bit2 clear) shadow `+0x110` fires between SetDrawCoords and DrawIt — shadow paints before its own body. (LIVE `0x006D8DB0`.)
- **D3.** Building back-bracket edges (DrawBehind) paint in Loop 1 *before* the building sprite; front edges + pips paint in Loop 2 *after* all sprites. (SELECTION_BRACKETS §4.4.)
- **D4.** Building turret/garrison-fire overlay paints once, AFTER all Layer-2 bodies, in `g_BuildingClass_Array` registration order (NOT y-sorted). (LIVE `0x006D8DB0`.)
- **D5.** Layer paint order is fixed: 0 Underground → 1 Surface → 2 Ground → 3 Air → 4 Top. Within a layer ≠ 2, draw order = insertion order (unsorted). (DRAW_ORDER_DEPTH §2.)

**FLH / offsets:**
- **D6.** Fire origin = `GetRenderCoords + Matrix(type+0x720 X-translate, RotateZ(quantAngle), translate(flhZ, sign·flhY, 0))`, where `quantAngle = (((facing>>10)+1>>1 & 0x1F)-8)·(π/16)` and `sign` flips by `CurrentBurstIndex` LSB. Output is a **world CoordStruct**, not a screen offset. (LIVE `0x006F3AD0`.)
- **D7.** Building fire origin: if garrison fire active → `Type+0x1588+port*8` iso-pixel→world + GetRenderCoords; elif both pixel offsets `0xFFFF` → GetTurretDrawPosition (voxel turret) or generic GetFLH (+`Type+0x11E0/4` if `+0x16C5`); elif `PrimaryFireDualOffset` → pixel→world + generic GetFLH; else fixed pixel→world + GetRenderCoords. (LIVE `0x00453840`.)

**Depth / sort:**
- **D8.** Layer-2 sort key = lepton `renderX + renderY`, **Z elevation excluded** (base `ObjectClass::GetYSort` reads only render-coord +0 and +4, never +8/Z — verified `decompile_function 0x005F6BD0` this review: `return *(iVar1+4) + *piVar2`); ties resolve by insertion order; **no per-id/class secondary tie-break in the comparator**. Elevation affects screen-Y position (via AdjustForZ) but NOT sort. (DRAW_ORDER_DEPTH §3/§9.) **Current Rust folds Z into the key → DRIFT.**
  - **D8b (sort applies to layer 2 ONLY — VERIFIED live this run).** `DisplayClass__Submit_Object 0x004A9720` inserts with `sorted = (InWhichLayer()==2)`; layers 0/1/3/4 take the unsorted append path (`DynamicVector__Insert 0x005519C0`: `param_3==0 → tail append`). Therefore **Air layer (3) and Top (4) have NO depth sort at all** — overlapping elevated objects (aircraft, in-flight missiles, jumpjet infantry) paint in submission order. Any Rust depth-sort of the Air/particle layer is DRIFT. The two-pass loop's SECOND (DrawExtras) pass does NOT re-sort — it walks each layer buffer by index in stored order (`decompile_function 0x006D8DB0` 2nd `do`-loop). So the "equal-depth tie-break in the second pass" is simply the first pass's buffer order replayed.
  - **D8c (equal-key tie-break = stable FIFO — VERIFIED at assembly this run).** `DynamicVector__SortedInsert 0x00551A90` (disassembled) walks from index 0; `ObjectClass__YSortComparator 0x005F6220` = `GetYSort(new) < GetYSort(existing)` with **strict `<`**; the loop breaks at the first existing element whose key is strictly greater, inserting the new element AFTER all equal-key elements. Equal X+Y therefore preserves insertion order with no secondary key. Confirms D8 "ties = insertion order" at the byte level and refutes any positional/id tie-break.
  - **D8a (per-class key bias — NOT "no secondary key").** Two YR-active classes OVERRIDE `+0xB8` and add a constant bias on top of the base `X+Y`: `BuildingClass::GetYSort 0x00449410` adds `+0x20` when `Type+0x16c5` set and `-0x10` when `Type+0x16b7` set (verified `decompile_function 0x00449410` this review); `AnimClass::GetYSortWithAdjust 0x00422BC0` (Ghidra-mislabeled `AnimClass__GetRenderColor`) adds the per-Anim `+0x104` YSortAdjust (verified `decompile_function 0x00422BC0` this review). So building-vs-unit and attached-anim-vs-owner draw order at the SAME `X+Y` is decided by these biases, not by insertion order alone. The Rust `y_sort_key` in §6 MUST take a per-class bias term, or building/anim layering will drift. (Corroborated: PERCLASS_VTABLE_B8_YSORT_OVERRIDE_CENSUS; LAYER_CLASS §3.10.)
- **D9.** AdjustForZ screen lift ≈ `ftol(z·0.14348 + (z≥728?1:0) + 0.5)` applied to screen-Y only. (FLH Rust note; DRAW_ORDER_DEPTH §9.)
- **D10.** Object clip padding: keep if screen-X in `(-0x169, DAT+0x168]` and screen-Y in `(-0xB5, DAT+0xB4]` (≈168×180 px halo). (LIVE `0x006D8DB0`.)

**Blitter / remap (OUTPUT, not mechanism):**
- **D11.** Sprite pixel writes iff source index ≠ 0 (index 0 = transparent). **For NORMAL opaque SHP objects (the overwhelming majority) the blitter is `Blitter_Opaque_RLE_Remap 0x004978C0` (selected with `0x800`, visual-state 0): it reads `g_ABuffer` for shroud/alpha but performs NO z-read and NO z-write — sprite-vs-sprite occlusion is painter's order only.** The z-tested write (`z_depth < zbuffer` then store z) applies only to the z-remap blitter `0x00495A50` (`+0xC0`, used for bridge body / terrain-adjacent). The GPU substitute must produce the same *visible* occlusion result; because normal sprites do NOT z-test, the X+Y sort order (D8) is the SOLE determinant of sprite occlusion — the GPU z-buffer must not override it. (LIVE `0x004978C0` no-z; LIVE `0x00495A50` z-loop; ZBUFFER_DEPTH_SYSTEM.)
- **D12.** TMP terrain tiles use `pixel_z <= zbuffer` (LessEqual); SHP via TechnoClass DrawSHP ORs `0x800` → selects blitters that **ignore** the z-buffer (sprite-vs-sprite is painter's order only). The z-writing blitter variants exist but are unreachable through the `0x800` dispatch for normal objects (dead for standard object rendering). (BRIDGE_RENDERING §17/§16; ZBUFFER_DEPTH_SYSTEM; LIVE `Blitter_selector 0x00490B90`.)
- **D13.** CC_Draw_Shape centering: flag `0x200` shifts by `-w/2,-h/2`; then add SHP frame-rect (x,y); `param_7` is z-depth (never added to screen position) and **forces flag `0x10` (z-enable) on when nonzero**. (LIVE `decompile_function 0x004AED70`; BRIDGE_RENDERING §2/§3.)
- **D13a (blitter selection rule — VERIFIED live this run).** `Blitter_selector 0x00490B90` picks the z+remap member `+0xC0` iff `(flags & 0x10) && (flags & 0x4000) && (flags & 0x800)`; the plain-z member `+0x14` iff `(flags & 0x10) && !(flags & 0x3000) && !(flags & 0x800)`. The `+0xC0` member's blit-method is `0x00495A50`. For the GPU substitute this only matters for bridge-body/terrain-adjacent draws (D14); normal sprites never take a z-tested member. (LIVE `0x00490B90`, `0x0048EBF0`.)
- **D14.** Bridge body overlay z = `(heightLevel+4)·-15 - 2` with z-buffer interaction (blitter `0xC0` → `0x00495A50`); bridge shadow/railing z = `heightLevel·-15 - 2`, no z (shadow blitter `0x4601`). (BRIDGE_RENDERING §3; blitter identity LIVE `0x00490B90`+`0x0048EBF0`.)

**DrawExtras decorations:**
- **D15.** Pip color: GREEN unless `HealthRatio ≤ ConditionYellow(0.5)` → YELLOW unless `≤ ConditionRed(0.25)` → RED. Health-driven only. (SELECTION_BRACKETS §10.)
- **D16.** Building pips: count `(fh×15)/2`, NW-edge anchor `pLoc+(3, -count·2+2)` step `(-4,+2)`, PIPS frames 0/1/2/4 drawn with flags `0x600` → final top-left `draw_point+(-5,-3)`. (SELECTION_BRACKETS §3.6.)
- **D17.** Infantry: PIPBRD frame 1 at `pLoc+(11, Δ-25)`, 8 pips from `pLoc+(-5, Δ-24)` step `(+2,0)`, frames 16/17/18. Unit/aircraft: PIPBRD frame 0 at `pLoc+(1, Δ-26)`, 17 pips from `pLoc+(-15, Δ-25)` step `(+2,0)`. `Δ = PixelSelectionBracketDelta`. (SELECTION_BRACKETS §3.6.)
- **D18.** Veterancy chevron: PIPS frame 14 (veteran) / 15 (elite) / 19 (rookie); infantry at `pLoc+(5,2)`, others `+(10,6)`; z `0xE00` z-adjust -2; gated by `!IsDisguised && VisualState!=5`. Drawn BEFORE brackets. (SELECTION_BRACKETS §3.3.)
- **D19.** Building selection brackets only (line-drawn); color palette `0xF` (white), `0xC` when `GetHeight()<-4`. (SELECTION_BRACKETS §3.4.)
- **D20.** Self-heal pip: organic frame 13 (period `SelfHealInfantryFrames=150`), mechanical frame 20 (period `SelfHealUnitFrames=300`); flash window = `frame % period < 6`, z `0x601` while flashing else `0x600`. (SELECTION_BRACKETS §5.7.)
- **D21.** Deploy-wrench: building + `+0x6E8` set; 6-frame anim `frame=(g_frame%period)*6/(period-1)`, z `0xE00`. (SELECTION_BRACKETS §3.2.)
- **D22.** Group-number overlay: digit `((group+1)&9)` from `"1234567890"`, at `pLoc+(-4, infantry?-36:-39)`, house-color RGB, 73×2 px text. (SELECTION_BRACKETS §5.9.)

**Minimap / pixelfx:**
- **D23.** Minimap pixel pack `((R>>RLoss)<<RShift)|…`; unexplored shroud writes literal `0`; fog dims by channel `>>1` (conditional — off by default); object dots from house color scheme; dirty-rect driven (no full upload). (MINIMAP §1–§5.)
- **D24.** PixelFX 1-px sparkles over visible water/ore cells, emitted per frame, drawn between the object pass and the UI/sidebar pass; opaque, no z. (PIXEL_FX_SPARKLES.)

---

## 6. Rust-native replacement boundary (the draw-helper service)

**Placement:** new module `src/render/draw_helpers/` (a render-layer service). It is consumed by `app_render`. It **does not** depend on `sim/`, `ui/`, `audio/`, `net/`; it takes a frozen sim snapshot view as input. This respects the #1 invariant (sim never depends on render) — the service is purely downstream.

**It is NOT a blitter.** The wgpu depth pipeline stays. The service owns *what to draw and where*, not *how pixels are written*.

**Surface (sketch — signatures illustrative, not load-bearing):**

```text
// src/render/draw_helpers/mod.rs  — render-layer, no sim/ dependency
pub struct DrawLayer { Underground, Surface, Ground, Air, Top }   // explicit 5 layers

/// The canonical Y-sort key: lepton renderX + renderY (Z EXCLUDED), ties = stable index.
/// Fixes the D8 drift: sort is elevation-independent.
/// `class_bias` carries the per-class +0xB8 override (D8a): BuildingClass adds
/// +0x20/-0x10 by Type flags; AnimClass adds its YSortAdjust (+0x104). Plain
/// ObjectClass-family objects pass bias = 0. Omitting this term drifts building/
/// anim layering at equal X+Y.
pub fn y_sort_key(render_lx: i32, render_ly: i32, class_bias: i32) -> i64 {
    (render_lx + render_ly + class_bias) as i64
}

/// Screen position from render-coord leptons; AdjustForZ applied to screen_y ONLY.
pub fn world_to_screen(render: Lepton3, camera: Camera) -> ScreenPos;
pub fn adjust_for_z(z_leptons: i32) -> i32;          // D9 formula, integer

/// FLH world-coordinate source (D6/D7) — fixed-point-free is OK (render side),
/// but PREFER integer-lepton output to match gamemd's ftol rounding bit-for-bit.
pub fn techno_fire_origin(t: &TechnoView, weapon_idx: i32, burst: u8) -> Lepton3;   // 32-way, sign flip
pub fn building_fire_origin(b: &BuildingView, weapon_idx: i32) -> Lepton3;          // sentinel / turret / dual / fixed

/// Decoration placement (DrawExtras) — returns instance descriptors, not pixels.
pub struct DecorationPlan { pips: Vec<PipDesc>, chevron: Option<PipDesc>,
                            brackets: BracketEdges, wrench: Option<..>, ... }
pub fn plan_decorations(t: &TechnoView, selected: bool, hovered: bool,
                        frame: u32) -> DecorationPlan;   // D15..D22 offsets baked in

/// The two-pass scheduler: produces an ordered draw list honoring D1..D5.
/// SORT ONLY layer index 2 by `y_sort_key` (D8b); layers 0/1/3/4 keep
/// SUBMISSION ORDER (no depth sort — gamemd appends them unsorted). The second
/// (extras) pass replays the same per-layer buffer order, never re-sorts.
/// Equal-key ties within layer 2 = stable FIFO (D8c). Sprite-vs-sprite is
/// painter's order only (D11) — the GPU z-buffer must not reorder them.
pub struct ObjectRenderPlan { loop1: Vec<DrawItem>, turret_pass: Vec<DrawItem>,
                              loop2_extras: Vec<DrawItem> }
pub fn plan_object_render(layers: &[Vec<ObjView>; 5],
                          buildings_in_reg_order: &[BuildingView]) -> ObjectRenderPlan;
```

**Ownership / layering:**
- `draw_helpers::offsets` (FLH, turret, pixel-offset, AdjustForZ, DamageFireAnims attach) — pure functions, no GPU.
- `draw_helpers::order` (layer assignment, `y_sort_key`, two-pass scheduler, turret pass) — pure, consumes ObjViews.
- `draw_helpers::decorations` (DrawExtras 9-step placement → `PipDesc`/`BracketEdges`) — pure.
- `draw_helpers::remap` (palette/house-ramp lookup that the existing `PaletteSet` shader consumes) — owns the *result table*, not a blitter.
- `app_render` keeps the wgpu glue (`merge_passes`, `draw_passes`) but consumes `ObjectRenderPlan` instead of a hand-coded call list. The merge keys off `y_sort_key`, not screen_y.

**Fixed-point note:** these are render-layer; f32 is allowed for projection. BUT to match gamemd's `Math__ftol` rounding exactly (which is player-observable at pip/bracket pixel level), the offset helpers SHOULD emit **integer leptons/pixels** with the same truncation order as gamemd, not f32 that rounds later. This is a contract requirement, not a sim-determinism requirement.

---

## 7. Old ad hoc Rust logic to retire / fold into the service

| Rust file:symbol | What it does now | Action |
|---|---|---|
| `src/app_instances/helpers.rs:compute_sprite_depth_params` | sort/depth key = `screen_y + z·HEIGHT_STEP` (Z folded in) | **RETIRE the sort use** → replace with `draw_helpers::order::y_sort_key` (lepton X+Y, Z excluded, D8). Keep a *separate* function for the cliff-occlusion depth value (GPU z-test only). |
| `src/app_render/build_instances.rs:sort_by_depth_desc` (called on every list) | sorts each class list by `.depth` (screen_y-derived) | **Re-key** to `y_sort_key`; depth field becomes GPU-only, sort field becomes lepton X+Y. |
| `src/app_render/draw_passes.rs:dispatch_draw_passes` | hand-coded ~40-call order + first/second bracket submissions emulating two-pass | **Fold** into `plan_object_render` consumption; keep wgpu dispatch, drop the per-class phase emulation in favor of the scheduler's loop1/turret/loop2 lists. |
| `src/util/flh_transform.rs:flh_to_screen_offset*` | f32 screen-space FLH offset | **Fold** into `draw_helpers::offsets`; add the world-coordinate D6 path; keep screen projection as a final step. The `_32way` variant already matches the quantization — reuse it. |
| `src/app_selection_brackets.rs` + `src/render/selection_overlay.rs` (bracket geometry split) | bracket edges + pip atlas | **Consolidate** bracket geometry + pip placement into `draw_helpers::decorations`; the two files keep only atlas/texture ownership. |
| `src/app_ui_overlays.rs:build_*_pip*/build_building_status_*` | per-class pip placement scattered | **Fold** offset math (D15–D22) into `decorations`; keep instance emission. |
| `src/app_render/merge_passes.rs` equal-depth tie-break (`d==best_d && gi>0` prefer SHP) | ad hoc "buildings before VXL at same depth" | **Replace** with the verified tie-break = stable FIFO insertion order (D8c: strict `<` comparator, no secondary key — confirmed at assembly `0x00551A90`). The current SHP-over-VXL preference is an invented tie-break — DRIFT. |
| Any Rust depth-sort applied to the Air layer / particles / aircraft / in-flight projectiles | (if present) sorts elevated objects by depth | **DRIFT — remove the sort for layers 0/1/3/4.** gamemd sorts ONLY layer 2 (D8b); Air/Top are submission-order. Elevated overlaps must keep insertion order, not depth order. (VERIFIED `0x004A9720`.) |
| GPU z-buffer used to occlude sprite-vs-sprite | (if cliff/terrain depth bleeds into sprite ordering) | **Ensure sprites are painter's-order only** between each other (D11): normal SHP objects do NOT z-test in gamemd. The GPU z-buffer must occlude sprites against TERRAIN/cliffs only, never reorder two sprites relative to the X+Y key. |

---

## 8. Migration slices (shadow-first, dependency-ordered) + acceptance tests

Mirrors the Mission/Radio rhythm: shadow → invert → authoritative → (no SNAPSHOT_VERSION bump here — render output is not hashed, but a deterministic golden-frame harness replaces the hash-invariant step). Each slice independently shippable.

**P0 — BLOCKING research gate (status after Pass 2; 2 of 3 sub-gates CLOSED):**
- **RESOLVED (Pass 2):** `Math__ftol` rounding = MSVC `_ftol2`, forced **round-toward-zero / truncate** (CW `0x0E7F` at `0x00822D80`, RC bits = `11`; `read_memory 0x00822D80` = `7f 0e`). Offset helpers MUST truncate toward zero (`.to_num::<i32>()` / `(int)` cast), never `.round()`; keep the multiply chain in the wider type and truncate only at gamemd's sub-step boundaries. (Cross-family GATE RESOLVED — folded in; corroborated this run by the same CW read.) Remaining nuance: the exact per-call truncation boundary inside `CoordsToClient`/`AdjustForZ` for pip/bracket pixel parity is consistent with the AdjustForZ shift in `0x006D8DB0` (`(coord + sign>>8) >> 8` then `- ftol(...)`); treat as VERIFIED-shape, with a golden pixel test as the acceptance.
- **RESOLVED (Pass 2):** Layer-2 insertion tie-break = stable FIFO, strict `<`, no secondary key (D8c, disassembled `0x00551A90` + `0x005F6220`). Only layer 2 sorts (D8b, `0x004A9720`). Building `GetRenderCoords -0x80,-0x80` lepton shift DOES feed the sort (GetYSort `0x005F6BD0` calls `+0xAC`). Air/Top layers append unsorted.
- **PARTIALLY RESOLVED (Pass 2):** the `0xC0` z-remap blitter is `0x00495A50` (z-tested intensity-remap), selected by `(0x10)&(0x4000)&(0x800)` — but it is used for **bridge-body/terrain-adjacent** draws, NOT normal sprites (those use `0x004978C0`, no z). So the GPU-substitute decision narrows to bridge body / TMP-adjacent remap only. **Still open:** decide GPU emulation vs accept-as-substitute for the bridge-body `0xC0` *pixel result* with a documented golden-image test. Do not make bridge-body remap authoritative until decided.

**P1 — Offset helpers (pure, no behavior change).** Extract FLH/turret/pixel-offset/AdjustForZ/DamageFireAnims into `draw_helpers::offsets`; add the D6 world-coordinate path alongside the existing screen path. SHADOW: assert new screen projection == old `flh_transform` output for all existing tests.
- Acceptance: `flh_offsets_match_legacy_screen_path` (golden table for N facings × FLH triplets); `flh_world_source_32way_burst_lateral_alternates` (D6); `building_fire_origin_sentinel_and_turret_branches` (D7, all 4 branches).

**P2 — Sort key correction (the headline parity fix).** Introduce `y_sort_key` (lepton X+Y, Z excluded). SHADOW: log-diff old screen_y key vs new key per frame on a fixture map with a ramp + bridge; INVERT once diffs are understood; make authoritative; depth field becomes GPU-cliff-only. **Apply the sort to layer 2 ONLY (D8b); layers 0/1/3/4 keep submission order.** **Class-bias term required (D8a):** Building `+0x20/-0x10`, Anim `+0x104`.
- Acceptance: `ysort_excludes_elevation` (two units same X+Y, different Z → identical sort rank); `ysort_equal_key_is_fifo` (D8c: two units same X+Y → first-submitted draws first, deterministic); `air_layer_is_submission_order_not_sorted` (D8b: two aircraft at same X+Y but different submission order keep submission order, NOT depth order); `ramp_unit_draw_order_matches_golden` (deterministic frame on a known hill map); `bridge_over_under_unit_order` (D8 + D14 interaction); `building_rendercoords_minus_80_feeds_sort` (D-§0: -0x80,-0x80 lepton shift contributes -0x100 to building sort key).

**P3 — Two-pass scheduler.** `plan_object_render` emits loop1/turret/loop2 lists honoring D1–D5; `draw_passes` consumes it. SHADOW: assert the new ordered list reproduces the current visible order on a no-decoration scene first, then enable decorations.
- Acceptance: `extras_after_all_bodies_same_layer` (D1); `turret_pass_after_layer2_regorder` (D4); `layer_paint_order_0_to_4` (D5); `foot_shadow_before_body` (D2).

**P4 — Decoration placement.** `plan_decorations` produces all D15–D22 descriptors; wire missing ones (veterancy chevron, wrench, self-heal flash, group-number, ammo/PipWrap, DrawExtraInfo, `PixelSelectionBracketDelta` application, bracket dim `<-4`).
- Acceptance: `pip_color_health_only` (D15); `veterancy_chevron_frame_and_offset` (D18); `pixel_selection_bracket_delta_applied` (regression for the parsed-but-unapplied bug); `selfheal_flash_window_6_frames` (D20); `group_number_digit_map` (D22).

**P5 — Remap result table** (gated on P0 decision). Centralize palette/house-ramp into `draw_helpers::remap`; if P0 says emulate the intensity blitter, add the GPU path; else document the accepted substitute with a pixel test.
- Acceptance: `house_ramp_result_matches_palette_range` (existing); `bridge_body_remap_pixel_test` (P0-dependent, golden image).

**P6 — Minimap pixel pipeline parity.** Align `render/minimap.rs` pack/shroud/dot/dirty-rect with D23; confirm fog `>>1` stays gated off by default.
- Acceptance: `minimap_shroud_writes_zero`; `minimap_pack_channels_match`; `minimap_fog_dim_only_when_enabled`.

**P7 — Golden-frame harness.** Deterministic replay → render N fixed frames on a canned skirmish → compare against a committed golden instance-list (positions, depths, draw order, decoration descriptors). This replaces the SNAPSHOT_VERSION/hash step since render output is not part of the sim hash.
- Acceptance: `golden_frame_draw_list_stable` (byte-stable ordered draw plan across runs); regression-locks P1–P6.

---

## 9. Sources & verification ledger

**LIVE this session — ORIGINAL pass (re-decompiled / read):**
- `decompile_function 0x00495A50` — blitter z-loop (`z<zbuf && px!=0` remap write).
- `read_memory 0x007E5618` → `0x0049A660`; `decompile_function 0x0049A660` — `Blitter__Constructor` (vtable identity; LABEL-DRIFT fix §0).
- `decompile_function 0x006D8DB0` — `Tactical_ObjectRenderingLoop` (two-pass body, turret pass, clip constants, `+0x10C/+0x104/+0x110` call order, `g_ScenarioClass & 0x1000` fog gate).
- `decompile_function 0x006F3AD0` — `TechnoClass::GetFLH` (32-way quant, burst sign flip, add to `+0xAC`).
- `decompile_function 0x00453840` — `BuildingClass::GetFLH` (garrison/sentinel/turret/dual/fixed branches).

**LIVE this session — PASS 2 (gate closures, 2026-06-04):**
- `decompile_function 0x00490B90` — `Blitter_selector`: full flag→member dispatch; `+0xC0` iff `(0x10)&(0x4000)&(0x800)`, plain-z `+0x14` iff `(0x10)&!(0x3000)&!(0x800)`.
- `decompile_function 0x0048EBF0` — `Blitter_init`: dispatch-table member `+0xC0` ← heap object with vtable `&PTR_..._007e5618` (= `0x00495A50` blit). Two init branches.
- `decompile_function 0x004AED70` — `CC_Draw_Shape`: flag bits (`0x1/0x2/0x4/0x6/0x10/0x200/0x800`), `param_7≠0 → flags|=0x10`, `0x200` centering.
- `decompile_function 0x004978C0` — `Blitter_Opaque_RLE_Remap` (normal-object blitter): reads `g_ABuffer`, **NO `g_ZBuffer`** read/write → painter's-order sprites.
- `decompile_function 0x00459EF0` — `BuildingClass::GetRenderCoords`: X,Y each `-0x80` leptons, Z untouched.
- `decompile_function 0x005F6BD0` — `ObjectClass::GetYSort`: `renderY + renderX` via `+0xAC`×2; Z excluded.
- `decompile_function 0x005F6220` — `ObjectClass::YSortComparator`: `GetYSort(new) < GetYSort(existing)`, strict `<`.
- `disassemble_function 0x00551A90` — `DynamicVector::SortedInsert`: insert loop = first-strictly-greater break → equal-key FIFO.
- `decompile_function 0x005519C0` — `DynamicVector::Insert`: `param_3 ? SortedInsert : tail-append`.
- `decompile_function 0x004A9720` — `DisplayClass::Submit_Object`: `sorted = (InWhichLayer()==2)` → only layer 2 sorts.
- `decompile_function 0x00449410` — `BuildingClass::GetYSort`: base + `(Type+0x16c5?+0x20:0) - (Type+0x16b7?+0x10:0)`.
- `decompile_function 0x00422BC0` — `AnimClass::GetYSortWithAdjust` (Ghidra-labeled `AnimClass__GetRenderColor`): base + `Anim+0x104`.
- `read_memory 0x007E4408` = `18 2d 44 54 fb 21 c9 bf` (double -π/16); `read_memory 0x00822D80` = `7f 0e` (ftol CW 0x0E7F, RC=truncate); `get_xrefs_to 0x00495A50` (vtable-slot only).

**DOC-SOURCED (corroborated by prior verified reports, NOT re-read live this run):**
- `docs/research/building-selection-brackets/SELECTION_BRACKETS_PIPS_DRAW_ORDER_GHIDRA_REPORT.md` — §2 slot catalog, §3 DrawExtras 9-step, §4 two-pass, §5 DrawPipScalePips, §10 health-only pips, §12 Rust status.
- `docs/research/DRAW_ORDER_DEPTH_SYSTEM.md` — layers, GetYSort X+Y, turret pass, AdjustForZ, §9 elevation-not-in-sort.
- `docs/research/TACTICAL_RENDER_PIPELINE_GHIDRA_REPORT.md` — 3-pass frame, ABuffer/ZBuffer lifecycle.
- `docs/research/bridges/06-render-presentation-audio/BRIDGE_RENDERING_GHIDRA_REPORT.md` — §2 CC_Draw_Shape centering, §3 z-depth/blitter selection, §17 blitter `0xC0` z-loop (cross-confirms `0x00495A50`).
- `docs/research/FLH_TURRET_AND_VISUAL_OFFSETS_GHIDRA_REPORT.md` — FLH fields, IsometricPixelToWorld, GetTurretDrawPosition, ActiveAnim offsets, Rust deltas.
- `docs/research/MINIMAP_GENERATED_PIXEL_COLOR_PIPELINE_GHIDRA_REPORT.md` — pack/shroud/fog/dot pipeline.
- `docs/research/BUILDING_DAMAGE_DESTRUCTION_GHIDRA_REPORT.md` §8 — DamageFireAnims `Type+0x15D8..0x1618` stride 8.
- `docs/research/LAYER_CLASS_GHIDRA_REPORT.md` §7 — render-loop consumption in LayerClass terms.
- `docs/research/PIXEL_FX_SPARKLES_GHIDRA_REPORT.md` — `DrawPixelFXSparkles 0x006D7840` position.
- `docs/research/TARGET_LINES_GHIDRA_REPORT.md` — palette index color extraction (DrawActionLines neighbor system).
- `docs/research/ZBUFFER_DEPTH_SYSTEM.md` — (NEWLY CITED Pass 2) normal SHP `0x800` blitters ignore z-buffer; only TMP terrain reads/writes z (`<=`); `Blitter_Opaque_RLE_Remap 0x004978C0` opaque-no-z; cross-confirms D11/D12.
- `docs/research/ANIMCLASS_DRAW_TRAVERSAL_LAYER_ORDERING_RESWARM_20260527.md` — (NEWLY CITED Pass 2) independent live confirmation: `Submit_Object` sorted=`layer==2` only; equal-key FIFO; `GetYSort=X+Y` Z-excluded; AnimClass `+0xB8 = +0x104`; `GetLayer 0x00424CB0` fallback=layer 3 (Air).
- `docs/research/PERCLASS_VTABLE_B8_YSORT_OVERRIDE_CENSUS_GHIDRA_REPORT.md` — (NEWLY CITED Pass 2) census of all `+0xB8` YSort overrides (corroborates D8a Building/Anim bias).
- `docs/research/traces/GARRISON_SHOT_Z_ADJUST_DEPTH_POSTFIX_TRACE.md` — (NEWLY CITED Pass 2) garrison-shot OccupantAnim depth = `YDrawOffset + ZAdjust - AdjustForZ() - 2`; relevant to turret/garrison overlay ordering (D4).

**Rust scanned:** `src/app_render/{draw_passes,merge_passes,build_instances,mod}.rs`, `src/app_skirmish_shell_render/draw_order.rs` (shell-menu only, NOT tactical), `src/app_instances/helpers.rs`, `src/util/flh_transform.rs`, `src/render/{batch,selection_overlay,minimap,pixel_fx_sparkles}.rs`.

**UNCHECKED / blocking (P0) — after Pass 2:** ONLY the bridge-body `0xC0` intensity-remap *pixel result* vs GPU-substitute decision remains (golden-image test). `Math__ftol` truncation order = RESOLVED (truncate, CW 0x0E7F). Layer-2 insert tie-break = RESOLVED (FIFO, `0x00551A90`/`0x005F6220` disassembled). Building `-0x80,-0x80` lepton shift = RESOLVED (feeds sort).
**DRIFT carried forward:** D8 (Rust folds Z into sort key), **D8a** (per-class YSort bias — Building `+0x20/-0x10`, Anim `+0x104` — MUST be in `y_sort_key`), **D8b (NEW — Rust must NOT depth-sort layers 0/1/3/4; Air/Top are submission-order)**, **D11 (NEW — GPU z-buffer must not reorder sprite-vs-sprite; gamemd normal sprites do not z-test)**, D6 (FLH screen-only vs world source), `PixelSelectionBracketDelta` unapplied, merge tie-break invented (SHP-over-VXL — should be FIFO per D8c), building fixed fire pixel offset missing.

---

## Reviewer follow-ups (adversarial audit 2026-06-04)

Audited read-only against the live binary + cited Rust. Load-bearing addresses all VERIFIED this review: `0x007E5618` vtable base (first DWORD `0x0049A660` = ctor, NOT the z-loop — §0 label-drift call is correct; `0x00495A50` is in fact the **second** slot pointer under that vtable, `read_memory 0x007E5618` = `60a64900 505a4900 ...`); `0x006D8DB0` two-pass body + fog gate (`*g_ScenarioClass_Instance & 0x1000`) + turret pass + clip constants; `0x006F3AD0` GetFLH 32-way quant + burst sign flip; `0x00453840`, `0x00459EF0` (`-0x80` shift), `0x005F65D0` (`+0x118 → +0x114` stub), `0x0070A990`/`0x00709A90` pip slots, `0x00551A90` = `DynamicVector__SortedInsert`, `0x004D1890`, `0x006D7840`. Retire-list Rust refs all exist and do what the doc says (`compute_sprite_depth_params:49` folds Z; `merge_passes.rs:304` `(d==best_d && gi>0)` invented SHP-over-VXL tie-break). Program fit OK (render-side service, no sim→render edge, no blitter port, no SNAPSHOT bump). TS-legacy handling OK (fog darkening / FoggedObject / subterranean / BounceClass all in INACTIVE list, none in the substrate design).

- **CORRECTED (this review): D8 "no secondary key" was too strong.** Base `ObjectClass::GetYSort 0x005F6BD0` is `X+Y`, Z excluded (confirmed) — but `BuildingClass::GetYSort 0x00449410` and `AnimClass::GetYSortWithAdjust 0x00422BC0` override `+0xB8` with additive bias. Captured as D8a + §2b rows + `y_sort_key(class_bias)`.
- **Minor naming:** §0/§2e write the fog gate as `g_ScenarioClass & 0x1000`; the live loop dereferences `g_ScenarioClass_Instance` (`*ptr & 0x1000`). Same gate, pointer-vs-value wording only.
- **Residual UNCHECKED for synthesis (unchanged, correctly flagged by doc):** `Math__ftol` truncation order; Layer-2 SortedInsert tie-break behavior (function identity confirmed; comparator `0x005F6220` reads only `+0xB8`, so insertion-order tie-break is plausible but the exact insert position for equal keys was not stepped this review); intensity-remap blitter `0xC0` GPU-substitute decision; whether building `GetRenderCoords -0x80` (verified to exist) actually feeds the Layer-2 sort path end-to-end.

> **NOTE (Pass 2): all three of those residual UNCHECKEDs are now CLOSED except the `0xC0` GPU-substitute *decision* (which is a design choice, not a binary fact). See "Pass 2 — Expansion" below.**

---

## Pass 2 — Expansion (JOB A gate closures + completeness sweep, 2026-06-04)

All addresses below were **live-decompiled / read THIS run** (citations inline). Default verdict for any unproven difference remains DRIFT.

### P2.1 — JOB A gate closures (changelog)

| Gate | Verdict | Resolution + evidence |
|---|---|---|
| Layer-2 / 2nd-pass insert tie-break at equal depth | **VERIFIED (FIFO, no secondary key)** | `DynamicVector__SortedInsert 0x00551A90` (disassembled): inserts at first existing element with strictly-greater key → equal keys keep insertion order. Comparator `ObjectClass__YSortComparator 0x005F6220` = `GetYSort(new) < GetYSort(existing)` strict `<`. The 2nd (DrawExtras) pass in `0x006D8DB0` does NOT re-sort — it replays buffer order. **NEW: only layer 2 is sorted at all** (`DisplayClass__Submit_Object 0x004A9720`, `sorted=layer==2`); layers 0/1/3/4 append unsorted (`DynamicVector__Insert 0x005519C0`). Air-layer (3) overlapping objects ⇒ pure submission order. |
| 0xC0 remap-blitter vs plain z-blitter decision | **VERIFIED** | `Blitter_selector 0x00490B90`: `+0xC0` iff `(0x10)&(0x4000)&(0x800)`; plain-z `+0x14` iff `(0x10)&!(0x3000)&!(0x800)`. `Blitter_init 0x0048EBF0`: `+0xC0` member's vtable = `&PTR_..._007e5618` whose slot+4 = `0x00495A50`. Flags from `CC_Draw_Shape 0x004AED70` (`param_7≠0→0x10`). **NEW nuance:** normal sprites never reach a z-tested member — they use `0x004978C0` (no z); the `0xC0` z path is bridge-body/terrain-adjacent only. |
| Building `GetRenderCoords -0x80` shift exact + frame | **VERIFIED** | `BuildingClass__GetRenderCoords 0x00459EF0`: `X -= 0x80; Y -= 0x80; Z unchanged`. Units = **leptons** (half-cell). Feeds Layer-2 sort via `GetYSort 0x005F6BD0` (`+0xAC`×2 sum). No analogous unit shift in `ObjectClass::GetRenderCoords` (base returns coords unshifted; only the building override subtracts). |
| Layering invariant (sim ⊥ render; output not blitter) | **RECONFIRMED** | Service stays downstream of `sim/`; contract = observable order/offset/z. Strengthened: normal sprites are painter's-order (no per-pixel z, `0x004978C0`), so a GPU painter's-order substitute keyed on X+Y IS the faithful reproduction; GPU z used only for terrain/cliff occlusion. |
| ftol truncation order (cross-family, folded in) | **VERIFIED (truncate)** | CW `0x0E7F` (`read_memory 0x00822D80` = `7f 0e`), RC bits=truncate-toward-zero. Offset helpers must `(int)`-cast / `.to_num::<i32>()`, never `.round()`. |
| FLH 32-way constant (cross-family, folded in) | **VERIFIED (-π/16)** | `read_memory 0x007E4408` = `18 2d 44 54 fb 21 c9 bf` = -0.19634954…; matches `GetFLH 0x006F3AD0` `(((facing>>10)+1>>1 & 0x1f)-8) * const`. |

### P2.2 — Newly-found functions / methods (sweep)

| Name | Address | Role | Status | Evidence |
|---|---|---|---|---|
| `Blitter_init` | `0x0048EBF0` | builds the ~90-entry blitter dispatch table (`+0x08..+0x168`); maps every flag-combo to a heap blitter object | VERIFIED | LIVE decompile |
| `Blitter_Opaque_RLE_Remap` | `0x004978C0` | normal-object opaque blitter (no z); RLE + intensity-remap; reads `g_ABuffer` | VERIFIED | LIVE decompile |
| `DynamicVector__Insert` | `0x005519C0` | `param_3 ? SortedInsert : tail-append` — the sorted/unsorted fork | VERIFIED | LIVE decompile |
| `DynamicVector__SortedInsert` | `0x00551A90` | the X+Y sorted insert with FIFO equal-key tie-break | VERIFIED | LIVE disasm |
| `ObjectClass__YSortComparator` | `0x005F6220` | strict-`<` comparator over `+0xB8` | VERIFIED | LIVE decompile |
| `DisplayClass__Submit_Object` | `0x004A9720` | layer assignment + insert; `sorted=layer==2` | VERIFIED | LIVE decompile |
| `DisplayClass__RemoveFromLayer` | `0x004A9770` | removes object from its current layer before re-submit (anti-double-insert) | DOC-NOTED (callee) | from `0x004A9720` callees |
| `AnimClass::GetLayer` | `0x00424CB0` | `+0xCC!=0 → 2`; else `AnimType+0x364`; else fallback **3 (Air)** | DOC (ANIMCLASS_DRAW_TRAVERSAL, live there) | corroborated |

### P2.3 — Newly-found globals / tables (sweep)

| Global | Address | Role | Status |
|---|---|---|---|
| ftol control word | `0x00822D80` | x87 CW `0x0E7F` (RC=truncate) driving all float→int | VERIFIED (`read_memory`) |
| `_g_BlitterFlagMask_0x3000` | (mask const) | masks bits `0x3000` in `Blitter_selector` dispatch | VERIFIED (in `0x00490B90`) |
| `g_ABuffer` | `0x0087E8A4` | alpha/shroud circular buffer the normal blitter reads (not z) | VERIFIED (in `0x004978C0`) |
| Blitter dispatch table object | (per-Surface instance, `param_1+0x08..0x168`) | ~90 blitter-variant member ptrs, lazy-init | VERIFIED (`0x0048EBF0`) |

### P2.4 — Material edge cases / TS-legacy separations found

- **Sprite-vs-sprite has NO z-test (material).** Confirmed `Blitter_Opaque_RLE_Remap 0x004978C0` never touches `g_ZBuffer`; the z-writing blitter variants are dead for normal `0x800` object draws (ZBUFFER_DEPTH_SYSTEM). ⇒ D11 rewritten; new retire-list row: GPU z must not reorder sprites. This is a parity-blocker if the Rust GPU pipeline lets cliff depth occlude unit-vs-unit.
- **Air/Top layers unsorted (material).** New D8b. Aircraft / in-flight projectiles / jumpjet infantry overlap order = submission order, not depth. Any Rust depth sort on those DRIFTS.
- **`InWhichLayer==2` is the SOLE sort gate (material).** Not "Ground objects are sorted because they're buildings/units" — it's literally the layer-index test in `0x004A9720`. An object that returns layer 2 from a non-standard class still gets sorted; one in layer 3 never does.
- **Two `Blitter_init` branches (non-material for parity).** `*(param_1+4)==1` builds a reduced blitter set (different vtables, e.g. `PTR_LAB_007e5a08` family) vs the full set; both populate `+0xC0`. Which branch runs depends on the target Surface's pixel format; does not change the observable selection rule. Flagged for completeness, not a DRIFT.
- **`Blitter_Opaque_RLE_Remap` is RLE; `0x00495A50` is linear.** The normal path decodes RLE runs (`*param_3==0 → skip count`); the z-remap path is a flat per-pixel loop. Both share the same intensity-remap table math (`iVar*0x200 + table[...] | palette`). Only the z-test and RLE differ — irrelevant to the GPU substitute (which is neither RLE nor software-blit) but documented so the remap *result table* (P5) is sourced correctly.

### P2.5 — Re-applied burden-of-proof to own doc

- D11 previously implied all sprite writes are z-tested ("for z-tested blitters"). **Demoted/clarified: normal sprites are NOT z-tested.** No equivalence claim retained without the `0x004978C0` proof.
- D8 "no secondary key" was already corrected to D8a (class bias) in the prior review; Pass 2 adds D8b (only-layer-2-sorts) and D8c (assembly-level FIFO proof) so the claim is now bit-grounded, not "plausible."
- The `0xC0` / `0x00495A50` relationship — previously left as "consistent but must not conflate" — is now a proven bit-exact bridge (vtable slot +4 = the `+0xC0` member's blit method), not an unproven equivalence.
