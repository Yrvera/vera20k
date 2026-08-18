# frontier-sidebar — Build sidebar / cameo strips (core-services-map profile)

**Slug:** `frontier-sidebar`
**Status:** PROMOTED from catalog stub (`_frontier.md` §B1) to full profile.
**Layer:** ui-render (HUD draw + input front-end; out-of-sim, render-pass-driven).
**Authority order:** binary → Ghidra → docs.

> **Session verification caveat (READ FIRST).** The Ghidra MCP server was **NOT reachable
> this session** — `list_instances` returned 0 UDS instances and the TCP fallback
> (`127.0.0.1:8089`) refused the connection; `check_tools` reports `decompile_function` /
> `get_function_by_address` / `search_functions` all `not_found`. I therefore **could not
> re-decompile any address live this session.** Every address below is marked with its
> verification provenance: **[CORPUS]** = corroborated across one or more
> `[ghidra/verified]` research docs (often with the original verifying call cited inline in
> that doc), **[CORPUS×N]** = independently cited in N separate verified docs. None are
> **[LIVE-2026-06-29]**. The representative address `SidebarClass::Draw @ 0x006A6C30` is
> **[CORPUS×8]** — the single most-cited sidebar address in the corpus — so its identity is
> high-confidence, but per RE discipline it is *not* re-verified against the binary today.
> A live re-pass should be run before this profile is treated as load-bearing for code.

---

## PURPOSE

The right-hand in-game HUD build bar. Owns: per-tab cameo strips (building / defense /
infantry / vehicle-aircraft / naval), production-progress overlay, the "new buildable"
cameo + tab flash, scroll buttons + smooth/page scroll, repair/sell/power-toggle gadgets,
cameo hit-testing → network production commands, tooltips, tab switching, and final blit of
the sidebar surface to screen. It also hosts (by class inheritance) the power bar, credits
text, and the radar/minimap composition.

This is a **render + input** service, not a sim service. Sim state (FactoryClass progress,
HouseClass buildable set) is *read* each frame to drive cameo content/flash; player clicks
are *converted to EventClass network commands* and handed to the lockstep queue. The
sidebar holds no authoritative gameplay state.

---

## CLASS HIERARCHY & OWNERSHIP

RTTI-confirmed mega-class chain (from `SIDEBAR_SYSTEM_GHIDRA_REPORT.md` §1, ghidra/verified):

```
GScreenClass → MapClass → DisplayClass → RadarClass → PowerClass
  → SidebarClass → TabClass → ScrollClass → MouseClass
```

`SidebarClass` is **not** a standalone object — it is a band of fields
(`+0x1544 .. +0x5515`) inside the single global GScreen mega-instance at `0x0087F7E8`
(whole object 0x556C bytes, static ctor `0x0040D190`). `SidebarClass` ctor `0x006A4F20`
[CORPUS], primary vtable `0x007F3058` (55 slots). This means "the sidebar" and "the radar"
and "the tactical map" are the **same C++ object** at different inheritance bands — the
sidebar's radar/power coupling is *inheritance*, not aggregation.

### State it owns (globals/structs, addresses from `SIDEBAR_SYSTEM_GHIDRA_REPORT.md` §26 [CORPUS])

| Global | Role |
|---|---|
| `0x0087F7E8` | the GScreen/Sidebar mega-instance (sidebar fields `+0x1544..+0x5515`) |
| `0x00886f90..0x00886fac` | viewport/sidebar boundary rect (SidebarX/Width/TopClip/BottomY) |
| `0x00886f94` | `SidebarWidth = 0x9E = 158px` (hardcoded) |
| `0x00880d48 + N×0xF94` | per-strip state arrays (4 strips) — `StripClass` instances |
| `0x00b07c48..0x00b07dc8` | 4 tab button gadgets (stride 0x60) |
| `0x00b07e80..0x00b0b300` | 240 `SelectClass` cameo click zones (4 strips × 60 × 0x38) |
| `0x00b0b468..0x00b0b484` | sidebar chrome SHP ptrs (SIDE1/2/3/ADDON/clock) |
| `DAT_00887300` | `SidebarSurface` ptr (168 × screen_height, the draw target) |
| `DAT_00887368` | tooltip singleton |
| `DAT_00a8b230 + 0x34B8` | NewSidebar (YR-mode) flag — `!= 0` ⇒ YR layout |
| `CameoEntry` (stride 0x34) | per-slot build item: `+0x88` = `FlashEndFrame`, progress, status, RTTI+TypeIndex key |

`CameoEntry.FlashEndFrame` (strip + slot×0x34 + 0x88) is the absolute frame counter at
which the new-buildable pulse stops; `StripClass::Draw` reads it, `InsertEntry` /
`Recalculate` zero it (per `CAMEO_FLASH_END_FRAME_WRITER_GHIDRA_REPORT.md` [CORPUS]).

---

## KEY FUNCTIONS (all addresses [CORPUS] — see caveat; not re-verified live this session)

| Address | Name | Role | Provenance |
|---|---|---|---|
| `0x006A6C30` | `SidebarClass::Draw` (vtable slot 16) | **REPRESENTATIVE** — main per-frame sidebar compositor; calls `StripClass::Draw` → `PowerClass::Draw` → blit | [CORPUS×8] |
| `0x006A9540` | `StripClass::Draw` (4210 bytes, largest) | draws all cameos for one strip: chrome, cameo art, progress overlay, ready text, flash | [CORPUS×6] |
| `0x006A8B30` | `StripClass::AI` (1938 bytes) | per-frame strip update: scroll animation, build-progress poll, tab-flash scheduling, auto-deliver, `EVA_ConstructionComplete` | [CORPUS×5] |
| `0x006A6300` | `SidebarClass::AddCameo` | inserts a new buildable into a strip + fires `EVA_NewConstructionOptions` (`@0x006a6415`) + sets tab-flash state | [CORPUS×3] |
| `0x006A8710` | `StripClass::InsertEntry` | comparator-sorted insertion into the persistent `CameoEntry` array; exact (RTTI,TypeIndex) duplicate rejection; zeros FlashEndFrame | [CORPUS×2] |
| `0x006A8420` | `CompareItems` (748 bytes) | build-list sort order (super-last, factory-match, tech level, cost, UIName) | [CORPUS] |
| `0x006A6140` | `Sidebar_UpdateFromProduction` | **incoming sim hook** — production start/change → rebuild local-player sidebar; no FlashEndFrame write | [CORPUS×3] |
| `0x006A7780` | `SidebarClass::Action` (vtable slot 10, 1428 bytes) | sidebar input/gadget dispatch (tabs, scroll, repair, sell, power) | [CORPUS×2] |
| `0x006AAD00` | `SelectClass::Action` | **cameo click handler** → emits network cmd 0x0E (begin), 0x0F (suspend), 0x10 (cancel-one) | [CORPUS×2] |
| `0x006A70E0` | `SidebarClass::BlitToScreen` (952 bytes) | blit `SidebarSurface` → primary surface | [CORPUS] |
| `0x006A7590` | `SwitchTab` (342 bytes) | switch active build tab | [CORPUS] |
| `0x006AA600` | `StripClass::Recalculate` (1711 bytes) | remove completed/cancelled cameos; re-sort | [CORPUS] |
| `0x0069DFC0` | tab-flash scheduler (`Start_Flash`) | 10-tick on/off pulse on a tab gadget for charged-super / completed-aircraft | [CORPUS] |
| `0x006AC480` | `DrawCameoText` | cameo name / ready text (font path A, no fade) | [CORPUS] |
| `0x006AC210` | `ResolveTooltip` / Description (vtable slot 33) | tooltip text for any sidebar widget | [CORPUS] |

> **Stub reconciliation.** The `_frontier.md` §B1 stub named `SidebarClass__Action
> @ 0x006A7780` as the "action/hit-test" fn. That address is correct for the **sidebar-level**
> input dispatcher (tabs/scroll/repair/sell), but the **cameo** hit-test that produces
> production commands is `SelectClass::Action @ 0x006AAD00` (per `BUILD_QUEUE_GHIDRA_REPORT.md`
> and `SIDEBAR_SYSTEM` §15, which notes the entry was corrected from a mid-body
> `0x006AB970` to `0x006AAD00` via `get_function_by_address`). Both are sidebar-owned;
> this profile lists both. All four stub addresses (`0x006A6C30`, `0x006A6300`,
> `0x006A6140`, `0x006A7780`) are corroborated as-claimed.

---

## PLUG POINT (render / tick)

**Primary: the RENDER pass, out-of-sim.** The sidebar's draw entry is reached from the
sidebar compositor `MainGame_SidebarDraw @ 0x006D0A30` (per
`SIDEBAR_POWER_CREDITS_READY_TEXT_LAYOUT_GHIDRA_REPORT.md` [CORPUS]: "draws the
sidebar/radar background, calls `CreditsClass__Draw`, then continues sidebar composition" →
reaches `SidebarClass::Draw @ 0x006A6C30`). The frame-level retained-surface copy is driven
from `RenderFrame_main @ 0x004F44F0` (per `TOOLTIP_MANAGER_SIDEBAR_OVERLAP_PIXELS`
[CORPUS]). This is **NOT** a `LogicClass::PerTickUpdate` rung — it runs in the frame/render
driver after the sim tick, like `TacticalClass_Draw @ 0x006D3D10`.

**Per-tick coupling points (the sidebar reacts to the sim, it does not own a tick rung):**
- `StripClass::AI @ 0x006A8B30` runs the sidebar's per-frame state machine (scroll/flash/
  progress). It is invoked from the sidebar `Action`/periodic-update path, not from a
  `PerTickUpdate` `+0x5C` AI head — the sidebar is a render-side singleton, not an
  `ObjectClass` in the LogicClass live vectors.
- `Sidebar_UpdateFromProduction @ 0x006A6140` is the **sim → sidebar** rebuild hook, called
  from production state changes (see incoming edges). It uses the general redraw-dirty
  helper `0x004f42f0` (per spine rung-2 doc, which lists `Sidebar_UpdateFromProduction`
  among the ~90 callers of that redraw helper) — i.e. it rides the redraw/dirty path, RNG-
  neutral, no `PerTickUpdate` rung of its own.

**Spine rung tie-in:** the sidebar has **no dedicated rung** in the 28-rung
`LogicClass::PerTickUpdate` spine. Its content is updated as a *side effect* of the sim
rungs that mutate production/house state (notably the factory/house production rungs and
building unlimbo/limbo lifecycle), which call `Sidebar_UpdateFromProduction` and set the
`HouseClass+0x1FC` ProductionDirty flag. The actual sidebar paint then happens in the
post-tick render pass via `MainGame_SidebarDraw → SidebarClass::Draw`.

---

## OUTGOING EDGES (this service depends on …)

| → Service | Via symbol / mechanism | Evidence |
|---|---|---|
| `frontier-render-tactical` | shares the `SidebarSurface` (`DAT_00887300`) and the DirectDraw primary; `SidebarClass::BlitToScreen @ 0x006A70E0` blits onto the same render frame; viewport rect (`0x00886fa8` tactical width = screen − 158) is the sidebar's complement | `SIDEBAR_SYSTEM` §3/§28; `MainGame_SidebarDraw @ 0x006D0A30` |
| `frontier-radar` | inheritance: `RadarClass` is a base of `SidebarClass`; `SidebarClass::Draw` → `PowerClass::Draw` → `RadarClass::Draw @ 0x00653100` in the same composition pass | `SIDEBAR_DRAW_COMPOSITION_ORDER…` finding 4; `SIDEBAR_ODD_STATE_OVERLAP_STACK` sources [CORPUS] |
| `frontier-audio-eva` (VoxClass) | `SidebarClass::AddCameo @ 0x006a6415` calls `VoxClass__PlayEVA(EVA_NewConstructionOptions)`; `StripClass::AI @ 0x006a8e2f` calls `EVA_ConstructionComplete`; `StripClass::AddEntry @ 0x006a8837` calls `EVA_NewConstructionOptions` | `EVA_SYSTEM_DEEP_DIVE` call-site table [CORPUS] |
| `frontier-net-eventqueue` | `SelectClass::Action @ 0x006AAD00` cameo clicks emit network cmds 0x0E (begin), 0x0F (suspend), 0x10 (cancel-one); sell click → cmd via `EventClass::Execute` | `BUILD_QUEUE_GHIDRA_REPORT` §"Sidebar Click → Network Command" [CORPUS] |
| `factory-house` (studied) | reads `FactoryClass::GetProgress @ 0x004CA120` (StageClass.Value 0..0x36) for the progress overlay; reads `IsComplete @ 0x004CA130`; reads `Production_HasChanged` (+0x28) each strip AI; buildable set gated by `HouseClass::CanBuild @ 0x004F7870`; `HouseClass+0x1FC` ProductionDirty triggers rebuild | `BUILD_QUEUE`, `STAGECLASS_FRAME_ADVANCE`, `BUILDINGCLASS_PREREQUISITES` [CORPUS] |
| `rules-class` (studied) | cameo art/UIName/cost/TechLevel/Cameo= keys feeding `CompareItems @ 0x006A8420` sort and cameo display come from TechnoTypeClass (parsed from rulesmd/artmd) | `SIDEBAR_SYSTEM` §27; `TECHNOTYPECLASS_BASE` [CORPUS] |
| `drawing-helpers` (studied) | `CC_Draw_Shape`/`DrawSHP @ 0x004aed70`, `DrawText @ 0x004a60e0`, font path `FUN_00434CD0` for cameo/ready text; `AlphaBlendRect @ 0x00621b80` for dark strips | `BITFONT_SHELL_TEXT` 3.9b; `ALPHABLENDRECT_0xAF…`; `SIDEBAR_SYSTEM` §25 [CORPUS] |
| `lookup-tables` (studied) | palette/ConvertClass remap (`DAT_0087f6cc` SIDEBAR.PAL chrome, CAMEO.PAL cameos) for the 8→16-bit cameo blit | `SIDEBAR_CAMEO_CHROME_CONVERTCLASS_SETUP_0052BA60`; `SIDEBAR_LAYER_PALETTE…` [CORPUS] |
| `frontier-super` (SuperClass) | super-weapon cameos read `Super::GetProgressFrame @ 0x006cbee0` (0..0x36) and `Super::NameReadiness @ 0x006cc2b0` for status text; charged super triggers tab flash via `StripClass::AI` | `SIDEBAR_SYSTEM` §25; `SIDEBAR_TAB_FLASH_SCHEDULER` [CORPUS] |
| `gadget-dialog` (studied) | sidebar buttons are `GadgetClass`-derived (`SBGadgetClass`, `SelectClass`); hit-test via `GadgetClass::Hit_Test @ 0x004e15a0`, input via `GadgetClass::Input @ 0x004e1640` | `SIDEBAR_SYSTEM` §25 [CORPUS] |

## INCOMING EDGES (… depends on this service)

| ← From service | Via symbol / mechanism | Evidence |
|---|---|---|
| `factory-house` / production | `HouseClass::Begin_Production @ 0x004FA350` calls `Sidebar_UpdateFromProduction @ 0x006A6140`; `FactoryClass::AI` / build-speed path calls it on rate change | `CAMEO_FLASH_END_FRAME_WRITER`; `FACTORY_CLASS_BUILD_SPEED` line 537 [CORPUS] |
| `factory-house` (BuildingClass lifecycle) | building Unlimbo/OnDestroyed/OnSpyInfiltrate set `HouseClass+0x1FC` ProductionDirty → sidebar rebuild on next update | `BUILDINGCLASS_PREREQUISITES` §8; `BUILDING_SYSTEMS` line 46 [CORPUS] |
| `frontier-input-command` | mouse/keyboard input enters via `GScreenClass::Input @ 0x004f4320` → `DisplayClass::Input @ 0x006922e0` → sidebar `Action`/`SelectClass::Action`; tab-switch / scroll hotkeys reach `SwitchTab` | `SIDEBAR_SYSTEM` §15/§25 [CORPUS] |
| `frontier-render-tactical` (compositor) | `MainGame_SidebarDraw @ 0x006D0A30` and `RenderFrame_main @ 0x004F44F0` invoke `SidebarClass::Draw` and copy the sidebar surface each frame | `SIDEBAR_POWER_CREDITS_READY_TEXT_LAYOUT`; `TOOLTIP_MANAGER_SIDEBAR_OVERLAP_PIXELS` [CORPUS] |

---

## ACTIVE-IN-YR / TS-LEGACY

- **Core sidebar (chrome, cameo strips, progress overlay, scroll, hit-test, EVA cues,
  radar/power composition): ACTIVE in standard YR.** Fires every match, continuously.
- **YR "NewSidebar" layout** (`DAT_00a8b230 + 0x34B8 != 0`) is the live YR path; the
  `== 0` branch is the classic RA2 layout (different layout constants). YR uses the
  NewSidebar branch.
- **Tab-flash SHP (new-buildable tab animation via `DAT_00B0B478`): DORMANT in YR.**
  `DAT_00B0B478` is never loaded in stock YR (per `SIDEBAR_TAB_FLASH_SCHEDULER` /
  `SIDEBAR_CONSTRUCTION` §10), so the per-tab SHP flash on new-buildable arrival does not
  play — the *cameo* darken-pulse and the charged-super/aircraft tab pulse
  (`0x0069DFC0`) DO run. Surface honestly: this is a code path present but visually inert
  for the new-buildable case in stock YR.
- **`CameoEntry.FlashEndFrame` new-item cameo pulse:** the reader (`StripClass::Draw`)
  and zero-setters exist, but **no nonzero writer was found** in the corpus
  (`CAMEO_FLASH_END_FRAME_WRITER` — exhaustive). Effect is therefore inert/never-positive
  in stock YR unless a writer is located in a live re-pass. Treat as UNCHECKED-but-likely-
  dormant, not as a confirmed live flash.
- **Observer-mode strip branch** (player ptr == observer ptr `DAT_00AC1198`): a real
  alternate branch in `StripClass::Draw`; active only in observer/spectator, not normal
  skirmish.
- No Tiberian-Sun-only sidebar mechanics identified in this pass (the RA2-vs-YR split is a
  live runtime flag, not dead TS code).

---

## REMAINING UNCERTAINTY / RE-PASS TODO

1. **Live Ghidra re-verification of every address above** — none were re-decompiled this
   session (MCP offline). Highest priority: re-confirm `SidebarClass::Draw @ 0x006A6C30`,
   `Sidebar_UpdateFromProduction @ 0x006A6140`, and `SelectClass::Action @ 0x006AAD00`.
2. **Locate the nonzero writer of `CameoEntry.FlashEndFrame`** (or confirm none exists) to
   settle whether the new-item cameo pulse is observable in stock YR.
3. **Exact `MainGame_SidebarDraw → SidebarClass::Draw` call edge** — confirmed by corpus
   inference (the doc says it "continues sidebar composition"); a live decompile of
   `0x006D0A30` should show the direct call/branch to `0x006A6C30`.

## SOURCES (existing verified docs cross-referenced this session)

- `docs/research/SIDEBAR_SYSTEM_GHIDRA_REPORT.md` (class hierarchy, vtable, function ref §25, globals §26)
- `docs/research/SIDEBAR_DRAW_COMPOSITION_ORDER_AND_SURFACE_ORIGIN_GHIDRA_REPORT.md`
- `docs/research/SIDEBAR_ODD_STATE_OVERLAP_STACK_GHIDRA_REPORT.md`
- `docs/research/SIDEBAR_POWER_CREDITS_READY_TEXT_LAYOUT_GHIDRA_REPORT.md` (MainGame_SidebarDraw plug)
- `docs/research/SIDEBAR_TIMING_AND_TOOLTIPS_GHIDRA_REPORT.md`
- `docs/research/SIDEBAR_TAB_FLASH_SCHEDULER_GHIDRA_REPORT.md`
- `docs/research/SIDEBAR_ADDCAMEO_INSERTENTRY_ORDER_STATUS_GHIDRA_REPORT.md`
- `docs/research/CAMEO_FLASH_END_FRAME_WRITER_GHIDRA_REPORT.md`
- `docs/research/BUILD_QUEUE_GHIDRA_REPORT.md` (click → network command)
- `docs/research/EVA_SYSTEM_DEEP_DIVE_GHIDRA_REPORT.md` (AddCameo/StripAI EVA call sites)
- `docs/research/FACTORY_CLASS_BUILD_SPEED_GHIDRA_REPORT.md` (Sidebar_UpdateFromProduction caller)
- `docs/research/BUILDINGCLASS_PREREQUISITES_GHIDRA_REPORT.md` (ProductionDirty rebuild)
- `docs/research/STAGECLASS_FRAME_ADVANCE_PRIMITIVE_GHIDRA_REPORT.md` (progress = StageClass.Value)
- `docs/research/core-services-map/_spine-rung-2.md` (redraw-dirty helper caller list)
- `docs/research/core-services-map/_frontier.md` §B1 (seed stub)
