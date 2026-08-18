# WarheadTypeClass — Full Struct Layout

**Source:** Ghidra decompilation of `gamemd.exe` (Yuri's Revenge 1.001)
**Constructor:** `0x0075CEC0` (normal), `0x0075E0C0` (Load/deserialization)
**ReadINI:** `0x0075D3A0` (full body; `0x0075D590` is a mid-function label, not separate)
**Detonate:** `0x004690B0` (on BulletClass, not WarheadTypeClass — see WARHEAD_DETONATE report)
**FindOrAllocate:** `0x0075E3B0`
**Confidence:** HIGH — all offsets verified from ReadINI string xrefs and constructor defaults.

**Total struct size:** 0x1CC (460 bytes)

---

## Inheritance

```
AbstractClass (vtable + ID system)
  └── AbstractTypeClass (Name at +0x24, extends to ~+0x97)
        └── WarheadTypeClass (+0x98 onward = warhead-specific fields)
```

The first 0x98 bytes are inherited from AbstractTypeClass (4 vtable pointers at
+0x00/+0x04/+0x08/+0x0C, then AbstractType fields including the name string at +0x24).

---

## Complete Field Table

**param_1 type in ReadINI:** `int` (ECX/ESI = direct `this` pointer). All offsets below
are direct byte offsets — no multiplication needed.

### AbstractTypeClass base (0x00 — 0x97)

| Offset | Size | Type | Field | Notes |
|--------|------|------|-------|-------|
| 0x00   | 4    | ptr  | vtable (primary) | WarheadTypeClass vtable |
| 0x04   | 4    | ptr  | vtable (secondary 1) | IPersistStream |
| 0x08   | 4    | ptr  | vtable (secondary 2) | IRTTITypeInfo |
| 0x0C   | 4    | ptr  | vtable (secondary 3) | INoticeSink |
| 0x10   | 4    | int  | UniqueID | Assigned by AbstractClass |
| 0x24   | ~100 | char[] | Name | Section name string (e.g. "SA", "HE", "Super") |
| ...    | ...  | ...  | (other AbstractTypeClass fields) | Through 0x97 |

### Deform & Verses (0x98 — 0xFF)

| Offset | Size | Type | INI Key | Default | Notes |
|--------|------|------|---------|---------|-------|
| 0x98   | 8    | double | `Deform` | 0.0 | Ground deformation amount. Parsed as double via ReadDouble. |
| 0xA0   | 88   | double[11] | `Verses` | 1.0 each (=100%) | Armor type damage multipliers. 11 entries for 11 armor types. Parsed from comma-separated list; values with `%` suffix are divided by 100 (e.g. `80%` → 0.8), bare floats used directly. |
| 0xF8   | 8    | double | `ProneDamage` | 1.0 (=100%) | Damage multiplier for prone infantry. Parsed same as Verses. |

**Verses array indices (each 8-byte double):**

| Index | Offset | Armor Type |
|-------|--------|------------|
| 0     | 0xA0   | None |
| 1     | 0xA8   | Flak |
| 2     | 0xB0   | Plate |
| 3     | 0xB8   | Light |
| 4     | 0xC0   | Medium |
| 5     | 0xC8   | Heavy |
| 6     | 0xD0   | Wood |
| 7     | 0xD8   | Steel |
| 8     | 0xE0   | Concrete |
| 9     | 0xE8   | Special_1 |
| 10    | 0xF0   | Special_2 |

**Verses parsing (at 0x0075DE06):** The INI string is tokenized by `,`. For each token,
if it contains `%` (char 0x25), `atoi()` is called and the result multiplied by 0.01.
Otherwise `atof()` is called directly. The constant at `0x007E3808` = 0.01 (double).

### DeformThreshold & AnimList vector (0x100 — 0x11F)

| Offset | Size | Type | INI Key | Default | Notes |
|--------|------|------|---------|---------|-------|
| 0x100  | 4    | int  | `DeformThreshhold` | 0 | Damage threshold before terrain deforms. Note: INI key has double-h typo matching original. |
| 0x104  | 28   | DynamicVectorClass\<AnimTypeClass*\> | `AnimList` | empty | Impact animation types. Parsed as comma-separated list of anim names. |

**AnimList DynamicVectorClass layout at 0x104:**

| Offset | Field |
|--------|-------|
| 0x104  | vtable ptr (to 0x7EB6D4) |
| 0x108  | data ptr (AnimTypeClass** array) |
| 0x10C  | capacity |
| 0x110  | grow_flag (byte) + is_allocated (byte) + pad |
| 0x114  | count |
| 0x118  | growth_step (default=10) |
| 0x11C  | extra/tail field |

### InfDeath through CombatLightSize (0x120 — 0x13F)

| Offset | Size | Type | INI Key | Default | Notes |
|--------|------|------|---------|---------|-------|
| 0x120  | 4    | int   | `InfDeath` | 0 | Infantry death animation index (0-based). Controls which death anim plays. |
| 0x124  | 4    | float | `CellSpread` | 0.0 | Splash damage radius in cells. 0 = single target only. |
| 0x128  | 4    | float | `CellInset` | 0.0 | Unknown / undocumented. Read as float via ReadDouble→fstp float. |
| 0x12C  | 4    | float | `PercentAtMax` | 1.0 (=100%) | Damage percentage at max CellSpread distance. 1.0 = full damage everywhere within radius. |
| 0x130  | 1    | bool  | `CausesDelayKill` | false | Whether this warhead causes delayed kills. |
| 0x131  | 3    | —     | (padding) | — | |
| 0x134  | 4    | int   | `DelayKillFrames` | 5 | Number of frames to delay the kill. |
| 0x138  | 4    | float | `DelayKillAtMax` | 1.0 | Damage fraction at maximum delay range. |
| 0x13C  | 4    | float | `CombatLightSize` | 0.0 | Size of the combat light flash on impact. |

### Particle system pointer (0x140)

| Offset | Size | Type | INI Key | Default | Notes |
|--------|------|------|---------|---------|-------|
| 0x140  | 4    | ptr  | `Particle` | null (0) | ParticleSystemTypeClass* — read as string, looked up via FindByName (`0x00644890`). |

### Boolean flags block (0x144 — 0x15B)

All 1-byte booleans. Order matches the ReadINI call sequence.

| Offset | INI Key | Default | Notes |
|--------|---------|---------|-------|
| 0x144  | `Wall` | false | Damages walls. |
| 0x145  | `WallAbsoluteDestroyer` | false | Instantly destroys walls regardless of HP. |
| 0x146  | `PenetratesBunker` | false | Damage passes through to units garrisoned in buildings. |
| 0x147  | `Wood` | false | Damages wooden objects (trees, fences). |
| 0x148  | `Tiberium` | false | Damages tiberium/ore. TS legacy key name but active in YR. |
| 0x149  | (auto) OrganicImmune | false | **Auto-computed.** Set to `true` if `Verses[4]==0.0 && Verses[6]==0.0` (Medium and Wood armor both immune). Not an INI key — calculated at end of ReadINI. |
| 0x14A  | `Sparky` | false | Creates sparks on impact. |
| 0x14B  | `Sonic` | false | Sonic weapon (Dolphin). Special visual/audio effects. |
| 0x14C  | `Fire` | false | Fire-type weapon. Uses fire death animation. |
| 0x14D  | `Conventional` | false | Conventional weapon — does not trigger building rubble anim on kill. |
| 0x14E  | `Rocker` | false | Rocks/pushes vehicles hit by this warhead. |
| 0x14F  | `DirectRocker` | false | Like Rocker but pushes directly away from impact point. |
| 0x150  | `Bright` | false | Creates bright flash on impact. |
| 0x151  | `CLDisableRed` | false | Disable red channel in combat light. |
| 0x152  | `CLDisableGreen` | false | Disable green channel in combat light. |
| 0x153  | `CLDisableBlue` | false | Disable blue channel in combat light. |
| 0x154  | `EMEffect` | false | Electromagnetic pulse — disables mechanical units. |
| 0x155  | `MindControl` | false | Mind-controls the target. |
| 0x156  | `Poison` | false | Poisons infantry (toxic). |
| 0x157  | `IvanBomb` | false | Attaches a Crazy Ivan bomb to target. |
| 0x158  | `ElectricAssault` | false | Electric bolt weapon (Tesla). |
| 0x159  | `Parasite` | false | Parasitic weapon (Terror Drone). |
| 0x15A  | `Temporal` | false | Chrono-erases the target. |
| 0x15B  | `IsLocomotor` | false | This warhead changes the target's locomotor. |

### Locomotor CLSID (0x15C — 0x16B)

| Offset | Size | Type | INI Key | Default | Notes |
|--------|------|------|---------|---------|-------|
| 0x15C  | 16   | GUID | `Locomotor` | `{4A582747-9839-11d1-B709-00A024DDAFD1}` (TeleportLocomotion) | CLSID of the locomotor to apply when `IsLocomotor=true`. Read via `CCINIClass::ReadCLSID` at `0x00527920`. |

### Post-locomotor booleans and int (0x16C — 0x17B)

| Offset | Size | Type | INI Key | Default | Notes |
|--------|------|------|---------|---------|-------|
| 0x16C  | 1    | bool | `Airstrike` | false | Marks this as an airstrike warhead. |
| 0x16D  | 1    | bool | `Psychedelic` | false | Psychedelic effect (Yuri Prime mind blast visual). |
| 0x16E  | 1    | bool | `BombDisarm` | false | Disarms Ivan bombs on target. |
| 0x16F  | 1    | —    | (padding) | — | |
| 0x170  | 4    | int  | `Paralyzes` | 0 | Number of frames to paralyze (stun) the target. 0 = no paralysis. |
| 0x174  | 1    | bool | `Culling` | false | Unknown exact behavior. Possibly related to target selection/culling. |
| 0x175  | 1    | bool | `MakesDisguise` | false | Makes the target appear disguised (spy-related). |
| 0x176  | 1    | bool | `NukeMaker` | false | Triggers a nuclear explosion effect. |
| 0x177  | 1    | bool | `Radiation` | false | Applies radiation to the impact area. |
| 0x178  | 1    | bool | `PsychicDamage` | false | Deals psychic damage (bypasses some defenses). |
| 0x179  | 1    | bool | `AffectsAllies` | true | Whether this warhead damages allied units. **Default true** — set to 1 in constructor. |
| 0x17A  | 1    | bool | `Bullets` | false | Visual: uses bullet-style impact (SA-type warheads). |
| 0x17B  | 1    | bool | `Veinhole` | false | Damages veinholes. TS legacy but key exists. |

### Screen shake (0x17C — 0x18B)

| Offset | Size | Type | INI Key | Default | Notes |
|--------|------|------|---------|---------|-------|
| 0x17C  | 4    | int  | `ShakeXlo` | 0 | Minimum horizontal screen shake pixels. |
| 0x180  | 4    | int  | `ShakeXhi` | 0 | Maximum horizontal screen shake pixels. |
| 0x184  | 4    | int  | `ShakeYlo` | 0 | Minimum vertical screen shake pixels. |
| 0x188  | 4    | int  | `ShakeYhi` | 0 | Maximum vertical screen shake pixels. |

### DebrisTypes vector (0x18C — 0x1A7)

| Offset | Size | Type | INI Key | Default | Notes |
|--------|------|------|---------|---------|-------|
| 0x18C  | 28   | DynamicVectorClass\<VoxelAnimTypeClass*\> | `DebrisTypes` | empty | Voxel debris animation types spawned on impact. Parsed as comma-separated list of VoxelAnimType names (lookup via `0x0074B960`). |

**DebrisTypes DynamicVectorClass layout:**

| Offset | Field |
|--------|-------|
| 0x18C  | vtable ptr (to 0x7F0D3C) |
| 0x190  | data ptr (VoxelAnimTypeClass** array) |
| 0x194  | capacity |
| 0x198  | grow_flag (byte) |
| 0x199  | is_allocated (byte) |
| 0x19A  | pad (2 bytes) |
| 0x19C  | count |
| 0x1A0  | growth_step |
| 0x1A4  | extra/tail |

### DebrisMaximums vector (0x1A8 — 0x1C3)

| Offset | Size | Type | INI Key | Default | Notes |
|--------|------|------|---------|---------|-------|
| 0x1A8  | 28   | DynamicVectorClass\<int\> | `DebrisMaximums` | empty | Maximum count per debris type. Parsed as comma-separated list of integers. Paired with DebrisTypes — index i gives max count for debris type i. |

**DebrisMaximums DynamicVectorClass layout:**

| Offset | Field |
|--------|-------|
| 0x1A8  | vtable ptr (to 0x7E4DD8) |
| 0x1AC  | data ptr (int* array) |
| 0x1B0  | capacity |
| 0x1B4  | grow_flag (byte) |
| 0x1B5  | is_allocated (byte) |
| 0x1B6  | pad (2 bytes) |
| 0x1B8  | count |
| 0x1BC  | growth_step |
| 0x1C0  | extra/tail |

### MaxDebris / MinDebris (0x1C4 — 0x1CB)

| Offset | Size | Type | INI Key | Default | Notes |
|--------|------|------|---------|---------|-------|
| 0x1C4  | 4    | int  | `MaxDebris` | 0 | Maximum total debris pieces spawned. Clamped: if MaxDebris < MinDebris, MaxDebris = MinDebris. |
| 0x1C8  | 4    | int  | `MinDebris` | 0 | Minimum debris pieces spawned. Clamped >= 0. |

**Total struct size: 0x1CC (460 bytes)**

---

## INI Key Summary (alphabetical)

All 43 INI keys read by `WarheadTypeClass::ReadINI` at `0x0075D3A0`:

| INI Key | Offset | Type | Default |
|---------|--------|------|---------|
| `AffectsAllies` | 0x179 | bool | true |
| `Airstrike` | 0x16C | bool | false |
| `AnimList` | 0x104 | DynVec\<AnimType*\> | empty |
| `BombDisarm` | 0x16E | bool | false |
| `Bright` | 0x150 | bool | false |
| `Bullets` | 0x17A | bool | false |
| `CausesDelayKill` | 0x130 | bool | false |
| `CellInset` | 0x128 | float | 0.0 |
| `CellSpread` | 0x124 | float | 0.0 |
| `CLDisableBlue` | 0x153 | bool | false |
| `CLDisableGreen` | 0x152 | bool | false |
| `CLDisableRed` | 0x151 | bool | false |
| `CombatLightSize` | 0x13C | float | 0.0 |
| `Conventional` | 0x14D | bool | false |
| `Culling` | 0x174 | bool | false |
| `DebrisMaximums` | 0x1A8 | DynVec\<int\> | empty |
| `DebrisTypes` | 0x18C | DynVec\<VoxelAnimType*\> | empty |
| `Deform` | 0x98 | double | 0.0 |
| `DeformThreshhold` | 0x100 | int | 0 |
| `DelayKillAtMax` | 0x138 | float | 1.0 |
| `DelayKillFrames` | 0x134 | int | 5 |
| `ElectricAssault` | 0x158 | bool | false |
| `EMEffect` | 0x154 | bool | false |
| `Fire` | 0x14C | bool | false |
| `InfDeath` | 0x120 | int | 0 |
| `IsLocomotor` | 0x15B | bool | false |
| `IvanBomb` | 0x157 | bool | false |
| `Locomotor` | 0x15C | GUID (16 bytes) | TeleportLocomotion CLSID |
| `MakesDisguise` | 0x175 | bool | false |
| `MaxDebris` | 0x1C4 | int | 0 |
| `MinDebris` | 0x1C8 | int | 0 |
| `MindControl` | 0x155 | bool | false |
| `NukeMaker` | 0x176 | bool | false |
| `Paralyzes` | 0x170 | int | 0 |
| `Parasite` | 0x159 | bool | false |
| `Particle` | 0x140 | ptr (ParticleSysType*) | null |
| `PenetratesBunker` | 0x146 | bool | false |
| `PercentAtMax` | 0x12C | float | 1.0 |
| `Poison` | 0x156 | bool | false |
| `ProneDamage` | 0xF8 | double | 1.0 |
| `Psychedelic` | 0x16D | bool | false |
| `PsychicDamage` | 0x178 | bool | false |
| `Radiation` | 0x177 | bool | false |
| `Rocker` | 0x14E | bool | false |
| `DirectRocker` | 0x14F | bool | false |
| `ShakeXhi` | 0x180 | int | 0 |
| `ShakeXlo` | 0x17C | int | 0 |
| `ShakeYhi` | 0x188 | int | 0 |
| `ShakeYlo` | 0x184 | int | 0 |
| `Sonic` | 0x14B | bool | false |
| `Sparky` | 0x14A | bool | false |
| `Temporal` | 0x15A | bool | false |
| `Tiberium` | 0x148 | bool | false |
| `Veinhole` | 0x17B | bool | false |
| `Verses` | 0xA0 | double[11] | 1.0 each |
| `Wall` | 0x144 | bool | false |
| `WallAbsoluteDestroyer` | 0x145 | bool | false |
| `Wood` | 0x147 | bool | false |

**Auto-computed field (not an INI key):**

| Field | Offset | Logic |
|-------|--------|-------|
| OrganicImmune | 0x149 | Set to `true` if `Verses[4]==0.0` (Medium) AND `Verses[6]==0.0` (Wood). Otherwise `false`. Computed at end of ReadINI after Verses parsing. |

---

## Methods Found

| Address | Name | Notes |
|---------|------|-------|
| 0x0075CEC0 | `WarheadTypeClass::Constructor` | Normal constructor. Initializes all fields to defaults. |
| 0x0075D3A0 | `WarheadTypeClass::ReadINI_Body` | Full ReadINI implementation. Reads all ~55 INI keys. __fastcall, this=ECX. |
| 0x0075D590 | `WarheadTypeClass::ReadINI` | **Mid-function label** inside ReadINI_Body, NOT a separate function. Ghidra created a function boundary here but it's actually continuous code from 0x0075D3A0. |
| 0x0075E0C0 | `WarheadTypeClass::Constructor` (Load) | Deserialization constructor for save/load. |
| 0x0075E3B0 | `WarheadTypeClass::FindOrAllocate` | Finds existing warhead by name or creates a new one. |
| 0x004690B0 | `WarheadTypeClass::Detonate` | Actually BulletClass::Detonate — see separate WARHEAD_DETONATE report. |

---

## TS Legacy / YR-Specific Notes

- **`Tiberium`** (0x148): Key name is TS legacy ("Tiberium" = ore in TS). In YR it controls
  whether the warhead damages ore/gems. Active in YR — not dormant.

- **`Veinhole`** (0x17B): TS had veinholes as terrain features. YR has no veinholes in
  standard maps but the key is still parsed and stored. Dormant in practice.

- **`Sparky`** (0x14A): Creates spark visual effects. Used by some YR units (e.g. electric weapons).

- **`Culling`** (0x174): Unclear exact behavior in YR. May relate to target filtering.
  Needs further investigation in Detonate or area damage code.

- **`CellInset`** (0x128): Read from INI but actual usage in area damage code is unclear.
  No YR warheads set this key in rulesmd.ini.

- **`CausesDelayKill` / `DelayKillFrames` / `DelayKillAtMax`** (0x130-0x138): Delay kill
  system. No standard YR warheads use these keys. Likely TS legacy or unused feature.

---

## Corrections to Existing Detonate Report

The WARHEAD_DETONATE_GHIDRA_REPORT.md had some defaults listed incorrectly:

1. **ProneDamage default:** Listed as 0.0, actually **1.0** (constructor sets 0xF8 to double 1.0).
2. **PercentAtMax default:** Listed as 0.0, actually **1.0** (constructor sets 0x12C to float 1.0).
3. **OrganicImmune auto-set condition:** Listed as "Verses[2]==0 && Verses[4]==0", actually
   checks **Verses[4] (offset 0xC0, Medium armor) and Verses[6] (offset 0xD0, Wood armor)**.
4. **Deform type:** Listed as double with default 1.0, actually default is **0.0** (constructor
   sets offset 0x98 to zero).
5. **DelayKillFrames default:** Not listed; is **5** (not 0).
6. **DelayKillAtMax default:** Not listed; is **1.0** (not 0).

---

## Verified 2026-04-19

Re-checked the **boolean flag block (0x144 – 0x15B)** against two independent
sources — the doc's claims all hold.

- **Angle 1 — `WarheadTypeClass::ReadINI_Body` at `0x0075D3A0`:** every
  `CCINIClass::ReadBool` call pairs the documented INI key string with the
  documented struct offset. Order in the decomp does not match offset order
  (e.g. the compiler emitted 0x14D `Conventional` before 0x14B `Sonic`), but
  each `(key, offset)` pairing matches this doc.
- **Angle 2 — `WarheadTypeClass::Detonate` at `0x004690B0`:** the dispatcher's
  nested if-else reads `+0x155` (MindControl), `+0x157` (IvanBomb), `+0x158`
  (ElectricAssault), `+0x159` (Parasite), `+0x15A` (Temporal), `+0x15B`
  (IsLocomotor) exactly as documented, and each branch selects the matching
  effect handler. This confirms not only the offsets but the semantics.

Spot-confirmed in this pass: `+0x14B = Sonic` is read in
`FootClass::ReceiveDamage` (`0x004D735F`) as the "warhead-forces-parasite-off"
trigger — this matches the in-game Dolphin-vs-Giant-Squid interaction.

No corrections to the flag block needed.
