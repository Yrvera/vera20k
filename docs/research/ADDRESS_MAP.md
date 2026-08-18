# gamemd.exe Address Map

Quick-lookup index of all reverse-engineered addresses. Every entry is verified
via Ghidra MCP decompilation and documented in the source column's report.

**Usage:** Ctrl+F for an address, function name, or system keyword.
Before implementing from this map, always re-verify via live Ghidra decompilation.

---

## Functions

### Core Engine

| Address | Name | Size | Source |
|---------|------|------|--------|
| `0x006BB9A0` | WinMain | 1826 B | GAMEMD_ARCHITECTURE |
| `0x0048CCC0` | Main_Game | 125 lines | GAMEMD_ARCHITECTURE |
| `0x0055D360` | Main_Tick (per-frame update) | 485 lines | GAMEMD_ARCHITECTURE |
| `0x0048C8B0` | State Machine (game state dispatch) | — | GAMEMD_ARCHITECTURE |
| `0x0052D9A0` | New_Scenario | — | GAMEMD_ARCHITECTURE |
| `0x0055DEE0` | LogicClass::AI (input/event dispatcher, not the object-AI tick loop) | — | GAMEMD_ARCHITECTURE |
| `0x0055AFB0` | LogicClass::PerTickUpdate (active-object tick loop, vtable+0x5C) | — | LOGICCLASS_PERTICKUPDATE_SCHEDULER |
| `0x004D2370` | Map::Logic (cell updates, tiberium growth) | — | GAMEMD_ARCHITECTURE |
| `0x004F4480` | Render frame | — | GAMEMD_ARCHITECTURE |
| `0x0066D530` | RulesClass::ReadGeneral (INI parser) | 18793 B | LOCOMOTION_MATH |
| `0x00712170` | TechnoTypeClass::ReadINI | 3471 lines | TECH_TREE_REPORT |
| `0x004068E0` | Register heap pool (per-class memory) | — | GAMEMD_ARCHITECTURE |

### Coordinate System

| Address | Name | Size | Source |
|---------|------|------|--------|
| `0x006D2140` | TacticalClass::CoordsToClient2 (full projection) | — | COORDINATE_SYSTEM |
| `0x006D1FE0` | TacticalClass::CellToPixel (no Z, no scroll) | — | COORDINATE_SYSTEM |
| `0x006D20E0` | AdjustForZ (height to screen Y) | — | COORDINATE_SYSTEM |
| `0x0045B070` | HeightFactor initialization | — | COORDINATE_SYSTEM |
| `0x0049F2F0` | Foundation direction table initialization | — | COORDINATE_SYSTEM |

### Selection Brackets & Health Pips

| Address | Name | Size | Source |
|---------|------|------|--------|
| `0x006F5190` | `TechnoClass__DrawExtras` (brackets front + pips, render phase 0x13) | 462 lines | SELECTION_BRACKETS |
| `0x006F60D0` | `TechnoClass__DrawBehind` (brackets back edges, render phase 0x12) | 86 lines | SELECTION_BRACKETS |
| `0x006F5EF0` | `TechnoClass__DrawBracketCorner` (25% stubs at both ends of one edge) | 51 lines | SELECTION_BRACKETS |
| `0x006DBB60` | `Tactical__DrawLine3D` (project 3D coords → draw line on surface) | 374 B | SELECTION_BRACKETS |
| `0x006D1EB0` | `Tactical__WorldToScreenSub` (sx=30*(wx-wy), sy=15*(wx+wy)) | — | SELECTION_BRACKETS |
| `0x006D20E0` | `AdjustForZ` (z_leptons → screen_y pixel offset, threshold 728) | — | SELECTION_BRACKETS |
| `0x006D1BA8` | `Tactical__ComputeZMultiplier` (cos(angle)*60/scale → g_AdjustForZ_Mult) | — | SELECTION_BRACKETS |
| `0x006CE240` | `CoordStruct__VecAdd` (result = A + B, __thiscall, RET 8) | — | SELECTION_BRACKETS |
| `0x00710700` | `CoordStruct__VecDiv` (result = A / n, __thiscall) | — | SELECTION_BRACKETS |
| `0x00464AF0` | `BuildingTypeClass__Dimension2` (returns {fw*256, fh*256, Height*HF}) | — | SELECTION_BRACKETS |
| `0x006F64A0` | `TechnoClass__DrawHealthBar` (vtable+0x44C, health pips for all types) | — | SELECTION_BRACKETS |
| `0x006DA380` | `Tactical__PickObjectAtScreenPoint` (isometric distance ≤200, fallback: cell occupier) | 86 lines | SELECTION_BRACKETS |
| `0x004CAD50` | `Sin_Lookup_Table4096` (4096 entries, conversion factor 4096/(2π) at 0x007e8970) | — | SELECTION_BRACKETS |
| `0x004CAC40` | `Sqrt_Approx` (table-based float sqrt at 0x008650bc) | — | SELECTION_BRACKETS |
| `0x0045B070` | `HeightFactor_Init` (g_HeightFactor = sin(30°) * scale * 0.5) | — | SELECTION_BRACKETS |
| `0x005B2950` | Render phase dispatch (switch 0x00-0x18, vtable offsets per phase) | — | SELECTION_BRACKETS |

#### Selection Bracket Globals

| Address | Name | Source |
|---------|------|--------|
| `0x0089DDB8` | `g_HeightFactor` (Height INI → lepton Z; 1 unit ≈ 15 screen px) | SELECTION_BRACKETS |
| `0x00B0CD48` | `g_AdjustForZ_Mult` (Z leptons → screen Y pixels multiplier) | SELECTION_BRACKETS |
| `0x0087F6C4` | Palette data pointer (color table at +0x174) | SELECTION_BRACKETS |
| `0x0085D0A4` | Sine lookup table (4096 float entries, one full circle) | SELECTION_BRACKETS |
| `0x008650BC` | Sqrt lookup table (float, used by Sqrt_Approx) | SELECTION_BRACKETS |
| `0x008192B8` | `g_FoundationWidthTable` (int array by foundation_id) | SELECTION_BRACKETS |
| `0x00819310` | `g_FoundationHeightTable` (int array by foundation_id) | SELECTION_BRACKETS |

#### Selection Bracket Vtable Offsets (TechnoClass/BuildingClass)

| Vtable Offset | Name | Render Phase |
|---------------|------|-------------|
| `+0x048` | GetCoords (returns world position) | — |
| `+0x07C` | GetDimensions / Dimension2 (foundation + height) | — |
| `+0x084` | GetTypeClass | — |
| `+0x0AC` | GetRenderCoords (draw anchor) | — |
| `+0x1C8` | GetPixelSelectionBracketDelta (stored at TypeClass+0x3E0) | — |
| `+0x244` | **DrawBehind** (back bracket edges, 5 edges) | 0x12 |
| `+0x248` | **DrawExtras** (front bracket edges + pips, 4+3 edges) | 0x13 |
| `+0x448` | DrawHealthBar (health pips overlay) | — |
| `+0x44C` | DrawPips (rank/veterancy pips) | — |
| `+0x454` | DrawActionLines? (non-selected overlay) | — |

### Rendering — Tactical

| Address | Ghidra Label | Size | Source |
|---------|-------------|------|--------|
| `0x006D3F50` | `TacticalClass_Draw` (two-phase: 1=terrain, 2=objects) | — | ZBUFFER_DEPTH |
| `0x006D7560` | Tile rendering (CoordsToClient → draw) | — | COORDINATE_SYSTEM |
| `0x006D6D10` | Cell content rendering (has -0x1E X offset) | — | COORDINATE_SYSTEM |
| `0x006D8DB0` | `Tactical_ObjectRenderingLoop` (5-layer iteration) | — | ZBUFFER_DEPTH |
| `0x006D5030` | Building placement overlay renderer | — | COORDINATE_SYSTEM |
| `0x0047EC90` | Building placement per-cell draw (PLACE.SHP) | — | COORDINATE_SYSTEM |
| `0x006D5730` | LaserFencePost placement extension shadow | — | COORDINATE_SYSTEM |
| `0x006D59D0` | FirestormWall placement extension shadow | — | COORDINATE_SYSTEM |
| `0x006D5C50` | Overlay wall placement extension shadow | — | COORDINATE_SYSTEM |
| `0x006D2B60` | `Tactical_ZBufferDirtyClear` (Phase 1 step 1) | — | ZBUFFER_DEPTH |
| `0x006D3660` | `Tactical_ShroudEdgesAndIcons` (Phase 1 step 2) | — | ZBUFFER_DEPTH |
| `0x006D2DE0` | `Tactical_TerrainShadows` (Phase 1 step 3) | — | ZBUFFER_DEPTH |
| `0x006D3470` | `Tactical_BaseTerrainCells` (Phase 1 step 4) | — | ZBUFFER_DEPTH |
| `0x006D3290` | `Tactical_SmudgesAndCraters` (Phase 1 step 5) | — | ZBUFFER_DEPTH |
| `0x006D3AC0` | `Tactical_BuildingOverlays` (Phase 1 step 6) | 319 B | ZBUFFER_DEPTH |
| `0x006D3040` | `Tactical_Overlays` (Phase 1 step 7: walls, ore) | — | ZBUFFER_DEPTH |
| `0x006D3870` | `Tactical_Animations` (Phase 1 step 8) | — | ZBUFFER_DEPTH |
| `0x004801F0` | Shroud/fog edge rendering | — | GAMEMD_ARCHITECTURE |

### Rendering — Sprites & SHP

| Address | Ghidra Label | Size | Source |
|---------|-------------|------|--------|
| `0x00705E00` | `TechnoClass_DrawSHP` (RET 0x40 = 17 params) | — | ZBUFFER_DEPTH |
| `0x004AED70` | `CC_Draw_Shape` (main SHP draw, 16 params, RET 0x38) | 195 lines | ZBUFFER_DEPTH |
| `0x00480350` | `CellOverlay_TileDraw` (wall/overlay → tile renderer) | — | ZBUFFER_DEPTH |
| `0x00422CA0` | AnimClass::DrawIt (ZAdjust + gradient 0 or 2) | 459 lines | ZBUFFER_DEPTH |
| `0x0069E900` | `SHP_GetFrameCompressionFlag` (bit 1: std vs ext blitter) | 15 lines | ZBUFFER_DEPTH |
| `0x0069E7E0` | `SHP_GetFrameRect` (x/y/w/h for frame index) | — | ZBUFFER_DEPTH |
| `0x0069E740` | `SHP_GetFrameData` (decompressed pixel data ptr) | — | ZBUFFER_DEPTH |
| `0x0069E580` | `SHP_Resolve` (resolve SHP ref, ensure loaded) | 75 lines | ZBUFFER_DEPTH |
| `0x005B40B0` | `LoadFileFromMIX` (generic file loader from MIX) | 101 lines | ZBUFFER_DEPTH |

### Rendering — Voxels

| Address | Ghidra Label | Size | Source |
|---------|-------------|------|--------|
| `0x00755DB0` | VXL file loader | — | VOXEL_RENDERING |
| `0x00754C00` | Light direction setup | — | VOXEL_RENDERING |
| `0x00754CB0` | Master lighting matrix precomputation | 3290 B | VOXEL_RENDERING |
| `0x00758B70` | Palette remap table builder (256×32) | 665 B | VOXEL_RENDERING |
| `0x007559B0` | Slope matrix lookup | — | VOXEL_SLOPE_TILT |
| `0x00755A40` | Slope matrix interpolation (transition) | — | VOXEL_SLOPE_TILT |
| `0x005AEF60` | Dynamic body tilt X-axis | — | VOXEL_SLOPE_TILT |
| `0x005AF080` | Dynamic body tilt Y-axis | — | VOXEL_SLOPE_TILT |
| `0x005AF980` | HVA matrix multiply | — | VOXEL_RENDERING |
| `0x005AFB80` | Matrix × vector multiply | — | BUILDING_SYSTEMS |
| `0x00706640` | `TechnoClass__Draw` (Z-mode flags for VXL) | — | ZBUFFER_DEPTH |
| `0x00706ED0` | `TechnoClass__Render` (VXL uncached → StandardBlitter) | — | ZBUFFER_DEPTH |
| `0x00707480` | `VXL_CacheBlit` (VXL cached → ExtendedBlitter) | — | ZBUFFER_DEPTH |
| `0x00706BD0` | VXL turret draw (hardcoded `0x2001`, no Z) | — | ZBUFFER_DEPTH |
| `0x00754510` | `VXL_Sort_Rasterize` (bubble-sort sections by Z) | — | ZBUFFER_DEPTH |
| `0x00757120` | `VXL_Rasterizer_Mirror` (per-pixel depth in DepthMap) | — | ZBUFFER_DEPTH |

### Rendering — Z-Buffer Infrastructure

| Address | Ghidra Label | Size | Source |
|---------|-------------|------|--------|
| `0x007BC970` | `ZBuffer_Constructor` (0x30-byte surface) | — | ZBUFFER_DEPTH |
| `0x007BCF50` | `ZBuffer_RectClear` (thin wrapper) | — | ZBUFFER_DEPTH |
| `0x007BCFB0` | `ZBuffer_Clear` (row fill 0xFFFF) | 106 lines | ZBUFFER_DEPTH |
| `0x007BD130` | `ZBuffer_GetScanlinePtr` | — | ZBUFFER_DEPTH |
| `0x004114B0` | `CircBuf_GetScanlinePtr` (generic, also A-buf) | 16 lines | ZBUFFER_DEPTH |
| `0x0043AD00` | `PixelBuffer_Init` (init descriptor, optional alloc) | 17 lines | ZBUFFER_DEPTH |
| `0x0043AE50` | `PixelBuffer_Free` (free if owned, zero descriptor) | 13 lines | ZBUFFER_DEPTH |
| `0x00547CF0` | `TMP_TileBlitter` (60×29 diamond, **only active Z R+W**) | 862 lines | ZBUFFER_DEPTH |
| `0x0045E8F0` | `LoadBuildingZShape` (BUILDNGZ.SHA, −65 remap) | — | ZBUFFER_DEPTH |
| `0x00484680` | `Cell_ComputeZAdjust` (heightLevel → cellZAdjust) | — | ZBUFFER_DEPTH |

### Rendering — Blitter Pipeline

| Address | Ghidra Label | Size | Source |
|---------|-------------|------|--------|
| `0x004373B0` | `SHP_StandardBlitter` (per-scanline + Z-gradient) | 244 lines | ZBUFFER_DEPTH |
| `0x00437A10` | `SHP_ExtendedBlitter` (shadow/Z-shape variant) | 171 lines | ZBUFFER_DEPTH |
| `0x00490B90` | `Blitter_selector` (std path, dispatches on flags) | 164 lines | ZBUFFER_DEPTH |
| `0x00490E50` | `Blitter_selector_extended` (ext path, dispatches on flags) | 145 lines | ZBUFFER_DEPTH |
| `0x0048EBF0` | `Blitter_Init_All` (creates all blitter objects) | 1367 lines | ZBUFFER_DEPTH |
| `0x007BC040` | `Blitter_ClipAndSetup` (clip rects + lock surfaces, does NOT touch Blitter) | — | ZBUFFER_DEPTH |
| `0x007BBE20` | `ClipRectPair` (clip two paired rects against viewports) | — | ZBUFFER_DEPTH |
| `0x00437F30` | `CopyPoint` (copy x,y pair, thiscall) | — | ZBUFFER_DEPTH |
| `0x00437F10` | `AddPoints` (dest = this + src, component-wise x,y add) | — | ZBUFFER_DEPTH |

### Rendering — Per-Scanline Blitter Functions

| Address | Ghidra Label | Behavior | Source |
|---------|-------------|----------|--------|
| `0x004978C0` | `Blitter_Opaque_RLE_Remap` | **Opaque** RLE, NO Z, NO blend (**normal objects**) | ZBUFFER_DEPTH |
| `0x00493DF0` | `Blitter_Scanline_Opaque_Remap` | Opaque non-RLE + remap, no Z | ZBUFFER_DEPTH |
| `0x00493F30` | `Blitter_Scanline_Remap_Intensity` | Remap + A-buf intensity, no Z | ZBUFFER_DEPTH |
| `0x00494020` | `Blitter_Scanline_Remap_Intensity_Fwd` | Wrapper → above | ZBUFFER_DEPTH |
| `0x00494080` | `Blitter_Scanline_Blend25pct_Remap` | 25% blend (cloaking), no Z | ZBUFFER_DEPTH |
| `0x004941E0` | `Blitter_Scanline_Blend50pct_Remap` | 50% blend (cloaking), no Z | ZBUFFER_DEPTH |
| `0x00497CF0` | `Blitter_ZWriteOnly_RLE_Remap_NoZWrite` | 50% RLE (cloaking), ignores Z | ZBUFFER_DEPTH |
| `0x00495BC0` | `Blitter_ZBuf_Intensity25pct_WritesZ` | Z R+W 25% (unreachable w/ 0x800) | ZBUFFER_DEPTH |
| `0x00497100` | `Blitter_ZClip_Plain16_WritesZ` | Z R+W BUILDNGZ (unreachable w/ 0x800) | ZBUFFER_DEPTH |

### Rendering — Blitter Vtables

| Address | Description | Offset | Source |
|---------|------------|--------|--------|
| `0x007E5470` | **Normal opaque RLE**, `flags&6==0 + 0x800` | 0x124 (ext) | ZBUFFER_DEPTH |
| `0x007E5440` | Z-write RLE + 50% blend (cloaking) | 0x130 (ext) | ZBUFFER_DEPTH |
| `0x007E54D0` | Z R+W plain16 (dead code w/ 0x800) | 0x10c (ext) | ZBUFFER_DEPTH |
| `0x007E5600` | Z R+W intensity 25% (dead code w/ 0x800) | — | ZBUFFER_DEPTH |
| `0x007E57C8` | Z-shape overlay + remap | 0x70 (std) | ZBUFFER_DEPTH |
| `0x007E57B0` | Z R+W + 25% blend (cloaking) | 0x74 (std) | ZBUFFER_DEPTH |
| `0x007E5798` | Z-write + 50% blend (cloaking) | 0x78 (std) | ZBUFFER_DEPTH |
| `0x007E57E0` | Z R+W opaque remap | 0x68 (std) | ZBUFFER_DEPTH |

### Rendering — Visual State / Z-Mode Dispatch

| Address | Ghidra Label | Size | Source |
|---------|-------------|------|--------|
| `0x00703860` | `TechnoClass_GetVisualState` (vtable+0x68 base) | 88 lines | ZBUFFER_DEPTH |
| `0x004544A0` | `BuildingClass_GetVisualState` (vtable+0x68 override) | 304 B | ZBUFFER_DEPTH |
| `0x004DA4E0` | `FootClass_GetVisualState` (vtable+0x68: Infantry/Unit/Aircraft) | — | ZBUFFER_DEPTH |
| `0x0055ABC0` | `ILocomotion_Visual_Character_Base` (always returns 0) | — | ZBUFFER_DEPTH |
| `0x00456F80` | `BuildingClass_AdjustZHeight` (vtable+0x464, ±500) | 13 lines | ZBUFFER_DEPTH |
| `0x0050B6F0` | Cloaking eligibility check (player ownership) | 13 lines | ZBUFFER_DEPTH |

### Rendering — Layer System

| Address | Ghidra Label | Size | Source |
|---------|-------------|------|--------|
| `0x0048E050` | `Layer_NameToIndex` (stricmp over 5-entry table) | — | ZBUFFER_DEPTH |
| `0x0048E090` | `Layer_IndexToName` (returns from string table) | — | ZBUFFER_DEPTH |
| `0x004A862A` | Layer array init (5 × DynamicVectorClass, cap=10) | — | ZBUFFER_DEPTH |
| `0x004A9720` | `Layer_AddObject` (vtable+0x78 → layer add) | — | ZBUFFER_DEPTH |

### Rendering — Shroud/Fog Pipeline

| Address | Ghidra Label | Size | Source |
|---------|-------------|------|--------|
| `0x006D3660` | `Tactical_ShroudEdgesAndIcons` (Phase 1 step 2, A-buf only) | — | SHROUD_FOG |
| `0x004801F0` | Shroud/fog edge dispatcher (calls 0x006d8700 twice) | — | SHROUD_FOG |
| `0x006D8700` | Shroud edge bitmask calculator (8-neighbor adjacency) | — | SHROUD_FOG |
| `0x0047EFE0` | Shroud edge SHP blitter (direct A-buffer write) | — | SHROUD_FOG |
| `0x0047F250` | Fog edge SHP blitter (blended A-buffer write) | — | SHROUD_FOG |

### Rendering — Cloaking Visual Pipeline

| Address | Ghidra Label | Size | Source |
|---------|-------------|------|--------|
| `0x006FB740` | `TechnoClass__CloakingTick` (state machine per-tick) | 1354 B | CLOAKING_VISUAL |
| `0x00703770` | `TechnoClass__StartCloaking` (vtable+0x460, 0→1) | — | CLOAKING_VISUAL |
| `0x007036C0` | `TechnoClass__StartUncloaking` (vtable+0x45C, 2→3) | — | CLOAKING_VISUAL |
| `0x0070ED80` | `TechnoClass__ModifyCloakDrawFlags` (vtable+0x43C, player shimmer) | — | CLOAKING_VISUAL |
| `0x006FBC90` | `TechnoClass__ShouldUncloak` (vtable+0x2A4, uncloak decision) | — | CLOAKING_VISUAL |
| `0x006FBDC0` | `TechnoClass__CanAutoCloak` (vtable+0x2A0, cloak eligibility) | — | CLOAKING_VISUAL |
| `0x004D3780` | `TechnoClass__DoCloak` (wrapper + trigger events) | — | CLOAKING_VISUAL |
| `0x006F4EB0` | `TechnoClass__DoUncloak` (wrapper + mind-control scatter) | — | CLOAKING_VISUAL |
| `0x004578C0` | `BuildingClass__ShouldUncloak` (vtable+0x2A4 override) | 327 B | CLOAKING_VISUAL |
| `0x00457770` | `BuildingClass__CanCloak` (vtable+0x2A0 override) | 327 B | CLOAKING_VISUAL |
| `0x006FB170` | `TechnoClass__UpdateCloakShroud` (gap gen fog apply) | — | CLOAKING_VISUAL |
| `0x006FB470` | `TechnoClass__RemoveCloakShroud` (gap gen fog remove) | — | CLOAKING_VISUAL |
| `0x004DBDA0` | `FootClass__IsCloakable` (vtable+0x288, checks CloakStop+loco) | 24 lines | CLOAKING_VISUAL |
| `0x0070C5A0` | `TechnoClass__HasStealthAbility` (reads +0x3D2 flag) | 7 lines | CLOAKING_VISUAL |
| `0x004870B0` | `CellClass__IsVisibleToHouse` (per-house 32-bit visibility bitmask) | 7 lines | CLOAKING_VISUAL |
| `0x004870D0` | `CellClass__GapCountForHouse` (per-house gap counter array) | 7 lines | CLOAKING_VISUAL |
| `0x0050B6F0` | `TechnoClass__IsPlayerControlled` (MP: ==PlayerPtr; SP: flags) | 13 lines | CLOAKING_VISUAL |
| `0x005F5850` | `TechnoClass__ProcessCloakMode` (cloak direction, cell, triggers) | 42 lines | CLOAKING_VISUAL |
| `0x00454DB0` | `BuildingClass__UpdateGapGenerator_Tick` (gap gen state machine) | 2076 B | CLOAKING_VISUAL |
| `0x006FC0B0` | `TechnoClass__GetFireError` (DecloakToFire check, returns FireError) | 412 lines | CLOAKING_VISUAL |
| `0x0048284A` | `CrateClass__PickupCloak` (sets HasStealthAbility=1 for nearby units) | 455 B | CLOAKING_VISUAL |
| `0x00517CC0` | `InfantryClass__InitFromType` (copies Cloakable → HasStealthAbility) | — | CLOAKING_VISUAL |
| `0x00494330` | `Blitter_Shimmer_75pct_Remap` (75/25 blend, shimmer visual) | 197 B | CLOAKING_VISUAL |
| `0x00737BA0` | UnitClass::Unlimbo (sets CloakState=2 if HasStealth) | — | CLOAKING_VISUAL |
| `0x0065AAA0` | RadioClass::Transmit_Radio (dock reservation wrapper) | — | CHRONO_MINER_TELEPORT |
| `0x0065A970` | RadioClass::Transmit_Radio_Impl | — | CHRONO_MINER_TELEPORT |
| `0x00706640` | TechnoClass::Draw (checks warp flags for 50% translucency) | — | CHRONO_MINER_TELEPORT |
| `0x0070C5B0` | TechnoClass::IsWarpingOut (vtable+0x1D4, returns +0x270 byte) | 6 B | CHRONO_MINER_TELEPORT |
| `0x0070C5C0` | TechnoClass::IsBeingWarped (vtable+0x1D8, returns +0x271 byte) | 6 B | CHRONO_MINER_TELEPORT |
| `0x0070C5F0` | TechnoClass::IsNotWarping (checks both +0x270 and +0x271) | — | CHRONO_MINER_TELEPORT |
| `0x0070C610` | TechnoClass::SetGhostCell (stores CellClass* at +0x218) | — | CHRONO_MINER_TELEPORT |
| `0x0070E380` | TechnoClass::ScaleByTemporalVisualPhase (temporal fade curve) | — | CHRONO_MINER_TELEPORT |
| `0x0070E4B0` | TechnoClass::ScaleByWarpInVisualPhase (gap gen visual) | — | CHRONO_MINER_TELEPORT |
| `0x0070E5A0` | TechnoClass::UpdateTemporalVisual (10-phase fade state machine) | — | CHRONO_MINER_TELEPORT |
| `0x0070E920` | TechnoClass::UpdateGapVisual | — | CHRONO_MINER_TELEPORT |
| `0x0071AF20` | TemporalClass::InitiateWarp (sets IsWarpingOut on target) | — | CHRONO_MINER_TELEPORT |
| `0x0071ABC0` | TemporalClass::DetachFromTarget | — | CHRONO_MINER_TELEPORT |
| `0x004DE7B0` | `TechnoClass__AddSensorsAt` (vtable+0x4E8, reveals cloaked) | — | SENSOR_CLOAK |
| `0x004DE940` | `TechnoClass__RemoveSensorsAt` (vtable+0x4EC) | — | SENSOR_CLOAK |
| `0x00455820` | `BuildingClass__AddSensorArrayAt` (vtable+0x4F4, power check) | — | SENSOR_CLOAK |
| `0x004556D0` | `BuildingClass__RemoveSensorArrayAt` (vtable+0x4F8) | — | SENSOR_CLOAK |
| `0x00455A80` | `BuildingClass__AddDetectDisguiseAt` (vtable+0x4FC) | — | SENSOR_CLOAK |
| `0x00455980` | `BuildingClass__RemoveDetectDisguiseAt` (vtable+0x500) | — | SENSOR_CLOAK |
| `0x00487150` | `CellClass__IncrementSensorCount` (per-house at +0x7C) | — | SENSOR_CLOAK |
| `0x00487160` | `CellClass__DecrementSensorCount` | — | SENSOR_CLOAK |

### Cloaking — Vtable Addresses (verified)

| Address | Label | Class |
|---------|-------|-------|
| `0x007F5C70` | `vtable__UnitClass` | UnitClass primary |
| `0x007EB058` | `vtable__InfantryClass` | InfantryClass primary |
| `0x007E3EBC` | `vtable_BuildingClass` | BuildingClass primary |
| `0x007E22A4` | `vtable__AircraftClass` | AircraftClass primary |

### Rendering — Health Bars

| Address | Name | Size | Source |
|---------|------|------|--------|
| `0x006F64A0` | TechnoClass::DrawHealthBar (vtable+0x44C) | — | HEALTH_BAR_POSITIONING |
| `0x006F5190` | TechnoClass::DrawExtras (vtable+0x110) | — | HEALTH_BAR_POSITIONING |
| `0x006F60D0` | DrawBehind — isometric line brackets (buildings) | — | HEALTH_BAR_POSITIONING |

### Buildings — Core

| Address | Name | Size | Source |
|---------|------|------|--------|
| `0x0043B740` | BuildingClass::Constructor | — | MCV_DEPLOY |
| `0x0043D290` | BuildingClass::DrawBody | — | COORDINATE_SYSTEM |
| `0x0043EF90` | BuildingClass::GetCurrentFrame | 484 B | BUILDING_SYSTEMS |
| `0x00441F60` | BuildingClass::Place / OccupyMap | 718 B | BUILDING_SYSTEMS |
| `0x00447AC0` | BuildingClass::GetCoords (foundation center) | — | COORDINATE_SYSTEM |
| `0x004500A0` | BuildingClass::GetTargetCoords | — | COORDINATE_SYSTEM |
| `0x0041BEA0` | BuildingClass::GetCell (NW corner) | — | COORDINATE_SYSTEM |
| `0x0044E7B0` | BuildingClass::GetPowerOutput | — | BUILDING_SYSTEMS |
| `0x0044E880` | BuildingClass::GetPowerDrain | — | BUILDING_SYSTEMS |
| `0x0044E8F0` | BuildingClass::GetType (vtable+0x28) | — | COORDINATE_SYSTEM |
| `0x00445880` | **BuildingClass::Limbo** (vtable slot 53) — remove-from-map cleanup. Renamed 2026-04-24 from "OnDestroyed" (that label was wrong). | 1478 B | BUILDINGCLASS_MASTER_V3, BUILDINGCLASS_ON_DESTROYED |
| `0x004415F0` | **BuildingClass::DestructionEffects** (vtable slot 315) — the real HP=0 handler (survivors, debris, tiberium spill). Named 2026-04-24. | 2407 B | BUILDINGCLASS_MASTER_V3, BUILDINGCLASS_ON_DESTROYED |
| `0x0044EBF0` | **BuildingClass::Destroy** (vtable slot 55) — aborts factory production, ejects queued units. Corrected 2026-04-24 (v2 master labeled this "Limbo" — wrong). | 908 B | BUILDINGCLASS_MASTER_V3, BUILDINGCLASS_VTABLE_COMPLETE |
| `0x00442D90` | BuildingClass::SpawnSurvivors | 1651 B | BUILDING_SYSTEMS |
| `0x00445F80` | **BuildingClass::OnConstructionComplete** (vtable slot 311) — one-shot post-Unlimbo handler (ProduceCash init, anims, LightSource, AddSensorArrayAt, OnPowerOn, etc.). Decomped in T13 (residual Q B23). | — | BUILDINGCLASS_MASTER_V3, BUILDINGCLASS_RESIDUAL_Q_R4 |
| `0x00453E20` | **BuildingClass::Load** (vtable slot 5 — IPersistStream::Load) | 870 B | BUILDINGCLASS_SAVE_LOAD, BUILDINGCLASS_MASTER_V3 |
| `0x00454190` | **BuildingClass::Save** (vtable slot 6 — IPersistStream::Save) | 187 B | BUILDINGCLASS_SAVE_LOAD, BUILDINGCLASS_MASTER_V3 |
| `0x00454260` | **BuildingClass::Save_ChecksumFields** (vtable slot 13) — CRC input helper | — | BUILDINGCLASS_VTABLE_COMPLETE |
| `0x00459E80` | **BuildingClass::GetClassID** (vtable slot 3) — returns GUID `{0E272DC6-9C0F-11D1-B709-00A024DDAFD1}` | — | BUILDINGCLASS_VTABLE_COMPLETE |
| `0x00459EC0` | **BuildingClass::WhatAmI** (vtable slot 11) — returns 6 (AbstractType::Building) | — | BUILDINGCLASS_VTABLE_COMPLETE |
| `0x00459E70` | **BuildingClass::SizeOf** (vtable slot 12) — returns 0x720 (1824 bytes) | — | BUILDINGCLASS_VTABLE_COMPLETE |
| `0x00459F20` | **BuildingClass::ScalarDeletingDestructor** (vtable slot 8) — MSVC vcall dtor pattern | — | BUILDINGCLASS_VTABLE_COMPLETE |
| `0x00442C40` | **BuildingClass::Init_Managers** (vtable slot 9) — overrides TC; registers in HouseClass, inits power/rate timers | — | BUILDINGCLASS_VTABLE_COMPLETE |
| `0x00452630` | **BuildingClass::IsDeployable** (vtable slot 37) — (v2-corrected; NOT CanAcceptUpgrade) | — | BUILDINGCLASS_VTABLE_COMPLETE |
| `0x0044D880` | **BuildingClass::Mission_Hunt** (vtable slot 143) — deploy contents / slave-deploy | — | BUILDINGCLASS_VTABLE_COMPLETE |
| `0x00449A50` | **BuildingClass::Mission_Construction** (vtable slot 145) — 2-state build-up anim driver | 354 B | BUILDINGCLASS_MISSION_GUARD_AND_CONSTRUCTION |
| `0x0044B760` | **BuildingClass::Mission_Guard** (vtable slot 133) — trivial stub (MOV EAX, 0x1C2; RET) | 2 instr | BUILDINGCLASS_MISSION_GUARD_AND_CONSTRUCTION |
| `0x004F7870` | **HouseClass::CanBuild** — prerequisite evaluation entry point | — | BUILDINGCLASS_PREREQUISITES |
| `0x0045DD90` | **BuildingTypeClass::constructor** — real ctor. NOT `0x004653C0` (that's `FindOrAllocate`; v2 master mislabeled it). | 1921 B | BUILDINGTYPECLASS_CTOR_DEFAULTS |
| `0x004653C0` | **BuildingTypeClass::FindOrAllocate** — allocates 0x1798 via operator_new, then calls ctor at 0x0045DD90. Different from the ctor itself. | — | BUILDINGTYPECLASS_CTOR_DEFAULTS |
| `0x007E4570` | **BuildingTypeClass vtable (primary)** | — | BUILDINGTYPECLASS_CTOR_DEFAULTS |
| `0x0043DA80` | **BuildingClass::DrawBody_VXL** (vtable slot 313) — VXL/extras pass. Function boundary created in T7. | — | BUILDINGCLASS_DRAWBODY, BUILDINGCLASS_VTABLE_COMPLETE |
| `0x0044F820` | BuildingClass::ReadFromINI | 1651 B | BUILDING_SYSTEMS |
| `0x0044FEC0` | BuildingClass::SaveToINI | — | BUILDING_SYSTEMS |
| `0x0044B780` | BuildingClass::MissionRepairAndProduce | 4605 B | BUILDING_SYSTEMS |
| `0x00543330` | BuildingTypeClass::SetOwnerAndOccupy | 1050 B | BUILDING_SYSTEMS |
| `0x0045EE70` | BuildingTypeClass::CanBePlacedAt | — | BUILDING_SYSTEMS |
| `0x0045F160` | Get foundation cell data | — | BUILDING_SYSTEMS |
| `0x0047C620` | Cell passability (building placement) | — | GAMEMD_ARCHITECTURE |
| `0x0047CA80` | Pathfinding update (cardinal direction) | — | BUILDING_SYSTEMS |
| `0x00481810` | Pathfinding update (continued) | — | BUILDING_SYSTEMS |
| `0x0047D2B0` | Full passability recalc | — | BUILDING_SYSTEMS |
| `0x00568300` | Cell in-bounds check | — | BUILDING_SYSTEMS |
| `0x0047C520` | Look up building in cell | — | BUILDING_SYSTEMS |

### Buildings — Animation System (21 slots)

| Address | Name | Size | Source |
|---------|------|------|--------|
| `0x00451890` | CreateAnimForSlot (workhorse, 18 callers) | — | BUILDING_ANIM_STATE |
| `0x00451E40` | ClearAnimSlot (single or all 21) | — | BUILDING_ANIM_STATE |
| `0x00451EE0` | SetDamagedState (health cross ConditionYellow) | — | BUILDING_ANIM_STATE |
| `0x00451750` | SetAnimSlotImage (art variant selection) | — | BUILDING_ANIM_STATE |
| `0x00451F60` | UpdateAnimFacingAndDirection | — | BUILDING_ANIM_STATE |
| `0x00452000` | UpdateAllAnimFacings | — | BUILDING_ANIM_STATE |
| `0x00452170` | SetAnimRemap (palette on all 21) | — | BUILDING_ANIM_STATE |
| `0x004521C0` | StartCloaking (all 21 translucent) | — | BUILDING_ANIM_STATE |
| `0x00452210` | StopCloaking (all 21 opaque) | — | BUILDING_ANIM_STATE |
| `0x004509D0` | UpdateAnimation tick (2387 B monster) | 2387 B | BUILDING_ANIM_STATE |
| `0x00450630` | UpdateRepairAndPower tick | 915 B | BUILDING_ANIM_STATE |
| `0x00427CB0` | FindAnimType (name lookup) | — | BUILDING_ANIM_STATE |

### Buildings — Power, Walls, Gap Generator

| Address | Name | Size | Source |
|---------|------|------|--------|
| `0x0044E7B0` | BuildingClass::GetPowerOutput | ~200 B | POWER_SYSTEM |
| `0x0044E880` | BuildingClass::GetPowerDrain | ~120 B | POWER_SYSTEM |
| `0x004555D0` | BuildingClass::IsOperational | ~240 B | POWER_SYSTEM |
| `0x00447F10` | BuildingClass::GetFireError | ~340 B | POWER_SYSTEM |
| `0x0044ACF0` | BuildingClass::Mission_Attack | ~680 B | POWER_SYSTEM |
| `0x00452260` | GoOnline (player power on) | ~250 B | POWER_SYSTEM |
| `0x00452360` | GoOffline (player power off) | ~180 B | POWER_SYSTEM |
| `0x004545D0` | OnPowerOff (anim toggle) | ~300 B | POWER_SYSTEM |
| `0x004547C0` | OnPowerOn (anim toggle) | ~280 B | POWER_SYSTEM |
| `0x004571E0` | OnSpyInfiltrate | 965 B | POWER_SYSTEM |
| `0x0050BC90` | HouseClass::SetBlackout | ~40 B | POWER_SYSTEM |
| `0x004549B0` | UpdateGapAndSpecialEffects | — | POWER_SYSTEM |
| `0x00454DB0` | UpdateGapGenerator_Tick | 2076 B | POWER_SYSTEM |
| `0x0063F7C0` | PowerClass::LoadGraphics | — | POWER_SYSTEM |
| `0x0063F850` | PowerClass::CalcSegments | — | POWER_SYSTEM |
| `0x0063F960` | PowerClass::SplitPowerDisplay | — | POWER_SYSTEM |
| `0x0063FEA0` | PowerClass::AnimationTick | 1279 B | POWER_SYSTEM |
| `0x0063FB20` | PowerClass::Draw | 664 B | POWER_SYSTEM |
| `0x0063F6B0` | PowerClass::Constructor | 118 B | POWER_SYSTEM |
| `0x0063F730` | PowerClass::Init_Clear | 122 B | POWER_SYSTEM |
| `0x0063F7E0` | PowerClass::Destructor_IO | 35 B | POWER_SYSTEM |
| `0x006403A0` | PowerClass::RegisterTooltip | 160 B | POWER_SYSTEM |
| `0x00640450` | PowerClass::GetTooltipText | 82 B | POWER_SYSTEM |
| `0x00447110` | BuildingClass::TogglePowerOrGate | 246 B | POWER_SYSTEM |
| `0x0070C5B0` | TechnoClass::IsWarpingOut (vtable+0x1D4) — returns +0x270 (corrected 2026-07-18: was "IsInLimboOrWarped"; binary shows `TechnoClass__IsWarpingOut` via decompile_function 0x0070C5B0 and get_function_by_address 0x0070C5B0 — matches the name already used for this same address at line 263 — INFERENCE_HARDENED) | 6 B | POWER_SYSTEM |
| `0x00450590` | BuildingClass::PowerCheck_upgrade | 151 B | POWER_SYSTEM |
| `0x00450630` | BuildingClass::UpdateRepairAndPower | 914 B | POWER_SYSTEM |
| `0x00452480` | BuildingClass::ApplyOfflineEffects | 189 B | POWER_SYSTEM |
| `0x0043FB20` | BuildingClass::Update | 2650 B | POWER_SYSTEM |
| `0x00448260` | BuildingClass::ChangeOwner | ~4517 B | POWER_SYSTEM |
| `0x0050E1B0` | HouseClass::HasPowerSurplus | 15 B | POWER_SYSTEM |
| `0x00500910` | HouseClass::GetFactoryCount | 84 B | POWER_SYSTEM |
| `0x0050C0A0` | HouseClass::GetBuildTimeMult | 127 B | POWER_SYSTEM |
| `0x004CA6E0` | FactoryClass::RecalcAllRates | 108 B | POWER_SYSTEM |
| `0x004C9B20` | FactoryClass::AI | — | POWER_SYSTEM |
| `0x004C9C70` | FactoryClass::StartProduction | — | POWER_SYSTEM |
| `0x004C9FB0` | FactoryClass::CalcRate | — | POWER_SYSTEM |
| `0x006CAF90` | SuperClass::Constructor | 388 B | POWER_SYSTEM |
| `0x006CB4D0` | SuperClass::Suspend | 135 B | POWER_SYSTEM |
| `0x006CB7B0` | SuperClass::Deactivate | 96 B | POWER_SYSTEM |
| `0x006CBEE0` | SuperClass::AnimStage | 396 B | POWER_SYSTEM |
| `0x006CC390` | SuperClass::Launch | ~6834 B | POWER_SYSTEM |
| `0x00711EE0` | TechnoTypeClass::GetBuildTime | 27 B | POWER_SYSTEM |
| `0x005F5C60` | ObjectClass::GetHealthRatio | 31 B | POWER_SYSTEM |
| `0x0070EFD0` | TechnoClass::IsUnderEMP | 13 B | POWER_SYSTEM |
| `0x00577D90` | MapClass::ClearShroud | 403 B | POWER_SYSTEM |
| `0x00577AB0` | MapClass::ResetShroud | 243 B | POWER_SYSTEM |
| `0x0071E940` | TriggerCondition::Evaluate | 2263 B | POWER_SYSTEM |
| `0x006EFC70` | AI::SuperLaunchCheck_SingleSW | 481 B | POWER_SYSTEM |
| `0x006EFE60` | AI::SuperLaunchCheck_DualSW | 708 B | POWER_SYSTEM |
| `0x006F0130` | AI::SuperLaunchCheck_DualSW_v2 | 625 B | POWER_SYSTEM |
| `0x004AED70` | CC_Draw_Shape | 1316 B | POWER_SYSTEM |
| `0x00454730` | TriggerSpecialAnims | — | BUILDING_ANIM_STATE |
| `0x00452A40` | ConnectWalls (on placement) | — | BUILDING_SYSTEMS |
| `0x004533A0` | RecalculateWallConnections | 1151 B | BUILDING_SYSTEMS |
| `0x00452DC0` | ExtendWallInDirection | 660 B | BUILDING_SYSTEMS |
| `0x00453240` | OnWallDestroyed | — | BUILDING_SYSTEMS |
| `0x00454DB0` | UpdateGapGenerator_Tick | 2076 B | BUILDING_SYSTEMS |
| `0x004549B0` | UpdateGapAndSpecialEffects | — | BUILDING_SYSTEMS |
| `0x004FCE30` | HouseClass::PowerRatio | ~50 B | POWER_SYSTEM |
| `0x00508C30` | HouseClass::AI_AssessPower | ~360 B | POWER_SYSTEM |
| `0x00508DF0` | HouseClass::UpdateRadarPowerState | ~350 B | POWER_SYSTEM |
| `0x00508F60` | HouseClass::CheckPoweredBuildings | ~160 B | POWER_SYSTEM |
| `0x0050AF10` | HouseClass::HandlePowerTransition | ~400 B | POWER_SYSTEM |
| `0x006F47A0` | FactoryClass::GetProductionSpeed | ~150 B | POWER_SYSTEM |

### Buildings — Sell, Garrison, Deploy

| Address | Name | Size | Source |
|---------|------|------|--------|
| `0x004555D0` | CanSellOrUndeploy | — | BUILDING_SYSTEMS |
| `0x00457DE0` | SellBuilding | 1029 B | BUILDING_SYSTEMS |
| `0x004585C0` | SpawnUnitsWithParachute | — | BUILDING_SYSTEMS |
| `0x00458200` | CheckAutoSellOrCivilian | 301 B | BUILDING_SYSTEMS |
| `0x00457CE0` | BuildingClass::CanDock (garrison eligibility) | — | GARRISON_SYSTEM |
| `0x004525F0` | BuildingClass::CanGarrison (gate passability) | — | GARRISON_SYSTEM |
| `0x00522910` | BuildingClass::AddGarrisonOccupant | — | GARRISON_SYSTEM |
| `0x004581F0` | BuildingClass::GetOccupantCount (vtable+0x408) | — | GARRISON_SYSTEM |
| `0x00458DD0` | BuildingClass::IsOccupied (vtable+0x400) | — | GARRISON_SYSTEM |
| `0x004526F0` | BuildingClass::GetWeapon (vtable+0x3F8) | — | GARRISON_SYSTEM |
| `0x00458E00` | BuildingClass::GetHalfFoundationSize (vtable+0x404) | — | GARRISON_SYSTEM |
| `0x0043E7B0` | BuildingClass::UpdateGarrisonFire (render) | — | GARRISON_SYSTEM |
| `0x005196A0` | InfantryClass::Mission_Enter | — | GARRISON_SYSTEM |
| `0x0070FD70` | BuildingClass::EnterTransport | 207 B | GARRISON_SYSTEM |
| `0x0070FE50` | BuildingClass::ExitTransport | 86 B | GARRISON_SYSTEM |
| `0x004575B0` | BuildingClass::EjectOccupants | — | GARRISON_SYSTEM |
| `0x006FDD50` | TechnoClass::Fire_At (garrison dmg/ROF) | — | GARRISON_SYSTEM |
| `0x006FCFA0` | TechnoClass::GetROF (garrison ROF div) | — | GARRISON_SYSTEM |
| `0x006F7220` | TechnoClass::InRange (garrison range) | — | GARRISON_SYSTEM |
| `0x004571E0` | OnSpyInfiltrate | 965 B | BUILDING_SYSTEMS |
| `0x00443860` | SetRallyPoint | 806 B | BUILDING_SYSTEMS |
| `0x00443B90` | ToggleGate | 207 B | BUILDING_SYSTEMS |
| `0x00447110` | TogglePowerOrGate | 247 B | BUILDING_SYSTEMS |

### Buildings — VXL Turret

| Address | Name | Size | Source |
|---------|------|------|--------|
| `0x00458810` | BuildVXLTurretMatrix | 432 B | BUILDING_SYSTEMS |
| `0x00453BF0` | GetTurretDrawPosition | — | BUILDING_SYSTEMS |

### Buildings — Docking & Refinery

| Address | Name | Size | Source |
|---------|------|------|--------|
| `0x004593A0` | UndockUnit | 208 B | BUILDING_SYSTEMS |
| `0x00457CE0` | BuildingClass::CanDock | — | HARVESTER_DOCK_UNLOAD |
| `0x0044B780` | BuildingClass::MissionRepairAndProduce (UnitRepair/Hospital/Armory/Bunker/CY/Reload) | ~4000 B | MISSION_REPAIR_AND_PRODUCE |
| `0x00458E50` | BuildingClass::BunkerStateMachine (6-state bunker entry) | — | MISSION_REPAIR_AND_PRODUCE |
| `0x0074FFF0` | VeterancyStruct::IsRookie (XP < 0.0) | — | MISSION_REPAIR_AND_PRODUCE |
| `0x00750090` | VeterancyStruct::SetVeteran (XP = 1.0) | — | MISSION_REPAIR_AND_PRODUCE |
| `0x007500B0` | VeterancyStruct::SetElite (XP = 2.0) | — | MISSION_REPAIR_AND_PRODUCE |
| `0x0070FD70` | BuildingClass::EnterTransport | — | HARVESTER_DOCK_UNLOAD |
| `0x0065A970` | RadioClass::Transmit_Radio_Impl (cmd 2=DOCK, 3=CLEAR) | — | HARVESTER_DOCK_UNLOAD |
| `0x0065AAA0` | RadioClass::Transmit_Radio (wrapper → vtable+0x27C) | — | HARVESTER_DOCK_UNLOAD |
| `0x0065ACB0` | RadioClass::Transmit_Radio_ToFirst (send to dock[0]) | — | HARVESTER_DOCK_UNLOAD |
| `0x0065ACE0` | RadioClass::Broadcast_Radio_ToAll (send to all docked) | — | HARVESTER_DOCK_UNLOAD |
| `0x006F4AB0` | TechnoClass::Receive_Radio (vtable+0x194, base handler, repair via 0x1C) | — | HARVESTER_DOCK_UNLOAD |
| `0x0043C2D0` | BuildingClass::Receive_Radio (vtable+0x194, 2959 B, full dock protocol) | 2959 B | HARVESTER_DOCK_UNLOAD |
| `0x00737430` | UnitClass::Receive_Radio (vtable+0x194, 1826 B) | 1826 B | HARVESTER_DOCK_UNLOAD |
| `0x0065A820` | ObjectClass::Receive_Radio (base, vtable+0x194) | — | HARVESTER_DOCK_UNLOAD |
| `0x0073D630` | UnitClass::Mission_Deploy_Building (refinery dump + MCV deploy) | 3966 B | HARVESTER_DOCK_UNLOAD |
| `0x0073E5E0` | UnitClass::Mission_Harvest (state machine: find ore→gather→return→dock) | ~2508 B | HARVESTER_DOCK_UNLOAD |
| `0x0073D450` | UnitClass::Harvest_Ore_Tick (extract one bale from cell) | — | HARVESTER_DOCK_UNLOAD |
| `0x004DFCB0` | FootClass::Find_Nearest_Dock | — | HARVESTER_DOCK_UNLOAD |
| `0x004595C0` | BuildingClass::FullUndock (clear anims, smoke, pathfind exit, scatter) | — | UNIT_MISSION_DEPLOY_BUILDING |
| `0x00739390` | UnitClass::Deploy (MCV deploy handler) | — | UNIT_MISSION_DEPLOY_BUILDING |

### StorageClass (Ore Cargo)

| Address | Name | Size | Source |
|---------|------|------|--------|
| `0x006C9680` | StorageClass::GetAmount (reads ore amount by type index) | — | HARVESTER_DOCK_UNLOAD |
| `0x006C96B0` | StorageClass::Remove (subtracts ore from storage) | — | HARVESTER_DOCK_UNLOAD |
| `0x006C9820` | StorageClass::FindFirstNonEmpty (first occupied ore slot) | — | HARVESTER_DOCK_UNLOAD |
| `0x006C9650` | StorageClass::GetTotal (sum of all 4 float slots) | — | ORE_VALUE_CREDIT_DEPOSIT |
| `0x006C9710` | StorageClass::AddAmount (add ore by type index) | — | ORE_VALUE_CREDIT_DEPOSIT |
| `0x006C97A0` | StorageClass::GetTotalValue (total × TiberiumClass values) | — | ORE_VALUE_CREDIT_DEPOSIT |

### HouseClass — Credits & Ore Deposit

| Address | Name | Size | Source |
|---------|------|------|--------|
| `0x004F9950` | HouseClass::GiveMoney (adds to +0x30C) | — | HARVESTER_DOCK_UNLOAD |
| `0x004F9790` | HouseClass::SpendMoney (deducts from +0x30C) | — | HARVESTER_DOCK_UNLOAD |
| `0x004F9610` | HouseClass::DepositOreCredits (normal harvester, updates display) | — | HARVESTER_DOCK_UNLOAD |
| `0x004F9700` | HouseClass::DepositWeedCredits (weeder bulk add) | — | HARVESTER_DOCK_UNLOAD |
| `0x00522D50` | BuildingClass::DepositOreFromStorage (base + PurifierBonus) | — | ORE_VALUE_CREDIT_DEPOSIT |
| `0x004A2600` | CreditsClass::AI (smooth display counter animation, step=diff/8) | — | ORE_VALUE_CREDIT_DEPOSIT |

### Buildings — Superweapons

| Address | Name | Size | Source |
|---------|------|------|--------|
| `0x0044C980` | Mission_Missile (super weapon state machine) | 3105 B | BUILDING_SYSTEMS |

### ChronoSphere Superweapon

| Address | Name | Size | Source |
|---------|------|------|--------|
| `0x006CC200` | SuperClass::Launch (case 3=ChronoSphere, case 4=ChronoWarp) | — | CHRONOSPHERE_SUPERWEAPON |
| `0x007E9A90` | TeleportLocomotionClass CLSID | 16 B | CHRONOSPHERE_SUPERWEAPON |

### Buildings — Damage Fire Anims

| Address | Name | Size | Source |
|---------|------|------|--------|
| `0x0043C0D0` | DamageFireAnims spawn/update | — | DAMAGE_FIRE_ANIMS |
| `0x0043B5E0` | DamageFireAnims (related) | — | DAMAGE_FIRE_ANIMS |
| `0x006D2070` | DamageFireAnims (related) | — | DAMAGE_FIRE_ANIMS |
| `0x0045EC90` | DamageFireAnims (related) | — | DAMAGE_FIRE_ANIMS |
| `0x0045ECA0` | DamageFireAnims (related) | — | DAMAGE_FIRE_ANIMS |
| `0x00421EA0` | AnimClass::Constructor (type, coords, delay, loops, flags, zAdj, reverse) | — | ANIM_CLASS |
| `0x00423AC0` | AnimClass::AI (per-tick frame advance, loop, self-destruct) | — | ANIM_CLASS |
| `0x00422CA0` | AnimClass::DrawIt (SHP rendering, RING1 warp ring special path) | — | ANIM_CLASS |
| `0x004255B0` | AnimClass::Destroy (detach, clear owner, release sound, optional StopSound, deferred delete; does not spawn ExpireAnim) | — | ANIM_CLASS |
| `0x00424F00` | AnimClass::Start (sound, particles, scorch on start) | — | ANIM_CLASS |
| `0x00424CE0` | AnimClass::Middle (begin playback after delay expires) | — | ANIM_CLASS |
| `0x00424B50` | AnimClass::SetOwnerObject (attach/detach from TechnoClass) | — | ANIM_CLASS |

### TemporalClass & Warp Pipeline

| Address | Name | Size | Source |
|---------|------|------|--------|
| `0x0071A760` | TemporalClass::Update (virtual, WarpPoints countdown + erasure) | — | TEMPORAL_WARP_PIPELINE |
| `0x006297F0` | TemporalClass::AI (5-state visual animation machine) | — | TEMPORAL_WARP_PIPELINE |
| `0x0071AF20` | TemporalClass::InitiateWarp (start erasing target) | — | TEMPORAL_WARP_PIPELINE |
| `0x0062A4A0` | WarpAttachClass::Detach (shared by Temporal+Parasite, detach+teleport) | — | TEMPORAL_WARP_PIPELINE |
| `0x0062AB40` | WarpAttachClass::CanPlaceAtTarget | — | TEMPORAL_WARP_PIPELINE |
| `0x00629FD0` | WarpAttachClass::UpdateAttack (dispatch to AI) | — | TEMPORAL_WARP_PIPELINE |
| `0x00427D00` | AnimTypeClass::ReadINI (~55 fields from art.ini) | — | ANIM_CLASS |
| `0x00428B80` | AnimTypeClass::FindByName | — | ANIM_CLASS |
| `0x00427CB0` | AnimTypeClass::FindByIndex | — | ANIM_CLASS |
| `0x0065C7E0` | Random::RandomRanged | Inclusive sorted-bound deterministic RNG helper using ScenarioClass random state in representative gameplay callers | RANDOM_RANDOMRANGED_0065C7E0 |
| `0x005F5C60` | GetHealthRatio | — | BUILDING_ANIM_STATE |

### HouseClass (Player/AI)

| Address | Name | Size | Source |
|---------|------|------|--------|
| `0x004F7870` | HouseClass::CanBuild | 407 lines | TECH_TREE_REPORT |
| `0x004FA350` | HouseClass::Begin_Production | 1222 B | BUILDING_SYSTEMS |
| `0x004FB0E0` | HouseClass::Place_Production | 1426 B | BUILDING_SYSTEMS |
| `0x004C9FF0` | HouseClass::Abandon_Production | 301 B | BUILDING_SYSTEMS |
| `0x004FC0B0` | HouseClass::MPlayer_Defeated | 1559 B | HOUSECLASS |
| `0x004F9A50` | HouseClass::IsAlliedWith | — | HOUSECLASS |
| `0x004F9B70` | HouseClass::MakeAlly | — | HOUSECLASS |
| `0x004F9F90` | HouseClass::BreakAlliance | — | HOUSECLASS |
| `0x00502A80` | HouseClass::Added_To_Game | — | HOUSECLASS |
| `0x005025F0` | HouseClass::Removed_From_Game | — | HOUSECLASS |
| `0x004FF980` | HouseClass::Recount | — | BUILDING_SYSTEMS |
| `0x004FD500` | HouseClass::AI_Building_Strategy | 1074 B | BUILDING_SYSTEMS |
| `0x004FD9A0` | AI_Check_Build_Need | 819 B | BUILDING_SYSTEMS |
| `0x004FDD10` | AI_Manage_Build_Queue | 1736 B | BUILDING_SYSTEMS |
| `0x004FE3E0` | AI_Choose_Building | 1653 B | BUILDING_SYSTEMS |
| `0x004770E0` | Prerequisite keyword parser | — | TECH_TREE_REPORT |
| `0x0049FAE0` | Count owned units of type | — | HOUSECLASS |

### FactoryClass (Production)

| Address | Name | Size | Source |
|---------|------|------|--------|
| `0x004C9C70` | FactoryClass::StartProduction | 405 B | BUILDING_SYSTEMS |
| `0x004C9E60` | FactoryClass::Suspend | — | BUILDING_SYSTEMS |
| `0x004C9EA0` | FactoryClass::CompletionStep | 272 B | BUILDING_SYSTEMS |
| `0x004CA5A0` | FactoryClass::StartNextQueued | — | BUILDING_SYSTEMS |
| `0x004CA620` | FactoryClass::RemoveFromQueue | — | BUILDING_SYSTEMS |
| `0x004CA6B0` | FactoryClass::IsInQueue | — | BUILDING_SYSTEMS |
| `0x004CA6E0` | FactoryClass::UpdateAllStepDelays | — | BUILDING_SYSTEMS |
| `0x004CA130` | FactoryClass::HasCompleted | — | SIDEBAR_READY_TEXT |
| `0x004CA120` | FactoryClass::GetProgress | — | SIDEBAR_READY_TEXT |

### Object / Techno / Foot Class Methods (verified via UnitClass vtable 0x7F5C70)

| Address | Name | Vtable | Source |
|---------|------|--------|--------|
| `0x005F65A0` | ObjectClass::GetCoords | +0x048 | DRIVE_LOCOMOTION_CLASS |
| `0x005F5F30` | ObjectClass::GetHeight (returns Z at +0xA4) | +0x1D0 | DRIVE_LOCOMOTION_CLASS |
| `0x005F6960` | ObjectClass::GetOccupiedCell | +0x1BC | DRIVE_LOCOMOTION_CLASS |
| `0x007441B0` | ObjectClass::Mark_Occupation (set bit 0x20) | +0x0F0 | DRIVE_LOCOMOTION_CLASS |
| `0x00744210` | ObjectClass::Clear_Occupation (clear bit 0x20) | +0x0F4 | DRIVE_LOCOMOTION_CLASS |
| `0x005B3040` | MissionClass::GetCurrentMission | +0x184 | DRIVE_LOCOMOTION_CLASS |
| `0x004D3710` | TechnoClass::SetSpeedPercentage | +0x544 | DRIVE_LOCOMOTION_CLASS |
| `0x00741970` | TechnoClass::Set_Destination (Teleporter swap logic at 0x7423CD) | +0x480 | CHRONO_MINER_TELEPORT |
| `0x004DB1A0` | FootClass::GetCurrentSpeed | +0x538 | DRIVE_LOCOMOTION_CLASS |
| `0x004D3810` | FootClass::CanReachDestination (zone check) | +0x2CC | DRIVE_LOCOMOTION_CLASS |
| `0x0073F0A0` | UnitClass::Can_Enter_Cell (465 lines) | +0x1AC | DRIVE_LOCOMOTION_CLASS |

### ReadINI Functions (vtable +0x64)

| Address | Name | Size | Source |
|---------|------|------|--------|
| `0x00712170` | TechnoTypeClass::ReadINI | 3471 lines | TECH_TREE_REPORT |
| `0x00772080` | WeaponTypeClass::ReadINI | — | READINI_FIELD_MAPS |
| `0x0075D590` | WarheadTypeClass::ReadINI | — | READINI_FIELD_MAPS |
| `0x0046BEE0` | BulletTypeClass::ReadINI | — | READINI_FIELD_MAPS |
| `0x005240A0` | InfantryTypeClass::ReadINI | 1725 B | READINI_FIELD_MAPS |
| `0x0041CC20` | AircraftTypeClass::ReadINI | 388 B | READINI_FIELD_MAPS |
| `0x006CEA20` | SuperWeaponTypeClass::ReadINI | — | READINI_FIELD_MAPS |
| `0x006F32D0` | BuildingTypeClass::ReadINI | — | READINI_FIELD_MAPS |
| `0x00427D00` | AnimTypeClass::ReadINI | — | BUILDING_ANIM_STATE |
| `0x0044F820` | BuildingClass::ReadFromINI | 1651 B | BUILDING_SYSTEMS |

### RulesClass Parsers (singleton `0x008871E0`, size `0x18C0`)

| Address | Name | Size | Source |
|---------|------|------|--------|
| `0x00665650` | RulesClass::Constructor (defaults) | 9174 B | RULESCLASS_CONSTRUCTOR_DEFAULTS |
| `0x00667A30` | RulesClass::Destructor | — | RULESCLASS_GHIDRA_REPORT |
| `0x006686C0` | RulesClass::Process (outer orchestrator) | — | RULESCLASS_GHIDRA_REPORT |
| `0x00668BF0` | RulesClass::Read_INI (inner dispatcher, 33 steps) | — | RULESCLASS_GHIDRA_REPORT |
| `0x0066D530` | RulesClass::ReadGeneral (`[General]`) | 18.8 KB | RULESCLASS_FIELDS.csv |
| `0x006691E0` | RulesClass::ReadAudioVisual (`[AudioVisual]`) | ~10 KB | RULESCLASS_FIELDS.csv |
| `0x0066BBB0` | RulesClass::ReadCombatDamage (`[CombatDamage]`) | — | RULESCLASS_FIELDS.csv |
| `0x00672AE0` | RulesClass::ReadAI (`[AI]`) | — | RULESCLASS_FIELDS.csv |
| `0x00674240` | RulesClass::ReadIQ (`[IQ]`) | — | RULESCLASS_FIELDS.csv |
| `0x00668FB0` | RulesClass::ReadSpecialWeapons (`[SpecialWeapons]`) | — | RULESCLASS_GHIDRA_REPORT |
| `0x0066B900` | RulesClass::ReadCrateRules (`[CrateRules]`) | — | RULESCLASS_GHIDRA_REPORT |
| `0x0066CF70` | RulesClass::ReadRadiation (`[Radiation]`) | — | RULESCLASS_GHIDRA_REPORT |
| `0x0066D150` | RulesClass::ReadElevationModel (`[ElevationModel]`) | — | RULESCLASS_GHIDRA_REPORT |
| `0x0066D1F0` | RulesClass::ReadWallModel (`[WallModel]`) | — | RULESCLASS_GHIDRA_REPORT |
| `0x006743D0` | RulesClass::ReadJumpjetControls (`[JumpjetControls]`) | — | RULESCLASS_GHIDRA_REPORT |
| `0x00671EA0` | RulesClass::ReadMultiplayerDialogSettings (`[MultiplayerDialogSettings]`) | — | RULESCLASS_GHIDRA_REPORT |
| `0x00674000` | RulesClass::ReadSpeedTypeLandTypeTable (per-LandType → global `0x0089EA44`) | — | RULESCLASS_GHIDRA_REPORT |
| `0x0066D270` | DifficultyClass::Read_INI (called 3× for Easy/Normal/Difficult at `Rules+0x1538`/`+0x1588`/`+0x15D8`) | — | RULESCLASS_DIFFICULTY_SLOTS |
| `0x0066D3A0` | RulesClass::ReadColors (`[Colors]` → global palette, not RulesClass) | — | RULESCLASS_GHIDRA_REPORT §5 step 1 |
| `0x0066D480` | RulesClass::ReadColorAdd (`[ColorAdd]` → RulesClass+`0x1874`, 14 slots × 3 B in stock YR) | — | RULESCLASS_COLORADD_TABLE |
| `0x00673E80` | RulesClass::ReadPowerups (`[Powerups]` → 4 parallel globals, 19 slots) | — | RULESCLASS_POWERUPS_TABLE |
| `0x00674650` | RulesClass::ReadAdvancedCommandBar (`[AdvancedCommandBar]` / `[MultiplayerAdvancedCommandBar]` → global `DAT_00B0CB78`; dormant in stock YR) | — | RULESCLASS_HELPER_FUN_00674650 |
| `0x00679A10` | Type_Read_INI_All (iterates every type-class array, calls `vtable+0x64` on each) | — | RULESCLASS_GHIDRA_REPORT §5 step 21 |
| `0x0052BAD8` | Init_Game (site of `operator_new(0x18C0)` for RulesClass) | — | RULESCLASS_GHIDRA_REPORT |
| `0x00686B20` | ScenarioClass::Full_Init (invokes RulesClass::Process) | — | RULESCLASS_GHIDRA_REPORT |
| `0x006BE1C0` | Game_Shutdown (destroys RulesClass singleton) | — | RULESCLASS_GHIDRA_REPORT |

### Combat

| Address | Name | Size | Source |
|---------|------|------|--------|
| `0x006FDD50` | TechnoClass::Fire_At | 7167 B | BUILDING_SYSTEMS |
| `0x00701900` | TechnoClass::ReceiveDamage | 5154 B | BUILDING_SYSTEMS |
| `0x004666E0` | BulletClass::AI | 6422 B | GAMEMD_ARCHITECTURE |
| `0x004690B0` | WarheadTypeClass::Detonate | 4692 B | GAMEMD_ARCHITECTURE |
| `0x00489280` | Apply area damage | — | GAMEMD_ARCHITECTURE |

### Pathfinding

| Address | Name | Size | Source |
|---------|------|------|--------|
| `0x0042C900` | A* pathfind search | large | GAMEMD_ARCHITECTURE |
| `0x0042C290` | Zone pre-check (reachability) | — | GAMEMD_ARCHITECTURE |
| `0x0056D230` | Pathfinding validate (rally point) | — | BUILDING_SYSTEMS |
| `0x0056DC20` | Pathfinding validate (alternate) | — | BUILDING_SYSTEMS |
| `0x00483C80` | CellClass::RecalcZoneType | — | CELLCLASS_RECALCZONETYPE_00483C80 |
| `0x00565730` | CellClass::GetCell (from coords) | — | LOCOMOTION_MATH |
| `0x005657A0` | MapClass::Get_CellClass (packed cell → CellClass*) | — | DRIVE_LOCOMOTION_CLASS |

### Movement Helpers (used by Drive/Ship locomotors)

| Address | Name | Size | Source |
|---------|------|------|--------|
| `0x0041C380` | CoordStruct::Distance3D (sqrt, result in FPU) | ~48 B | DRIVE_LOCOMOTION_CLASS |
| `0x0047C3D0` | CellClass::Find_Nearest_Object | ~256 B | DRIVE_LOCOMOTION_CLASS |
| `0x0047EBA0` | CellClass::FindFirstBuilding | — | CHRONO_MINER_TELEPORT |
| `0x00480A30` | CellClass::Get_Center_Coords (cell→lepton center+Z) | ~80 B | DRIVE_LOCOMOTION_CLASS |
| `0x00481670` | CellClass::Scatter_Objects (scatter eligible objects) | ~400 B | DRIVE_LOCOMOTION_CLASS |
| `0x00483480` | CellClass::Mark_Objects_Redraw | ~30 B | DRIVE_LOCOMOTION_CLASS |
| `0x004C93D0` | RateTimer::Current (interpolated value read) | ~152 B | DRIVE_LOCOMOTION_CLASS |
| `0x004D3920` | FootClass::Find_Path (A* pathfinder) | ~2200 B | DRIVE_LOCOMOTION_CLASS |
| `0x004D94B0` | FootClass::Assign_Destination (stores NavCom, calls Head_To_Coord) | — | CHRONO_MINER_TELEPORT |
| `0x004DA2A0` | FootClass::Is_Mission_Harvest (mission==7?) | ~14 B | DRIVE_LOCOMOTION_CLASS |
| `0x004DA530` | FootClass::AI (IPiggyback swap check) | — | CHRONO_MINER_TELEPORT |
| `0x004DF040` | FootClass::Find_Docking_Bay | — | CHRONO_MINER_TELEPORT |
| `0x004DF0D0` | FootClass::Stop_Moving (zero movement deltas) | ~14 B | DRIVE_LOCOMOTION_CLASS |
| `0x004F9A90` | HouseClass::Is_Ally (alliance bitmask check) | ~80 B | DRIVE_LOCOMOTION_CLASS |
| `0x00578460` | MapClass::Is_Cell_In_Playfield (bounds check) | ~214 B | DRIVE_LOCOMOTION_CLASS |
| `0x00578AD0` | MapClass::Check_Crushable_Obstacle (scatter infantry) | ~148 B | DRIVE_LOCOMOTION_CLASS |
| `0x0065AE30` | PathType::Has_Valid_Steps (path result check) | ~36 B | DRIVE_LOCOMOTION_CLASS |
| `0x006B7D80` | RadioClass::Tether_Count (active tether links) | ~84 B | DRIVE_LOCOMOTION_CLASS |

### Locomotion — Drive

| Address | Name | Size | Source |
|---------|------|------|--------|
| `0x004AF540` | DriveLocomotionClass::Constructor | 160 B | DRIVE_LOCOMOTION_CLASS |
| `0x004AF5E0` | DriveLocomotionClass::Destructor | ~80 B | DRIVE_LOCOMOTION_CLASS |
| `0x004AF970` | DriveLocomotionClass::Is_Moving (COM thunk) | ~30 B | DRIVE_LOCOMOTION_CLASS |
| `0x004AFB40` | DriveLocomotionClass::Force_New_Slope | 53 B | DRIVE_LOCOMOTION_CLASS |
| `0x004AFB80` | DriveLocomotionClass::Is_Moving | 150 B | DRIVE_LOCOMOTION_CLASS |
| `0x004AFC20` | DriveLocomotionClass::Is_Moving_Now | ~80 B | DRIVE_LOCOMOTION_CLASS |
| `0x004AFC90` | DriveLocomotionClass::Destination | — | DRIVE_LOCOMOTION_CLASS |
| `0x004AFCC0` | DriveLocomotionClass::Head_To_Coord | — | DRIVE_LOCOMOTION_CLASS |
| `0x004AFD40` | DriveLocomotionClass::Set_Destination | ~120 B | DRIVE_LOCOMOTION_CLASS |
| `0x004AFE00` | DriveLocomotionClass::Stop_Moving | ~200 B | DRIVE_LOCOMOTION_CLASS |
| `0x004AFF60` | DriveLocomotionClass::Draw_Matrix | ~600 B | DRIVE_LOCOMOTION_CLASS |
| `0x004B0410` | DriveLocomotionClass::Shadow_Matrix | 178 B | DRIVE_LOCOMOTION_CLASS |
| `0x004B04D0` | DriveLocomotionClass::Update_Facing_From_Type | ~30 B | DRIVE_LOCOMOTION_CLASS |
| `0x004B0500` | DriveLocomotionClass::Process (main tick) | ~1600 B | DRIVE_LOCOMOTION_CLASS |
| `0x004B0AD0` | DriveLocomotionClass::Apply_Track_Delta | ~280 B | DRIVE_LOCOMOTION_CLASS |
| `0x004B0C40` | DriveLocomotionClass::Force_Track | 350 B | DRIVE_LOCOMOTION_CLASS |
| `0x004B0EF0` | DriveLocomotionClass::Do_Turn (ramp update) | ~30 B | DRIVE_LOCOMOTION_CLASS |
| `0x004B0F20` | DriveLocomotionClass::Process_Drive_Track | ~5860 B | DRIVE_LOCOMOTION_CLASS |
| `0x004B2630` | DriveLocomotionClass::Process_Movement | ~8500 B | DRIVE_LOCOMOTION_CLASS |
| `0x004B4780` | DriveLocomotionClass::Transform_Track_Coords | ~180 B | DRIVE_LOCOMOTION_CLASS |
| `0x004B4820` | DriveLocomotionClass::In_Which_Layer | — | DRIVE_LOCOMOTION_CLASS |
| `0x004B4870` | DriveLocomotionClass::Z_Adjust | — | DRIVE_LOCOMOTION_CLASS |
| `0x004B4890` | DriveLocomotionClass::Stop_And_Scatter | ~80 B | DRIVE_LOCOMOTION_CLASS |
| `0x004B48D0` | DriveLocomotionClass::Mark_All_Occupation_Bits | 65 B | DRIVE_LOCOMOTION_CLASS |
| `0x004B4D00` | DriveLocomotionClass::ScalarDeletingDestructor | ~60 B | DRIVE_LOCOMOTION_CLASS |

### Locomotion — Ship

| Address | Name | Size | Source |
|---------|------|------|--------|
| `0x0069EC50` | ShipLocomotionClass::Constructor | — | NAVAL_SYSTEM |
| `0x006A01A0` | ShipLocomotionClass::Apply_Track_Step | ~280 B | DRIVE_LOCOMOTION_CLASS |
| `0x006A05F0` | ShipLocomotionClass::Process_Drive_Track | 5737 B | DRIVE_LOCOMOTION_CLASS |
| `0x006A1C80` | ShipLocomotionClass::Process_Movement | 8470 B | DRIVE_LOCOMOTION_CLASS |
| `0x006A3DB0` | ShipLocomotionClass::Transform_Track_Coords | ~180 B | DRIVE_LOCOMOTION_CLASS |
| `0x0069FC10` | ShipLocomotionClass::Process | — | NAVAL_SYSTEM |
| `0x0069F290` | ShipLocomotionClass::Is_Moving | — | NAVAL_SYSTEM |
| `0x0069F3A0` | ShipLocomotionClass::Destination | — | NAVAL_SYSTEM |
| `0x0069F3D0` | ShipLocomotionClass::Head_To_Coord | — | NAVAL_SYSTEM |
| `0x0069F670` | ShipLocomotionClass::Draw_Matrix | 1189 B | NAVAL_SYSTEM |
| `0x0069FB20` | ShipLocomotionClass::Shadow_Matrix | — | NAVAL_SYSTEM |
| `0x0055A6C0` | LocomotionClass base constructor (shared) | — | NAVAL_SYSTEM |
| `0x0055A710` | LocomotionClass::Link_To_Object | — | NAVAL_SYSTEM |
| `0x0055ABF0` | LocomotionClass::Can_Enter_Cell | — | NAVAL_SYSTEM |
| `0x0055ABE0` | LocomotionClass::Is_To_Have_Shadow | — | NAVAL_SYSTEM |

### Locomotion — Fly

| Address | Name | Size | Source |
|---------|------|------|--------|
| `0x004CC9A0` | FlyLocomotionClass::Constructor | — | LOCOMOTION_MATH |
| `0x004CD600` | FlyLocomotionClass::Process | — | LOCOMOTION_MATH |
| `0x004CAE30` | FlyLocomotionClass::Atan2 | — | LOCOMOTION_MATH |

### Locomotion — Walk

| Address | Name | Size | Source |
|---------|------|------|--------|
| `0x0075AA90` | WalkLocomotionClass::Constructor | — | LOCOMOTION_MATH |

### Locomotion — Hover

| Address | Name | Size | Source |
|---------|------|------|--------|
| `0x00513C20` | HoverLocomotionClass::Constructor | — | LOCOMOTION_MATH |
| `0x00514310` | HoverLocomotionClass::Move | — | LOCOMOTION_MATH |
| `0x00515ED0` | HoverLocomotionClass::SpeedUpdate | — | LOCOMOTION_MATH |

### Locomotion — Teleport (Chrono)

| Address | Name | Size | Source |
|---------|------|------|--------|
| `0x00718000` | TeleportLocomotionClass::Constructor | — | LOCOMOTION_MATH |
| `0x00718080` | TeleportLocomotionClass::Is_Moving | 4 instr | CHRONO_MINER_TELEPORT |
| `0x007180A0` | TeleportLocomotionClass::Destination | — | CHRONO_MINER_TELEPORT |
| `0x00718100` | TeleportLocomotionClass::HeadToCoord (vtable[17]) | — | CHRONO_MINER_TELEPORT |
| `0x00718230` | TeleportLocomotionClass::Stop_Moving | — | CHRONO_MINER_TELEPORT |
| `0x00718260` | TeleportLocomotionClass::Update_Position | — | CHRONO_MINER_TELEPORT |
| `0x007187A0` | TeleportLocomotionClass::PostWarpValidation | — | CHRONO_MINER_TELEPORT |
| `0x00718B70` | TeleportLocomotionClass::Process | — | CHRONO_MINER_TELEPORT |
| `0x007192F0` | TeleportLocomotionClass::StateMachineTick (vtable[16]) | — | CHRONO_MINER_TELEPORT |
| `0x00719BF0` | TeleportLocomotionClass::TimerCheck | — | CHRONO_MINER_TELEPORT |
| `0x00719E90` | TeleportLocomotionClass::Begin_Piggyback | — | CHRONO_MINER_TELEPORT |
| `0x00719EE0` | TeleportLocomotionClass::End_Piggyback | — | CHRONO_MINER_TELEPORT |
| `0x00719F30` | TeleportLocomotionClass::Is_Ok_To_End | — | CHRONO_MINER_TELEPORT |
| `0x0071A100` | TeleportLocomotionClass::Is_Piggybacking | — | CHRONO_MINER_TELEPORT |
| `0x00719400` | TeleportLocomotionClass::InitiateWarp (spawns WarpOut anim rows at departure/arrival) | — | CHRONO_WARP_VISUAL |
| `0x007197D0` | TeleportLocomotion::Phase0_SetWarpingOut | — | CHRONO_WARP_VISUAL |
| `0x00719790` | TeleportLocomotionClass::ClearPendingWarpPhase | — | CHRONO_WARP_VISUAL |

### Chrono Warp Visual Rendering

| Address | Name | Size | Source |
|---------|------|------|--------|
| `0x0070E5A0` | TechnoClass::UpdateTemporalVisual (10-phase state machine) | 166 lines | CHRONO_WARP_VISUAL |
| `0x0070E4B0` | TechnoClass::ScaleByWarpInVisualPhase (6-phase chrono teleport visual) | — | CHRONO_WARP_VISUAL |
| `0x0070C610` | TechnoClass::SetGhostCell (sets +0x218 deploy preview cell) | 6 B | CHRONO_WARP_VISUAL |
| `0x0070C5F0` | TechnoClass::IsNotWarping (checks +0x270 and +0x271 both zero) | — | CHRONO_WARP_VISUAL |
| `0x0070ED80` | TechnoClass::ModifyCloakDrawFlags (adds translucency bits) | — | CHRONO_WARP_VISUAL |
| `0x0071AF20` | TemporalClass::InitiateWarp (Chrono Legionnaire erasure, NOT teleport) | 103 lines | CHRONO_WARP_VISUAL |
| `0x00490B90` | Blitter_selector (flag bits → blitter class, ZReadWarp for temporal) | — | CHRONO_WARP_VISUAL |
| `0x00490E50` | Blitter_selector_extended (alpha/RLE variant selection) | — | CHRONO_WARP_VISUAL |
| `0x00438A00` | IvanBomb::GetClockFrame (CHRONOSK.SHP frame 0-12, NOT chrono sparkle) | — | CHRONO_WARP_VISUAL |

### Uncertain / Needs Further Research

These addresses have been observed but not confidently identified. Do NOT label in Ghidra
until verified with ~90% confidence.

| Address | Observed Role | Confidence | Notes |
|---------|--------------|------------|-------|
| `0x0062A4A0` | **IDENTIFIED**: WarpAttachClass::Detach | 95% | Shared by TemporalClass and ParasiteClass. Detaches owner from target, teleports to target cell. Called when unit+0x694 (parasite attacker) != 0 |
| `0x0050B730` | Some global state check (returns bool) | ~40% | Called before many operations; possibly IsNetworkGame or IsCampaign |
| `0x005B3A00` | Mission timer calculation helper | ~50% | Called at end of many mission handlers before Math::ftol |
| `0x0053A130` | Unknown validation check (returns bool) | ~40% | Called in UnitRepair path of MissionRepairAndProduce |
| `0x00481A00` | Post-teleport cell update (CellClass reveal/occupation) | ~60% | Called after unit placed at dest in TeleportLoco |
| `0x006B0CC0` | Deploy/MCV related handler | ~40% | Called in Mission_Enter for deployed units |
| `0x006B0AE0` | **IDENTIFIED**: KillCredit_ReleasePassengers (releases cargo, assigns kills to ChronoSourceHouse) | 90% | Iterates occupants, calls SetKiller/SetKillerWeapon/RecordKill |
| `0x0062AB40` | **IDENTIFIED**: WarpAttachClass::CanPlaceAtTarget | 90% | Validates warp destination cell passability |
| `0x007E27F8` | Constant 900.0 (double, minutes→frames at 15fps: 60*15) | 95% | Used with HarvesterDumpRate, RepairRate etc |
| `0x008871E0` | g_RulesClass_Instance global pointer | 99% | Widely referenced |

### Locomotion — Jumpjet, Rocket, Tunnel, Mech, DropPod

| Address | Name | Size | Source |
|---------|------|------|--------|
| `0x0054AC40` | JumpjetLocomotionClass::Constructor | — | LOCOMOTION_MATH |
| `0x00661EC0` | RocketLocomotionClass::Constructor | — | LOCOMOTION_MATH |
| `0x00728A00` | TunnelLocomotionClass::Constructor | — | LOCOMOTION_MATH |
| `0x005AFEF0` | MechLocomotionClass::Constructor | — | LOCOMOTION_MATH |
| `0x004B5AB0` | DropPodLocomotionClass::Constructor | — | LOCOMOTION_MATH |

### Locomotion — Misc

| Address | Name | Size | Source |
|---------|------|------|--------|
| `0x004DA530` | FootClass::AI (locomotor piggyback swap) (corrected 2026-07-18: was "TechnoClass::AI"; binary shows `FootClass__AI` via get_function_by_address 0x004DA530 — matches the class name already used for this same address at lines 699/1632 — RTTI_LABEL_DRIFT) | — | LOCOMOTION_MATH |
| `0x004DFCB0` | FootClass::FindNearestDock (corrected 2026-07-18: was "TechnoClass::FindNearestDock"; binary shows `FootClass__Find_Nearest_Dock` via get_function_by_address 0x004DFCB0 — matches the class name already used for this same address at line 493 — RTTI_LABEL_DRIFT) | — | LOCOMOTION_MATH |
| `0x00578080` | Ground Z-height lookup | — | LOCOMOTION_MATH |
| `0x0055A930` | Gravity-assist speed calc | — | LOCOMOTION_MATH |
| `0x0055A730` | Build facing rotation matrix | — | VOXEL_SLOPE_TILT |
| `0x00729B40` | Turret/barrel tilt (ILocomotion::GetMatrix) | 754 B | VOXEL_SLOPE_TILT |

### Infantry

| Address | Name | Size | Source |
|---------|------|------|--------|
| `0x004810A0` | GetSubCell (quadrant detection) | — | INFANTRY_SUBCELL |

### MCV Deploy

| Address | Name | Size | Source |
|---------|------|------|--------|
| `0x006AFD60` | UnitClass::Mission_Deploy handler | — | MCV_DEPLOY |
| `0x0073E5E0` | UnitClass::Mission_Harvest (5-state state machine) | — | CHRONO_MINER_TELEPORT |
| `0x00740810` | UnitClass::Mission_Guard_Harvester | — | CHRONO_MINER_TELEPORT |
| `0x007393C0` | UnitClass::Deploy (core conversion) | — | MCV_DEPLOY |
| `0x0070FC90` | TechnoClass::OnDeployBegin | — | MCV_DEPLOY |
| `0x0070FBE0` | TechnoClass::OnUndeployComplete | — | MCV_DEPLOY |
| `0x0070FB50` | TechnoClass::CanAutoDeployHere | — | MCV_DEPLOY |
| `0x00710000` | TechnoClass::PerformDeploy (COM interface) | — | MCV_DEPLOY |
| `0x00465D70` | Deploy facing calculator | — | MCV_DEPLOY |
| `0x00713280` | TechnoTypeClass::ReadINI (deploy fields) | — | MCV_DEPLOY |

### Sidebar

| Address | Name | Size | Source |
|---------|------|------|--------|
| `0x006A4F20` | SidebarClass::Constructor | — | SIDEBAR_SYSTEM |
| `0x006A5000` | SidebarClass::One_Time (loads DARKEN.SHP) | — | SIDEBAR_SYSTEM |
| `0x006A5310` | SidebarClass::Init_IO | — | SIDEBAR_SYSTEM |
| `0x006A7780` | SidebarClass::AI (Action) | — | SIDEBAR_SYSTEM |
| `0x006A6C30` | SidebarClass::Draw | — | SIDEBAR_SYSTEM |
| `0x006A6140` | Sidebar update (from production) | — | BUILDING_SYSTEMS |
| `0x006A6300` | Add new construction option | 777 B | BUILDING_SYSTEMS |
| `0x006A8420` | Sidebar sort comparator | 748 B | BUILDING_SYSTEMS |
| `0x006A9540` | StripClass::Draw (cameo render) | 4210 B | SIDEBAR_SYSTEM |
| `0x006A5130` | Sidebar positioning init | — | SIDEBAR_SYSTEM |
| `0x006A5840` | Sidebar Init_Mixfiles (palettes/SHPs) | — | SIDEBAR_SYSTEM |
| `0x006A7D70` | Sidebar Activate/Toggle | — | SIDEBAR_SYSTEM |
| `0x006AC480` | DrawProgressBar | — | SIDEBAR_READY_TEXT |
| `0x0072EB50` | Right-side panel SHP loading | — | SIDEBAR_CONSTRUCTION |
| `0x0072FA10` | Left panel SHP loading | — | SIDEBAR_RADAR |
| `0x0072D460` | Radar background SHP loading | — | SIDEBAR_RADAR |
| `0x0072D830` | Radar transition movie SHP loading | — | SIDEBAR_RADAR |
| `0x00533FD0` | Sidebar surface creation | — | SIDEBAR_SYSTEM |

### Credits Counter

| Address | Name | Size | Source |
|---------|------|------|--------|
| `0x004A2350` | CreditsClass::Init | — | CREDITS_COUNTER |
| `0x004A2370` | CreditsClass::Draw | — | CREDITS_COUNTER |
| `0x004A2600` | CreditsClass::AI (counting animation) | — | CREDITS_COUNTER |
| `0x006D0E60` | DrawCreditsSHPBackground | — | CREDITS_COUNTER |
| `0x006D0A30` | Sidebar draw caller (credits) | — | CREDITS_COUNTER |
| `0x00750920` | CreditUp/CreditDown sound | — | CREDITS_COUNTER |
| `0x004A59E0` | ComputeTextRect | — | SIDEBAR_READY_TEXT |
| `0x004A60E0` | DrawText | — | SIDEBAR_READY_TEXT |

### Text & Fonts

| Address | Name | Size | Source |
|---------|------|------|--------|
| `0x00433880` | Font loader (→ GAME.FNT) | — | SIDEBAR_READY_TEXT |
| `0x00433990` | Font loader (inner) | — | SIDEBAR_READY_TEXT |
| `0x00433ED0` | Measure text width (proportional) | — | SIDEBAR_READY_TEXT |
| `0x00433CF0` | Measure text width (alternate) | — | SIDEBAR_READY_TEXT |
| `0x00734E60` | StringTable::LoadString (CSF lookup) | — | SIDEBAR_READY_TEXT |
| `0x00621B80` | AlphaBlendRect (pixel-level tint) | — | SIDEBAR_READY_TEXT |

### Map / Terrain

| Address | Name | Size | Source |
|---------|------|------|--------|
| `0x00547CF0` | TMP tile blitter | — | COORDINATE_ATOMS |
| `0x00547020` | TMP loader | — | COORDINATE_ATOMS |
| `0x005471B0` | Read slope type from tile data | — | VOXEL_SLOPE_TILT |
| `0x00578080` | CellClass::GetGroundHeight (wrapper) | — | BRIDGE_SYSTEM |
| `0x0047B3A0` | CellClass::GetGroundHeight (inner, height interp) | large | BRIDGE_SYSTEM |

### Bridge System

| Address | Name | Size | Source |
|---------|------|------|--------|
| `0x00486750` | CellClass::IsBridge | ~30 B | BRIDGE_SYSTEM |
| `0x00486770` | CellClass::IsWoodBridge | ~30 B | BRIDGE_SYSTEM |
| `0x0047D2B0` | CellClass::RecalcAttributes | large | BRIDGE_SYSTEM |
| `0x0047DD70` | CellClass::BlowUpBridge | — | BRIDGE_SYSTEM |
| `0x0047E8A0` | CellClass::AddContent (ground/bridge list select) | — | BRIDGE_SYSTEM |
| `0x0047EA90` | CellClass::RemoveContent (ground/bridge list select) | — | BRIDGE_SYSTEM |
| `0x004D9C60` | CheckBridgeTraversal (pathfinding height check) | — | BRIDGE_SYSTEM |
| `0x00545150` | Theater/Map Init (loads BridgeSet/WoodBridgeSet) | 976 B | BRIDGE_SYSTEM |
| `0x00570050` | ProcessBridgeDestruction_Low | — | BRIDGE_SYSTEM |
| `0x00571490` | ProcessBridgeDamageStateMachine_Low (18 states) | — | BRIDGE_SYSTEM |
| `0x00573540` | ProcessBridgeDestruction_High | — | BRIDGE_SYSTEM |
| `0x00575EE0` | RepairBridgeSegment (walk + repair 3-wide) | — | BRIDGE_SYSTEM |
| `0x00576BA0` | ProcessBridgeDamageStateMachine_High (18 states) | — | BRIDGE_SYSTEM |
| `0x00578D80` | IsOnBridgeRamp (6 ramp region check) | 219 B | BRIDGE_SYSTEM |
| `0x00487D50` | CellClass::GetEffectiveHeight (height + 4 if bit 0x80) | ~22 B | BRIDGE_SYSTEM |
| `0x00486380` | CellClass::IsClearTile (NOT IsBridgeCell — report 024 was wrong) | ~20 B | BRIDGE_SYSTEM |
| `0x004865B0` | CellClass::IsShorePieceTile (NOT IsOverlayBridge — report 025 wrong) | ~25 B | BRIDGE_SYSTEM |
| `0x0042ACF0` | PathfinderClass::UpdateBridgePassability (5x5 toggle 0x40000) | ~900 B | BRIDGE_SYSTEM |
| `0x0056DA10` | MapClass::FindBridgeRecord (bridge connection list lookup) | ~200 B | BRIDGE_SYSTEM |
| `0x00578E60` | MarkBridgesForRepair_Low | 428 B | BRIDGE_SYSTEM |
| `0x0057A0C0` | MarkBridgesForRepair_High | — | BRIDGE_SYSTEM |
| `0x0057B440` | ApplyBridgeTile (final tile placement) | — | BRIDGE_SYSTEM |
| `0x0057BAA0` | DestroyBridge_Low (tile-level) | 582 B | BRIDGE_SYSTEM |
| `0x0057CCF0` | DestroyBridge_High (tile-level) | 618 B | BRIDGE_SYSTEM |
| `0x0057F200` | RepairBridge_Low | — | BRIDGE_SYSTEM |
| `0x0057F440` | RepairBridge_High | — | BRIDGE_SYSTEM |
| `0x00587180` | ApplyDamageToCell (bridge damage dispatch) | — | BRIDGE_SYSTEM |
| `0x0056E990` | ToggleBridgePavement (bit 0x2000, propagate) | 483 B | BRIDGE_SYSTEM |
| `0x0056EB80` | SetOverlayAndPropagate | — | BRIDGE_SYSTEM |
| `0x0056ED40` | UpdateRamp_NS_DamageA_Low | — | BRIDGE_SYSTEM |
| `0x0056EE40` | UpdateRamp_NS_DamageB_Low | — | BRIDGE_SYSTEM |
| `0x0056EF50` | UpdateRamp_NS_CollapseA_Low (recursive) | — | BRIDGE_SYSTEM |
| `0x0056F2F0` | UpdateRamp_NS_CollapseB_Low (recursive) | — | BRIDGE_SYSTEM |
| `0x0056F690` | UpdateRamp_EW_DamageA_Low | — | BRIDGE_SYSTEM |
| `0x0056F7A0` | UpdateRamp_EW_DamageB_Low | — | BRIDGE_SYSTEM |
| `0x0056F8B0` | UpdateRamp_EW_CollapseA_Low (recursive) | — | BRIDGE_SYSTEM |
| `0x0056FC80` | UpdateRamp_EW_CollapseB_Low (recursive) | — | BRIDGE_SYSTEM |
| `0x00572230` | UpdateRamp_NS_DamageA_High | — | BRIDGE_SYSTEM |
| `0x00572330` | UpdateRamp_NS_DamageB_High | — | BRIDGE_SYSTEM |
| `0x00572440` | UpdateRamp_NS_CollapseA_High (recursive) | — | BRIDGE_SYSTEM |
| `0x005727E0` | UpdateRamp_NS_CollapseB_High (recursive) | — | BRIDGE_SYSTEM |
| `0x00572B80` | UpdateRamp_EW_DamageA_High | — | BRIDGE_SYSTEM |
| `0x00572C90` | UpdateRamp_EW_DamageB_High | — | BRIDGE_SYSTEM |
| `0x00572DA0` | UpdateRamp_EW_CollapseA_High (recursive) | — | BRIDGE_SYSTEM |
| `0x00573170` | UpdateRamp_EW_CollapseB_High (recursive) | — | BRIDGE_SYSTEM |
| `0x00576770` | UpdateAdjacentBridges_High | — | BRIDGE_SYSTEM |

### Display Class Chain

| Address | Name | Size | Source |
|---------|------|------|--------|
| `0x004F4220` | GScreenClass::Constructor | — | SIDEBAR_SYSTEM |
| `0x00565090` | MapClass::Constructor | — | SIDEBAR_SYSTEM |
| `0x004A8730` | DisplayClass::Constructor | — | SIDEBAR_SYSTEM |
| `0x00652960` | RadarClass::Constructor | — | SIDEBAR_SYSTEM |
| `0x0063F6B0` | PowerClass::Constructor | — | SIDEBAR_SYSTEM |
| `0x006CFE20` | TabClass::Constructor | — | SIDEBAR_SYSTEM |
| `0x00692290` | ScrollClass::Constructor | — | SIDEBAR_SYSTEM |
| `0x005BDA40` | MouseClass::Constructor | — | SIDEBAR_SYSTEM |
| `0x0040D190` | Global display singleton static constructor | — | SIDEBAR_SYSTEM |

### Rendering — Radar Minimap

| Address | Name | Size | Source |
|---------|------|------|--------|
| `0x00652CF0` | RadarClass::One_Time (hardcoded layout constants) | 29 inst | RADAR_MINIMAP |
| `0x00652E90` | RadarClass::Init_For_House (minimap_center_x) | 93 inst | RADAR_MINIMAP |
| `0x00653100` | RadarClass::Draw (per-frame entry, state machine) | 297 lines | RADAR_MINIMAP |
| `0x00653FA0` | RadarClass::DrawJammedMode (player list when jammed) | 131 lines | RADAR_MINIMAP |
| `0x00654490` | RadarClass::ComputeRadarMapBounds (iso projection) | — | RADAR_MINIMAP |
| `0x00654650` | RadarClass::RebuildRadarSurfaces (surface creation) | 65 lines | RADAR_MINIMAP |
| `0x006547C0` | RadarClass::GenerateTerrainSurface (zoom + box filter) | 218 lines | RADAR_MINIMAP |
| `0x00654EA0` | RadarClass::FillTerrainColors (raw RGB buffer fill) | — | RADAR_MINIMAP |
| `0x00655250` | RadarClass::ClearBackground (terrain dirty list proc) | 147 lines | RADAR_MINIMAP |
| `0x00655560` | RadarClass::AddObjectToTracker (hash insert) | 98 lines | RADAR_MINIMAP |
| `0x00655740` | RadarClass::RemoveObjectFromTracker (hash remove) | 42 lines | RADAR_MINIMAP |
| `0x00655B20` | RadarClass::Init (allocate tracker, bounds, surfaces) | — | RADAR_MINIMAP |
| `0x00655C50` | RadarClass::RenderCellPixel (per-pixel fog+object) | 380 inst | RADAR_MINIMAP |
| `0x006550C0` | RadarClass::CellToRadarPixel (cell → surface pixel) | 21 lines | RADAR_MINIMAP |
| `0x006551C0` | RadarClass::MarkTerrainDirty (37 callers) | 32 lines | RADAR_MINIMAP |
| `0x006562D0` | RadarClass::MarkCellDirty (add to dirty list) | — | RADAR_MINIMAP |
| `0x006563B0` | RadarClass::GenerateBrushShapes (22 diamond shapes) | 65 lines | RADAR_MINIMAP |
| `0x006565A0` | RadarClass::MarkObjectDirty (foundation pixels) | 70 lines | RADAR_MINIMAP |
| `0x00656150` | RadarClass::RenderAllCells (no-shroud fast path) | 77 lines | RADAR_MINIMAP |
| `0x00656580` | RadarClass::GetBucketIndex (hash function) | — | RADAR_MINIMAP |
| `0x00656750` | RadarClass::GetObjectAtRadarPixel (click → object) | — | RADAR_MINIMAP |
| `0x006568A0` | RadarClass::Load (deserialize from save) | — | RADAR_MINIMAP |
| `0x00656AC0` | RadarClass::Save (serialize to save) | — | RADAR_MINIMAP |
| `0x00656BE0` | RadarClass::ActivateDeactivate (state transitions) | — | RADAR_MINIMAP |
| `0x00656CB0` | RadarClass::SetRadarMode (high-level mode setter) | — | RADAR_MINIMAP |
| `0x00656DE0` | RadarClass::IsTacticalMapAvailable | — | RADAR_MINIMAP |
| `0x00656E50` | RadarClass::IsRadarJammed (mode==2 && active) | — | RADAR_MINIMAP |
| `0x00656EC0` | RadarClass::Update (main per-frame, 388 lines) | 2603 B | RADAR_MINIMAP |
| `0x006578F0` | RadarClass::PlayRadarMovie (3-phase loader) | 47 lines | RADAR_MINIMAP |
| `0x006579E0` | RadarClass::PerFrameMovieUpdate | — | RADAR_MINIMAP |
| `0x00657CE0` | RadarClass::RefreshRadar (full redraw entry) | — | RADAR_MINIMAP |
| `0x0065FA70` | CreateRadarEvent (distance dedup + alloc) | 36 lines | RADAR_MINIMAP |
| `0x0065FB80` | InitRadarEvent (populate 64-byte struct) | 73 lines | RADAR_MINIMAP |
| `0x0065FE00` | TickRadarEvent (per-event per-frame tick) | 86 lines | RADAR_MINIMAP |
| `0x00660000` | TickAndDrawRadarEvents (event loop) | 35 lines | RADAR_MINIMAP |
| `0x00660050` | DrawRadarEvent (draw rotating diamond) | 164 lines | RADAR_MINIMAP |
| `0x00660540` | DrawViewportRect (camera view rectangle) | 64 lines | RADAR_MINIMAP |
| `0x00660730` | ComputeViewportCorners (rotation matrix → 4 pts) | 33 lines | RADAR_MINIMAP |
| `0x00660530` | ObjectsMovedCheck (returns DAT_00B04DB8 != 0) | 7 lines | RADAR_MINIMAP |
| `0x006603B0` | CleanupExpiredEvents (remove dead from array) | 48 lines | RADAR_MINIMAP |
| `0x0047C060` | CellClass::GetRadarColor (priority: bldg→bridge→overlay→terrain) | 129 lines | RADAR_MINIMAP |
| `0x005FED00` | OverlayClass::GetRadarColor (bridge byte-swap) | 33 lines | RADAR_MINIMAP |
| `0x0069E860` | GetTiberiumRadarColor (from tileset density) | 20 lines | RADAR_MINIMAP |
| `0x005FDD20` | IsWallOverlay (checks overlay type ranges) | 44 lines | RADAR_MINIMAP |
| `0x00661190` | ApplyTheaterBrightness (multiply RGB × factor) | 23 lines | RADAR_MINIMAP |
| `0x00586360` | IsShrouded (cell+0x12C bit 3 check) | 308 B | RADAR_MINIMAP |
| `0x005864A0` | IsFogged (cell+0x13C counter check) | 308 B | RADAR_MINIMAP |
| `0x005865F0` | CellChangeNotify (triggers MarkObjectDirty) | 48 lines | RADAR_MINIMAP |
| `0x004AA050` | RevealCell (shroud removal → RefreshRadar) | 85 lines | RADAR_MINIMAP |
| `0x00431800` | HasSpySatelliteUpdate (24-slot vision check) | 44 lines | RADAR_MINIMAP |
| `0x00431700` | DrawSpySatelliteVision (iterate sat grid) | 36 lines | RADAR_MINIMAP |
| `0x00430650` | DrawOneSpySatellite (SHP + dirty marking) | 102 lines | RADAR_MINIMAP |
| `0x006C8C40` | GetRadarTimer (timeGetTime() >> 4 = 16ms units) | — | RADAR_MINIMAP |
| `0x00456580` | BuildingClass::RegisterOnRadar (multi-cell) | 20 lines | RADAR_MINIMAP |
| `0x0070CC90` | TechnoClass::RegisterOnRadar (single-cell) | 9 lines | RADAR_MINIMAP |
| `0x0070CCC0` | TechnoClass::UnregisterFromRadar | 9 lines | RADAR_MINIMAP |
| `0x005AF1A0` | RotateMatrix2D (2D rotation by angle) | 23 lines | RADAR_MINIMAP |
| `0x005AE860` | ResetMatrix (identity matrix) | — | RADAR_MINIMAP |
| `0x0063B0A0` | DrawRadarOverlays_Normal (game view blips) | 40 lines | RADAR_MINIMAP |
| `0x0063B150` | DrawRadarOverlays_Fog (game view dashed blips) | 76 lines | RADAR_MINIMAP |
| `0x0063C690` | DrawRadarOverlay_Normal (single player blip) | — | RADAR_MINIMAP |
| `0x0063CAE0` | DrawRadarOverlay_Fog (single player dashed blip) | — | RADAR_MINIMAP |
| `0x0063D0F0` | GetBackgroundColor (game-view selected highlight) | 47 lines | RADAR_MINIMAP |
| `0x00640710` | DrawStartPositions (map preview markers) | — | RADAR_MINIMAP |
| `0x00641140` | GenerateTerrainPreview (map preview colors) | — | RADAR_MINIMAP |
| `0x00642130` | CreatePalettedPreview (color quantization) | — | RADAR_MINIMAP |

### Misc / Network

| Address | Name | Size | Source |
|---------|------|------|--------|
| `0x005D4D50` | Process network messages (Call_Back) | — | GAMEMD_ARCHITECTURE |
| `0x0048D080` | Network service loop | — | GAMEMD_ARCHITECTURE |
| `0x0053B560` | Process queued events | — | GAMEMD_ARCHITECTURE |
| `0x0048DC90` | Desync handler | — | GAMEMD_ARCHITECTURE |
| `0x00554A60` | Create production anim | — | BUILDING_SYSTEMS |
| `0x00509140` | Update radar | — | BUILDING_SYSTEMS |
| `0x0056BEC0` | Wall overlay with height adjust | — | BUILDING_SYSTEMS |
| `0x00674000` | SpeedType table populator (from INI) | — | TERRAIN_COST |

---

## Global Variables

### Core State

| Address | Type | Name | Source |
|---------|------|------|--------|
| `0x00A8ED84` | int | g_CurrentFrameCounter (THE game tick) | GAMEMD_ARCHITECTURE |
| `0x008871E0` | RulesClass* | Rules singleton | GAMEMD_ARCHITECTURE |
| `0x00A8B238` | int | GameMode (pump-relevant active YR: 0=campaign/SP, 3=LAN/IPX, 4=WOL/Internet, 5=offline skirmish; modes 1/2 are legacy modem/serial — NOT offline skirmish) | GAMEMD_ARCHITECTURE; MODAL_PUMP_00623120_SERVICE_TICK_CONTRACT |
| `0x00A83D4C` | HouseClass* | PlayerPtr (local player) | HOUSECLASS |
| `0x00A8022C` | HouseClass*[] | HouseClass::Array (all houses) | HOUSECLASS |
| `0x00A80238` | int | HouseClass::Array.Count | HOUSECLASS |
| `0x00887640` | MouseClass* | Display chain (global UI object) | GAMEMD_ARCHITECTURE |
| `0x00887324` | TacticalClass* | Tactical (iso rendering, viewport) | GAMEMD_ARCHITECTURE |
| `0x00A8EDA0` | int | GameState (state machine 0-9) | GAMEMD_ARCHITECTURE |
| `0x00A8ED80` | char | GameRunning (pause flag) | GAMEMD_ARCHITECTURE |
| `0x00A8E9A0` | char | GameActive (master enable) | GAMEMD_ARCHITECTURE |
| `0x00A8E7AC` | int | MapEditorMode | GAMEMD_ARCHITECTURE |
| `0x0087F7E8` | MouseClass | Global display singleton instance | SIDEBAR_SYSTEM |
| `0x00A83E18` | CreditsClass | CreditsClass static instance | CREDITS_COUNTER |
| `0x00AC4E74` | SHPShape* | g_PowerBarSHP (POWERP.SHP) | POWER_SYSTEM |
| `0x00AC4D30` | char[256] | g_PowerTooltipBuf | POWER_SYSTEM |
| `0x007E1718` | double | Constant 1.0 (full power threshold) | POWER_SYSTEM |
| `0x007E2800` | double | Constant 0.0 (no power value) | POWER_SYSTEM |
| `0x007ED8C8` | double | Constant 400.0 (power bar scale factor) | POWER_SYSTEM |
| `0x007F4E80` | double | Constant 0.9 (GetBuildTime multiplier) | POWER_SYSTEM |
| `0x007F4E34` | float | Constant 0.01 (production speed floor) | POWER_SYSTEM |

### Object Arrays

| Address | Type | Name | Source |
|---------|------|------|--------|
| `0x00A83CE4` | DynVec<UnitTypeClass*> | UnitTypeClass::Array | HOUSECLASS |
| `0x00A83C6C` | DynVec<BuildingTypeClass*> | BuildingTypeClass::Array | HOUSECLASS |
| `0x00A83C78` | int | BuildingTypeClass::Array.Count | BUILDING_SYSTEMS |
| `0x00A8B21C` | DynVec<InfantryTypeClass*> | InfantryTypeClass::Array | HOUSECLASS |
| `0x00A8E34C` | DynVec<AircraftTypeClass*> | AircraftTypeClass::Array | HOUSECLASS |
| `0x00A83E34` | DynVec<FactoryClass*> | FactoryClass::Array | HOUSECLASS |
| `0x00A83E40` | int | FactoryClass::Array.Count | HOUSECLASS |
| `0x00A8EB44` | DynVec<BuildingClass*> | BuildingClass global array | BUILDING_SYSTEMS |
| `0x00A8EB50` | int | BuildingClass global count | BUILDING_SYSTEMS |

### Rendering

| Address | Type | Name | Source |
|---------|------|------|--------|
| `0x008A0360` | DynVec[5] (24B each) | Display layers (Underground/Surface/Ground/Air/Top) | ZBUFFER_DEPTH |
| `0x0087F924` | CellClass* | Cell array base (512×512 max) | GAMEMD_ARCHITECTURE |
| `0x00887314` | DSurface* | Primary surface (render target) | GAMEMD_ARCHITECTURE |
| `0x00887300` | DSurface* | Sidebar surface | SIDEBAR_SYSTEM |
| `0x00887644` | ZBuf* | Z-buffer surface (16-bit per-pixel) | ZBUFFER_DEPTH |
| `0x0087E8A4` | ABuf* | A-buffer (alpha/auxiliary surface) | ZBUFFER_DEPTH |
| `0x00884B90` | int | Sidebar needs redraw flag | CREDITS_COUNTER |
| `0x008A03FC` | SHP* | PLACE.SHP global pointer | COORDINATE_SYSTEM |
| `0x0089DDBC` | SHP* | BUILDNGZ.SHA (building Z-buffer shapes) | BUILDING_SYSTEMS |
| `0x0089DDC4` | SHP* | POWEROFF.SHP pointer | BUILDING_SYSTEMS |
| `0x0089DDC8` | SHP* | WRENCH.SHP pointer | BUILDING_SYSTEMS |
| `0x008B4154` | AnimTypeClass*[] | AnimTypes global array | BUILDING_ANIM_STATE |
| `0x0089C4D0` | Font* | GAME.FNT font object | SIDEBAR_READY_TEXT |

### Radar Minimap

| Address | Type | Name | Source |
|---------|------|------|--------|
| `0x00880A04` | DSurface* | Radar draw surface (events + viewport rect) | RADAR_MINIMAP |
| `0x00880C84` | int | Radar surface origin X on draw target | RADAR_MINIMAP |
| `0x00880C88` | int | Radar surface origin Y | RADAR_MINIMAP |
| `0x00880C8C` | int | Radar surface width | RADAR_MINIMAP |
| `0x00880C90` | int | Radar surface height | RADAR_MINIMAP |
| `0x00B04DAC` | int* | Radar event array (ptrs to 64-byte events) | RADAR_MINIMAP |
| `0x00B04DA8` | ptr | Radar event DynVec backing store | RADAR_MINIMAP |
| `0x00B04DB0` | int | Radar event array capacity | RADAR_MINIMAP |
| `0x00B04DB8` | int | Radar event count | RADAR_MINIMAP |
| `0x00B04DD8` | int | Radar event ring write index (mod 8) | RADAR_MINIMAP |
| `0x00B04D48` | int[8] | Radar event cell ring buffer (Spacebar cycling) | RADAR_MINIMAP |
| `0x00B04D88` | int | Radar event ring counter | RADAR_MINIMAP |
| `0x00B73550` | int | Shroud enabled flag | RADAR_MINIMAP |
| `0x00A83CD9` | byte[3] | Default radar background color (RGB) | RADAR_MINIMAP |
| `0x00A80220` | byte[3] | Selected unit highlight color (RGB) | RADAR_MINIMAP |
| `0x00AC4BF0` | DynVec* | Selected objects vector | RADAR_MINIMAP |
| `0x00836880` | byte[16] | Viewport dash pattern {1,1,1,1,0,0,0,0,...} | RADAR_MINIMAP |
| `0x0089C420` | int | Spy satellite SHP width | RADAR_MINIMAP |
| `0x0089C424` | int | Spy satellite SHP height | RADAR_MINIMAP |
| `0x0089C428` | int | Spy satellite refresh frame count | RADAR_MINIMAP |
| `0x0089C42C` | int | Spy satellite refresh interval | RADAR_MINIMAP |
| `0x0089C478` | SHP* | Spy satellite SHP pointer | RADAR_MINIMAP |
| `0x00886FA0` | int | Radar viewport offset X (from world center) | RADAR_MINIMAP |
| `0x00886FA4` | int | Radar viewport offset Y | RADAR_MINIMAP |
| `0x00886FA8` | int | Radar viewport width | RADAR_MINIMAP |
| `0x00886FAC` | int | Radar viewport height | RADAR_MINIMAP |
| `0x008A0DD0` | int | DirectDraw r_shift (11 for RGB565) | RADAR_MINIMAP |
| `0x008A0DD4` | int | DirectDraw r_loss (3) | RADAR_MINIMAP |
| `0x008A0DD8` | int | DirectDraw b_shift (0) | RADAR_MINIMAP |
| `0x008A0DDC` | int | DirectDraw b_loss (3) | RADAR_MINIMAP |
| `0x008A0DE0` | int | DirectDraw g_shift (5) | RADAR_MINIMAP |
| `0x008A0DE4` | int | DirectDraw g_loss (2 for RGB565, 3 for RGB555) | RADAR_MINIMAP |
| `0x007F0998` | byte[208] | Radar event type config (13 types × 16B each) | RADAR_MINIMAP |
| `0x007F041C` | float | 108.0 — radar inner height constant | RADAR_MINIMAP |
| `0x007F0420` | float | 140.0 — radar inner width constant | RADAR_MINIMAP |
| `0x007E1BD0` | TheaterEntry[6] | Theater table (0x70 bytes each). Brightness at +0x00 | RADAR_MINIMAP |
| `0x007E2220` | float | 255.0 — blue channel clamp in theater brightness | RADAR_MINIMAP |
| `0x007E5168` | float | 0.5 — half constant (inverse iso transform) | RADAR_MINIMAP |
| `0x007E1738` | double | 0.5 — rounding constant (inverse iso transform) | RADAR_MINIMAP |
| `0x007ED968` | float | 0.3333 — rotation deceleration factor | RADAR_MINIMAP |
| `0x007F0AE8` | float | 0.02 — rotation deceleration rate per tick | RADAR_MINIMAP |
| `0x008192B8` | int[22] | Foundation type table for brush shapes | RADAR_MINIMAP |

### Sidebar SHP Globals

| Address | SHP Name | Source |
|---------|----------|--------|
| `0x00B0FAF8` | SDTP.SHP (sidebar top cap) | SIDEBAR_CONSTRUCTION |
| `0x00B0FA74` | SDBTNBKGD.SHP (cameo background tile) | SIDEBAR_CONSTRUCTION |
| `0x00B0FAC4` | SDBTNANM.SHP (button animation overlay) | SIDEBAR_CONSTRUCTION |
| `0x00B0FB34` | Radar frame open SHP | SIDEBAR_RADAR |
| `0x00B0FB00` | Radar background SHP | SIDEBAR_RADAR |
| `0x00B0FB30` | Radar frame close SHP | SIDEBAR_RADAR |
| `0x00B0FB1C` | Minimap movie SHP | SIDEBAR_RADAR |
| `0x00B0FA68` | BKGDLG.SHP / BKGDLGY.SHP | SIDEBAR_RADAR |
| `0x00B0FAC8` | BKGDMD.SHP / BKGDMDY.SHP | SIDEBAR_RADAR |
| `0x00B0FAD4` | BKGDSM.SHP / BKGDSMY.SHP | SIDEBAR_RADAR |
| `0x00B0FA50` | SIDEBTTN.SHP | SIDEBAR_RADAR |
| `0x00B0FB08` | RADAR.SHP | SIDEBAR_RADAR |
| `0x00B0FA70` | CREDITS.SHP | SIDEBAR_RADAR |

### Constants & Data Tables

| Address | Type | Name | Source |
|---------|------|------|--------|
| `0x0089EA40` | float[12][9] | Speed/terrain table (from rules.ini, runtime) | TERRAIN_COST |
| `0x0082A594` | int[12][8] | Passability matrix (hardcoded) | GAMEMD_ARCHITECTURE |
| `0x007E7A28` | byte[16×16] | Drive track base curves — RawTrack[16] | DRIVE_LOCOMOTION_CLASS |
| `0x007E7B28` | byte[72×12] | Drive track descriptors — TurnTrack[72] | DRIVE_LOCOMOTION_CLASS |
| `0x007F2960` | byte[16×16] | Ship track base curves — RawTrack[16] | DRIVE_LOCOMOTION_CLASS |
| `0x007F2A40` | byte[72×12] | Ship track descriptors — TurnTrack[72] | DRIVE_LOCOMOTION_CLASS |
| `0x0089F688` | short[8][2] | 8-direction cell offsets (dx/dy, runtime) | DRIVE_LOCOMOTION_CLASS |
| `0x0089F6D8` | int[8][2] | 8-direction lepton offsets (dx/dy, runtime) | DRIVE_LOCOMOTION_CLASS |
| `0x008A0790` | int[3] | Drive NullCoord sentinel {0,0,0} | DRIVE_LOCOMOTION_CLASS |
| `0x00B077F8` | int[3] | Ship NullCoord sentinel {0,0,0} | DRIVE_LOCOMOTION_CLASS |
| `0x008A07D0` | int | Drive HeightStep (runtime) | DRIVE_LOCOMOTION_CLASS |
| `0x00B07838` | int | Ship HeightStep (runtime) | DRIVE_LOCOMOTION_CLASS |
| `0x00B0782C` | int | g_BridgeZ_Offset — ship Z-adjust for navigating under bridges. Used by ShipLocomotionClass only, NOT by rendering. Value=0 static, set at runtime by InitBridgeZOffset (0x0069ebd0) | SHIP_LOCOMOTION_CLASS |
| `0x008192B8` | int[] | Foundation width lookup table | BUILDING_SYSTEMS |
| `0x00817710` | int[3][6] | Z-gradient table (3 entries × 6 fields, 24B each) | ZBUFFER_DEPTH |
| `0x0081DA78` | char*[5] | Layer name string table (Underground/Surface/Ground/Air/Top) | ZBUFFER_DEPTH |
| `0x0081DC24` | uint | Blitter flag mask = 0x3000 (tests bits 12-13) | ZBUFFER_DEPTH |
| `0x00ABB120` | byte[] | Isometric diamond pixel offset table (runtime-init) | ZBUFFER_DEPTH |
| `0x00AA074C` | byte[] | Isometric diamond screen offset table (runtime-init) | ZBUFFER_DEPTH |
| `0x00AA154C` | byte[] | Isometric diamond scanline widths (stride 0x6CC, runtime-init) | ZBUFFER_DEPTH |
| `0x00B0CE7C` | Rect[] | Dirty rect list (for Z-buffer partial clear) | ZBUFFER_DEPTH |
| `0x00819310` | int[] | Foundation height lookup table | BUILDING_SYSTEMS |
| `0x0082A734` | int[16] | Bridge start heights (by tile offset) | BRIDGE_SYSTEM |
| `0x0082A774` | int[16] | Bridge walk directions (2=SE, 4=SW) | BRIDGE_SYSTEM |
| `0x0082A7B4` | int[16] | Bridge end heights (by tile offset) | BRIDGE_SYSTEM |
| `0x0082A7F4` | int[42] | Bridge height class (tile compatibility) | BRIDGE_SYSTEM |
| `0x0082A89C` | int[42] | Bridge direction class (ramp facing) | BRIDGE_SYSTEM |
| `0x0082A944` | int[16] | Bridge direction table (tile→direction) | BRIDGE_SYSTEM |
| `0x0081CC20` | int[4] | Tunnel direction table [2,4,6,0] | BRIDGE_SYSTEM |
| `0x0081CC30` | int[16] | Overlay variance Latin square | BRIDGE_SYSTEM |
| `0x007E37B0` | float | Pathfinding base cost = 1.0 | BRIDGE_SYSTEM |
| `0x007E37B4` | float | Bridge adjacency cost = 2.0 | BRIDGE_SYSTEM |
| `0x007E37B8` | float | Diagonal bridge cost = 10.0 | BRIDGE_SYSTEM |
| `0x007E37BC` | float | AStar temporary marker cost multiplier = 4.0 when destination CellClass+0x140 & 0x40000 | BRIDGE_SYSTEM |
| `0x0081870C` | float[4] | SpeedType base cost (Foot=1, Track=1000, Wheel=1, Float=1) | BRIDGE_SYSTEM |
| `0x00818CA0` | int[4] | Wall connection bitmask table | BUILDING_SYSTEMS |
| `0x007F4890` | int[32] | Shadow direction lookup table | BUILDING_ANIM_STATE |
| `0x00B0CD48` | double | AdjustForZ multiplier (~0.14348) | COORDINATE_SYSTEM |
| `0x0089DDB8` | int | HeightFactor storage (=104) | COORDINATE_SYSTEM |
| `0x007E1718` | double | 1.0 — speed cap ceiling, identity | DRIVE_LOCOMOTION_CLASS |
| `0x007E2800` | double | 0.0 — zero-speed check sentinel | DRIVE_LOCOMOTION_CLASS |
| `0x007E3548` | double | 0.2 — braking/traffic-jam target speed | DRIVE_LOCOMOTION_CLASS |
| `0x007E44E8` | double | 0.005 — slope tilt "flat" threshold | DRIVE_LOCOMOTION_CLASS |
| `0x007E48F0` | double | 1.5 — deceleration constant | LOCOMOTION_MATH |
| `0x007E6240` | double | 0.3 — Stop_Moving speed clamp + min decel | DRIVE_LOCOMOTION_CLASS |
| `0x007E7FA8` | double | 1/7 — residual tick interpolation factor | DRIVE_LOCOMOTION_CLASS |
| `0x007E7FC0` | double | 0.75 — damaged-unit speed multiplier | DRIVE_LOCOMOTION_CLASS |
| `0x007F1308` | double | 0.3 — min decel speed (normal, duplicate) | DRIVE_LOCOMOTION_CLASS |
| `0x007F1310` | double | 0.1 — min decel speed (alt/is_decelerating) | DRIVE_LOCOMOTION_CLASS |
| `0x007F1318` | double | 0.0015 — alt deceleration rate per tick | DRIVE_LOCOMOTION_CLASS |
| `0x008A07C4` | int | g_BridgeZOffset_Drive (runtime, bridge Z offset) | BRIDGE_SYSTEM |
| `0x0089C2D8` | int | g_PathfindHeightStep (A* pathfinding height step) | BRIDGE_SYSTEM |
| `0x008B3CAC` | int | g_FlyBridgeHeight (fly loco bridge Z = height*0.5) | BRIDGE_SYSTEM |
| `0x00AA0E28` | int | g_BridgeSet (high bridge base tile index, -1=none) | BRIDGE_SYSTEM |
| `0x00ABAD1C` | int | g_WoodBridgeSet (low bridge base tile index, -1=none) | BRIDGE_SYSTEM |
| `0x00B45188` | matrix[] | Slope tilt matrices (per slope type) | VOXEL_SLOPE_TILT |
| `0x00B43F08` | double | Corner slope tilt constant | VOXEL_SLOPE_TILT |
| `0x00B44310` | double | Edge slope tilt constant | VOXEL_SLOPE_TILT |
| `0x00B2FB79` | byte[768] | VXL global palette (256 RGB) | VOXEL_RENDERING |
| `0x00887470` | float[3] | Light direction vector | VOXEL_RENDERING |
| `0x00A8EF98` | int | InvalidCell sentinel | HOUSECLASS |
| `0x00B054D4` | ptr[] | ColorSchemeArray | HOUSECLASS |
| `0x0083C714` | str | "ChronoDelay" | LOCOMOTION_MATH |
| `0x0083C700` | str | "ChronoReinfDelay" | LOCOMOTION_MATH |
| `0x0083C6E8` | str | "ChronoDistanceFactor" | LOCOMOTION_MATH |
| `0x0083C6D8` | str | "ChronoTrigger" | LOCOMOTION_MATH |
| `0x0083C6C4` | str | "ChronoMinimumDelay" | LOCOMOTION_MATH |
| `0x0083C6B0` | str | "ChronoRangeMinimum" | LOCOMOTION_MATH |
| `0x0083C464` | str | "ChronoHarvTooFarDistance" | LOCOMOTION_MATH |
| `0x00839D68` | ptr[12] | LandType string pointer table | TERRAIN_COST |
| `0x0081DBA0` | str[] | SpeedType string constants | TERRAIN_COST |

### Network

| Address | Type | Name | Source |
|---------|------|------|--------|
| `0x00A802C8` | int | Command queue count (max 128) | GAMEMD_ARCHITECTURE |
| `0x00A802D0` | int | Command queue write index | GAMEMD_ARCHITECTURE |
| `0x00A802D4` | byte[128×0x6F] | Command buffer (128 slots, 111B each) | GAMEMD_ARCHITECTURE |
| `0x00A83A54` | DWORD[128] | Command timestamps | GAMEMD_ARCHITECTURE |
| `0x00A8B550` | int | Network frame budget (ms) | GAMEMD_ARCHITECTURE |

### Sidebar Layout Constants

| Address | Type | Value | Source |
|---------|------|-------|--------|
| `0x00886F94` | int | SidebarWidth = 158 (0x9E) | SIDEBAR_SYSTEM |
| `0x00886F90` | int | SidebarX | SIDEBAR_SYSTEM |
| `0x00886F98` | int | SidebarTopClip = 168 | SIDEBAR_SYSTEM |
| `0x007F5BF8` | int | SIDEBAR_WIDTH = 168 (0xA8) | SIDEBAR_RADAR |
| `0x008A00A4` | int | Screen width | SIDEBAR_RADAR |
| `0x008A00A8` | int | Screen height | SIDEBAR_RADAR |

---

## Vtables

| Address | Class | Source |
|---------|-------|--------|
| `0x007E3EBC` | BuildingClass vtable | COORDINATE_SYSTEM |
| `0x007F3058` | SidebarClass primary vtable (55 methods) | SIDEBAR_SYSTEM |
| `0x007EA6FC` | GScreenClass vtable | SIDEBAR_SYSTEM |
| `0x007ED404` | MapClass vtable | SIDEBAR_SYSTEM |
| `0x007E6114` | DisplayClass vtable | SIDEBAR_SYSTEM |
| `0x007F0344` | RadarClass vtable | SIDEBAR_SYSTEM |
| `0x007EFF54` | PowerClass vtable | POWER_SYSTEM |
| `0x007E88D0` | FactoryClass vtable | POWER_SYSTEM |
| `0x007F3FE8` | SuperClass vtable | POWER_SYSTEM |
| `0x007F4090` | SuperWeaponTypeClass vtable | POWER_SYSTEM |
| `0x007EDFB4` | TabClass vtable | SIDEBAR_SYSTEM |
| `0x007F1094` | ScrollClass vtable | SIDEBAR_SYSTEM |
| `0x007E1964` | MouseClass vtable | SIDEBAR_SYSTEM |
| `0x007E7EB0` | DriveLocomotionClass ILocomotion vtable (40 slots) | DRIVE_LOCOMOTION_CLASS |
| `0x007E7F7C` | DriveLocomotionClass IUnknown vtable | DRIVE_LOCOMOTION_CLASS |
| `0x007E7E8C` | DriveLocomotionClass IPiggyback vtable | DRIVE_LOCOMOTION_CLASS |
| `0x007EACFC` | HoverLocomotionClass ILocomotion vtable | LOCOMOTION_MATH |
| `0x007F5A24` | TunnelLocomotionClass ILocomotion vtable | LOCOMOTION_MATH |
| `0x007F69F8` | WalkLocomotionClass ILocomotion vtable | LOCOMOTION_MATH |
| `0x007E8278` | DropPodLocomotionClass ILocomotion vtable | LOCOMOTION_MATH |
| `0x007E89F4` | FlyLocomotionClass ILocomotion vtable | LOCOMOTION_MATH |
| `0x007F5000` | TeleportLocomotionClass ILocomotion vtable | LOCOMOTION_MATH |
| `0x007EDB6C` | MechLocomotionClass ILocomotion vtable | LOCOMOTION_MATH |
| `0x007F2D8C` | ShipLocomotionClass ILocomotion vtable | NAVAL_SYSTEM |
| `0x007ECD68` | JumpjetLocomotionClass ILocomotion vtable | LOCOMOTION_MATH |
| `0x007F0B1C` | RocketLocomotionClass ILocomotion vtable | LOCOMOTION_MATH |
| `0x007F50CC` | TeleportLocomotionClass IUnknown vtable | LOCOMOTION_MATH |
| `0x007F4FDC` | TeleportLocomotionClass IPiggyback vtable | LOCOMOTION_MATH |
| `0x007F2E58` | ShipLocomotionClass IUnknown vtable | NAVAL_SYSTEM |
| `0x007F2D68` | ShipLocomotionClass IPiggyback vtable | NAVAL_SYSTEM |

---

## RTTI String Addresses

| Address | String | Source |
|---------|--------|--------|
| `0x0083F8D0` | `.?AVSBGadgetClass@SidebarClass@@` | SIDEBAR_STRIPS |
| `0x0083F900` | `.?AVSelectClass@StripClass@SidebarClass@@` | SIDEBAR_STRIPS |
| `0x008269A8` | `SidebarUpCommandClass` | SIDEBAR_SYSTEM |
| `0x00826EF8` | `SidebarDownCommandClass` | SIDEBAR_SYSTEM |

---

## Prerequisite Group Addresses (RulesClass offsets)

| Keyword | ID | Array Offset | Count Offset | Source |
|---------|----|--------------|--------------|----|
| POWER | -1 | Rules+0x35C | Rules+0x368 | TECH_TREE_REPORT |
| FACTORY | -2 | Rules+0x378 | Rules+0x384 | TECH_TREE_REPORT |
| BARRACKS | -3 | Rules+0x394 | Rules+0x3A0 | TECH_TREE_REPORT |
| RADAR | -4 | Rules+0x3B0 | Rules+0x3BC | TECH_TREE_REPORT |
| TECH | -5 | Rules+0x3CC | Rules+0x3D8 | TECH_TREE_REPORT |
| PROC | -6 | Rules+0x3E8 | Rules+0x3F4 | TECH_TREE_REPORT |

---

## Class Layouts — Power System

### SuperClass (0x80 = 128 bytes, vtable 0x7F3FE8)

| Offset | Size | Type | Name |
|--------|------|------|------|
| +0x00 | 16 | | AbstractClass base (4 vtable pointers) |
| +0x10 | 20 | | AbstractClass fields (UniqueID, flags, refcount) |
| +0x24 | 4 | int | RechargeTimeOverride (-1 = use type default) |
| +0x28 | 4 | SuperWeaponTypeClass* | Type |
| +0x2C | 4 | HouseClass* | OwnerHouse |
| +0x30 | 12 | CDTimerClass | Timer (StartFrame, HighDword, Duration) |
| +0x3C | 4 | int | RechargeTime_Cached |
| +0x60 | 1 | bool | IsPresent |
| +0x6D | 1 | bool | IsCharged |
| +0x6E | 1 | bool | IsSuspended (OneTime/Persistent) |
| +0x6F | 1 | bool | IsReady (charge complete) |
| +0x70 | 1 | bool | IsPowerSuspended |
| +0x74 | 4 | int | ReadyFrame |
| +0x78 | 4 | int | LastAnimStage |
| +0x7C | 4 | int | ChargeDrainState (0=ready, 1=draining, 2=charging) |

### FactoryClass (0x74 = 116 bytes, vtable 0x7E88D0)

| Offset | Size | Type | Name |
|--------|------|------|------|
| +0x00 | 16 | | AbstractClass base (4 vtable pointers) |
| +0x10 | 20 | | AbstractClass fields |
| +0x24 | 4 | int | Progress (0 to 54, complete at 54) |
| +0x28 | 1 | bool | HasChanged_AI |
| +0x2C | 12 | CDTimerClass | Timer (StartFrame, HighDword, Step) |
| +0x38 | 4 | int | StepDelay (frames between steps, 1-255) |
| +0x3C | 4 | int | IncrementPerTick (always 1) |
| +0x40 | 24 | DynamicVectorClass | QueuedObjects (production queue) |
| +0x58 | 4 | TechnoClass* | CurrentObject |
| +0x5C | 1 | bool | IsInsufficientFunds |
| +0x5D | 1 | bool | IsDifferent (UI change flag) |
| +0x60 | 4 | int | Balance (remaining cost) |
| +0x64 | 4 | int | OriginalBalance (full cost) |
| +0x68 | 4 | int | SpecialItem (-1 = none) |
| +0x6C | 4 | HouseClass* | OwnerHouse |
| +0x70 | 1 | bool | IsSuspended |
| +0x71 | 1 | bool | CanAfford |

### RulesClass Power Offsets

| Offset | Type | INI Key |
|--------|------|---------|
| +0x570 | float | MinLowPowerProductionSpeed |
| +0x574 | float | MaxLowPowerProductionSpeed |
| +0x578 | float | LowPowerPenaltyModifier |
| +0x57C | float | MultipleFactory |
| +0x758 | double | WallBuildSpeedCoefficient |
| +0x89C | 4 | AI_PowerPlant_Allied (BuildingTypeClass*) |
| +0x8A0 | 4 | AI_PowerPlant_Soviet (BuildingTypeClass*) |
| +0x8A4 | 4 | AI_PowerPlant_SovietAdv (BuildingTypeClass*) |
| +0x8A8 | 4 | AI_PowerPlant_Yuri (BuildingTypeClass*) |
| +0xD64 | int | SpyPowerBlackout (frames) |
| +0xD68 | float | SpyMoneyStealPercent |
| +0x1700 | float | ConditionYellow |
| +0x1708 | float | ConditionRed |
| +0x1748 | double | BuildSpeed |

### SuperWeaponTypeClass Power Fields

| Offset | Type | INI Key |
|--------|------|---------|
| +0xB0 | int | RechargeTime (frames) |
| +0xB4 | int | Type (action enum 0-11) |
| +0xE5 | bool | UseChargeDrain |
| +0xE6 | bool | IsPowered |
| +0xE7 | bool | DisableableFromShell |

### TechnoTypeClass Power/Production Fields

| Offset | Type | INI Key |
|--------|------|---------|
| +0x3D4 | int | PipScale (enum: 0=none, 1=Ammo, 2=Tiberium, 3=Passengers, 4=Power, 5=MindControl) |
| +0x3D8 | bool | PipsDrawForAll |
| +0x3E4 | int | PipWrap |
| +0x410 | bool | PoweredUnit |
| +0x608 | float | BuildTimeMultiplier |
| +0x610 | int | Cost |

---

## Ghidra Struct/Enum Definitions (Z-Buffer System)

### Enums (in Ghidra Data Type Manager)

| Name | Size | Values | Source |
|------|------|--------|--------|
| `LayerType` | 4B | UNDERGROUND=0, SURFACE=1, GROUND=2, AIR=3, TOP=4 | ZBUFFER_DEPTH |
| `ZBufferMode` | 4B | NONE=0, READ=2, WRITE=4, READWRITE=6 | ZBUFFER_DEPTH |

### Structs (in Ghidra Data Type Manager)

| Name | Size | Fields | Confidence | Source |
|------|------|--------|-----------|--------|
| `ZBufferSurface` | 48B | originX(+0), originY(+4), width(+8), height(+C), flags(+10), innerSurface(+14), bufferStart(+18), bufferEnd(+1C), bufferWrapSize(+20), defaultZ(+24), strideWidth(+28), strideHeight(+2C) | HIGH — all from constructor `0x007bc970` | ZBUFFER_DEPTH |
| `ZGradientEntry` | 24B | field0(+0), field1(+4), increment(+8), threshold(+C), stepDir(+10), pathSelector(+14) | HIGH — verified from blitter assembly + memory dump | ZBUFFER_DEPTH |
| `PixelBufferDesc` | 12B | dataPtr(+0), size(+4), owned(+8), pad×3 | HIGH — from Init/Free decompilation | ZBUFFER_DEPTH |
| `DisplayLayerEntry` | 24B | vtable(+0), buffer(+4), count(+8), flags(+C), capacity(+10), extra(+14) | MEDIUM — vtable/buffer certain, other names approximate | ZBUFFER_DEPTH |
| `SHPFrameHeader` | 24B | offsetX(+0), offsetY(+2), width(+4), height(+6), flagByte(+8), pad×3, unk_0C-0E, pad4, unk_10, dataOffset(+14) | PARTIAL — offsets 0-8 verified, 9-23 unverified (marked unk_) | ZBUFFER_DEPTH |

### Structs Applied to Data Addresses

| Address | Type | Count | Source |
|---------|------|-------|--------|
| `0x00817710` | `ZGradientEntry` | 3 entries (entry 0, 1, 2) | ZBUFFER_DEPTH |
| `0x008A0360` | `DisplayLayerEntry` | 5 entries (layers 0-4, stride 24B) | ZBUFFER_DEPTH |

### Function Signatures Set (verified calling conventions + params)

| Address | Signature | Source |
|---------|-----------|--------|
| `0x00456F80` | `int __thiscall BuildingClass_AdjustZHeight(int z_height)` | ZBUFFER_DEPTH |
| `0x007BC970` | `ZBufferSurface* __thiscall ZBuffer_Constructor(int x, int y, int w, int h)` | ZBUFFER_DEPTH |
| `0x0043AD00` | `PixelBufferDesc* __thiscall PixelBuffer_Init(int data_ptr, uint size)` | ZBUFFER_DEPTH |
| `0x0043AE50` | `void __fastcall PixelBuffer_Free(void)` | ZBUFFER_DEPTH |
| `0x0048E050` | `int __cdecl Layer_NameToIndex(char* name)` | ZBUFFER_DEPTH |
| `0x0048E090` | `char* __cdecl Layer_IndexToName(uint index)` | ZBUFFER_DEPTH |
| `0x0069E900` | `uint __fastcall SHP_GetFrameCompressionFlag(uint frame_index)` | ZBUFFER_DEPTH |
| `0x00495BC0` | `void __thiscall Blitter_ZBuf_Intensity25pct_WritesZ(ushort* screen, byte* src, uint count, int base_z, ushort* zbuf, ushort* abuf, uint intensity)` | ZBUFFER_DEPTH |
| `0x004114B0` | `uint __thiscall CircBuf_GetScanlinePtr(int x, int y)` | ZBUFFER_DEPTH |

---

### DriveLocomotionClass — Static Initializers

| Address | Name | Size | Source |
|---------|------|------|--------|
| `0x004AF3E0` | `DriveLocomotionClass__StoreTileHeight` | ~30 B | DRIVE_LOCOMOTION_CLASS |
| `0x004AF400` | `DriveLocomotionClass__InitHeightStep_A` | ~40 B | DRIVE_LOCOMOTION_CLASS |
| `0x004AF440` | `DriveLocomotionClass__ComputeFromHeightStep` | ~40 B | DRIVE_LOCOMOTION_CLASS |
| `0x004AF470` | `DriveLocomotionClass__ComputeBridgeRenderOffset` | ~30 B | DRIVE_LOCOMOTION_CLASS |
| `0x004AF4A0` | `DriveLocomotionClass__ComputeBridgeZOffset` | ~20 B | DRIVE_LOCOMOTION_CLASS |
| `0x004AF4D0` | `DriveLocomotionClass__InitNullCoords2` | ~16 B | DRIVE_LOCOMOTION_CLASS |
| `0x004AF4E0` | `DriveLocomotionClass__InitNullCoords` | ~20 B | DRIVE_LOCOMOTION_CLASS |
| `0x004AF500` | `DriveLocomotionClass__InitHeightStep2` | ~20 B | DRIVE_LOCOMOTION_CLASS |
| `0x004AF520` | `DriveLocomotionClass__InitSomething3` | ~20 B | DRIVE_LOCOMOTION_CLASS |

### DriveLocomotionClass — IPiggyback & COM Thunks

| Address | Name | Size | Source |
|---------|------|------|--------|
| `0x004AF610` | `DriveLocomotionClass__Piggybacker_CLSID` | ~110 B | DRIVE_LOCOMOTION_CLASS |
| `0x004AF720` | `DriveLocomotionClass__QueryInterface_With_IPiggyback` | ~100 B | DRIVE_LOCOMOTION_CLASS |
| `0x004AF8E0` | `DriveLocomotionClass__Begin_Piggyback` | ~80 B | DRIVE_LOCOMOTION_CLASS |
| `0x004AF930` | `DriveLocomotionClass__End_Piggyback` | ~60 B | DRIVE_LOCOMOTION_CLASS |
| `0x004AF970` | `DriveLocomotionClass__Is_Ok_To_End` | ~60 B | DRIVE_LOCOMOTION_CLASS |
| `0x004B4CB0` | `DriveLocomotionClass__IUnknown_AddRef` | ~10 B | DRIVE_LOCOMOTION_CLASS |
| `0x004B4CC0` | `DriveLocomotionClass__IUnknown_Release` | ~10 B | DRIVE_LOCOMOTION_CLASS |
| `0x004B4CD0` | `DriveLocomotionClass__Is_Piggybacking` | ~10 B | DRIVE_LOCOMOTION_CLASS |
| `0x004B4D90` | `DriveLocomotionClass__ILocomotion_QueryInterface` | ~10 B | DRIVE_LOCOMOTION_CLASS |
| `0x004B4DA0` | `DriveLocomotionClass__ILocomotion_AddRef` | ~10 B | DRIVE_LOCOMOTION_CLASS |
| `0x004B4DB0` | `DriveLocomotionClass__ILocomotion_Release` | ~10 B | DRIVE_LOCOMOTION_CLASS |
| `0x004B4DC0` | `DriveLocomotionClass__IPiggyback_QueryInterface` | ~10 B | DRIVE_LOCOMOTION_CLASS |
| `0x004B4DD0` | `DriveLocomotionClass__IPiggyback_AddRef` | ~10 B | DRIVE_LOCOMOTION_CLASS |
| `0x004B4DE0` | `DriveLocomotionClass__IPiggyback_Release` | ~10 B | DRIVE_LOCOMOTION_CLASS |

### DriveLocomotionClass — Stubs & Simple Returns

| Address | Name | Return | Source |
|---------|------|--------|--------|
| `0x004B4820` | `DriveLocomotionClass__In_Which_Layer` | 2 (Ground) | DRIVE_LOCOMOTION_CLASS |
| `0x004B4870` | `DriveLocomotionClass__Z_Adjust` | 0 | DRIVE_LOCOMOTION_CLASS |
| `0x004B4880` | `DriveLocomotionClass__Z_Gradient` | 2 (Deg45) | DRIVE_LOCOMOTION_CLASS |
| `0x004B48D0` | `DriveLocomotionClass__Mark_All_Occupation_Bits` | void | DRIVE_LOCOMOTION_CLASS |
| `0x004B4C60` | `DriveLocomotionClass__Get_Status` | 0 | DRIVE_LOCOMOTION_CLASS |
| `0x004B4C70` | `DriveLocomotionClass__Acquire_Hunter_Seeker_Target` | void | DRIVE_LOCOMOTION_CLASS |
| `0x004B4C80` | `DriveLocomotionClass__Is_Surfacing` | false | DRIVE_LOCOMOTION_CLASS |

### LocomotionClass — Base Class Methods

| Address | Name | Size | Source |
|---------|------|------|--------|
| `0x0055A6C0` | `LocomotionClass__Constructor` | ~40 B | DRIVE_LOCOMOTION_CLASS |
| `0x0055A6F0` | `LocomotionClass__Destructor` | ~30 B | DRIVE_LOCOMOTION_CLASS |
| `0x0055A710` | `LocomotionClass__Link_To_Object` | ~20 B | DRIVE_LOCOMOTION_CLASS |
| `0x0055A8C0` | `LocomotionClass__Draw_Point` | ~40 B | DRIVE_LOCOMOTION_CLASS |
| `0x0055A8F0` | `LocomotionClass__Power_On` | ~20 B | DRIVE_LOCOMOTION_CLASS |
| `0x0055A910` | `LocomotionClass__Power_Off` | ~20 B | DRIVE_LOCOMOTION_CLASS |
| `0x0055A930` | `LocomotionClass__Is_Powered` | ~10 B | DRIVE_LOCOMOTION_CLASS |
| `0x0055A940` | `LocomotionClass__Is_Ion_Sensitive` | ~10 B | DRIVE_LOCOMOTION_CLASS |
| `0x0055A950` | `LocomotionClass__AddRef` | ~20 B | DRIVE_LOCOMOTION_CLASS |
| `0x0055A970` | `LocomotionClass__Release` | ~30 B | DRIVE_LOCOMOTION_CLASS |
| `0x0055A9B0` | `LocomotionClass__QueryInterface` | ~90 B | DRIVE_LOCOMOTION_CLASS |
| `0x0055AB70` | `LocomotionClass__Push` | ~10 B | DRIVE_LOCOMOTION_CLASS |
| `0x0055AB80` | `LocomotionClass__Shove` | ~10 B | DRIVE_LOCOMOTION_CLASS |
| `0x0055AB90` | `LocomotionClass__Tilt_Pitch_AI` | ~10 B | DRIVE_LOCOMOTION_CLASS |
| `0x0055ABB0` | `LocomotionClass__Z_Gradient_Default` | ~10 B | DRIVE_LOCOMOTION_CLASS |
| `0x0055ABC0` | `LocomotionClass__Visual_Character` | ~10 B | DRIVE_LOCOMOTION_CLASS |
| `0x0055ABD0` | `LocomotionClass__Shadow_Point` | ~10 B | DRIVE_LOCOMOTION_CLASS |
| `0x0055ABE0` | `LocomotionClass__Is_To_Have_Shadow` | ~10 B | DRIVE_LOCOMOTION_CLASS |
| `0x0055ABF0` | `LocomotionClass__Can_Enter_Cell` | ~10 B | DRIVE_LOCOMOTION_CLASS |
| `0x0055AC00` | `LocomotionClass__Force_Immediate_Destination` | ~10 B | DRIVE_LOCOMOTION_CLASS |
| `0x0055ACF0` | `LocomotionClass__Drawing_Code` | ~10 B | DRIVE_LOCOMOTION_CLASS |
| `0x0055AD00` | `LocomotionClass__Can_Fire` | ~10 B | DRIVE_LOCOMOTION_CLASS |
| `0x0055AD10` | `LocomotionClass__Apparent_Speed` | ~20 B | DRIVE_LOCOMOTION_CLASS |

### Speed/Land Type System

| Address | Name | Size | Source |
|---------|------|------|--------|
| `0x00674000` | `RulesClass__ReadSpeedTypeLandTypeTable` | ~200 B | DRIVE_LOCOMOTION_CLASS |
| `0x0048DFF0` | `SpeedType__FromName` | ~50 B | DRIVE_LOCOMOTION_CLASS |
| `0x0048E030` | `SpeedType__ToName` | ~20 B | DRIVE_LOCOMOTION_CLASS |
| `0x00483C80` | `CellClass__RecalcZoneType` | ~400 B | CELLCLASS_RECALCZONETYPE_00483C80 |
| `0x0081DA58` | g_SpeedTypeNameTable (8 string ptrs) | 32 B | DRIVE_LOCOMOTION_CLASS |

### Cell Occupation & Entry

| Address | Name | Size | Source |
|---------|------|------|--------|
| `0x007441B0` | `ObjectClass__Mark_Occupation` (bit 0x20, bridge/ground) | ~60 B | DRIVE_LOCOMOTION_CLASS |
| `0x00744210` | `ObjectClass__Clear_Occupation` | ~50 B | DRIVE_LOCOMOTION_CLASS |
| `0x007416A0` | `UnitClass__PerCellProcess` (crush, scatter on cell entry) | ~400 B | DRIVE_LOCOMOTION_CLASS |
| `0x004D3780` | `TechnoClass__DoCloak` (re-evaluate cloak on cell change) | ~80 B | DRIVE_LOCOMOTION_CLASS |

### Timer Classes

| Address | Name | Size | Source |
|---------|------|------|--------|
| `0x0046B640` | `CDTimerClass__Init` | ~20 B | DRIVE_LOCOMOTION_CLASS |
| `0x004C9480` | `CDTimerClass__Remaining` | ~50 B | DRIVE_LOCOMOTION_CLASS |
| `0x004C93D0` | `RateTimer__Current` (facing interpolation) | ~100 B | DRIVE_LOCOMOTION_CLASS |
| `0x004C9300` | `FacingClass__UpdateFacing` | ~80 B | DRIVE_LOCOMOTION_CLASS |

### Additional Helper Functions (from Process_Drive_Track agent)

| Address | Name | Size | Source |
|---------|------|------|--------|
| `0x005F6CD0` | `TechnoClass__CanCrushCheck` | — | PROCESS_DRIVE_TRACK |
| `0x0070D0D0` | `TechnoClass__HasWeaponAbility` | — | PROCESS_DRIVE_TRACK |
| `0x0075F540` | `CoordStruct__ScaleByFactor` (residual interp) | — | PROCESS_DRIVE_TRACK |
| `0x004DB810` | `FootClass__EnterCell` (vtable+0x1B4) | — | PROCESS_DRIVE_TRACK |
| `0x004DB9B0` | `FootClass__CheckNextPathOrScatter` (vtable+0x504) | — | PROCESS_DRIVE_TRACK |
| `0x005F5FA0` | `ObjectClass__SetHeight` (vtable+0x1CC) | — | PROCESS_DRIVE_TRACK |
| `0x0041C250` | `COM__CoCreateInstance_Locomotor` | — | DRIVE_LOCOMOTION_CLASS |
| `0x006743D0` | `RulesClass__ReadJumpjetControls` | ~200 B | DRIVE_LOCOMOTION_CLASS |

### DriveLocomotionClass — Runtime Globals

| Address | Name | Type | Source |
|---------|------|------|--------|
| `0x008A0758` | tile_height_angle (= π/2) | double | DRIVE_LOCOMOTION_CLASS |
| `0x008A0760` | scratch_state[4] | int[4] | DRIVE_LOCOMOTION_CLASS |
| `0x008A0770` | null_cell_coord | short[2] | DRIVE_LOCOMOTION_CLASS |
| `0x008A0780` | base_angle (runtime) | double | DRIVE_LOCOMOTION_CLASS |
| `0x008A0788` | slope_angle | double | DRIVE_LOCOMOTION_CLASS |
| `0x008A07A0` | bridge_render_offset | double | DRIVE_LOCOMOTION_CLASS |
| `0x008A07B8` | cell_center_x (= 128) | int | DRIVE_LOCOMOTION_CLASS |
| `0x008A07BC` | cell_center_y (= 128) | int | DRIVE_LOCOMOTION_CLASS |
| `0x00ABCD3C` | g_LocomotorGlobalRefCount | int | DRIVE_LOCOMOTION_CLASS |
| `0x00818858` | IID_IPersist (used for piggyback QI) | GUID | DRIVE_LOCOMOTION_CLASS |

### FootClass — Locomotion Integration

| Address | Name | Size | Source |
|---------|------|------|--------|
| `0x00520F40` | `FootClass__Locomotion_AI` | ~400 lines | DRIVE_LOCOMOTION_CLASS |
| `0x0045AEA0` | `LocomotionClass__QueryInterface_IPiggyback` | ~80 B | DRIVE_LOCOMOTION_CLASS |
| `0x004DA530` | `FootClass__AI` (main per-tick, calls ILocomotion::Process at 0x4da877) | 375 lines | DRIVE_LOCOMOTION_CLASS |
| `0x006C4010` | `DriveLocomotionClass__ClassFactory_CreateInstance` | ~50 B | DRIVE_LOCOMOTION_CLASS |
| `0x00481A00` | `CellClass__Can_Enter_Cell_General` | 787 lines | DRIVE_LOCOMOTION_CLASS |
| `0x0056D100` | `MapClass__Can_Reach_Zone` | ~50 B | DRIVE_LOCOMOTION_CLASS |

### A* Pathfinding System

| Address | Name | Size | Source |
|---------|------|------|--------|
| `0x004D3920` | `FootClass__Find_Path` (entry point, populates 24-entry path_queue) | ~500 lines | PATHFINDING_ASTAR |
| `0x004CBBA0` | `FootClass__Run_AStar` (wrapper: zone precheck → A* → smoothing) | — | PATHFINDING_ASTAR |
| `0x00429A90` | `AStar_main_loop` (cell-level A* search, 8+1 directions) | — | PATHFINDING_ASTAR |
| `0x0042A460` | `AStar_create_node` (allocate/init path node) | — | PATHFINDING_ASTAR |
| `0x00429830` | `AStar_compute_edge_cost` (LandTypeCost × cliff × bridge × speed) | — | PATHFINDING_ASTAR |
| `0x0042AA90` | `AStar_reconstruct_path` (trace parent chain to build direction queue) | — | PATHFINDING_ASTAR |
| `0x0042C290` | `Zone_precheck` (hierarchical zone graph mini-A*) | — | PATHFINDING_ASTAR |
| `0x0042B210` | `Path_smooth_corners` (post-processing pass 1) | — | PATHFINDING_ASTAR |
| `0x0042B7F0` | `Path_optimize_straight_segments` (post-processing pass 2) | — | PATHFINDING_ASTAR |
| `0x0042B420` | `Path_smooth_single_segment` (post-processing pass 3) | — | PATHFINDING_ASTAR |
| `0x00429780` | `Path_walk_directions_to_cell` (convert direction sequence to cell) | — | PATHFINDING_ASTAR |
| `0x0042A5B0` | `PathfinderClass__Reset` | — | PATHFINDING_ASTAR |
| `0x0042CCD0` | `PathfinderClass__UpdateHierarchicalEdges` | — | PATHFINDING_ASTAR |
| `0x0042CF80` | `PathfinderClass__InvalidateZoneEdge` | — | PATHFINDING_ASTAR |
| `0x0042DCA0` | `MinHeap__SiftDown` (A* open set) | — | PATHFINDING_ASTAR |
| `0x0056D3F0` | `ZoneMap__CellToZoneIndex` | — | PATHFINDING_ASTAR |
| `0x005840C0` | `ZoneMap__FloodFillReachableZones` | — | PATHFINDING_ASTAR |
| `0x00583180` | `MapClass__ResolvePathCoord_BridgeAware` | — | PATHFINDING_ASTAR |
| `0x0056E7C0` | `CellRect__CheckPassability` | — | PATHFINDING_ASTAR |
| `0x00586780` | `CellRect__CheckOccupancy` | — | PATHFINDING_ASTAR |
| `0x0056DC20` | `FootClass__Find_Nearby_Passable_Cell` | — | PATHFINDING_ASTAR |

### A* Pathfinding — Verified Data Tables

| Address | Name | Value/Size | Source |
|---------|------|-----------|--------|
| `0x0081870C` | LandTypeCostTable | 8 floats: {1.0, 1000.0, 1.0, 1.0, 60.0, 20.0, 8.0, 10000.0} | VERIFIED |
| `0x0081872C` | DirectionEpsilon | 9 floats: {.001,.005,.002,.006,.003,.007,.004,.008,.000} | VERIFIED |
| `0x00818760` | DirectionLookupTable | 9 ints: dy*3+dx → direction (0-7, 8=bridge) | VERIFIED |
| `0x007E37BC` | AStarTemporaryMarkerMultiplier | 4.0f when destination CellClass+0x140 has search-scoped bit 0x40000 | VERIFIED |
| `0x007E2AC8` | BridgeOneSideCost | 1.0f | VERIFIED |
| `0x007E37B4` | BridgeBothSidesCost | 2.0f | VERIFIED |
| `0x007E37B8` | BridgeNeitherSideCost | 10.0f | VERIFIED |
| `0x007E3794` | ZoneEdgeCostTable | 8 floats: {1,0,0,1,1,0,1,1} | VERIFIED |
| `0x007E3818` | ZoneDiagonalPenalty | 0.001 (double) | VERIFIED |
| `0x0082A594` | ZonePassabilityMatrix | 13×8 i32 (1=pass/2=block/3=special) | VERIFIED |
| `0x008650BC` | Sqrt_Approx lookup table | 8192 float entries | VERIFIED |

### DriveLocomotionClass — Extended Vtable Slots & Serialization

| Address | Name | Size | Source |
|---------|------|------|--------|
| `0x004AF780` | `DriveLocomotionClass__Load` (IPersistStream) | ~127 lines | DRIVE_LOCOMOTION_CLASS |
| `0x004AF800` | `DriveLocomotionClass__Save` (IPersistStream) | ~223 lines | DRIVE_LOCOMOTION_CLASS |
| `0x004B4920` | `DriveLocomotionClass__Is_To_Have_Shadow_Override` (vtable+0xA0) | ~32 B | DRIVE_LOCOMOTION_CLASS |
| `0x004B4B00` | `DriveLocomotionClass__Can_Use_Track` (vtable+0xA4) | ~217 B | DRIVE_LOCOMOTION_CLASS |
| `0x004B4D50` | `DriveLocomotionClass__Release_Piggybacked_Helper` | ~20 B | DRIVE_LOCOMOTION_CLASS |

### Refinery Docking — Force_Track Callers

| Address | Name | Tracks | Source |
|---------|------|--------|--------|
| `0x00458E50` | `BuildingClass__DockUnit` | 67-70 (facing-based) | DRIVE_LOCOMOTION_CLASS |
| `0x004593A0` | `BuildingClass__UndockUnit` | 71 | DRIVE_LOCOMOTION_CLASS |
| `0x004595C0` | `BuildingClass__FinishUndock` | 71 | DRIVE_LOCOMOTION_CLASS |

### DriveLocomotionClass — Factory & Data

| Address | Name | Type | Source |
|---------|------|------|--------|
| `0x007F3C84` | Drive ClassFactory function pointer | ptr | DRIVE_LOCOMOTION_CLASS |
| `0x007F5DFC` | UnitClass vtable slot +0x18C | ptr | DRIVE_LOCOMOTION_CLASS |

### IPiggyback Vtable (0x007E7E8C)

| Slot | Offset | Address | Method |
|------|--------|---------|--------|
| 0 | +0x00 | 0x4b4dc0 | QueryInterface |
| 1 | +0x04 | 0x4b4dd0 | AddRef |
| 2 | +0x08 | 0x4b4de0 | Release |
| 3 | +0x0C | 0x4af8e0 | Begin_Piggyback |
| 4 | +0x10 | 0x4af930 | End_Piggyback |
| 5 | +0x14 | 0x4af970 | Is_Ok_To_End |
| 6 | +0x18 | 0x4af610 | Piggybacker_CLSID |
| 7 | +0x1C | 0x4b4cd0 | Is_Piggybacking |

---

### OverlayClass (size=0xB0, RTTI=0x14)

| Address | Name | Notes | Source |
|---------|------|-------|--------|
| `0x005FC380` | `OverlayClass::Constructor` | Calls ObjectClass ctor, sets type at +0xAC | OVERLAY_CLASS_SYSTEM |
| `0x005FDF70` | `OverlayClass::Destructor` | Removes from global array, clears type | OVERLAY_CLASS_SYSTEM |
| `0x005FDF10` | `OverlayClass::GetClassID` | Returns CLSID from 0x7E96B0 | OVERLAY_CLASS_SYSTEM |
| `0x005FD8F0` | `OverlayClass::Load` | Restore from save file | OVERLAY_CLASS_SYSTEM |
| `0x005FD950` | `OverlayClass::Save` | Delegates to ObjectClass::Save | OVERLAY_CLASS_SYSTEM |
| `0x005FDF50` | `OverlayClass::What_Am_I` | Returns 0x14 (20) | OVERLAY_CLASS_SYSTEM |
| `0x005FDDE0` | `OverlayClass::GetType` | Returns *(this+0xAC) = OverlayTypeClass* | OVERLAY_CLASS_SYSTEM |
| `0x005FD270` | `OverlayClass::Unlimbo` | Coords→cell, check blocking, place | OVERLAY_CLASS_SYSTEM |
| `0x005FED00` | `OverlayClass::GetRadarColor` | Tiberium radar color with bridge byte-swap | OVERLAY_CLASS_SYSTEM |

### OverlayTypeClass (size=700, RTTI=0x15)

| Address | Name | Notes | Source |
|---------|------|-------|--------|
| `0x005FE250` | `OverlayTypeClass::Constructor` | Inits all fields, registers in global array | OVERLAY_CLASS_SYSTEM |
| `0x005FE770` | `OverlayTypeClass::ReadINI` | Reads all 17+ INI keys, forces Land/Armor for Tiberium | OVERLAY_CLASS_SYSTEM |
| `0x005FEC70` | `OverlayTypeClass::FindOrCreate` | Lookup by name, allocates 700B if new | OVERLAY_CLASS_SYSTEM |
| `0x005FEF00` | `OverlayTypeClass::What_Am_I` | Returns 0x15 (21) | OVERLAY_CLASS_SYSTEM |
| `0x005FEF10` | `OverlayTypeClass::Size_Of` | Returns 700 | OVERLAY_CLASS_SYSTEM |
| `0x005FE530` | `OverlayTypeClass::CreateInstance` | Allocates 0xB0, calls OverlayClass ctor | OVERLAY_CLASS_SYSTEM |
| `0x005FE570` | `OverlayTypeClass::CreateInstanceAtDefault` | Same with default coords | OVERLAY_CLASS_SYSTEM |
| `0x005FE4C0` | `OverlayTypeClass::GetDimensions` | Returns {0, 0x7FFF7FFF} | OVERLAY_CLASS_SYSTEM |
| `0x005FEDE0` | `OverlayTypeClass::GetRadarColor` | Returns +0x2B6 RGB | OVERLAY_CLASS_SYSTEM |
| `0x005FEC30` | `OverlayTypeClass::GetClassID` | Returns CLSID from 0x7E9600 | OVERLAY_CLASS_SYSTEM |
| `0x005FEAF0` | `OverlayTypeClass::Load` | Restore from save, reload SHP | OVERLAY_CLASS_SYSTEM |
| `0x005FEC10` | `OverlayTypeClass::Save` | Delegates to ObjectTypeClass::Save | OVERLAY_CLASS_SYSTEM |
| `0x005FEA50` | `OverlayTypeClass::SaveStream` | Saves ArrayIndex, Land, Wall, Tiberium, etc. | OVERLAY_CLASS_SYSTEM |
| `0x005FEA30` | `OverlayTypeClass::GetCoords` | Copies 12-byte 3D coordinate | OVERLAY_CLASS_SYSTEM |
| `0x005FDD20` | `IsWallOverlay` | **Misnomer**: overlay→TiberiumClass index mapper | OVERLAY_CLASS_SYSTEM |

### OverlayClass Vtable (0x7EF3D4)

| Slot | Offset | Address | Method |
|------|--------|---------|--------|
| 3 | +0x0C | 0x5fdf10 | GetClassID |
| 5 | +0x14 | 0x5fd8f0 | Load |
| 6 | +0x18 | 0x5fd950 | Save |
| 8 | +0x20 | 0x5fdf70 | Destructor |
| 11 | +0x2C | 0x5fdf50 | What_Am_I (returns 0x14) |
| 34 | +0x88 | 0x5fdde0 | GetType |
| 53 | +0xD4 | 0x5fd270 | Unlimbo |

### OverlayTypeClass Vtable (0x7EF600)

| Slot | Offset | Address | Method |
|------|--------|---------|--------|
| 3 | +0x0C | 0x5fec30 | GetClassID |
| 5 | +0x14 | 0x5feaf0 | Load |
| 6 | +0x18 | 0x5fec10 | Save |
| 11 | +0x2C | 0x5fef00 | What_Am_I (returns 0x15) |
| 12 | +0x30 | 0x5fef10 | Size_Of (returns 700) |
| 13 | +0x34 | 0x5fea50 | SaveStream |
| 25 | +0x64 | 0x5fe770 | ReadINI |
| 27 | +0x6C | 0x5fea30 | GetCoords |
| 32 | +0x80 | 0x5fe530 | CreateInstance |
| 35 | +0x8C | 0x5fe570 | CreateInstanceAtDefault |
| 36 | +0x90 | 0x5fe4c0 | GetDimensions |
| 39 | +0x9C | 0x5fede0 | GetRadarColor |

### Overlay-Related Globals

| Address | Type | Name | Source |
|---------|------|------|--------|
| `0x00A8EC54` | ptr | OverlayClass array base pointer | OVERLAY_CLASS_SYSTEM |
| `0x00A8EC58` | int | OverlayClass array capacity | OVERLAY_CLASS_SYSTEM |
| `0x00A8EC60` | int | OverlayClass array count | OVERLAY_CLASS_SYSTEM |
| `0x00A83D84` | ptr | OverlayTypeClass array base pointer | OVERLAY_CLASS_SYSTEM |
| `0x00A83D88` | int | OverlayTypeClass array capacity | OVERLAY_CLASS_SYSTEM |
| `0x00A83D90` | int | OverlayTypeClass array count (~250 in YR) | OVERLAY_CLASS_SYSTEM |
| `0x00B0F4EC` | ptr | TiberiumClass array base pointer | OVERLAY_CLASS_SYSTEM |
| `0x00B0F4F8` | int | TiberiumClass array count (2 in YR) | OVERLAY_CLASS_SYSTEM |
| `0x0081CC30` | int[16] | Overlay variety Latin square (4×4) | OVERLAY_CLASS_SYSTEM |
| `0x0081CD28` | int[12] | Ore density neighbor lookup table | OVERLAY_CLASS_SYSTEM |
| `0x00818CA0` | int[4] | Wall connection bitmask {1,2,4,8} = N,E,S,W | OVERLAY_CLASS_SYSTEM |
| `0x00AC1608` | packed | Default cell coords for overlay placement | OVERLAY_CLASS_SYSTEM |

### Wall-Related Functions

| Address | Name | Notes | Source |
|---------|------|-------|--------|
| `0x00452A40` | `BuildingClass::ConnectWalls` | 4-direction wall connectivity | OVERLAY_CLASS_SYSTEM |
| `0x00453060` | `BuildingClass::AdjustWallConnections` | Update neighbor wall frame | OVERLAY_CLASS_SYSTEM |
| `0x00453240` | `BuildingClass::OnWallDestroyed` | Chain connectivity update on destruction | OVERLAY_CLASS_SYSTEM |
| `0x004533A0` | `BuildingClass::RecalculateWallConnections` | Full recalc | OVERLAY_CLASS_SYSTEM |
| `0x00452DC0` | `BuildingClass::ExtendWallInDirection` | Wall extension placement | OVERLAY_CLASS_SYSTEM |
| `0x0056BEC0` | `WallOverlay_HeightAdjust` | Adjust wall height/passability | OVERLAY_CLASS_SYSTEM |
| `0x006D5C50` | `OverlayWall_PlacementShadow` | Wall placement preview shadow | OVERLAY_CLASS_SYSTEM |

---

*Updated 2026-03-31 from 30+ research docs. ~530 functions, ~190 globals, 40 vtables, 8 class layouts, 5 structs, 2 enums, 9 function signatures. All entries now have Ghidra labels (~1,570 labeled functions total). OverlayClass and OverlayTypeClass fully cataloged: 24 methods, 2 vtables, 17 INI keys, 12 globals, complete field layouts.*
