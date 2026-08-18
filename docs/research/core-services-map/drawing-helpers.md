# Core Service Profile — Drawing helpers (blitter + SHP/VXL draw primitives)

**Slug:** `drawing-helpers`
**Layer:** ui-render (render-side draw-helper service; strictly DOWNSTREAM of `sim/`)
**Tick/render position:** Render pass only — runs inside `TacticalClass::Draw` Pass 2 (objects); no role in `LogicClass::PerTickUpdate`.
**Primary doc:** `docs/research/DRAWING_HELPERS_ENGINE_SUBSTRATE_SERVICE_STUDY.md` (STUDY+DESIGN, Ghidra-verified, Pass 2 2026-06-04)
**Provenance:** Doc is Ghidra-verified with inline LIVE citations; this profile is doc-sourced + two confirmatory research-index lookups (intensity-LUT, LayerClass GetYSort). No re-verification of the whole doc.

---

## Purpose

The shared draw-primitive substrate beneath every per-class `DrawIt`. It owns **where a sprite lands on screen, in what order sprites paint, how depth resolves, how palette/remap is applied, and where decorations (pips/brackets/FLH-anchored effects) attach**. The contract is **observable output** — two-pass draw ORDER, draw OFFSETS, layer/z-resolution, palette/remap RESULT, pip/bracket placement — NOT a port of gamemd's software blitter (this engine uses a wgpu GPU depth pipeline; sprites are painter's-order, only terrain/cliff writes depth).

Critical proven property: gamemd's NORMAL opaque SHP sprites do painter's-order draw with **NO per-pixel z-test** (`Blitter_Opaque_RLE_Remap 0x004978C0` never touches `g_ZBuffer`). Sprite-vs-sprite occlusion is decided ENTIRELY by display-layer + X+Y insertion order. The z-tested remap blitter (`0x00495A50`) is for bridge-body / terrain-adjacent draws only.

---

## Owns (state / globals / structs)

- **The two-pass 5-layer object render loop** `Tactical_ObjectRenderingLoop 0x006D8DB0` — Loop 1 (sprites: pre-draw `+0x10C` → DrawIt `+0x104`, foot shadow `+0x110`), turret/garrison pass after layer 2, Loop 2 (DrawExtras `+0x110`).
- **Blitter family** — vtable base `0x007E5618` (slot 0 ctor `Blitter__Constructor 0x0049A660`, slot +4 z-loop `0x00495A50`); `Blitter_init 0x0048EBF0` builds the ~90-entry dispatch table (`Surface+0x08..+0x168`); `Blitter_selector 0x00490B90` picks a member by draw flags; `CC_Draw_Shape 0x004AED70` SHP draw entry; `Blitter_Opaque_RLE_Remap 0x004978C0` normal-sprite blitter (no z); `TMP_TileBlitter 0x00547CF0` terrain tile blit (z R+W, `<=`).
- **Display layers** `g_DisplayLayers 0x008A0360` (5× DynamicVector, 0x18 stride; Underground/Surface/Ground/Air/Top); flat-anim layer `0x008A0390`.
- **Depth/alpha buffers** `g_ZBuffer 0x00887644` (16-bit), `g_ABuffer 0x0087E8A4` (16-bit shroud/alpha; the normal blitter reads this, not z).
- **FLH / fire-origin math** `TechnoClass::GetFLH 0x006F3AD0` (32-way facing quant via const `_DAT_007e4408` = -π/16, burst lateral sign flip), `BuildingClass::GetFLH 0x00453840`, `GetTurretDrawPosition 0x00453BF0`, `IsometricPixelToWorld 0x006D2070`.
- **Decoration assets** `PIPBRD.SHP 0x00AC1478`, `PIPS.SHP 0x00AC147C`, `PIPS2.SHP 0x00AC1480`, `TALKBUBL.SHP 0x00AC1484`; foundation width/height tables `0x008192B8`/`0x00819310`; group-digit string `0x0081B3D0`.
- **Screen-projection / clip** `CoordsToClient`, `Tactical__WorldToScreenSub`, `Tactical__AdjustForZ` (z lepton → screen-Y lift, formula `≈ ftol(z·0.14348 + (z≥728?1:0) + 0.5)`); clip extents `DAT_00b0ce30/34`; ftol control word `0x00822D80` (CW 0x0E7F, truncate-toward-zero).
- **Minimap pixel pipeline** (RGB→16-bit pack→dots/shroud/fog) and **PixelFX sparkles** `DrawPixelFXSparkles 0x006D7840`.

---

## Key functions & globals (addresses)

| Symbol | Address | Role |
|---|---|---|
| `Tactical_ObjectRenderingLoop` | `0x006D8DB0` | two-pass 5-layer object renderer (the spine of this service) |
| `TacticalClass::Draw` | `0x006D3D10` | frame draw, calls object loop in Pass 2 |
| `RenderFrame_main` | `0x004F4480` | frame driver |
| Blitter z-loop (vtable 0x7E5618 slot+4) | `0x00495A50` | per-pixel `z<zbuf && px!=0` intensity-remap write (bridge/terrain only) |
| `Blitter_Opaque_RLE_Remap` | `0x004978C0` | normal-sprite blitter — RLE intensity-remap, reads `g_ABuffer`, NO z |
| `Blitter__Constructor` | `0x0049A660` | installs `vtable__Blitter` (`PTR..._007e5618`) |
| `Blitter_init` | `0x0048EBF0` | builds ~90-entry blitter dispatch table; `+0xC0` ← vtable 0x007E5618 |
| `Blitter_selector` | `0x00490B90` | flag→member dispatch (`+0xC0` z+remap iff `(0x10)&(0x4000)&(0x800)`; plain-z `+0x14` iff `(0x10)&!(0x3000)&!(0x800)`) |
| `CC_Draw_Shape` | `0x004AED70` | SHP draw entry; `0x200`=center, `param_7`(z)≠0→forces `0x10` |
| `TMP_TileBlitter` | `0x00547CF0` | terrain tile blit (z R+W `<=`) |
| `TechnoClass::GetFLH` | `0x006F3AD0` | 32-way FLH world-coord source + burst sign flip |
| `BuildingClass::GetFLH` | `0x00453840` | garrison/sentinel/turret/dual/fixed fire-origin branches |
| `GetTurretDrawPosition` | `0x00453BF0` | voxel-turret fire origin |
| `BuildingClass::GetRenderCoords` | `0x00459EF0` | X,Y each `-0x80` leptons (half-cell), Z untouched; feeds sort |
| `ObjectClass::GetYSort` | `0x005F6BD0` | sort key = render `X+Y` (Z excluded); calls `GetRenderCoords` (vtable+0xAC) twice |
| `BuildingClass::GetYSort` | `0x00449410` | base X+Y + (`Type+0x16c5`?+0x20) − (`Type+0x16b7`?+0x10) |
| `AnimClass::GetYSortWithAdjust` | `0x00422BC0` | base X+Y + `Anim+0x104` (Ghidra-mislabeled `AnimClass__GetRenderColor`) |
| `ObjectClass::YSortComparator` | `0x005F6220` | strict `<` over vtable+0xB8 (equal-key → FIFO) |
| `DisplayClass::Submit_Object` | `0x004A9720` | `sorted = (InWhichLayer()==2)` — ONLY layer 2 sorts |
| `DynamicVector::Insert` / `SortedInsert` | `0x005519C0` / `0x00551A90` | sorted/unsorted fork; FIFO equal-key insert |
| `DrawHealthBar`/`DrawPipScalePips`/`DrawVeterancyPips`/`DrawExtraInfo` | `0x006F64A0`/`0x00709A90`/`0x0070A990`/`0x0070AA60` | DrawExtras decoration slots |
| `DrawPixelFXSparkles` | `0x006D7840` | water/ore twinkles between object & UI pass |
| Intensity-LUT generator | `FUN_00420140` (cache `g_IntensityTableCache 0x0088A084`) | builds 256×256 ushort intensity table read by translucent blitters |
| `g_DisplayLayers`/`g_ZBuffer`/`g_ABuffer` | `0x008A0360`/`0x00887644`/`0x0087E8A4` | layer vectors + depth + alpha buffers |
| FLH 32-way const `_DAT_007e4408` | `0x007E4408` | -π/16 facing-bucket angle |

---

## Tick / render position

- Lives entirely in the **render pass**, never the sim tick. Invoked from `TacticalClass::Draw 0x006D3D10` Pass 2 (objects), which `RenderFrame_main 0x004F4480` calls (sidebar/UI composited between Pass 1 terrain and Pass 2 objects).
- Within Pass 2: Loop 1 draws all sprite bodies layer 0→4; the building turret/garrison pass runs once after layer 2 in `g_BuildingClass_Array` registration order; Loop 2 draws all DrawExtras decorations; PixelFX sparkles draw between the object pass and the UI/sidebar pass.
- Consumes a frozen sim snapshot (object coords, type fields, house, health); **never writes deterministic state** — not in the state hash. This respects the #1 invariant (sim never depends on render).

---

## Depends-on (outgoing edges)

| Target slug | Via symbol / field | Evidence |
|---|---|---|
| `abstract-object` | `ObjectClass::GetYSort 0x005F6BD0` (sort key), `GetRenderCoords` vtable+0xAC, `InWhichLayer` vtable+0x78, `GetHeight` vtable+0x1C8, `WhatAmI` vtable+0x2C | The object render loop reads these base ObjectClass vtable slots to lay out / order / clip every sprite. GetYSort sums render `X+Y`; Submit_Object gates sort on `InWhichLayer()==2`. LIVE `0x006D8DB0`, `0x005F6BD0`, `0x004A9720`; LAYER_CLASS §4/§9. |
| `techno-foot` | `TechnoClass::GetFLH 0x006F3AD0`, per-class `DrawIt`/`DrawShadow`/`DrawExtras` vtable slots (`+0x104/+0x110/+0x10C`), `TechnoClass::GetWeapon 0x0070E140`, type FLH fields (`Type+0x898`/`+0xA94`) | Loop 1 dispatches each object's `DrawIt`; FLH/turret math reads TechnoClass type fields for fire-origin offsets. LIVE `0x006D8DB0`, `0x006F3AD0`. |
| `lookup-tables` | per-blitter intensity table `BlitterInfo+8` + remap palette `BlitterInfo+4`; intensity-LUT generator `FUN_00420140` (`g_IntensityTableCache 0x0088A084`); read-only palette/remap LUTs | The blitter family reads static intensity/remap tables to convert palette index → 16-bit pixel. CLOAK_FX_SHADER_BRIDGE §6.4/§6.5; WARP_TRANSLUCENCY_BLITTER §6. |
| `cell-map` | `g_DisplayLayers 0x008A0360` ownership via `DisplayClass::Submit_Object 0x004A9720` / `Remove_From_Layer 0x004A9770`; viewport (g_Tactical) in `CoordsToClient`; `IsometricPixelToWorld 0x006D2070` | DisplayClass (the map/display owner) holds the 5 layer vectors the render loop walks; screen projection uses the tactical viewport. LIVE `0x006D8DB0`; LAYER_CLASS §9. |
| `rules-class` | ConditionYellow (0.5) / ConditionRed (0.25) for pip color (D15); SelfHealInfantryFrames(150)/SelfHealUnitFrames(300) flash period (D20); PixelSelectionBracketDelta | DrawExtras pip/self-heal placement reads these RulesClass-parsed tunables. SELECTION_BRACKETS §10/§5.7. |
| `factory-house` | house color scheme for minimap object dots (D23) + DrawExtraInfo house-color text label `0x0070AA60`; house remap ramp feeding the blitter remap palette | Object dots and the house-color name label/remap come from the owning HouseClass color scheme. MINIMAP §; SELECTION_BRACKETS (DrawExtraInfo). |
| `bridge-helpers` | bridge-body overlay z = `(heightLevel+4)·-15 - 2` via z-remap blitter `0xC0`/`0x00495A50`; bridge shadow/railing z = `heightLevel·-15 - 2` (no-z blitter `0x4601`) (D14) | The only normal use of the z-tested remap blitter is the bridge body; bridge predicates/heights drive the z-depth arg. BRIDGE_RENDERING §3/§17. |
| `random-scenario` | `*g_ScenarioClass_Instance & 0x1000` SpecialFlag fog-of-war darkening gate (DORMANT — off by default in YR) | Loop 1/Loop 2 read the ScenarioClass SpecialFlags to decide fog darkening; branch normally not taken (only black shroud active). LIVE `0x006D8DB0`. Edge is DORMANT/legacy. |

---

## Used-by (incoming edges)

| Source slug | Via symbol | Evidence |
|---|---|---|
| `logicclass` | `RenderFrame_main 0x004F4480` → `TacticalClass::Draw 0x006D3D10` → `Tactical_ObjectRenderingLoop 0x006D8DB0` | The frame driver (render side of the main loop) invokes this service once per frame after the sim tick. Render is downstream of LogicClass's per-tick state, not part of it. TACTICAL_RENDER_PIPELINE. |
| `techno-foot` | per-class `DrawIt`/`DrawExtras` call back into `CC_Draw_Shape 0x004AED70`, `GetFLH`, `DrawHealthBar`/pip helpers | Each Techno/Foot subclass's draw override consumes this service's draw primitives, FLH, and decoration helpers. Mutual (loop dispatches DrawIt; DrawIt re-enters the blitter). SELECTION_BRACKETS §2/§5. |
| `abstract-object` | `GetYSort`/`GetRenderCoords` are ObjectClass methods the loop calls, but the loop also defines layer/sort semantics around them | The loop is the consumer that gives ObjectClass's render-coord/ysort outputs meaning; ObjectClass exposes the slots, drawing-helpers orders by them. LAYER_CLASS §4. |
| `gadget-dialog` | sidebar/UI composited between Pass 1 and Pass 2; PixelFX/decorations drawn before the UI pass | The in-game gadget tree (sidebar, cursor) renders around this service's object pass; ordering of object pass vs UI pass is shared. TACTICAL_RENDER_PIPELINE. |

---

## Open / unverified edges

- **`0xC0` bridge-body intensity-remap PIXEL RESULT vs GPU substitute (P0 design gate, OPEN).** Whether to GPU-emulate the bridge-body z-remap blitter `0x00495A50` pixel output or accept a documented substitute is a design choice with a golden-image acceptance test, not a binary fact. Until decided, do not make bridge-body remap authoritative. (`lookup-tables`/`bridge-helpers` result-equivalence UNCHECKED.)
- **Palette-remap RESULT equivalence (`lookup-tables`).** The Rust `PaletteSet` + shader path substitutes the per-blitter intensity table; result-equivalence for intensity/translucent blitters (bridge body, shadow, cloak shimmer) is UNCHECKED vs the native 256×256 LUT (`FUN_00420140`).
- **`cell-map` viewport-projection rounding boundary.** The exact per-call `Math__ftol` truncation boundary inside `CoordsToClient`/`AdjustForZ` for pip/bracket pixel parity is VERIFIED-shape (truncate, CW 0x0E7F) but the sub-step boundary needs a golden pixel test.
- **`factory-house` minimap fog dim (`>>1`).** Fog channel `>>1` dim path is gated off by default in YR; parity of the Rust minimap fog path is UNCHECKED (but DORMANT in stock skirmish).
- **Layer index for airborne aircraft.** LAYER_CLASS §11 found airborne aircraft go to layer **4 (Top)**, while ANIMCLASS_DRAW_TRAVERSAL fallback = layer **3 (Air)**; both are unsorted (submission order), so the observable ordering is the same, but the exact layer index per class is a residual `cell-map`/`abstract-object` detail.

---

## Carried-forward DRIFT vs current Rust (from primary doc §0/§7)

- **D8** Rust folds Z into the Layer-2 sort key (`screen_y + z·HEIGHT_STEP`); gamemd uses lepton `X+Y`, Z excluded.
- **D8a** missing per-class YSort bias (Building `+0x20/-0x10`, Anim `+0x104`).
- **D8b** Rust must NOT depth-sort layers 0/1/3/4 (Air/Top are submission-order).
- **D11** GPU z-buffer must not reorder sprite-vs-sprite (normal sprites are painter's-order, no z-test).
- **D6** FLH is screen-space offset in Rust; gamemd produces a world coord.
- `merge_passes.rs` invented SHP-over-VXL equal-depth tie-break (should be FIFO per D8c).
- `PixelSelectionBracketDelta` parsed but not applied; building fixed fire pixel offset missing.
