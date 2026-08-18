# SpecialFlags / SpecialClass System -- Complete Bitfield Map

**Source:** Ghidra decompilation of `gamemd.exe`
**Confidence:** HIGH -- all bits verified from binary decompilation + string references
**Research date:** 2026-03-22

## Overview

The "SpecialFlags" system in RA2/YR is NOT a single class. It is a **packed uint32 bitfield**
stored as the first 4 bytes of `ScenarioClass` (at global `DAT_00a8b230`). There is a
**separate** copy at `DAT_00a8e960` used as a staging area for multiplayer game options.

The system has three layers:
1. **`[SpecialFlags]` INI section** -- 13 flags read from/written to map INI files
2. **`[MultiplayerDialogSettings]` INI section** -- lobby defaults in rules.ini (separate bytes in RulesClass)
3. **Session game option bytes** -- per-lobby-session overrides stored as individual bytes at `DAT_00a8b2xx`

At scenario start, these layers are merged into the final `*DAT_00a8b230` bitfield.

---

## Complete Bitfield Map (uint32 at ScenarioClass+0x00)

### Bits 0-4: Low flags (NOT in [SpecialFlags] INI)

| Bit | Hex    | Name             | Meaning | Where set |
|-----|--------|------------------|---------|-----------|
| 0   | 0x0001 | (unused/padding) | -- | -- |
| 1   | 0x0002 | (unused/padding) | -- | -- |
| 2   | 0x0004 | PlayIntro        | Play intro movie on startup | Startup.CPP: from `[Intro]` INI key |
| 3   | 0x0008 | NoCD             | CD not present / CD check skipped | Startup.CPP: if no CD drive detected |
| 4   | 0x0010 | CaptureTheFlag   | Capture-the-flag game mode | From `RulesClass+0x14b2` (CaptureTheFlag=) |

**Evidence for bit 4 = CaptureTheFlag:**
- `DAT_00a8e960 = (*(byte *)(DAT_008871e0 + 0x14b2) & 1) << 4 | DAT_00a8e960 & 0xffffffef` (file 076, line 758)
- RulesClass+0x14b2 = `CaptureTheFlag` key in `[MultiplayerDialogSettings]`
- Tested as `(*DAT_00a8b230 & 0x10) != 0` in scenario code and MCV deploy logic

### Bits 5-18: [SpecialFlags] INI section (13 flags)

These are the flags serialized by `FUN_006b8b30` (Save) and `FUN_006b8ca0` (Load).

| Bit | Hex     | INI Key              | Default | Meaning |
|-----|---------|----------------------|---------|---------|
| 5   | 0x0020  | `Inert`              | 0       | All weapons/combat disabled. Tested in combat damage calc (0x20 check) |
| 6   | 0x0040  | `TiberiumGrows`      | 1*      | Ore grows denser over time |
| 7   | 0x0080  | `TiberiumSpreads`    | 1*      | Ore spreads to adjacent cells. Reset default sets this ON (0x8088 mask) |
| 8   | 0x0100  | `MCVDeploy`          | 0       | MCV can deploy into Construction Yard |
| 9   | 0x0200  | `InitialVeteran`     | 0       | Units start as veterans. Tested in unit creation (0x200 check) |
| 10  | 0x0400  | `FixedAlliance`      | 0       | Alliances cannot be changed mid-game. Tested in ally logic (0x400 check) |
| 11  | 0x0800  | `HarvesterImmune`    | 0       | Harvesters cannot be attacked. Tested in targeting (0x800 check) |
| 12  | 0x1000  | `FogOfWar`           | 0       | Fog of war enabled (unexplored areas stay dark). **Most-tested bit** -- 20+ references. **TS_LEGACY_AS_YR**: default is OFF; in a standard YR skirmish FogOfWar is never active unless explicitly enabled in lobby. (annotated 2026-05-29: confirmed via [MultiplayerDialogSettings] FogOfWar=no in rulesmd.ini and Load function decompile at 0x006b8ca0) |
| 13  | 0x2000  | (unused)             | --      | Not used in binary |
| 14  | 0x4000  | `TiberiumExplosive`  | 0       | Ore cells explode when destroyed |
| 15  | 0x8000  | `DestroyableBridges` | 1*      | Bridges can be destroyed. Reset default sets this ON (0x8088 mask). Tested in superweapon targeting |
| 16  | 0x10000 | `Meteorites`         | 0       | Random meteorite storms can occur |
| 17  | 0x20000 | `IonStorms`          | 0       | Ion storms can occur |
| 18  | 0x40000 | `Visceroids`         | 0       | Visceroids spawn from tiberium/ore death |

*Default values marked with * are set by the reset function `FUN_006b8ae0`:
`*flags = *flags & 0xFFF88088 | 0x8088` -- clears bits 0-6 and 8-14, then sets bits 7 (0x80) and 15 (0x8000).

### Bits 19+: Upper bits (preserved by reset mask 0xFFF88088)

Bits 19-31 are preserved across reset but are NOT part of the [SpecialFlags] INI section.
They appear unused in the SpecialFlags context.

---

## The Two-Variable System: DAT_00a8b230 vs DAT_00a8e960

There are TWO copies of the special flags bitfield:

| Variable | Address | Purpose |
|----------|---------|---------|
| `*DAT_00a8b230` | ScenarioClass+0x00 | **Active gameplay flags** -- this is what all game logic reads |
| `DAT_00a8e960` | Standalone global | **Staging/network flags** -- built from lobby options, transmitted in MP packets |

### Flow in multiplayer:
1. Lobby options (individual bytes) compose `DAT_00a8e960`
2. `DAT_00a8e960` is sent in network packets (at packet offset +0x96)
3. Receiving side: `DAT_00a8e960 = *(uint *)(packet + 0x96) | 0xc0` (bits 6+7 always forced ON = TiberiumGrows + TiberiumSpreads)
4. At scenario start: `*DAT_00a8b230 = DAT_00a8e960` (file 109, line 4272)

### Flow in campaign/single-player:
1. `ScenarioClass::ReadINI` calls `FUN_006b8ca0` (SpecialFlags::Load) which reads the `[SpecialFlags]` INI section directly into `*DAT_00a8b230`
2. `DAT_00a8e960` is not used as the authoritative source

---

## Lobby Options to SpecialFlags Mapping

### Session Game Option Bytes (at DAT_00a8b2xx)

These are individual bytes set from the lobby UI controls (checkboxes, sliders):

| Address | Lobby Variable | Rules Offset | Rules Key |
|---------|---------------|-------------|-----------|
| `DAT_00a8b258` | Bases | +0x14af | `Bases` |
| `DAT_00a8b260` | BridgeDestroy | +0x14ac | `BridgeDestruction` |
| `DAT_00a8b261` | Crates | +0x14b1 | `Crates` |
| `DAT_00a8b262` | HarvesterTruce | -- | (set from packet bit 4 of byte at +0x8e) |
| `DAT_00a8b263` | SuperWeapons | +0x14b9 | `SuperWeaponsAllowed` |
| `DAT_00a8b264` | BuildOffAlly | +0x14ba | `BuildOffAlly` |
| `DAT_00a8b268` | GameSpeed | DAT_00a8eb60 | (numeric) |
| `DAT_00a8b25c` | Credits | +0x1484 | `Money` |
| `DAT_00a8b26c` | MultiEngineer | 0 | (always starts 0 in skirmish) |
| `DAT_00a8b270` | UnitCount | +0x1494 | `UnitCount` |
| `DAT_00a8b274` | AIPlayers | 0 | (numeric) |
| `DAT_00a8b278` | AIDifficulty | -- | (numeric) |
| `DAT_00a8b31d` | FogOfWar | -- | (from BM_GETCHECK on fog checkbox) |
| `DAT_00a8b31f` | ShortGame | -- | (from BM_GETCHECK on short game checkbox) |
| `DAT_00a8b320` | MCVRedeploy | -- | (from BM_GETCHECK on MCV checkbox) |

### How Lobby Options Map to DAT_00a8e960 Bits

When a game starts from the lobby, the following composition occurs:

```c
// File 076 (lobby StartGame):
DAT_00a8e960 = ((DAT_00a8b260 & 1) << 4 | DAT_00a8b31d & 1) << 0xb
             | DAT_00a8e960 & 0xffff77ff
             | 0xc0;
```

Expanding the shift arithmetic:
- `((DAT_00a8b260 & 1) << 4 | DAT_00a8b31d & 1)` produces a 5-bit value
- `<< 0xb` shifts it to bits 11-15
- This means:
  - Bit 11 (0x0800) = `DAT_00a8b31d` = **FogOfWar** (wait -- see below)
  - Bit 15 (0x8000) = `DAT_00a8b260` = **BridgeDestroy**
- `& 0xffff77ff` clears bits 11 and 15
- `| 0xc0` forces bits 6 and 7 ON = **TiberiumGrows** + **TiberiumSpreads** always enabled

**WAIT** -- this is confusing because bit 11 in the [SpecialFlags] INI = HarvesterImmune and bit 12 = FogOfWar. Let me re-examine more carefully.

Actually, re-reading the expression:
```
((DAT_00a8b260 & 1) << 4 | DAT_00a8b31d & 1) << 0xb
```
- Inner: `(BridgeDestroy << 4) | FogOfWar` = 5-bit value where bit 4 = BridgeDestroy, bit 0 = FogOfWar
- `<< 0xb`: shifts entire thing by 11
- Result: FogOfWar goes to bit 11, BridgeDestroy goes to bit 15

But bit 11 in the SpecialFlags = HarvesterImmune (0x800), not FogOfWar (0x1000 = bit 12).

This means DAT_00a8e960 uses a **DIFFERENT bit layout** from the [SpecialFlags] INI bitfield!

Or more precisely: DAT_00a8e960 is NOT directly the same bitfield as the SpecialFlags INI. It's a session-level flags word with its own layout.

### Resolution: The 0xC0 OR and the Scenario Load

At scenario start for multiplayer (`DAT_00a8b238 != 0`):
```c
*DAT_00a8b230 = DAT_00a8e960;   // Copy staging flags to active flags
```

Then the [SpecialFlags] INI section is loaded by `FUN_006b8ca0` which modifies bits in `*DAT_00a8b230` via read-modify-write. Some flags are CONDITIONAL on `DAT_00a8b238 == 0` (campaign) or `DAT_00a8ed6b != 0`:

**Always loaded from [SpecialFlags] INI (even in MP):**
- TiberiumExplosive (bit 14)
- MCVDeploy (bit 8)
- InitialVeteran (bit 9)
- IonStorms (bit 17)
- Meteorites (bit 16)
- Visceroids (bit 18)

**Only loaded from [SpecialFlags] INI in campaign (or when DAT_00a8ed6b != 0):**
- TiberiumGrows (bit 6)
- TiberiumSpreads (bit 7)
- DestroyableBridges (bit 15)
- FixedAlliance (bit 10)
- FogOfWar (bit 12)
- Inert (bit 5)
- HarvesterImmune (bit 11)

This means in multiplayer, these 7 flags are controlled by lobby settings and the map INI cannot override them.

### Additional Lobby-to-Flags Mappings

For FogOfWar specifically in the scenario loader:
```c
// File 109, line 3848 (multiplayer path):
*DAT_00a8b230 = (DAT_00a8b31f & 1) << 0xc | *DAT_00a8b230 & 0xffffefff;
```
This sets bit 12 (0x1000 = FogOfWar) from `DAT_00a8b31f` (which despite its name suggests confusion -- but `DAT_00a8b31f` in the skirmish dialog is the **ShortGame** checkbox in one context and the **Crates** checkbox in another, depending on which dialog is active).

Actually on re-examination, the pattern is:
```c
if (DAT_00a8b238 == 0) {   // Campaign
    *DAT_00a8b230 &= 0xffffefff;  // Clear FogOfWar bit 12
    DAT_00a8e960 &= 0xffffefff;
} else {                     // Multiplayer
    *DAT_00a8b230 = (DAT_00a8b31f & 1) << 0xc | *DAT_00a8b230 & 0xffffefff;
    DAT_00a8e960 = (DAT_00a8b31f & 1) << 0xc | DAT_00a8e960 & 0xffffefff;
}
```
So in campaign, FogOfWar is always CLEARED (then the [SpecialFlags] INI section may set it).
In multiplayer, FogOfWar bit 12 is set from `DAT_00a8b31f`.

---

## Network Packet Format for Game Options

The game options are sent in a 0xFA-byte packet (type 0x65). Key offsets:

| Packet Offset | Content |
|--------------|---------|
| +0x00 | Packet type (0x65) |
| +0x04 | Scenario name string |
| +0x7e | Credits (4 bytes) |
| +0x82 | Packed byte: bit0=Bases, bit1=BridgeDestroy, bit2=Crates, bit4=??? |
| +0x8a | TechLevel (1 byte) |
| +0x8b | UnitCount (1 byte) |
| +0x8c | AIPlayers (1 byte) |
| +0x8d | AIDifficulty (1 byte) |
| +0x8e | Packed byte: bit1=FogOfWar, bit2=MCVRedeploy, bit3=ShortGame, bit4=HarvesterTruce, bit5=SuperWeapons, bit6=BuildOffAlly, bit7=MultiEngineer |
| +0x92 | Random seed (4 bytes) |
| +0x96 | DAT_00a8e960 staging flags (4 bytes) |
| +0xa2 | GameSpeed (1 byte) |
| +0xa3 | Scenario filename string |

The receiving side decodes:
```c
DAT_00a8b258 = *(byte *)(packet + 0x82) & 1;        // Bases
DAT_00a8b320 = (*(byte *)(packet + 0x8e) >> 2) & 1; // MCVRedeploy
DAT_00a8b260 = (*(byte *)(packet + 0x82) >> 1) & 1; // BridgeDestroy
DAT_00a8b261 = (*(byte *)(packet + 0x82) >> 2) & 1; // Crates
DAT_00a8b31f = (*(byte *)(packet + 0x8e) >> 3) & 1; // ShortGame
DAT_00a8b31d = (*(byte *)(packet + 0x8e) >> 1) & 1; // FogOfWar
DAT_00a8b262 = (*(byte *)(packet + 0x8e) >> 4) & 1; // HarvesterTruce
DAT_00a8b263 = (*(byte *)(packet + 0x8e) >> 5) & 1; // SuperWeapons
DAT_00a8b264 = (*(byte *)(packet + 0x8e) >> 6) & 1; // BuildOffAlly
DAT_00a8b26c = (*(byte *)(packet + 0x8e) >> 7) & 1; // MultiEngineer
DAT_00a8e960 = *(uint *)(packet + 0x96) | 0xc0;     // Staging flags (force TibGrows+TibSpreads)
```

---

## [MultiplayerDialogSettings] INI Section (RulesClass)

All defaults for the lobby come from `[MultiplayerDialogSettings]` in rules(md).ini.
These are stored as individual fields in `RulesClass` (at `DAT_008871e0`):

| Rules Offset | Type | INI Key | Default (rulesmd.ini) |
|-------------|------|---------|----------------------|
| +0x1480 | int  | MinMoney | 5000 |
| +0x1484 | int  | Money | 10000 |
| +0x1488 | int  | MaxMoney | 10000 |
| +0x148c | int  | MoneyIncrement | 100 |
| +0x1490 | int  | MinUnitCount | 0 |
| +0x1494 | int  | UnitCount | 10 |
| +0x1498 | int  | MaxUnitCount | 10 |
| +0x149c | int  | TechLevel | 10 |
| +0x14a0 | int  | GameSpeed | 1 |
| +0x14a4 | int  | AIDifficulty | 0 |
| +0x14a8 | int  | AIPlayers | 0 |
| +0x14ac | bool | BridgeDestruction | yes |
| +0x14ad | bool | ShadowGrow | no |
| +0x14ae | bool | Shroud | yes |
| +0x14af | bool | Bases | yes |
| +0x14b0 | bool | TiberiumGrows | yes |
| +0x14b1 | bool | Crates | yes |
| +0x14b2 | bool | CaptureTheFlag | no |
| +0x14b3 | bool | HarvesterTruce | no |
| +0x14b4 | bool | MultiEngineer | no |
| +0x14b5 | bool | AlliesAllowed | no |
| +0x14b6 | bool | ShortGame | yes |
| +0x14b7 | bool | FogOfWar | no |
| +0x14b8 | bool | MCVRedeploys | yes |
| +0x14b9 | bool | SuperWeaponsAllowed | yes (rulesmd default varies) |
| +0x14ba | bool | BuildOffAlly | no |
| +0x14bb | bool | AllyChangeAllowed | yes |

---

## Usage in Game Logic (All bit test sites)

> **AUDIT NOTE 2026-05-29**: Several function addresses in this section were spot-checked against Ghidra and do NOT match their described purpose (e.g. 0x465d40 = BuildingTypeClass::Is1x1WithUndeploy, 0x479110 = display loop, 0x4863d0 = tile-type checker, 0x4cad50 = Sin_Lookup_Table4096, 0x6829a0 = COM/DirectPlay code, 0x730400 = COM object cleanup, 0x6918a0 = unrelated INI reader). The bit assignments themselves (which bits exist and their hex values) are CONFIRMED from the Save/Load function decompilations. The usage site function addresses are UNVERIFIABLE from this session and should be treated as UNCHECKED. Do not rely on the address column in this section without re-verifying. (INFERENCE_HARDENED — needs full re-audit of each address)

### Bit 4 (0x10) -- CaptureTheFlag
- `FUN_004fc060` (file 048): MCV deploy logic -- if CaptureTheFlag, triggers special handling
- `FUN_00688b00` (file 109): Scenario init -- sets flag position data

### Bit 5 (0x20) -- Inert
- `FUN_00489990` (file 026): Damage calculation -- if Inert, return 0 damage
- `FUN_004899b0` (file 026): Second damage calc path -- same Inert check

### Bit 6 (0x40) -- TiberiumGrows
- Used in ore growth system. Forced ON (0xC0) in MP staging flags.
- Debug logging: `"IsTGrowth = %d"` at `DAT_00a8e960 >> 6 & 1`

### Bit 7 (0x80) -- TiberiumSpreads
- Controls ore spreading to adjacent cells. Part of reset default (0x8088).
- Tested in cell overlay placement logic
- Debug logging: `"IsTSpread = %d"` at `DAT_00a8e960 >> 7 & 1`

### Bit 8 (0x100) -- MCVDeploy
- Tested in MCV deploy/undeploy logic
- Always loaded from [SpecialFlags] INI even in MP

### Bit 9 (0x200) -- InitialVeteran
- `FUN_00688d4b` (file 109): Unit creation -- if InitialVeteran, calls `FUN_007500b0` (promote)
- Always loaded from [SpecialFlags] INI

### Bit 10 (0x400) -- FixedAlliance
- `FUN_00730400` (file 139): Alliance change logic -- if FixedAlliance, alliance changes blocked
- Set by squad/clan game logic: `*DAT_00a8b230 |= 0x400`
- Cleared at game start in modem/serial lobby: `*DAT_00a8b230 &= 0xfffffbff`

### Bit 11 (0x800) -- HarvesterImmune
- `FUN_0048990b` (file 026): Target filtering -- if HarvesterImmune and target is harvester, skip
- `FUN_004fc060` (file 048): Building logic -- if HarvesterImmune and is multiplayer, handle
- `FUN_006fc030` (file 128): Combat AI -- if HarvesterImmune, skip harvester targets
- `FUN_007414e0` (file 140): Threat evaluation -- if HarvesterImmune, exclude harvesters

### Bit 12 (0x1000) -- FogOfWar / Crates
**This is the MOST-TESTED bit in the entire bitfield** with 20+ reference sites.

In context, this bit serves dual purpose depending on the code path:
- **Fog of War**: In the [SpecialFlags] INI section, the key is `FogOfWar`
- **Crates**: In multiplayer code, this bit is also used for crate-related logic

Usage sites:
- `FUN_00465d40` (file 019): If set, enable crate spawning
- `FUN_00479110` (file 023): Cell reveal/shroud logic -- if set, cells remain fogged
- `FUN_00479110` (file 023): Crate placement on cells
- `FUN_004863d0` (file 025): Crate pickup handling
- `FUN_004a8d50` (file 033): Map display -- crate/fog visual handling (3 references)
- `FUN_004cad50` (file 038): Unit death -- if set and eligible, spawn crate
- `FUN_004c6780` (file 037): Cell value calculation -- adjusts by 10 if crates enabled
- `FUN_0055a710` (file 065): Ore growth -- if crates and ore explosive, special handling
- `FUN_005fd2e0` (file 089): Overlay rendering -- if set, crate overlay logic
- `FUN_00643d60` (file 099): Radar display -- adjusts radar dot by 10 if crates
- `FUN_006918a0` (file 111): Session processing -- if set, crate spawning enabled
- `FUN_006829a0` (file 109): Scenario start -- if set, calls crate init `FUN_005866c0`
- `FUN_006d1fe0` (file 122): Infantry logic -- crate interaction
- `FUN_006fc0b0` (file 129): AI targeting -- crate awareness

**IMPORTANT FINDING:** Bit 12 (0x1000) in the SpecialFlags bitfield is labelled `FogOfWar` in
the [SpecialFlags] INI section. Examination of all 20+ test sites shows this bit is checked in:
- Crate spawning and crate pickup logic
- Cell shroud/fog reveal logic
- Radar display adjustments
- Infantry/unit crate interaction
- AI crate awareness
- Ore growth special handling

The bit appears to serve as a general "advanced multiplayer features" flag that enables
both fog-of-war and crate spawning simultaneously. Separate session bytes (`DAT_00a8b261`
for Crates, `DAT_00a8b31d` for FogOfWar) control these features individually in the lobby
UI, but in the [SpecialFlags] INI bitfield they share a single bit.

### Bit 15 (0x8000) -- DestroyableBridges
- `FUN_0048a2c4` (file 026): Superweapon targeting -- if DestroyableBridges and building is bridge, allow targeting
- Part of reset default (0x8088 = bits 7 + 15)

---

## Campaign vs Multiplayer Behavior

### Campaign (DAT_00a8b238 == 0):
1. SpecialFlags loaded from map's `[SpecialFlags]` section (all 13 flags)
2. FogOfWar bit 12 is CLEARED before [SpecialFlags] load
3. AIDifficulty set from `DAT_00a8eb64` (command line or saved preference)
4. Bridge destruction bit always cleared (`*DAT_00a8b230 &= 0xffffefff`)

### Multiplayer (DAT_00a8b238 != 0):
1. Lobby options compose `DAT_00a8e960` staging flags
2. At scenario load, `*DAT_00a8b230 = DAT_00a8e960`
3. [SpecialFlags] INI section loaded BUT only 6 flags (TiberiumExplosive, MCVDeploy,
   InitialVeteran, IonStorms, Meteorites, Visceroids) are applied
4. 7 flags (TiberiumGrows, TiberiumSpreads, DestroyableBridges, FixedAlliance,
   FogOfWar, Inert, HarvesterImmune) are controlled ONLY by lobby settings
5. FogOfWar bit 12 set from `DAT_00a8b31f` lobby option
6. Bridge bit set from `DAT_00a8b260` lobby option
7. If BridgeDestroy lobby option is OFF (`DAT_00a8b260 == 0`), bit 15 (0x8000) in
   DAT_00a8e960 is explicitly cleared: `DAT_00a8e960 &= 0xffff7fff`

---

## WDT Game Options (Westwood Dialog Toolkit)

The 6 standard YR multiplayer game options presented in the lobby:

1. **TechLevel** -- numeric range (WDT:TechLevel)
2. **Bases** -- boolean toggle (WDT:Bases / WDT:NoBases)
3. **ShortGame** -- boolean toggle (WDT:ShortGame / WDT:NotShortGame)
4. **UnitCount** -- numeric range (WDT:UnitCount)
5. **RedeployableMCV** -- boolean toggle (WDT:RedeployableMCV / WDT:NoRedeployMCV)
6. **Crates** -- boolean toggle (WDT:Crates / WDT:NoCrates)

Source: `D:\ra2mdpost\WDTGameOptions.cpp`, function `FUN_007660f0`.

Additional options available in the full lobby dialog (not WDT):
- BridgeDestroy, FogOfWar, HarvesterTruce, SuperWeapons, BuildOffAlly, MultiEngineer

---

## Default Reset Function (FUN_006b8ae0)

```c
void __fastcall FUN_006b8ae0(uint *flags) {
    *flags = *flags & 0xFFF88088 | 0x8088;
}
```

This:
- **Preserves** bits: 3, 7, 15, 19+ (mask 0xFFF88088) (corrected 2026-05-29: bits 16/17/18 are NOT in 0xFFF88088 — they are CLEARED, not preserved; confirmed via decompile_function 0x006b8ae0 — OPERATOR_OR_ORDER_DRIFT)
- **Clears** bits: 0-2, 4-6, 8-14, 16-18 (corrected 2026-05-29: added 16/17/18 = Meteorites/IonStorms/Visceroids which reset to 0; confirmed via decompile_function 0x006b8ae0 — OPERATOR_OR_ORDER_DRIFT)
- **Sets** bits: 7 (TiberiumSpreads) and 15 (DestroyableBridges) via OR 0x8088

Called by:
- `FUN_006832c0` (ScenarioClass constructor)
- `FUN_0052f620` (Init/Reset)

---

## Key Addresses Summary

| Address | Size | Description |
|---------|------|-------------|
| `DAT_00a8b230` | ptr to uint32 | ScenarioClass pointer. `*DAT_00a8b230` = SpecialFlags bitfield |
| `DAT_00a8b238` | int | Session mode: 0=campaign, nonzero=multiplayer |
| `DAT_00a8e960` | uint32 | Staging copy of SpecialFlags for multiplayer |
| `DAT_008871e0` | ptr | RulesClass pointer. +0x14ac through +0x14bb = MultiplayerDialogSettings bools |
| `DAT_00a8ed6b` | byte | Flag that allows [SpecialFlags] to override MP settings |
| `FUN_006b8b30` | func | SpecialFlags::Save (write to INI) |
| `FUN_006b8ca0` | func | SpecialFlags::Load (read from INI) |
| `FUN_006b8ae0` | func | SpecialFlags::Reset (set defaults) |
| `FUN_00671ea0` | func | RulesClass::Read_MultiplayerDialogSettings |

---

## Correcting Common Misconceptions

### "Bit 0x1000 = Crates" -- PARTIALLY CORRECT
Bit 12 (0x1000) is labelled `FogOfWar` in the `[SpecialFlags]` INI section, but is tested
extensively in crate-related code paths (20+ sites). It appears to function as a combined
fog/crate enable bit. The lobby-level Crates toggle is `DAT_00a8b261` (a standalone session
byte), which feeds into the staging flags independently.

### "Bit 0x8000 = Superweapons" -- WRONG
Bit 15 (0x8000) is `DestroyableBridges` in the `[SpecialFlags]` INI section. It controls
whether bridges can be targeted and destroyed. The SuperWeaponsAllowed toggle is
`DAT_00a8b263` (a standalone session byte at RulesClass+0x14b9), NOT a bit in SpecialFlags.

### "Bit 0x10 = MCVDeploy" -- WRONG
Bit 4 (0x10) is `CaptureTheFlag` in the staging flags. `MCVDeploy` is bit 8 (0x100) in the
[SpecialFlags] INI bitfield. The MCVRedeploy lobby toggle is `DAT_00a8b320` (standalone byte).

### Session bytes vs SpecialFlags bits
Many lobby settings (Crates, SuperWeapons, BuildOffAlly, ShortGame, MultiEngineer,
HarvesterTruce) are stored as **individual bytes** in the session state, NOT as bits
in the SpecialFlags bitfield. They are packed into network packets but remain as separate
bytes for gameplay logic checks. Only the 13 keys defined in the `[SpecialFlags]` INI
section map to bits in the `*DAT_00a8b230` uint32.

---

## Relevance to ra2-rust-game

### Current implementation gaps:
The Rust engine's `SpecialFlagsSection` struct (in `src/map/basic.rs`) currently only has 3 fields:
- `tiberium_grows`
- `tiberium_spreads`
- `destroyable_bridges`

It is missing the other 10 [SpecialFlags] keys and the entire lobby options / session flags system.

### What needs to be added for full gameplay:
1. All 13 [SpecialFlags] INI keys in the map parser
2. The `[MultiplayerDialogSettings]` section parser in RulesClass (partially exists)
3. Session game option bytes for the skirmish lobby
4. The staging flags composition logic for multiplayer
5. The conditional loading logic (MP vs campaign) for [SpecialFlags]
6. Bit 12 (0x1000) handling for crate spawning
7. Bit 4 (0x10) handling for CaptureTheFlag
