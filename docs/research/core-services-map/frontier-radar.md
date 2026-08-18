# frontier-radar — Radar / minimap

**Service slug:** `frontier-radar`
**Status:** promoted from catalog stub (`_frontier.md` §B2) to full profile.
**Layer:** ui-render (sidebar HUD; render-side, sim-fed)
**Manager / global:** `RadarClass` — a base in the GScreen → Display → Radar → Tactical
single-inheritance chain. The live in-game radar object IS the same singleton that owns the
tactical viewport (`g_Tactical`); `RadarClass` is the radar/minimap slice of that vtable
chain, not a separate object.

> **Verification provenance (READ THIS).** Ghidra MCP was **unreachable this session**
> (`list_instances` returned 0; TCP `127.0.0.1:8089` refused; no alternate port answered).
> Per CLAUDE.md authority order (binary → Ghidra → docs), I fell back to the **docs tier**:
> every address below is **corroborated across multiple independent `ghidra/verified`
> research docs** that each cite their own decompile + disassembly-range evidence inline.
> These addresses are therefore **CORROBORATED-BY-VERIFIED-DOCS, not re-verified live this
> session.** Where the stub and the verified docs agree, I mark VERIFIED-IN-DOCS; where I
> found additional/corrected detail, I note it. A future session with live Ghidra should
> re-confirm the representative address (`RadarClass__Draw @ 0x00653100`) with one
> `decompile_function` call to upgrade this to live-verified.

---

## PURPOSE

The minimap in the sidebar: it builds a generated colour surface for the whole map (terrain
+ overlay/tiberium), overlays object dots from a per-pixel tracker, applies shroud (black) /
fog (half-bright) per cell, draws radar-event ping diamonds and spy-satellite vision, draws
the viewport camera-window rectangle, and blits the changed region into the shared sidebar
surface. It also owns the click-to-recenter / minimap-drag input path (the radar object's own
vtable input handler), radar mode/open-close transition animation, and the radar online/offline
(power-gated, jammed) state.

It is **render-side, sim-fed**: the minimap content is regenerated from sim state
(`CellClass` terrain, object reveal/conceal, fog/shroud bits, radar events created by combat /
production / triggers), but the minimap itself produces no gameplay output — it is pure HUD.

---

## WHAT IT OWNS (globals / structs, with addresses)

`RadarClass` instance fields (offsets verified-in-docs via `RADAR_SURFACE_SIZING_ZOOM_SAMPLING`,
`RADAR_GENERIC_TERRAIN_PIXEL_DIRTY_PIPELINE`, `MINIMAP_GENERATED_PIXEL_COLOR_PIPELINE`,
`SOVIET_RADAR_MINIMAP_CONTENT_INSET`):

| Field | Role | Source doc |
|---|---|---|
| `+0x11E4 / +0x11EC` | sidebar-surface chrome origin (radar frame `0x20` draw anchor, normally `(0,48)`) | content-inset / generated-pixel |
| `+0x11F0 / +0x11F4` | sidebar-local minimap aperture base, normally `(16,49)` | surface-sizing |
| `+0x120C..+0x1218` | accumulated sidebar-local dirty rect (x,y,w,h) | terrain-dirty-pipeline |
| `+0x121C` | primary / live display DSurface (16-bit packed) — final minimap pixels | generated-pixel §1 |
| `+0x1220` | secondary / generated terrain BSurface (16-bit packed) | generated-pixel §1 |
| `+0x1228 / +0x1234` | terrain dirty-cell dedup list | terrain-dirty-pipeline |
| `+0x123C` | raw RGB terrain buffer (3 bytes / radar-space pixel) | generated-pixel §1 |
| `+0x1260 / +0x126C` | pixel dirty list | terrain-dirty-pipeline |
| `+0x1274` | pixel dirty visited bitfield | terrain-dirty-pipeline |
| `+0x14D9` | "radar dirty, needs update" flag | terrain-dirty-pipeline |
| `+0x14AC / +0x14B0` | radar active/open state (content blit gate, `==1 && ==1`) | generated-pixel §10 |
| `+0x14DA` | force-full-radar-redraw flag (movie/transition) | generated-pixel §11 |

`RadarClass` vtable: `vtable_RadarClass @ 0x007F0320` (constructor `RadarClass__constructor
@ 0x00652960` installs it; verified-in-docs `MINIMAP_GADGETCLASS_CLICK_PROVENANCE` via
`read_memory 0x007F0320`). Key slots: `+0x18` → radar input handler `0x006539D0`; `+0x48` →
`GScreenClass__Input @ 0x004F4320`; `+0x4C` → `Minimap_Chat_Dispatch @ 0x00653850`.

Module-scope radar globals (verified-in-docs `MINIMAP_GADGETCLASS_CLICK_PROVENANCE` §3.3,
`RADAR_EVENT_PING_PIXEL_SHAPES_COLORS`):
- radar surface origin/size: `DAT_00880C84` (origin x), `DAT_00880C88` (origin y),
  `DAT_00880C8C` (w), `DAT_00880C90` (h) — used by both the click handler and event draw.
- radar-event array + count (`g_RadarEventCount`), iterated `0..count-1` ascending.
- the destination is the shared `g_SidebarSurface` (owned by `frontier-sidebar`, written here).

The object **tracker** is a 256-bucket hash owned by `RadarClass`
(`AddObjectToTracker @ 0x00655560`): `bucket = (pixel_x + pixel_y * -5) & 0xFF`; local-player
objects inserted at front, others appended; 16-byte entries `{object,x,y}`.

---

## KEY FUNCTIONS (addresses corroborated-by-verified-docs)

Representative function (the stub's `RadarClass__Draw @ 0x00653100`) — **VERIFIED-IN-DOCS,
confirmed as the per-frame radar draw entry** by five independent verified reports
(`MINIMAP_GENERATED_PIXEL_COLOR_PIPELINE`, `SIDEBAR_ODD_STATE_OVERLAP_STACK`,
`RADAR_EVENT_PING_PIXEL_SHAPES_COLORS`, `RADAR_GENERIC_TERRAIN_PIXEL_DIRTY_PIPELINE`,
`MINIMAP_GADGETCLASS_CLICK_PROVENANCE`). It is **not** PerTickUpdate; it is reached from the
sidebar draw pass (see Plug Point). The stub address is **correct**.

| Function | Address | Role | Provenance |
|---|---|---|---|
| `RadarClass__Draw` | `0x00653100` | per-frame radar draw entry; mode dispatch; calls `Update` | VERIFIED-IN-DOCS (representative) |
| `RadarClass__Update` | `0x00656EC0` | dirty-driven content update + ordered overdraw + sidebar blit | VERIFIED-IN-DOCS |
| `RadarClass__GenerateTerrainSurface` | `0x006547C0` | builds raw-RGB + secondary 16-bit terrain surface | VERIFIED-IN-DOCS |
| `RadarClass__FillTerrainColors` | `0x00654EA0` | per-cell terrain colour fill (calls `CellClass__GetRadarColor`) | VERIFIED-IN-DOCS |
| `RadarClass__RenderCellPixel` | `0x00655C50` | final per-pixel writer (object/fog/shroud/terrain order) | VERIFIED-IN-DOCS |
| `RadarClass__RebuildRadarSurfaces` | `0x00654650` | (re)sizes primary/secondary surfaces, aspect-fit | VERIFIED-IN-DOCS |
| `RadarClass__AddObjectToTracker` | `0x00655560` | 256-bucket object tracker insert | VERIFIED-IN-DOCS (stub: trackers) |
| `RadarClass__RemoveObjectFromTracker` | `0x00655740` | tracker remove + dirty | VERIFIED-IN-DOCS |
| `RadarClass__GetObjectAtRadarPixel` | `0x00656750` | reverse lookup for click/tooltip | VERIFIED-IN-DOCS |
| `RadarClass__MarkTerrainDirty` | `0x006551C0` | terrain dirty-cell dedup | VERIFIED-IN-DOCS |
| `RadarClass__MarkCellDirty` | `0x006562D0` | pixel dirty-cell dedup | VERIFIED-IN-DOCS |
| `RadarClass__RefreshRadar` | `0x00657CE0` | full per-pixel repaint (gap/spysat/reveal) | VERIFIED-IN-DOCS |
| `RadarClass__One_Time` | `0x00652CF0` | init aperture/chrome fields | VERIFIED-IN-DOCS |
| `RadarClass__Init_For_House` | `0x00652E90` | per-house aperture init | VERIFIED-IN-DOCS |
| `RadarClass__PerFrameMovieUpdate` | `0x006579E0` | radar mode-3 open/close transition movie | VERIFIED-IN-DOCS |
| `RadarClass__constructor` | `0x00652960` | installs `vtable_RadarClass @ 0x007F0320` | VERIFIED-IN-DOCS |
| radar input handler (vtable+0x18) | `0x006539D0` | click-to-recenter / minimap-drag owner | VERIFIED-IN-DOCS |
| `CellClass__GetRadarColor` | `0x0047C060` | per-cell terrain/overlay radar RGB source | VERIFIED-IN-DOCS |
| `OverlayClass__GetRadarColor` | `0x005FED00` | overlay/tiberium colour override | VERIFIED-IN-DOCS |
| `GetTiberiumRadarColor` | `0x0069E860` | tiberium colour from SHP frame metadata | VERIFIED-IN-DOCS |
| `CreateRadarEvent` | `0x0065FA70` | enqueue a radar-event ping | VERIFIED-IN-DOCS (stub) |
| `TickAndDrawRadarEvents` | `0x00660000` | per-update event tick+draw | VERIFIED-IN-DOCS |
| `DrawRadarEvent` | `0x00660050` | draw one rotating 4-edge ping diamond | VERIFIED-IN-DOCS (stub) |
| `ComputeViewportCorners` | `0x00660730` | corner generator (events + viewport rect) | VERIFIED-IN-DOCS |
| `TechnoClass__RegisterOnRadar` | `0x0070CC90` | per-techno tracker registration (one pixel) | VERIFIED-IN-DOCS |
| `BuildingClass__RegisterOnRadar` | `0x00456580` | per-building tracker registration (every footprint pixel) | VERIFIED-IN-DOCS |

> **Stub correction / addition.** The stub's incoming-edge line named only
> `BuildingClass__RegisterOnRadar @ 0x00456580`. The verified docs show registration is
> **virtual / object-driven via TWO entries**: `TechnoClass__RegisterOnRadar @ 0x0070CC90`
> (one pixel) and `BuildingClass__RegisterOnRadar @ 0x00456580` (whole footprint). Both
> addresses corroborated; the techno entry was missing from the stub.

---

## PLUG POINT (render pass, not PerTickUpdate)

**Not on a PerTickUpdate rung.** The radar runs in the **render pass**, nested inside the
sidebar draw, as the verified chain (`SIDEBAR_ODD_STATE_OVERLAP_STACK` §2):

```
RenderFrame_main @ 0x004F4580            (frame compositor, out-of-sim)
  └─ SidebarClass__Draw @ 0x006A6C30     (sidebar-local surface)
       └─ PowerClass__Draw @ 0x0063FB20  (always calls RadarClass::Draw)
            └─ RadarClass__Draw @ 0x00653100   ← THIS SERVICE
                 └─ RadarClass__Update @ 0x00656EC0
```

So the minimap draws **after** the build-strip and power-bar, **before** the sidebar
blit-to-screen, every rendered frame — entirely **out-of-sim** (after `Main_Tick` /
`LogicClass::PerTickUpdate` have run). It does NOT correspond to a spine rung
(A–AB) for its draw.

**Sim-side feed (when it IS touched per tick):** the *tracker* and *event queue* are mutated
from the sim during the tick — object reveal/conceal calls `TechnoClass/BuildingClass
RegisterOnRadar` (membership churn driven by object lifecycle, near the LogicClass object
pass), and `CreateRadarEvent` is called from combat / production / lightning / superweapon /
trigger code paths during their rungs. The render-side `RadarClass__Update` then consumes
that accumulated dirty state. So the **read/consume side is render-pass; the write/dirty side
rides on whatever sim rung created the change** (object pass for tracker churn; combat /
production / super rungs for events).

---

## ORDERED COMPOSITION (inside `RadarClass__Update @ 0x00656EC0`)

Verified-in-docs (`MINIMAP_GENERATED_PIXEL_COLOR_PIPELINE` §6/§9/§10,
`RADAR_EVENT_PING_PIXEL_SHAPES_COLORS` §2):

1. Clear/restore dirty terrain into the secondary surface (`CellClass__GetRadarColor` path).
2. Render dirty object/terrain pixels into the primary surface via `RenderCellPixel`, whose
   per-pixel order is: **object dot → fog (half-bright `>>1`) → shroud (literal `0`) →
   terrain copy**.
3. Flush the pixel dirty vector.
4. `TickAndDrawRadarEvents @ 0x00660000` — ping diamonds (only types `0,1,2,3,4,5,11,12`
   draw a visible colour; default types resolve to black and skip the draw block).
5. `DrawSpySatelliteVision` (conditional).
6. If active content (`+0x14B0==1 && +0x14AC==1`): draw radar chrome frame `0x20`, then blit
   the accumulated dirty primary-surface rect into `g_SidebarSurface`, then draw the
   viewport-window rect (expanded `x-1,y-1,w+2,h+2`).

---

## OUTGOING EDGES (this service depends on …)

| Target service | Via symbol | Evidence | Notes |
|---|---|---|---|
| `cell-map` | `CellClass__GetRadarColor @ 0x0047C060`, `OverlayClass__GetRadarColor @ 0x005FED00`, `IsShrouded @ 0x00586360`, `IsFogged @ 0x005864A0` | generated-pixel §2/§3/§4/§7 | per-cell terrain/overlay radar colour + shroud/fog bits read from `CellClass`. **Primary dependency** (stub: most-depends-on). |
| `abstract-object` | tracker fed by `ObjectClass::Reveal/Conceal` → `RegisterOnRadar`; `RenderCellPixel` reads object owner/colour bytes | object-dot-priority §3/§5; generated-pixel §8 | object dots come from the live object tracker; membership = object lifecycle. **Primary dependency** (stub). |
| `factory-house` | object dot colour packs house/colour-scheme bytes; radar online/offline is power-gated by HouseClass | `HOUSE_COLORSCHEME_TO_RADAR_DOT_PACKED_COLOR`; generated-pixel §8 | dot colour = owning house's colour scheme; radar availability gated by house power/radar facility. |
| `lookup-tables` | radar zoom/aspect inverse transform + DD loss/shift packing globals | generated-pixel §1/§6 | cell↔radar-pixel projection + 16-bit channel pack. |
| `frontier-sidebar` | writes `g_SidebarSurface` (radar chrome frame `0x20` + content blit + viewport rect) | sidebar-overlap §2; generated-pixel §10 | radar is drawn into the sidebar surface, after power bar, before sidebar blit-to-screen. |
| `frontier-blitter` | surface vtable line/rect helpers (`+0x78/+0x90` line path; `Surface__DrawLineGradient_ABufModulated_ZClipped @ 0x004BDF00`; `FUN_007BC2B0` clip); final 16-bit packed pixels | radar-event §6/§7; surface-line-helpers report | event diamonds + viewport rect + final blit use the surface raster back-end. |

---

## INCOMING EDGES (… depends on this service)

| Source service | Via symbol | Evidence | Notes |
|---|---|---|---|
| `techno-foot` | `TechnoClass__RegisterOnRadar @ 0x0070CC90` (one pixel) | object-dot-priority §5 | every revealed mobile object registers a tracker dot. |
| `factory-house` | `BuildingClass__RegisterOnRadar @ 0x00456580` (whole footprint) | object-dot-priority §5; building-footprint audit | buildings register every footprint pixel as dots. |
| `frontier-sidebar` | `PowerClass__Draw @ 0x0063FB20` → `RadarClass__Draw @ 0x00653100` | sidebar-overlap §2 | the sidebar draw chain calls radar draw every frame (radar is a sub-pass of the sidebar). |
| `frontier-input-command` | radar vtable input handler `0x006539D0` (via `GScreenClass__Input @ 0x004F4320` / Win32 `FUN_006930A0`) | gadgetclass-click-provenance §2/§3 | minimap click-to-recenter / drag-pan is owned by RadarClass's own vtable input handler; it also suppresses tactical band-box selection when the click is in radar bounds. |
| combat / production / super / trigger paths | `CreateRadarEvent @ 0x0065FA70` (types `13` impact/super, `14` bridge-repair, plus visible combat/dropzone/beacon types) | radar-event §11/§12 | sim events enqueue radar pings (most non-visible types still ring/EVA but draw black). |
| `frontier-super` (spy satellite) | `RadarClass__RefreshRadar @ 0x00657CE0` / `DrawSpySatelliteVision` | spy-satellite-reveal report | spy-satellite reveal repaints the radar through the normal pixel path. |

---

## ACTIVE-IN-YR / TS-LEGACY

- **Active in standard YR: YES.** The generated raw-RGB → secondary → primary → sidebar-blit
  pipeline, shroud-black pixels, object-dot tracker, overlay/tiberium colours, radar-event
  pings (visible types), click-to-recenter, and the viewport-rect overlay are all live for an
  ordinary in-game player with radar online. Verified-in-docs across the radar report family.
- **Fog half-bright branch: CONDITIONAL (TS-legacy-flavoured).** `RenderCellPixel`'s fog branch
  (read secondary terrain pixel, halve unpacked channels `>>1`, repack) is binary-live, but
  standard YR ships `FogOfWar=no` (`ini/rules.ini`), so explored cells normally stay fully
  visible on the minimap; the half-bright fog dim is not seen in default skirmish. This matches
  the project-wide "fog of war is TS legacy, off by default in YR" rule (MEMORY:
  `feedback`/CLAUDE.md). Implement shroud-black; treat minimap fog-dim as opt-in only.
- **Radar mode-3 open/close transition movie** (`PerFrameMovieUpdate @ 0x006579E0`,
  `DAT_00B04A38` frame `0x20`) is active in YR — the radar slide-in/out chrome animation.
- **Default-colour radar-event types** (`6..10,13,14,15,16`) enter the event/ring/dedup system
  but resolve to black and **skip the draw block** — they are live (EVA/ring) but produce no
  visible minimap diamond. Not TS-dead, just intentionally invisible on the minimap.

---

## SOURCES (docs tier — Ghidra MCP unreachable this session)

All `ghidra/verified` reports; each cites its own decompile + disasm-range evidence inline:
- `docs/research/MINIMAP_GENERATED_PIXEL_COLOR_PIPELINE_GHIDRA_REPORT.md` (representative
  address `0x00653100`, full pixel pipeline, tracker, shroud/fog, blit order)
- `docs/research/SIDEBAR_ODD_STATE_OVERLAP_STACK_GHIDRA_REPORT.md` (plug point: PowerClass→Radar
  draw chain; render-pass placement)
- `docs/research/RADAR_EVENT_PING_PIXEL_SHAPES_COLORS_GHIDRA_REPORT.md` (events, colours,
  `CreateRadarEvent`/`DrawRadarEvent`, update order)
- `docs/research/RADAR_OBJECT_DOT_PRIORITY_VISIBILITY_GATES_GHIDRA_REPORT.md` (tracker,
  `TechnoClass/BuildingClass RegisterOnRadar`, `GetObjectAtRadarPixel`)
- `docs/research/MINIMAP_GADGETCLASS_CLICK_PROVENANCE_GHIDRA_REPORT.md` (input handler
  `0x006539D0`, vtable `0x007F0320`, recenter/drag, band-box suppression)
- `docs/research/RADAR_GENERIC_TERRAIN_PIXEL_DIRTY_PIPELINE_GHIDRA_REPORT.md` (dirty lists, field offsets)
- `docs/research/RADAR_SURFACE_SIZING_ZOOM_SAMPLING_GHIDRA_REPORT.md` (aperture/surface sizing)
- `docs/research/SOVIET_RADAR_MINIMAP_CONTENT_INSET_GHIDRA_REPORT.md` (chrome/aperture offsets)
- `docs/research/HOUSE_COLORSCHEME_TO_RADAR_DOT_PACKED_COLOR_GHIDRA_REPORT.md` (dot colour ← house)
- `docs/research/SPY_SATELLITE_REVEAL_RADAR_PIXEL_PIPELINE_GHIDRA_REPORT.md` (`RefreshRadar`)
- Stub: `docs/research/core-services-map/_frontier.md` §B2.

**Re-verify next session (live Ghidra):** `decompile_function 0x00653100` to confirm
`RadarClass__Draw` is the per-frame draw entry and calls `RadarClass__Update @ 0x00656EC0`;
spot-check `0x006539D0` (vtable+0x18 input handler) and `0x0070CC90`/`0x00456580` (the two
RegisterOnRadar entries) to upgrade those edges from CORROBORATED-BY-DOCS to LIVE-VERIFIED.
