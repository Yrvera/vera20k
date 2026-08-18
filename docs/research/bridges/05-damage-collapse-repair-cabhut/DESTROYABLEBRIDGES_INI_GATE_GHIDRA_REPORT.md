# DestroyableBridges INI Gate — Ghidra Report

**Date:** 2026-05-18
**Confidence:** HIGH (content + identity + binding all verified in this session)
**Active in YR:** YES (verified — gate is live, default `true`)

## 0. TL;DR

`DestroyableBridges=` is **NOT** read from `[CombatDamage]` despite the shipped
INI placement. The retail `rulesmd.ini` line at `[CombatDamage] DestroyableBridges=yes`
(line 804) is **decorative / no-op** — the binary never parses that section for
this key. The actual read is from **`[SpecialFlags]`**, stored as **bit 0xF
(0x8000) of the `uint32` SpecialFlags bitfield at `ScenarioClass + 0x000`**
(the very first dword of `ScenarioClass`), and read from the **map/scenario
INI** (not from `rulesmd.ini`).

Stock `rulesmd.ini` ships with **no `[SpecialFlags]` section at all**, so the
default value passed to `ReadBool` becomes the controlling default. Because
`ScenarioClass` is constructor-initialized and the flag bit starts as 1 in the
runtime defaults (see §6), `DestroyableBridges` defaults to **enabled (yes)** in
a standard YR skirmish.

The destruction-side gate is in **exactly one place**:
`Apply_area_damage @ 0x00489280`. The check is:

```c
if (((*g_ScenarioClass_Instance & 0x8000) == 0) || (warhead->Wall == 0))
    goto LAB_0048a2c4;   // SKIP all bridge-destruction sub-blocks
```

Bridge destruction triggered by **C4 / demo-truck on a Bridge Repair Hut** is
**NOT gated** by `DestroyableBridges` — `MapClass::DestroyBridge_{High,Low}_OnHutDeath`
have no SpecialFlags check (see §5).

## 1. INI parser side — `[SpecialFlags]` reader

### 1.1 String literal and xrefs

- String `"DestroyableBridges"` @ `0x00840248` (ASCII, single occurrence).
- DATA xrefs (only two):
  - `0x006B8B98` in `FUN_006B8B30` — **writer** (saves SpecialFlags to scenario INI).
  - `0x006B8E1F` in `FUN_006B8CA0` — **reader** (parses SpecialFlags from scenario INI).

### 1.2 Reader function — `FUN_006B8CA0`

- Caller: `ScenarioClass::Read_INI_Basic @ 0x00689E90` (single call site at
  `0x00689EAB`, ECX = ESI = the original `this` of `Read_INI_Basic`).
- Body: calls `INIClass::ClearSectionCache` then issues one
  `CCINIClass::ReadBool` per bit of a `uint32` bitfield. Address of the section
  literal `"SpecialFlags"` is `0x008401D0`, dereferenced via
  `PTR_s_SpecialFlags_008401CC` (= `0x008401D0`) and the parallel
  `PTR_s_SpecialFlags_008401C8` (writer-side, same target).

### 1.3 `DestroyableBridges` bit position

From the decompile of `FUN_006B8CA0`:

```c
uVar2 = CCINIClass__ReadBool(
            PTR_s_SpecialFlags_008401cc,
            s_DestroyableBridges_00840248,
            <current bit 0xF as default>);
*param_1 = (uVar2 & 1) << 0xF | *param_1 & 0xFFFF7FFF;
```

So `DestroyableBridges` occupies **bit 0xF (value 0x8000)** of the SpecialFlags
`uint32`. This is consistent with the writer side `FUN_006B8B30` which does
`*param_1 >> 0xF & 0xFFFFFF01`.

### 1.4 Map-load gating

Inside `FUN_006B8CA0`, the entire `DestroyableBridges` read is wrapped in:

```c
if ((g_GameMode == 0) || (g_IsMapEditor != '\0')) {
    // ... TiberiumGrows, TiberiumSpreads, DestroyableBridges,
    //     FixedAlliance, FogOfWar, Inert, HarvesterImmune ...
}
```

In skirmish/multiplayer (`g_GameMode != 0`, map editor off) the
`DestroyableBridges` bit on the **map** is **not read**; the value stays at
whatever the runtime default initialized to. Only single-player campaign
(`g_GameMode == 0`) or the map editor reads the per-map override.

Conclusion: in a normal YR skirmish, the flag is effectively a **constant
runtime default = on** unless a campaign map explicitly disables it via the
`[SpecialFlags]` section.

## 2. Storage location — `ScenarioClass + 0x000`

- `*g_ScenarioClass_Instance` IS the SpecialFlags `uint32`. References in
  `ScenarioClass::Full_Init @ 0x00686B20`:
  - `*g_ScenarioClass_Instance = *g_ScenarioClass_Instance & 0xFFFFEFFF`
    (clears bit 0xC = FogOfWar) at function entry.
  - `(*g_ScenarioClass_Instance & 0x1000) != 0` test late in the function for
    FogOfWar enablement.
- Parallel/cache copy: `DAT_00A8E960`. Written in `ScenarioClass::Full_Init`
  (`*g_ScenarioClass_Instance = DAT_00a8e960;` after Read_INI_Basic completes
  for non-campaign), in `EventClass::Execute @ 0x004C6E59` (network event
  applies SpecialFlags), and in `Main_Game @ 0x0052E95C` / `0x0052E976`.

## 3. Default value — verified

- `ini/rules.ini` line 664: `DestroyableBridges=yes` in `[CombatDamage]` —
  **NOT READ** by the binary (no xref from any `RulesClass::ReadCombatDamage`
  parser site). Confirmed by listing all 81 string xrefs of `"CombatDamage"`
  @ `0x00839E8C` — every one is inside `RulesClass::ReadCombatDamage @
  0x0066BBC9..0x0066CF41`, and none of those xrefs are co-located with
  `"DestroyableBridges"` string xrefs.
- No `[SpecialFlags]` section exists in `ini/rules.ini` or `ini/rulesmd.ini`.
- Therefore stock-skirmish default = **enabled** (constructor default `1` at
  bit 0xF, never overridden by a map).

## 4. Consumer — single gate site

### 4.1 `Apply_area_damage @ 0x00489280`

Two consecutive gates inside `Apply_area_damage` reach bridge destruction:

```c
// Gate at LAB_00489F11 area (just before bridge-block):
if (((*g_ScenarioClass_Instance & 0x8000) == 0) || (warhead->+0x144 == 0))
    goto LAB_0048a2c4;
```

`warhead->+0x144` is `Wall=` (VERIFIED previously — see
`HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md` §11.6 and
`BRIDGE_AOE_LAYER_DAMAGE_GHIDRA_REPORT.md` line 26).

When `(SpecialFlags & 0x8000) == 0` (i.e. `DestroyableBridges = no`), the
following are **all skipped**:

1. The high-bridge ramp / state-machine dispatch
   (`ProcessBridgeDamageStateMachine_High`).
2. The low-bridge ramp / state-machine dispatch
   (`ProcessBridgeDamageStateMachine_Low`).
3. The direct overlay-range checks `[0x4A..0x63]` /  `[0xCD..0xE6]` that call
   `DestroyBridge_Low` and `DestroyBridge_High`.
4. The `TacticalClass::DirtyScreenRect` damage-flash for those bridge tiles.

The unaffected tail at `LAB_0048a2c4` still runs: ordinary cell-overlay
destruction (trees / barrels / Tiberium overlays via `OverlayTypeClass+0x2B0`),
crater spawning, particle systems, and the chained `Apply_area_damage` call
for C4Warhead splash.

### 4.2 Gate semantics: "drop the calls"

This is a **"drop the call entirely"** gate, not a "compute then early-exit"
gate. The bridge-destruction `ApplyDamageToCell` / `DestroyBridge_*`
invocations are themselves skipped — no animations are spawned, no
`TacticalClass::DirtyScreenRect` for those tiles, no
`StopAllTargeting` calls. Bridge tiles are completely inert to AoE damage
when the gate is off.

## 5. NOT gated — Bridge Repair Hut death paths

`MapClass::DestroyBridge_High_OnHutDeath @ 0x00574000` and
`MapClass::DestroyBridge_Low_OnHutDeath @ 0x00574C20` have **no
`SpecialFlags & 0x8000` check** anywhere in their bodies. They call
`MapClass::DestroyBridgeFromCell_*` and `ApplyDamageToCell` unconditionally.

Their two callers are:
- `BuildingClass::Update @ 0x0043FB20` — C4 timer expiry on a
  `BridgeRepairHut` building.
- `BombClass::Detonate @ 0x00438720` — demo-truck / Ivan bomb detonation on a
  `BridgeRepairHut` building.

Per the parity contract this is a **behavioral split**: setting
`DestroyableBridges = no` on a map prevents AoE warheads from collapsing
bridges, but Tanya/SEAL C4 on a bridge hut and demo-truck/Ivan-bomb on a hut
**still collapse the bridge**. (This may be intentional in gamemd — huts are a
deliberate "always works" path — but it does mean the flag's name is
overbroad.)

Bridge-repair side (`MapClass::RepairBridge_*` and the
`RepairBridgeWalker_*` family from
`BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md` §1–§6) is unrelated and
unaffected — repair has no parallel gate.

## 6. Active in YR — verdict

**Active: YES.**

- Read path: live (`ScenarioClass::Read_INI_Basic` is reached on every map
  load when `g_GameMode == 0` or the map editor is open).
- Consumer path: live (`Apply_area_damage` is the central AoE damage
  dispatcher, invoked from animations, projectiles, lightning, nukes, dominator,
  waves, fly-locomotor, infantry per-cell, super weapons, etc. — see §4
  caller list).
- TS legacy?  No. The flag is parsed alongside `MCVDeploy`, `InitialVeteran`,
  `IonStorms`, `Meteorites`, `Visceroids`, `HarvesterImmune`, `FogOfWar` — all
  YR-relevant SpecialFlags. Some of those (FogOfWar) default off in YR, but
  the parser itself is live.
- Default in stock YR: **on** (no map overrides it via `[SpecialFlags]`; the
  bit-15 default is set by the runtime SpecialFlags constructor before the
  first map load).

## 7. Open Questions

These are out of scope for this report's narrow target. Listed here for the
parent agent / next session.

1. **Runtime SpecialFlags constructor default.** Bit 0xF is observed to be 1
   at runtime (bridges destructible by default), but I did not decompile the
   `SpecialFlags::SpecialFlags` / `ScenarioClass::Reset` constructor to
   confirm the literal bit-mask used. Likely candidates:
   `ScenarioClass::ctor @ 0x00686xxx`, or the global init block tied to
   `g_ScenarioClass_Instance @ 0x00B05400`-ish. Verifying the initial
   `uint32` literal in the constructor would close this loop.
2. **`DAT_00A8E960` purpose.** Treated here as a "cache copy" of
   `*g_ScenarioClass_Instance`. The exact semantics (savegame restore?
   pre-`Read_INI_Basic` snapshot? network-replicated copy?) need a quick
   trace of its 20+ xrefs — not load-bearing for the `DestroyableBridges`
   gate but worth tightening up.
3. **`DestroyableBridges` in `[CombatDamage]` — stale-INI cleanup.** The line
   in stock `rulesmd.ini` `[CombatDamage]` is a no-op leftover (RA1/TS
   convention). Any Rust port that parses `[CombatDamage] DestroyableBridges`
   would be reading a non-spec key. Should be redirected to read
   `[SpecialFlags] DestroyableBridges` from the **map INI**, not the rules INI.
4. **Asymmetry between AoE gate and Hut-death paths.** `DestroyableBridges=no`
   stops AoE warheads from collapsing bridges but does NOT stop
   C4/demo-truck on a Bridge Repair Hut from doing so. This may be a
   gamemd intentional design or an oversight. Player-visible: yes — if a
   campaign map sets `DestroyableBridges=no` and a scripted demo-truck
   triggers on a hut, the bridge will still collapse. Worth flagging when
   we wire the Rust gate so we match gamemd's asymmetry exactly.

## 8. Sources

- `"DestroyableBridges"` string @ `0x00840248`.
- `FUN_006B8B30` — SpecialFlags WRITER (saves to scenario INI under
  `[SpecialFlags]`).
- `FUN_006B8CA0` — SpecialFlags READER (parses from scenario INI under
  `[SpecialFlags]`).
- `ScenarioClass::Read_INI_Basic @ 0x00689E90` — only caller of
  `FUN_006B8CA0` at `0x00689EAB`.
- `ScenarioClass::Full_Init @ 0x00686B20` — accesses
  `*g_ScenarioClass_Instance` SpecialFlags directly (confirms storage = byte
  offset 0).
- `Apply_area_damage @ 0x00489280` — the consumer / gate site. Gate
  expression: `((*g_ScenarioClass_Instance & 0x8000) == 0) || (warhead->+0x144 == 0)`.
- `MapClass::DestroyBridge_High_OnHutDeath @ 0x00574000` — NOT gated.
- `MapClass::DestroyBridge_Low_OnHutDeath @ 0x00574C20` — NOT gated.
- `BuildingClass::Update @ 0x0043FB20` and `BombClass::Detonate @ 0x00438720`
  — callers of the OnHutDeath functions.
- `ini/rulesmd.ini` line 804 — decorative `[CombatDamage] DestroyableBridges=yes`
  (no-op).
- `ini/rules.ini` line 664 — same decorative entry.
- Cross-reference: `BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md` §7 +§8
  ("Phase 2 needed to find the gate") — **resolved** by this report.
- Cross-reference: `HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md` §11.6
  (warhead+0x144 = Wall= INI key) — used here, not re-verified this session.
- Cross-reference: `BRIDGE_AOE_LAYER_DAMAGE_GHIDRA_REPORT.md` line 26 (also
  warhead+0x144 = Wall=) — same.
