# Rendering Parity Checklist — gamemd.exe vs Our Engine

Complete feature comparison verified from Ghidra decompilation.

---

## Phase 1 — Terrain Pass (8 steps)

| Step | Function | What it does | Our status |
|------|----------|-------------|------------|
| 1 | ZBufferDirtyClear | Clear Z-buffer dirty rects to 0xFFFF | N/A (GPU depth buffer cleared per frame) |
| 2 | layer_shroud_edges | Draw shroud/fog edge transitions | PARTIAL — fog mask bilinear but no edge tiles |
| 3 | layer_terrain_shadows | Terrain height shadows (hills shadow lower ground) | MISSING — diagonal ray-march from high to low cells, redraws tiles at shadow positions |
| 4 | layer_base_terrain | TMP terrain tiles with per-pixel Z R+W | IMPLEMENTED — zdepth shader |
| 5 | layer_smudges | Crater/scorch marks on terrain (SmudgeClass) | MISSING — SmudgeClass objects drawn via Cell_ContentRendering |
| 6 | layer_building_overlays | Flat AnimClass objects below objects | MISSING — AnimClass (RTTI 0x24) from DisplayLayerEntry list, drawn before object pass |
| 7 | layer_overlays | Wall/ore/bridge/fence overlays | IMPLEMENTED — passthrough pipeline |
| 8 | layer_animations | Flat BuildingClass objects below objects | MISSING — BuildingClass (RTTI 6) from DisplayLayerEntry list, drawn before object pass |

---

## Phase 2 — Object Pass

| Feature | What it does | Our status |
|---------|-------------|------------|
| Layer 0 (Underground) | Subterranean units (tunnel locomotion) | MISSING — no underground rendering |
| Layer 1 (Surface) | Flat ground-level effects | MISSING — not rendered |
| Layer 2 (Ground) Y-sort | Buildings, infantry, vehicles, ground anims | IMPLEMENTED — unified merge |
| Layer 2 shadow pass | Per-object shadow drawn BEFORE body | MISSING — shadow SHP frames not rendered |
| Layer 2 turret pass | Building turrets after all layer 2 | IMPLEMENTED — separate pass |
| Layer 3 (Air) | Aircraft, airborne projectiles | PARTIAL — aircraft render but not in separate layer |
| Layer 4 (Top) | Top-most effects | MISSING — not rendered |
| Health bars / selection | Second pass over all layers for extras | IMPLEMENTED — separate pass |

---

## Post-Object Effects

| Feature | Function | Our status |
|---------|----------|------------|
| Laser beams | FUN_00550240 | MISSING — Prism Tower, etc. |
| Electric bolts | FUN_004c2830 | MISSING — Tesla coil, etc. |
| Particle effects | FUN_00556d40 + FUN_006591b0 | MISSING |
| Waypoint overlays | FUN_006dbe20 | PARTIAL — some debug overlays |
| Band box selection | Tactical__DrawBandBoxRect | IMPLEMENTED — drag selection |
| Rally point lines | FUN_006dad60 + FUN_006da9d0 | MISSING |
| Building placement ghost | BuildingPlacement_OverlayRenderer | IMPLEMENTED |

---

## Z-Buffer / Depth System

| Feature | gamemd.exe | Our engine |
|---------|-----------|------------|
| Terrain per-pixel Z R+W | TMP_TileBlitter | zdepth shader — MATCH |
| Wall overlay Z R+W | TMP_TileBlitter (if tile has Z-data) | Passthrough (no Z) — DIFFERENT but acceptable |
| Bridge overlay Z R+W | Blitter 0xC0 (Less compare, R+W) | Passthrough (no Z) — MISSING |
| SHP sprite Z | 0x800 flag → ignores Z | Passthrough — MATCH |
| Cliff redraw | Not in gamemd.exe | zdepth + Less — OUR IMPROVEMENT |
| Per-scanline Z gradient | 3-entry Bresenham table | MISSING — cosmetic |
| BUILDNGZ.SHA | Loaded but unreachable (dead code) | Not loaded — MATCH (both unused) |

---

## Bridge Rendering

| Feature | gamemd.exe | Our engine |
|---------|-----------|------------|
| Bridge body overlay position | Get_Draw_Offset (-16 EW, -31 NS) | -16 / -31 — MATCH |
| Bridge body Z interaction | Blitter 0xC0 Z R+W | Passthrough — MISSING |
| Bridge frame variation | Latin square table [0,1,2,3...] for states 0/9 | IMPLEMENTED |
| Bridge shadow frames | Shadow half of SHP at ground level | MISSING |
| Bridge shadow NS displacement | X-15, Y+7 for all NS (states 9-17) | MISSING |
| Bridge railing overlays | Separate SHP via FUN_00547230 | MISSING |
| Bridge damage state frames | States 1-8 (EW) / 10-17 (NS) | MISSING (binary destroy only) |
| Bridge pavement bit (0x2000) | Alternate surface variant | MISSING |
| Destroyed bridge visual removal | Cell marked destroyed → skip rendering | IMPLEMENTED |
| ZFudgeBridge | Per-unit Z-depth fudge near bridges | MISSING |

---

## Per-Object Rendering

| Feature | gamemd.exe | Our engine |
|---------|-----------|------------|
| Object shadow | Shadow SHP frame drawn before body per object | MISSING |
| Chrono warp translucency | Alpha scaling via temporal/warp phase | IMPLEMENTED (alpha field) |
| Cloaking visual | Special blitter selection for cloak states | MISSING |
| Building garrison fire | UpdateGarrisonFire after layer 2 turrets | MISSING |
| Mind control link lines | CaptureManagerClass::DrawLinks | MISSING |
| Tractor beam lines | After object pass | MISSING |
| Spy indicators | Allied building highlight (FUN_00430ac0) | MISSING |
| Radar overlay drawing | DrawRadarOverlays_Normal + _Fog | PARTIAL — minimap exists |

---

## ZFudge System (per-unit depth correction)

| Feature | gamemd.exe | Our engine |
|---------|-----------|------------|
| ComputeZFudge (0x4DAFF0) | max(cliff, column, tunnel, bridge) fudge | MISSING |
| ZFudgeCliff | TechnoTypeClass+0xDC0 | MISSING |
| ZFudgeColumn | TechnoTypeClass+0xDC4 | MISSING |
| ZFudgeTunnel | TechnoTypeClass+0xDC8 | MISSING |
| ZFudgeBridge | TechnoTypeClass+0xDCC | MISSING |
| FUN_00704350 additional Z | Slope/cliff/building proximity | MISSING |

---

## Summary — What We're Missing (by impact)

### HIGH IMPACT (visually noticeable)
1. **Per-object shadows** — every unit/building should cast a shadow
2. **Terrain height shadows** — hills cast shadows on lower ground
3. **Laser beams / electric bolts** — Prism Tower, Tesla coil visuals
4. **Bridge shadows** — shadow frames at ground level below bridges

### MEDIUM IMPACT (gameplay-relevant visuals)
5. **Flat anims below objects** — Tesla glow, nuke flash, ground effects
6. **Smudges** — craters from explosions, scorch marks
7. **Rally point lines** — lines from production buildings to rally point
8. **Bridge damage state frames** — cracked/broken bridge visuals
9. **Cloaking visual** — mirage/stealth tank shimmer effect
10. **Mind control lines** — links between Yuri units and controlled units

### LOW IMPACT (polish)
11. **Bridge railings** — side rail overlays
12. **ZFudge system** — depth correction near cliffs/bridges/tunnels
13. **Garrison fire visuals** — muzzle flash from garrisoned buildings
14. **Spy indicators** — allied building highlights
15. **Per-scanline Z gradient** — sub-pixel depth precision
16. **Bridge pavement bit** — alternate surface variant
17. **Layer 0/1/3/4 separation** — underground, surface, air, top layers
