# Core Service Profile — Tactical (world) draw pass

**Slug:** `frontier-render-tactical`
**Layer:** ui-render (render-side service; strictly DOWNSTREAM of `sim/` — the #1 invariant: it reads a frozen sim snapshot, never writes deterministic state, never appears in the state hash)
**Tick/render position:** RENDER pass (out-of-sim). `TacticalClass::Draw 0x006D3D10` runs from `RenderFrame_main 0x004F4480`, called inside `Map__Logic()` of the live-gameplay block in `Main_Tick 0x0055D360` — i.e. BEFORE `LogicClass::PerTickUpdate`, not a rung of it. Its sim-side **AI counterpart** (camera scroll interp + view commit + radar-refresh timer) IS a per-tick rung: `TacticalClass::AI/Update 0x006D2540` is **PerTickUpdate Rung Y** (`g_Tactical->vtable+0x5C`, spine §2 row 25).
**Primary docs (Ghidra-verified):** `TACTICAL_RENDER_PIPELINE_GHIDRA_REPORT.md` (three-pass internals, full Pass-2 26-step order), `GSCREEN_RTACTICAL_GHIDRA_REPORT.md` (the manager singletons + RenderFrame_main composition contract + TacticalClass layout), `SPATIAL_PRIMITIVES_LAYER_GHIDRA_REPORT.md` / `COORD_TRANSFORM_AUDIT_GHIDRA_REPORT.md` (world↔screen transforms), `LAYER_CLASS_GHIDRA_REPORT.md` / `DISPLAYCLASS_GHIDRA_REPORT.md` (the layer draw list it walks), `MAIN_TICK_RENDER_LOGIC_COUPLING_GHIDRA_REPORT.md` (frame/tick coupling + no sub-frame interp).
**Provenance:** Doc-sourced from the above Ghidra-verified reports + research-index cross-lookups (coord transforms, layer system, DisplayClass). **Live Ghidra re-verification of the representative address was NOT possible this session — the Ghidra MCP instance was unreachable (`list_instances` empty, TCP 127.0.0.1:8089 refused, no UDS sockets).** Every address below therefore carries its **prior** verifying citation from the cited doc family; addresses are CONVERGENT across ≥2 independent verified reports where flagged. Treat as **located/cross-doc-verified**, not freshly re-decompiled. Re-confirm `0x006D3D10` with `decompile_function 0x006D3D10` once Ghidra is back up.

---

## Purpose

Draws the isometric battlefield viewport each frame: it manages the dirty-rect/scroll buffer state, paints the 8 terrain layers (shroud/fog edges → terrain shadows → base tiles → smudges → building overlays → walls/ore → ground anims) into the back surface + ABuffer/ZBuffer, then on the object pass walks the z-ordered draw list and paints every unit/building/projectile/effect plus all world-space overlays (rally lines, capture links, placement ghost, waypoints, brackets, garrison pips, band-box, radar event markers, SW target circles, PixelFX glow, floating timer text). It also **owns the world↔screen↔cell coordinate transforms** (the isometric projection) that every pick/draw/cursor decision routes through.

The contract is **observable output**: the three-pass sandwich ORDER (terrain → chrome → objects), the Pass-2 26-step draw ORDER, the layer/z resolution, the dirty-rect cadence, the projection math to the pixel. gamemd's internal double-buffer + circular ABuffer/ZBuffer + 800-entry dirty-rect ring is a software-rasterizer implementation detail; the Rust port substitutes a wgpu GPU depth pipeline and a full-viewport repaint, so internals diverge — but the **draw order, projection result, and layer occlusion must match**.

**Boundary vs sibling render services (do not double-own):**
- The **per-object two-pass loop internals** (`Tactical_ObjectRenderingLoop 0x006D8DB0`, FLH/turret math, YSort keys, pip/bracket placement, blitter family) are owned by **`drawing-helpers`**. This service OWNS the *frame driver* `0x006D3D10` that *calls* that loop in Pass 2, the *terrain layer heads*, the *buffer/scroll/dirty-rect lifecycle*, and the *transforms*.
- The **z-ordered draw-list membership** (LayerClass submit/remove, 5 layer vectors) is owned by **`frontier-render-layer`** (A2). This service WALKS that list; it does not own its churn.
- The **final raster blit to the DirectDraw primary surface** + the `BlitTrans*`/`RLEBlitTrans*` template family is owned by **`frontier-blitter`** (A3).
- **Sidebar/radar chrome** composited between Pass 1 and Pass 2 belongs to `frontier-sidebar` / `frontier-radar` (via `MouseClass::Draw 0x006D0A20`).

---

## Owns (state / globals / structs)

- **The frame draw dispatcher** `TacticalClass::Draw 0x006D3D10` — three-pass state machine selected by `param_3` (0=scroll/buffer mgmt, 1=terrain, 2=objects; 3=combined full path). Called **3× per frame** (params 0,1,2) by `RenderFrame_main 0x004F4480`.
- **The manager singleton** `g_Tactical 0x00887324` — `TacticalClass*` (AbstractClass-derived; NOT in the GScreen display chain), `0x0E18` (3608) bytes, allocated via `operator_new(0xE18)` in scenario init `FUN_006851F0` and **re-created on every scenario load** (destructor via vtable[8] in `FUN_006BE1C0`). TacticalClass vtable `0x007F4348`. Owns: viewport corners `+0xB0..+0xBC`, current viewport `+0xD64/+0xD68`, scroll target/speed/progress `+0xD0..+0xDC`, auto-scroll period `+0xDAC` (← `Rules+0x50`), dirty-cell count/list `+0xE0/+0xE4`, ~800-entry dirty-rect ring, per-frame AI dedup `+0xA8`, sorted visible-building list `+0xDB0` (max 500), isometric camera matrix block (12+ floats from `+0xDE4`).
- **The display-chain singleton (sibling, calls into this service)** `g_DisplayChain 0x00887640` — `MouseClass*` mega-object, `0x556C` bytes; its `GScreenClass` base owns `RedrawFlag +0x0C` (0/1/2 = none/partial/full redraw) consumed + cleared by `RenderFrame_main`.
- **The 8 terrain layer heads (Pass 1)** — `Tactical_ZBufferDirtyClear 0x006D2B60`, `Tactical_layer_shroud_edges 0x006D3660`, `Tactical_layer_terrain_shadows 0x006D2DE0`, `Tactical_layer_base_terrain 0x006D3470`, `Tactical_layer_smudges 0x006D3290`, `Tactical_layer_building_overlays 0x006D3AC0`, `Tactical_layer_overlays 0x006D3040`, `Tactical_layer_animations 0x006D3870`.
- **The world↔screen↔cell transforms** — `Tactical::WorldToScreenSub 0x006D1EB0` (pure iso, no Z/scroll), `TacticalClass::CellToPixel 0x006D1FE0` (alias of WorldToScreenSub), `CoordsToClient 0x006D1F10` (+ Z + viewport scroll), `CoordsToClient2 0x006D2140` (+ camera offset `+0xB0/+0xB4`, returns visibility), `AdjustForZ 0x006D20E0` (Z leptons → screen-Y lift), inverse `TacticalScreenToCell 0x006D6590` (matrix-inverse + ≤180-iter height-correction loop, `0xB3` cap, + bridge neighbor shift).
- **Surface/buffer globals it drives (back-end owned by `frontier-blitter`)** — `g_CompositionSurface 0x0088731C`, `g_BackSurface 0x008872FC`, `g_ABuffer 0x0087E8A4` (16-bit shroud/fog alpha; 0x00=black … 0x7F=visible — WRITTEN only in Pass 1 Step 2, READ in Pass 2), `g_ZBuffer 0x00887644` (16-bit depth; written by terrain tiles, read by z-tested blitters), dirty-rect list `g_DirtyRectList`/count `0x00B0CE88`, clip extents `0x00B0CE30/34`.

---

## Key functions & globals (addresses) — re-verify `0x006D3D10` when Ghidra returns

| Symbol | Address | Role | Cross-doc convergence |
|---|---|---|---|
| `TacticalClass::Draw` (representative fn) | `0x006D3D10` | three-pass per-frame world draw entry | TACTICAL_RENDER_PIPELINE; GSCREEN_RTACTICAL §title; MAIN_TICK_RENDER_LOGIC §5; HIGH_BRIDGE_UNDER_DECK §3 — **4 independent verified docs** |
| `RenderFrame_main` | `0x004F4480` | frame driver; calls Draw 3× (params 0/1/2); composites chrome between Pass 1 and Pass 2 | GSCREEN_RTACTICAL §6; TACTICAL_RENDER_PIPELINE |
| `g_Tactical` | `0x00887324` | TacticalClass singleton (viewport+camera) | GSCREEN_RTACTICAL §1/§4 |
| `TacticalClass::AI/Update` | `0x006D2540` | **PerTickUpdate Rung Y** — camera scroll interp + view commit + radar-refresh (vtable[23]/+0x5C) | GSCREEN_RTACTICAL §title/§sources; LOGICCLASS spine §2 row 25 |
| `Tactical_ObjectRenderingLoop` | `0x006D8DB0` | main object renderer (Pass-2 step 8) — **owned by `drawing-helpers`**, called here | TACTICAL_RENDER_PIPELINE; LAYER_CLASS §1 |
| `Tactical::WorldToScreenSub` | `0x006D1EB0` | pure iso lepton→world-pixel | SPATIAL_PRIMITIVES §3; COORD_TRANSFORM_AUDIT §2 |
| `TacticalClass::CellToPixel` | `0x006D1FE0` | alias of WorldToScreenSub | SPATIAL_PRIMITIVES §3 |
| `CoordsToClient` | `0x006D1F10` | + Z + viewport scroll | SPATIAL_PRIMITIVES §3; COORD_TRANSFORM_AUDIT §2 |
| `CoordsToClient2` | `0x006D2140` | + camera offset, returns visibility | SPATIAL_PRIMITIVES §3 |
| `AdjustForZ` | `0x006D20E0` | Z lepton → screen-Y px (`ftol(z·0.14348 + (z≥728?1:0) + 0.5)`) | SPATIAL_PRIMITIVES §3 |
| `TacticalScreenToCell` (inverse) | `0x006D6590` | screen-px → cell; ≤180-iter height loop (`0xB3` cap) + bridge shift | TACTICAL_SCREEN_PIXEL_TO_CELL_INVERSE §7/§8 |
| `Tactical_layer_base_terrain` | `0x006D3470` | iso tile render (writes ZBuffer) | TACTICAL_RENDER_PIPELINE Pass1 Step4 |
| `Tactical_layer_shroud_edges` | `0x006D3660` | the **only** ABuffer writer (shroud+fog edges) | TACTICAL_RENDER_PIPELINE Pass1 Step2 |
| `Tactical_layer_overlays` | `0x006D3040` | walls / ore / tiberium overlays | TACTICAL_RENDER_PIPELINE Pass1 Step7 |
| `Tactical_layer_animations` | `0x006D3870` | flat ground-level anims | TACTICAL_RENDER_PIPELINE Pass1 Step8 |
| `Tactical_ZBufferDirtyClear` | `0x006D2B60` | dirty-rect ZBuffer reset | TACTICAL_RENDER_PIPELINE Pass1 Step1 |
| `g_CompositionSurface` / `g_BackSurface` | `0x0088731C` / `0x008872FC` | double-buffer pair | TACTICAL_RENDER_PIPELINE §Surface |
| `g_ABuffer` / `g_ZBuffer` | `0x0087E8A4` / `0x00887644` | shroud-alpha / depth circular buffers | TACTICAL_RENDER_PIPELINE §Surface |
| `MouseClass::Draw` (chrome, sibling) | `0x006D0A20` | sidebar/radar/UI drawn between Pass 1 & Pass 2 | GSCREEN_RTACTICAL §6 |

---

## Tick / render position

- **Render pass, out-of-sim.** Per-frame order (GSCREEN_RTACTICAL §6 / MAIN_TICK_RENDER_LOGIC, LOGICCLASS spine §1): `Main_Tick` → Input → `Process_Command` → `Map__Logic()` { command/event execution, cell/tiberium logic, **`RenderFrame_main`** } → state-hash record/verify → **`PerTickUpdate`** (the rung ladder) → postlude (frame-counter bump). So the world draw fires INSIDE `Map__Logic`, **before** the PerTickUpdate ladder for that frame.
- `RenderFrame_main 0x004F4480` calls `TacticalClass::Draw 0x006D3D10` **three times** (param 0 scroll, param 1 terrain, param 2 objects) with `MouseClass::Draw` (sidebar/radar/UI chrome) interleaved **between Pass 1 and Pass 2** — the single most important composition rule: chrome paints over terrain but beneath objects.
- **No sub-frame interpolation** (MAIN_TICK_RENDER_LOGIC §5, verified via `decompile_function 0x006D3D10`): positions read are integer cell/pixel values from TacticalClass fields; no alpha/lerp/fractional accumulator between sim ticks. One sim tick = one rendered frame's worth of positions.
- **The sim-side hook is Rung Y** (`TacticalClass::AI 0x006D2540`): camera-scroll interp, view commit, radar-refresh timer; internal early-out when the display-suppress guard (`DAT_00a8d5f8 & 2`) is set; per-frame dedup via `this+0xA8 == frame`. This rung draws **zero RNG** (wall-clock `timeGetTime` only) and is the only part of this service that is part of the deterministic per-tick spine.

---

## Depends-on (outgoing edges)

| Target slug | Via symbol / field | Evidence |
|---|---|---|
| `cell-map` | per-cell occupancy/layers/height/shroud read by every Pass-1 layer head (`0x006D3470` tile, `0x006D3660` shroud, `0x006D3040` overlays) via the isometric cell sweep; viewport (`g_Tactical`) sits beside MapClass in the display chain | The terrain passes iterate CellClass to render tiles/shroud/ore; the inverse transform `0x006D6590` re-corrects Y by the resolved cell's height level (`CellClass+0x11B`) and reads the bridge flag (`+0x140 & 0x100`). TACTICAL_RENDER_PIPELINE Pass1; SPATIAL_PRIMITIVES §4/§5. |
| `frontier-render-layer` | walks `g_DisplayLayers 0x008A0360` (5 LayerClass vectors) in Pass-2 step 8 via `Tactical_ObjectRenderingLoop 0x006D8DB0`; flat-anim layer `0x008A0390` walked in Pass-1 steps 6/8 | The object pass consumes the z-ordered draw list that LayerClass owns; only Layer 2 (Ground) is Y-sorted. LAYER_CLASS §1/§4; DISPLAYCLASS §7. |
| `abstract-object` | reads base ObjectClass vtable slots through the object loop: `GetRenderCoords`/`GetYSort`, `InWhichLayer`, `GetHeight`, `WhatAmI`, per-class `DrawIt`/`DrawShadow`/`DrawExtras` (`+0x104/+0x110/+0x10C`) | Pass-2 lays out / z-sorts / clips every sprite by walking the live object list and calling its ObjectClass render slots. (Loop internals owned by `drawing-helpers`; this service supplies the frame/projection context.) TACTICAL_RENDER_PIPELINE Pass2 Step8. |
| `lookup-tables` | AdjacentCell / iso coord tables for the world↔screen sweep; the `Math__ftol` truncation path in `AdjustForZ`/`CoordsToClient` (CW 0x0E7F) | The isometric projection (`0x006D1EB0`/`0x006D1FE0`/`0x006D20E0`) uses fixed 60/30-px cell constants + ftol rounding; cell-walk tables index neighbors. SPATIAL_PRIMITIVES §3; COORD_TRANSFORM_AUDIT §2. |
| `drawing-helpers` | calls `Tactical_ObjectRenderingLoop 0x006D8DB0` (the two-pass 5-layer renderer + blitter dispatch + FLH/pip/bracket math) in Pass-2 step 8; `DrawPixelFXSparkles 0x006D7840` in step 24 | The frame driver delegates the actual per-object sprite draw + decorations to the drawing-helpers substrate. TACTICAL_RENDER_PIPELINE Pass2; drawing-helpers profile §used-by. |
| `frontier-blitter` | locks/unlocks `g_CompositionSurface`/`g_BackSurface` via surface vtable (`+0x5C` lock, `+0x60` unlock); writes `g_ABuffer`/`g_ZBuffer`; circular-buffer scroll `CircBuf__Scroll 0x00410ED0` / `FUN_007BCB50`, fill `CircBuf__FillAll 0x004112D0` | Pass 0 manages the back-buffer swap + ABuffer/ZBuffer scroll/clear; all layer heads write through the surface/blitter back-end. TACTICAL_RENDER_PIPELINE Pass0/§Surface. |
| `random-scenario` | reads `ScenarioClass SpecialFlags & 0x1000` (FogOfWar) to gate fog darkening in shroud edge + object alpha; reads `ScenarioClass` placement/mode bytes for placement-ghost | The fog-darkening branch (Pass-1 shroud + Pass-2 object alpha) is gated on the FogOfWar Special bit, **OFF by default in YR** → DORMANT edge (only black shroud active). TACTICAL_RENDER_PIPELINE §ABuffer; spine Rung F note. |

---

## Used-by (incoming edges)

| Source slug | Via symbol | Evidence |
|---|---|---|
| `logicclass` | `Main_Tick 0x0055D360` → `Map__Logic()` → `RenderFrame_main 0x004F4480` → `TacticalClass::Draw 0x006D3D10`; and **PerTickUpdate Rung Y** `0x006D2540` ticks `g_Tactical->+0x5C` | The main loop invokes the world draw once per frame (inside Map__Logic, before the rung ladder) and ticks the tactical camera/scroll AI as spine Rung Y. Render is downstream of sim state, not part of the hash. LOGICCLASS spine §1/§2; MAIN_TICK_RENDER_LOGIC. |
| `frontier-input-command` | screen→cell hit-testing routes through this service's inverse transform `TacticalScreenToCell 0x006D6590` (+ `CoordsToClient` family for cursor/legal-action resolution) | `DisplayClass::DetermineAction` / cursor resolution converts the mouse pixel to a cell via this service's projection before deciding what order a click issues. SPATIAL_PRIMITIVES §4; TACTICAL_SCREEN_PIXEL_TO_CELL_INVERSE. |
| `frontier-radar` | minimap click→viewport recenter writes `g_Tactical` viewport target; radar event markers drawn in Pass-2 steps 20/21 (`DrawRadarOverlays_Normal 0x0063B0A0` / `_Fog 0x0063B150`) reading the ABuffer | Minimap clicks recenter THIS service's viewport; radar event overlays composite into the object pass using its ABuffer fog alpha. MINIMAP_CLICK_DRAG_INVERSE_TRANSFORM; TACTICAL_RENDER_PIPELINE Pass2 20/21. |
| `frontier-sidebar` | `MouseClass::Draw 0x006D0A20` (sidebar/radar/power/credits/command bar) composited between this service's Pass 1 (terrain) and Pass 2 (objects) | The chrome draw is sandwiched inside the three-pass sequence this service defines; the ordering contract is shared. GSCREEN_RTACTICAL §6. |
| `frontier-render-layer` | this service is the sole consumer that gives LayerClass membership meaning — it walks the 5 vectors in z-order each frame | LayerClass churn is render-side bookkeeping; the tactical object pass is what turns it into pixels. LAYER_CLASS §4/§9. |

---

## Active-in-YR vs TS-legacy

- **Active every frame of every skirmish:** the whole three-pass pipeline + all 8 terrain layers + the object pass + transforms. No INI key configures screen composition, viewport size, or pass selection — all layout is hardcoded per resolution (640/800/1024+), all three passes run unconditionally (GSCREEN_RTACTICAL §9).
- **Rung Y (`0x006D2540`) is active every tick** (camera scroll + radar refresh); only the display-suppress guard (`DAT_00a8d5f8 & 2`) pauses it. Draws 0 RNG.
- **DORMANT / gated (kept in the order, no effect in stock YR):** the FogOfWar darkening branch in shroud-edge + object alpha (Special `0x1000` OFF by default in YR) — only black shroud is active; `ShroudGrow=no` so the per-tick shroud-creep rung (D) that would dirty cells here never fires. These are the same TS-legacy gates the LogicClass spine flags for Rungs D/F. The draw-side branch exists but is unreached in a normal skirmish.
- **No TS-legacy in the core composition path itself** — `GScreenClass`/`TacticalClass` orchestration is not gated; it runs every frame.

---

## Open / unverified edges

- **Representative address live-re-verification PENDING.** `0x006D3D10` was NOT freshly decompiled this session (Ghidra MCP unreachable). It is cross-doc-convergent across 4 verified reports; re-run `decompile_function 0x006D3D10` + `get_function_callers 0x006D3D10` (expect `RenderFrame_main 0x004F4480`) to close this.
- **Isometric camera matrix offset map (`lookup-tables`/`cell-map`).** TacticalClass writes 12+ floats from `+0xDE4`; `FUN_005AEA10` (matrix scale) may also write earlier offsets. Full matrix layout + whether RA2's projection differs from the Rust one by sub-pixel amounts is UNCHECKED (GSCREEN_RTACTICAL §12 OQ1; COORDINATE_SYSTEM_GAMEMD may cover it).
- **`Rules+0x50` → `ScrollPeriod` (+0xDAC) INI key** unidentified (likely `[General]AutoScrollPeriod` or similar) — GSCREEN_RTACTICAL §12 OQ5.
- **Recursive `RenderFrame_main` finalize call** (`vtable[15]` re-entry with `redrawFlags=0`) — a chrome-only finalize pass; second-entry call stack UNCHECKED (GSCREEN_RTACTICAL §12 OQ4). Not parity-critical for the Rust full-repaint model.
- **`drawing-helpers` boundary overlap.** The Pass-2 object-loop internals are documented under `drawing-helpers`; the split is "this service = frame driver + terrain + buffers + transforms, drawing-helpers = per-object loop + blitter". A reader should follow `drawing-helpers` for sprite-level detail and this profile for frame-level orchestration. No factual conflict, but the two profiles share `0x006D8DB0` / `0x006D3D10` as the seam.
- **Rust port divergence (intentional, GSCREEN_RTACTICAL §11):** Rust uses a wgpu GPU depth pipeline + full-viewport repaint, so it does NOT reproduce the ABuffer/ZBuffer circular scroll, the dirty-rect ring, the RedrawFlag partial-repaint selection, or the chrome-between-passes surface sandwich (GPU z-buffer handles layering). Parity is on **draw order + projection result + occlusion**, not the buffer plumbing. Cinematic scroll interpolation (`+0xD0..+0xDC` + `FUN_006D8B30` listener) is NOT implemented Rust-side (map-ping snap-to-target doesn't animate).
