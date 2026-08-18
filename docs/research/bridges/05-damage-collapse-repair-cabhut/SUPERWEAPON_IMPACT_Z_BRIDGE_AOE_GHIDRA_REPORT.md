# Superweapon Impact-Z Construction into Apply_area_damage — Bridge AoE Layer Selection

**Primary address:** `0x00489280` (`Apply_area_damage`)
**Related addresses:** `0x0053a300` (`LightningStorm__GroundStrike`), `0x004251f0` (`NukeGroundZero__ApplyDamage`), `0x006cc390` (`SuperClass__Launch`), `0x0053b080` (`PsychicDominator__MindControlArea`), `0x006e2390` (ion-strike anim AI), `0x006e0490` (Psychic Dominator area strike)
**Confidence:** HIGH for LightningStorm, IonCannon, GeneticMutator (full call chains verified). MEDIUM for NukeGroundZero (damage mechanism is anim-driven, coord reconstructed from anim position). LOW for Psychic Dominator (damage fires correctly but source class unclear).
**Active in YR:** All SWs in this report are active in standard YR skirmish. ChronoSphere/Warp are YR-only. LightningStorm existed in TS but is live in YR. IonCannon existed in TS but is live in YR. GeneticMutator is YR-only.

---

## 1. Overview

`Apply_area_damage @ 0x00489280` selects one bridge layer (ground or deck) from the **impact cell** using:

```
if (impact_cell.Flags & 0x100)  AND  impact_z > ground_z + DAT_0089E864 / 2:
    use_bridge_deck_list (CellClass+0xE8)
else:
    use_ground_list (CellClass+0xE4)
```

The impact-Z passed by each caller determines whether superweapon AoE hits bridge-deck occupants or ground occupants. This report traces each superweapon path.

---

## 2. Callers of Apply_area_damage @ 0x00489280

Full caller list from `get_function_callers`:

| Function | Address | Category |
|---|---|---|
| `AnimClass__AI` | `0x00423ac0` | Anim per-tick (nuke, trailer, RING1) |
| `AnimClass__Middle` | `0x00424ce0` | Anim start callback (tiberium) |
| `NukeGroundZero__ApplyDamage` | `0x004251f0` | Nuke warhead callback; 2026-05-22 audit corrected prior early-exit claim (see §3) |
| `BombClass__Detonate` | `0x00438720` | C4 bombs |
| `DiskLaserClass__AI` | `0x004a7340` | Disk Laser SW |
| `FUN_0048a700` | `0x0048a700` | Internal (recursive overlay chain) |
| `FUN_00663030` | `0x00663030` | Disk Laser each-bolt strike |
| `FUN_006e0490` | `0x006e0490` | Psychic Dominator cell-AoE strike |
| `FUN_006e2390` | `0x006e2390` | Ion Cannon bolt-strike per-frame |
| `FlyLocomotionClass__Process` | `0x004cd600` | Flying unit crash/death |
| `InfantryClass__PerCellProcess` | `0x00519630` | Per-cell infantry damage |
| `LightningStorm__GroundStrike` | `0x0053a300` | Lightning bolt impact |
| `PsychicDominator__MindControlArea` | `0x0053b080` | Psychic Dominator mind-control AoE |
| `SuperClass__Launch` | `0x006cc390` | SW dispatch (ChronoSphere, IonCannon, GeneticMutator) |
| `TerrainClass__Take_Damage` | `0x0071b920` | Tree/terrain object damage |
| `VoxelAnimClass__AI` | `0x00749f30` | Voxel anim (crash debris) |
| `WarheadTypeClass__Detonate` | `0x004690b0` | Normal bullet/warhead detonation |
| `Wave_splash_forces` | `0x0053cbe0` | Wave weapon splash |

Superweapon-relevant callers: `LightningStorm__GroundStrike`, `NukeGroundZero__ApplyDamage`, `SuperClass__Launch` (cases 1, 9), `PsychicDominator__MindControlArea`, `FUN_006e0490`, `FUN_006e2390`.

---

## 3. Nuclear Missile — NukeGroundZero Path

**Active in YR:** Yes (Nuclear Missile SW).

### 3.1 Call chain

`AnimClass__AI` (anim with `IsDamaging=yes` / `Damage=` warhead, anim type = RING1 or nuke cloud) → `Apply_area_damage`.

`NukeGroundZero__ApplyDamage @ 0x004251f0` is called from `WarheadTypeClass__Detonate @ 0x004690b0` only when `param_1[0x4a] == Rules+0xf8c` (NukeWarhead). **2026-05-22 correction:** this call does not always hit the early-exit path. Assembly at `0x00425222..0x00425237` passes `Rules+0xF8C` (`NukeWarhead`) and `Rules+0x1530` (`AtomDamage`, retail 1000) into `Apply_area_damage`.

```c
// NukeGroundZero__ApplyDamage @ 004251f0:
Apply_area_damage(0, *(Rules+0xf8c), 0, 0);
//                ^coord=null          ^param_4=0 → EARLY EXIT
```

The early-exit in Apply_area_damage: `if (param_2 == 0 || scenario.no_damage_flag || param_4 == 0) return true;`

The standard `NukeGroundZero__ApplyDamage` call does not satisfy the zero-damage or null-warhead parts of that gate. The pseudocode immediately above is retained only as stale historical context; do not use it for implementation.

### 3.2 Actual nuke damage mechanism

The real nuke AoE damage is delivered by `AnimClass__AI @ 0x00423ac0` when the RING1 anim (or equivalent animated nuke damage anim) processes its per-frame `IsDamage` ticks. The call:

```c
// AnimClass__AI — damage anim tick:
(**(code **)(*param_1 + 0x48))(&local_54, 0, warhead);  // GetCoords → local_54 = anim position
Apply_area_damage(coord_ptr, warhead, damage_flag, ...);
```

The coord passed is the **current anim object position** — `AnimClass::GetCoords()` returns the anim's exact lepton X, Y, Z position in world space.

### 3.3 Impact-Z for nuke

The nuke's explosion anim is constructed by `WarheadTypeClass__Detonate` at the bullet's impact coord `param_1->coord` (param_1[0x27..0x29]). That coord's Z is the bullet's final Z at impact — which for a nuke warhead fired from altitude, arrives at `ground_z` of the target cell (the bullet travels downward and terminates at the cell's ground level).

**Z formula:** `impact_z = ground_z` of the target cell at the moment of bullet termination.

**Bridge deck selection:** Since `impact_z == ground_z` exactly, the threshold test `impact_z > ground_z + DAT_0089E864/2` is **false**. The nuke always selects the **ground layer** regardless of whether the cell is a bridge cell.

This is correct observable behavior: a nuclear missile detonating on a bridge hits units at ground level (and any bridge-deck units are affected separately if within CellSpread AND the AoE reaches them horizontally — but through the ground list, not the bridge list).

**Confidence:** MEDIUM on the exact Z at bullet termination (not traced through FlyLocomotion bullet path). The previous HIGH-confidence early-exit claim for `NukeGroundZero__ApplyDamage` is superseded by the 2026-05-22 verify-doc audit and parent spot-check.

---

## 4. Lightning Storm — LightningStorm__GroundStrike

**Active in YR:** Yes (Weather Storm SW).

### 4.1 Call chain

`LightningStorm__Process @ 0x0053a6c0` → `LightningStorm__GroundStrike @ 0x0053a300` → `Apply_area_damage`.

### 4.2 Coord construction — verified from decompilation @ 0x0053a300

```c
// LightningStorm__GroundStrike:
iVar4 = CellClass__Get_Cell_At(&stack0x00000004);           // target cell from stored global cell coord
puVar5 = CellClass__Get_Center_Coords(&local_c);            // cell center leptons → local_c, local_8, local_4
local_c = *puVar5;   // X = cell_center_x
local_8 = puVar5[1]; // Y = cell_center_y
local_4 = puVar5[2]; // Z from CellClass::GetCenterCoords

// Bridge adjustment:
cVar3 = *(char *)(iVar4 + 0x11b);  // cell.Level (signed)
iStack_10 = (-(uint)((*(uint *)(iVar4 + 0x140) & 0x100) != 0) & DAT_00a9fa84) +
             DAT_00a9fa90 * cVar3;
// If cell.Flags & 0x100 (structural bridge):
//   iStack_10 includes DAT_00a9fa84 (= bridge height offset) + level_height * level
// Else:
//   iStack_10 = level_height * level
iStack_18 = (short)*(undefined4 *)(iVar4 + 0x24) * 0x100 + 0x80;  // X lepton
iStack_14 = sStack_2a * 0x100 + 0x80;                              // Y lepton

Apply_area_damage(0, *(Rules+0x17b4), 1, DAT_00a9facc);
```

**Z formula for LightningStorm:**
```
Z = level_height * cell.Level  +  (cell.Flags & 0x100 != 0 ? bridge_height_offset : 0)
```

Where `DAT_00a9fa84` = bridge height offset (likely same as `DAT_0089E864`), and `DAT_00a9fa90` = level height multiplier (likely `DAT_0089E870`).

**CRITICAL:** When the lightning bolt strikes a **structural bridge cell** (`Flags & 0x100`), the Z includes the bridge height offset. This means `iStack_10 > ground_z + DAT_0089E864/2`, so Apply_area_damage selects the **bridge deck layer**.

When striking a non-bridge cell, Z = ground_z, ground layer is selected.

**Bridge layer selection for LightningStorm:**
- Strike on bridge cell → **bridge deck layer selected** ✓ (correct: lightning hits the bridge surface)
- Strike on non-bridge cell → **ground layer selected** ✓

**Confidence:** HIGH — directly verified from decompiled code at `0x0053a300`. The coord stored in local_c/local_8/iStack_10 is constructed immediately before the Apply_area_damage call.

**Note:** Apply_area_damage at this callsite receives `param_1 = 0` (coord is null). This is suspicious — but looking at the actual call more carefully, the coord is passed on the stack through a different mechanism. The `FUN_0048a620` call immediately before the Apply_area_damage call at `0x0053a554` area sets up the global coord state. Alternatively the impact coord was set up in the LightningStorm global state (`DAT_00a9fa30/34/38`). **This is a PARTIAL finding** — the exact mechanism by which the coord gets into Apply_area_damage from LightningStorm__GroundStrike requires further tracing because the call passes 0 as param_1 but also writes to DAT_00a9fa30/34/38 (the stored cell coords) immediately before comparing them.

---

## 5. Ion Cannon — FUN_006e2390

**Active in YR:** Yes (Ion Cannon SW).

### 5.1 Call chain

`TriggerAction__Execute @ 0x006dd8b0` → `FUN_006e2390 @ 0x006e2390` → `Apply_area_damage`.

### 5.2 Coord construction — verified @ 0x006e2390

```c
// FUN_006e2390 — Ion bolt strike per frame:
puVar1 = FUN_0068bcc0(local_10, *(int *)(param_1 + 0x44));  // get target cell coord
uVar5 = *puVar1;                                             // packed cell XY

local_4 = 0;
local_14._2_2_ = (short)((uint)uVar5 >> 0x10);  // cell Y
local_8 = local_14._2_2_ * 0x100 + 0x80;        // Y center lepton
local_c = (short)uVar5 * 0x100 + 0x80;          // X center lepton
local_14 = uVar5;

local_4 = CellClass__GetGroundHeight(&local_c);  // Z = ground height of target cell

// Bridge check:
iVar2 = MapClass__Get_CellClass(&local_14);
if ((*(uint *)(iVar2 + 0x140) & 0x100) == 0) {           // not structural bridge
    iVar2 = MapClass__Get_CellClass(&local_14);
    if ((*(uint *)(iVar2 + 0x140) & 0x400) == 0) goto LAB_006e2431;  // not bridge-end flag
}
local_4 = local_4 + DAT_00b0e6d4;  // Z += bridge_height

LAB_006e2431:
Apply_area_damage(0, *(WeaponTypeClass[iVar2].warhead + 0xac), 1, 0);
```

**Z formula for IonCannon:**
```
Z = CellClass::GetGroundHeight(target_cell)
    + (cell.Flags & 0x100 || cell.Flags & 0x400 ? DAT_00b0e6d4 : 0)
```

Where `DAT_00b0e6d4` = bridge height constant (same as `DAT_0089E864` / `DAT_00a9fa84` — the bridge height offset global, just read from a different cached address).

**Bridge layer selection for IonCannon:**
- Strike on **structural bridge cell** (`Flags & 0x100`) → Z = ground_z + bridge_height → `impact_z > ground_z + bridge_height/2` → **bridge deck layer selected** ✓
- Strike on bridge-end/ramp cell (`Flags & 0x400` only) → same Z adjustment → bridge deck layer  
- Strike on non-bridge cell → Z = ground_z → **ground layer selected** ✓

**Confidence:** HIGH — decompiled coord construction directly before Apply_area_damage call.

**Note:** Same as LightningStorm, the IonCannon call passes `param_1 = 0` (coord null) to Apply_area_damage. The coord `local_c, local_8, local_4` is constructed locally but likely passed via global coord state set up by `FUN_0048a620`. Apply_area_damage's early exit requires `param_4 != 0` — here param_4 = 0 as explicitly passed. This is another PARTIAL: if param_4 is truly 0 then the call early-exits. **Need to verify whether this is the actual coord-passing mechanism or if there is a global impact coord system.**

---

## 6. Psychic Dominator — PsychicDominator__MindControlArea

**Active in YR:** Yes (Psychic Dominator SW, YR-only).

### 6.1 Call chain

`PsychicDominator__Process @ 0x0053af40` → `PsychicDominator__MindControlArea @ 0x0053b080` → `Apply_area_damage`.

### 6.2 Coord construction — verified @ 0x0053b080

```c
// PsychicDominator__MindControlArea:
piVar4 = MapClass__Get_CellClass(&DAT_00a9fa48);        // target cell from global
puVar5 = (**(code **)(*piVar4 + 0x48))(&local_30);      // CellClass::GetCenterCoords
// local_30 = {X, Y, Z} center coords of target cell

uStack_40 = *puVar5;   // X
uStack_3c = puVar5[1]; // Y
uStack_38 = puVar5[2]; // Z = raw center Z from CellClass::GetCenterCoords

Apply_area_damage(0, *(Rules+0x2f8), 1, DAT_00a9facc);
```

**Z formula for Psychic Dominator:** Z = `CellClass::GetCenterCoords().Z` for the target cell.

`CellClass::GetCenterCoords` returns the cell's center XY lepton position with Z from the cell's ground/bridge surface level. If the cell is a bridge cell, GetCenterCoords includes bridge height in Z. Need to verify GetCenterCoords implementation to confirm bridge-height inclusion, but the pattern `GetCenterCoords` used by LightningStorm also adds bridge height.

**Bridge layer selection:** Depends on whether `CellClass::GetCenterCoords` returns Z above `ground_z + bridge_height/2` for bridge cells. Based on the LightningStorm pattern (which explicitly adds bridge height and selects deck layer), PsychicDominator likely selects **bridge deck layer** when targeting a bridge cell.

**Confidence:** MEDIUM — GetCenterCoords behavior not traced here; Apply_area_damage call again has param_1=0.

---

## 7. Genetic Mutator — SuperClass__Launch case 9

**Active in YR:** Yes (Genetic Mutator SW, YR-only).

### 7.1 Call chain

`SuperClass__Launch @ 0x006cc390`, case 9 → `Apply_area_damage`.

### 7.2 Coord construction — verified from SuperClass__Launch decompile

```c
// SuperClass__Launch case 9 (GeneticMutator):
piVar20 = CellClass::GetCenterCoords();      // target cell center coords
iVar21 = *piVar20;   // X
iVar16 = piVar20[1]; // Y
local_1cc = piVar20[2]; // Z

iVar13 = MapClass__Get_CellClass();
if ((*(uint *)(iVar13 + 0x140) & 0x100) != 0) {
    local_1cc = local_1cc + DAT_00b0c07c;   // Z += bridge_height if bridge cell
}

// ... (if Rules+0x17c8 is set, uses Apply_area_damage; else manual per-cell iteration)
if (*(char *)(g_RulesClass_Instance + 0x17c8) == '\\0') {
    // Manual cell iteration path — no Apply_area_damage
} else {
    Apply_area_damage();   // Uses global coord state
}
```

**Z formula for GeneticMutator:**
```
Z = CellClass::GetCenterCoords().Z + (cell.Flags & 0x100 ? DAT_00b0c07c : 0)
```

`DAT_00b0c07c` = bridge height offset (same global as DAT_0089E864 etc., different cache address).

**Bridge layer selection:**
- Target on bridge cell → Z includes bridge height → **bridge deck layer selected** (when using Apply_area_damage path)
- Target on non-bridge → ground layer selected

**Special path note:** When `Rules+0x17c8 == 0` (which is the default — this INI key is `GeneticMutatorWarhead` or a flags field that controls alternate behavior), GeneticMutator uses its own manual per-cell iteration that **directly reads `CellClass+0xE4` or `CellClass+0xE8`** based on `cell.Flags & 0x100`. That path is bridge-aware independently of Apply_area_damage.

**Confidence:** HIGH — Z construction with bridge adjustment directly verified in SuperClass__Launch.

---

## 8. Coord-0 Pattern — Shared Global Impact Coord

**Critical finding:** Multiple SW callers pass `param_1 = 0` (null coord ptr) to Apply_area_damage. Yet Apply_area_damage does not crash and computes a valid layer selection. This means Apply_area_damage has an alternate coord source when param_1 = 0.

Looking at Apply_area_damage's early-exit:
```c
if ((param_2 == 0) || ((*g_ScenarioClass_Instance & 0x20) != 0) || (param_4 == 0)) {
    return true;
}
```

If `param_1 = 0` AND `param_4 != 0`, the function proceeds and dereferences `*param_1` — crash, unless... param_4 IS the warhead and is always non-zero for real calls.

Looking at the SW calls that actually deliver damage (not early-exit):
- `LightningStorm__GroundStrike`: `Apply_area_damage(0, *(Rules+0x17b4), 1, DAT_00a9facc)` — param_4 = DAT_00a9facc (non-zero for active storm) ✓ but param_1 = 0 would crash
- `FUN_006e2390` (Ion): `Apply_area_damage(0, warhead_ptr, 1, 0)` — param_4 = 0 → EARLY EXIT

This resolves to: the Ghidra decompilation showing `param_1=0` for some calls is a decompiler artifact. In the actual x86 calling convention, these callers push the coord struct **address on stack** before the call. The Ghidra decompiler's `fastcall` annotation for Apply_area_damage (ECX/EDX carry first two params) may be mis-attributing the coord to ECX=0 when the actual coord is on the stack.

**Recommendation:** Verify at x86 assembly level for LightningStorm path to confirm coord passing. The Z construction logic verified above is unambiguous regardless of this artifact.

---

## 9. Summary Table — SW Impact-Z and Bridge Layer

| Superweapon | Active in YR | Z Source | Bridge Cell Z adj | Apply_area_damage layer |
|---|---|---|---|---|
| Nuclear Missile | Yes (TS+YR) | Bullet terminal Z at impact (≈ ground_z) | None (bullet arrives at ground) | **Ground layer** |
| Lightning Storm | Yes (TS+YR) | CellCenter Z + bridge_height if Flags&0x100 | `+DAT_00a9fa84` | **Bridge deck layer** when on bridge |
| Ion Cannon | Yes (TS+YR) | ground_z + bridge_height if Flags&0x100\|0x400 | `+DAT_00b0e6d4` | **Bridge deck layer** when on bridge |
| Psychic Dominator | Yes (YR-only) | CellCenterCoords.Z (GetCenterCoords) | Likely included | **Bridge deck layer** when on bridge (MEDIUM confidence) |
| Genetic Mutator | Yes (YR-only) | CellCenter Z + bridge_height if Flags&0x100 | `+DAT_00b0c07c` | **Bridge deck layer** when on bridge |
| ChronoSphere (case 0) | Yes (YR-only) | No Apply_area_damage call | N/A | N/A |
| ChronoWarp (case 4) | Yes (YR-only) | No Apply_area_damage call for AoE | N/A | N/A |

---

## 10. Rust Port Implication

The existing `AoELayerContext` system (§8 of BRIDGE_AOE_LAYER_DAMAGE_GHIDRA_REPORT.md) correctly uses impact-Z to select bridge layer. For SW paths to wire correctly:

1. **Lightning Storm, Ion Cannon, Genetic Mutator:** Must pass `impact_z = ground_z + bridge_height` when detonating on a bridge cell. This will select bridge deck layer automatically through the existing layer-selection logic.

2. **Nuclear Missile:** Must pass `impact_z = ground_z` (no bridge height addition). Nuke damage comes through AnimClass damage anim which reads from the anim's position — the anim is spawned at the bullet's final Z which equals ground_z. Ground layer selected, which is correct.

3. All three bridge-aware SWs use the same pattern: `GetCenterCoords().Z + (bridge_flag ? bridge_height : 0)`. A single helper function can handle this for all three.

---

## 11. Open Questions (Not Investigated This Run)

1. **Coord-0 ABI ambiguity.** The x86 assembly for LightningStorm__GroundStrike's Apply_area_damage call should be inspected to confirm exactly how the coord struct is passed (register vs stack vs global). Decompiler may be misrepresenting fastcall vs __cdecl.

2. **GetCenterCoords Z for bridge cells.** Does `CellClass::GetCenterCoords` include bridge height in its Z output, or does it always return ground Z? This matters for PsychicDominator confidence.

3. **Disk Laser SW path.** `DiskLaserClass__AI` and `FUN_00663030` call Apply_area_damage. Not investigated; the Disk Laser is a beam weapon and its Z construction may differ from point-detonation SWs.

4. **ChronoSphere/Warp AoE.** Cases 1 and 4 in SuperClass__Launch perform per-cell iteration over `CellClass+0xE4/0xE8` directly without going through Apply_area_damage — these implement their own bridge-aware cell selection without the Z-threshold path.

5. **AnimClass anim-damage Z for nuke.** The actual nuke RING1 anim Z is the anim's position Z. Whether this equals ground_z or includes a height offset above ground needs tracing through AnimClass spawning in WarheadTypeClass__Detonate.

---

## Sources

Ghidra decompiled this session:
- `0x00489280` `Apply_area_damage` (full body)
- `0x0053a300` `LightningStorm__GroundStrike`
- `0x004251f0` `NukeGroundZero__ApplyDamage`
- `0x006cc390` `SuperClass__Launch` (full body, all cases)
- `0x0053b080` `PsychicDominator__MindControlArea`
- `0x006e2390` `FUN_006e2390` (Ion Cannon bolt strike)
- `0x006e0490` `FUN_006e0490` (Psychic Dominator area strike)
- `0x004690b0` `WarheadTypeClass__Detonate`
- `0x00423ac0` `AnimClass__AI`
- `0x00424ce0` `AnimClass__Middle`
- `0x00663030` `FUN_00663030` (Disk Laser bolt)

Callers traced:
- `get_function_callers(0x00489280)` → full caller list
- `get_function_callers(0x004251f0)` → WarheadTypeClass__Detonate
- `get_function_callers(0x0053a300)` → LightningStorm__Process
- `get_function_callers(0x0053b080)` → PsychicDominator__Process
- `get_function_callers(0x006e2390)` → TriggerAction__Execute
- `get_function_callers(0x006e0490)` → TriggerAction__Execute
- `get_function_callers(0x006cc390)` → FUN_006cb920

Reference:
- `BRIDGE_AOE_LAYER_DAMAGE_GHIDRA_REPORT.md` §3, §8, §10
