---
name: WeaponTypeClass — 2026-04-24 Verification & Consumers
description: Re-verifies the 2026-04-06 WeaponTypeClass full struct layout against the binary, clarifies DVC internals, and documents consumer-site offset usage.
type: research
date: 2026-04-24
binary: gamemd.exe (Yuri's Revenge 1.001)
---

# WeaponTypeClass — Verification & Consumers (Ghidra Report)

**Primary address(es):**
- `0x00771c70` — `WeaponTypeClass::Constructor`
- `0x00771f00` — `WeaponTypeClass::Constructor` (copy/minimal variant, body `00771f00`–`00771f4f`)
- `0x00771f50` — `WeaponTypeClass::~Destructor` (**misnamed "ReadINI_part1"** in the Ghidra project)
- `0x00772080` — `WeaponTypeClass::ReadINI`
- `0x00772fa0` — `WeaponTypeClass::FindOrAllocate`
- `0x00773030` — `WeaponTypeClass::FindByName`

**Consumer sites verified:**
- `0x006fdd50` — `TechnoClass::Fire_At` (reads 20+ distinct weapon offsets; used as ground-truth cross-check)

**Struct size:** `0x160` bytes (352) — confirmed directly from `operator_new(0x160)` inside `FindOrAllocate` at `0x00772fa0`.

**Overall confidence:** HIGH — all field offsets, defaults, INI keys, and parsing order independently re-verified by decompiling Constructor (`0x00771c70`), ReadINI (`0x00772080`), Destructor (`0x00771f50`), FindOrAllocate (`0x00772fa0`), and cross-referenced against reads in `TechnoClass::Fire_At` (`0x006fdd50`).

**Active in YR:** Yes. All 63 INI keys parsed by `ReadINI` are active (none gated behind `SpecialFlags` or TS-only conditions). Standard YR rulesmd.ini uses 40+ of them; ~20 are modder-available holdovers that retail YR weapons don't set.

---

## 1. Relationship to Prior Report

The existing **[WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md](../../ra2-rust-game-docs/WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md)** (dated 2026-04-06) is the authoritative struct-layout reference. This report does NOT duplicate that layout.

What this report adds:

1. **Re-verified** all 63 INI keys and offsets in the 2026-04-06 doc are still correct against the live binary (nothing has moved).
2. **Clarifies DVC internal field semantics** (ActiveCount / GrowthStep, previously labeled ambiguously as "Sound list data field 1..3").
3. **Documents consumer-site usage** — every offset that `TechnoClass::Fire_At` reads, as a second source of truth.
4. **Clarifies the "ReadINI_part1" function** — it is actually the destructor, not a ReadINI helper.
5. **Documents the `AttachedParticleSystem` indirect lookup** via `DAT_00a83d6c`.
6. **Confirms the "Supress" typo** is in the binary (there is no "Suppress" string).

No contradictions or corrections to the 2026-04-06 struct table.

---

## 2. Constructor Defaults — Re-verified

All constructor writes at `0x00771c70` match the 2026-04-06 doc. Selected confirmations:

| Byte offset | Default | Evidence (Constructor) |
|-------------|---------|------------------------|
| `0x9C` Burst | `1` | `param_1[0x27] = 1` |
| `0x133` DecloakToFire | **`true`** | `*(uint8*)(param_1 + 0x133) = 1` |
| `0x137` RevealOnFire | **`true`** | `*(uint8*)(param_1 + 0x137) = 1` |
| `0x141` FireWhileMoving | **`true`** | `*(uint8*)(param_1 + 0x141) = 1` |
| `0x143` FireInTransport | **`true`** | `*(uint8*)(param_1 + 0x143) = 1` |
| `0x14E` LaserDuration | `10` | `*(uint8*)(param_1 + 0x14e) = 10` |
| All other bytes | `0` | Explicit zero-writes throughout ctor |
| `0x156–0x157` | padding | Not initialized (alignment gap before `RadLevel` int at `0x158`) |
| `0x15D–0x15F` | padding | Not initialized (tail alignment to `0x160`) |

`param_1` is typed `undefined4 *` (i.e., `int *`): array accesses like `param_1[0x27]` are **byte_offset = index × 4**; casts like `*(uint8*)((int)param_1 + 0x137)` are **direct byte offsets**.

---

## 3. ReadINI Ordering — Re-verified

Every string xref in `ReadINI` (`0x00772080`) matches the 2026-04-06 parse-order table, in the same order. Selected re-verified string literals in `.rdata`:

| INI key | Rodata addr | Stores to offset |
|---------|-------------|------------------|
| `AmbientDamage` | `0x849548` | `this + 0x98` |
| `IsSonic` | `0x849540` | `this + 0x130` |
| `DecloakToFire` | `0x84951c` | `this + 0x133` |
| `RevealOnFire` | `0x8494e0` | `this + 0x137` |
| `FireWhileMoving` | `0x84947c` | `this + 0x141` |
| `DisguiseFakeBlinkTime` | `0x849448` | `this + 0x13c` |
| `Supress` | `0x849440` | `this + 0x146` |
| `Burst` | `0x849438` | `this + 0x9c` |
| `MinimumRange` | `0x849428` | `this + 0xb8` |
| `IsMagBeam` | `0x84928c` | `this + 0x15c` |
| `AttachedParticleSystem` | `0x849274` | `this + 0x11c` (via index lookup, see §6) |

**Typo note:** The string at `0x849440` is **`Supress`** (single-P). A byte-scan confirms no `Suppress` string exists in the binary. Mods and our Rust parser must key on the misspelled name.

`Damage`, `Speed`, `ROF`, `Range`, `Warhead`, `Projectile`, and `Report`/`DownReport` keys all share string literals with other classes (not unique to `WeaponTypeClass`).

**Special parse paths:**
- `Speed` → `CCINIClass::ReadSpeed` (converts 0–100 → 0–255)
- `Range`, `MinimumRange` → `CCINIClass::ReadRange` (cells → leptons, ×256)
- `LaserInnerColor`/`LaserOuterColor`/`LaserOuterSpread` → `CCINIClass::ReadColorRGB` (returns pointer to 3-byte RGB triplet; stored as packed bytes starting at `0x120`/`0x123`/`0x126`)
- `Report`, `DownReport` → `CCINIClass::ReadSoundList` (fills a `DynamicVectorClass<int>` of sound indices)
- `Anim` → manual `CRT::strtok(",")` loop, each token resolved via `AnimTypeClass::FindByName` and appended to the `DynamicVectorClass<AnimTypeClass*>` at `this + 0xF4`
- `Warhead` → `WarheadTypeClass::FindOrAllocate` (creates if missing)
- `Projectile` → `BulletTypeClass::FindOrAllocate` at `FUN_0046c790`
- `AttachedParticleSystem` → See §6

---

## 4. DVC Layout (28 bytes) — Clarified

The three `DynamicVectorClass` members (Report at `0xBC`, DownReport at `0xD8`, Anim at `0xF4`) are each 28 bytes (`0x1C`). The 2026-04-06 doc labels the internal dwords as "Sound list data field 1..3", which is misleading. The live DVC layout, confirmed from the constructor (init) + destructor (teardown) + `CopyFrom` pattern in ReadINI:

| DVC inner offset | Field | Init value | Evidence |
|-----------------|-------|------------|----------|
| `+0x00` | vtable | `&PTR_FUN_007e4dd8` (sound list) or `&PTR_FUN_007eb6d4` (AnimTypeClass*) | Constructor writes typed vtable after generic DVC ctor |
| `+0x04` | `Buffer` (element pointer) | `NULL` | Zeroed |
| `+0x08` | `VectorMax` (capacity) | `0` | Zeroed (grows via `GrowthStep`) |
| `+0x0C` (byte) | — | `0` | Unknown single byte (often zero) |
| `+0x0D` (byte) | `IsAllocated` | `0` | Checked in dtor: `if (Buffer != 0 && IsAllocated) free(Buffer)` |
| `+0x0E..0x0F` | padding | — | Alignment |
| `+0x10` | `ActiveCount` | `0` | Constructor: `param_1[0x33]/[0x3a]/[0x41] = 0` |
| `+0x14` | `GrowthStep` | `10` | Constructor: `param_1[0x34]/[0x3b]/[0x42] = 10` |
| `+0x18` | unknown (likely `ZoneVector` extra / current-index cursor) | `0` | Copied from `ReadSoundList` return in ReadINI |

Applied to the three vectors:

| Vector | vtable | Buffer | VectorMax | IsAllocated | ActiveCount | GrowthStep | Extra |
|--------|--------|--------|-----------|-------------|-------------|------------|-------|
| Report | `0xBC` | `0xC0` | `0xC4` | `0xC9` | `0xCC` | `0xD0` | `0xD4` |
| DownReport | `0xD8` | `0xDC` | `0xE0` | `0xE5` | `0xE8` | `0xEC` | `0xF0` |
| Anim | `0xF4` | `0xF8` | `0xFC` | `0x101` | `0x104` | `0x108` | `0x10C` |

**Consumer evidence for these offsets** (from `Fire_At`):
- Report sound play: `if (0 < *(int*)(weapon + 0xCC)) VocClass::PlayAt(...)` — confirms `0xCC` is ActiveCount.
- Anim directional select: `if (*(int*)(weapon + 0x104) == 8) idx = ((facing >> 0xC + 1) >> 1) & 7; else idx = 0;` then `*(int**)(weapon + 0xF8)[idx]` — confirms `0x104` is ActiveCount and `0xF8` is the Buffer pointer of the Anim DVC. The 8-anim fast path selects by facing; otherwise falls back to the first entry.

---

## 5. Destructor at `0x00771f50` — Misnamed

The function Ghidra labels `WeaponTypeClass__ReadINI_part1` is **actually the destructor** (the 2026-04-06 doc correctly flags this). Observed sequence at `0x00771f50`:

1. Re-writes the four vtables (standard MSVC destructor-vtable-restore idiom).
2. Calls `FUN_007258d0` (base `AbstractTypeClass` partial teardown).
3. Zeros `Projectile` (`0xA0`) and `Warhead` (`0xAC`) pointers (ownership transferred; does not free).
4. Removes self from the global `WeaponTypeClass` array at `DAT_0088756C` (linear O(n) removal + memmove shift).
5. Tears down the three DVCs **in reverse order**: Anim → DownReport → Report. For each: restore vtable to the generic DVC vtable (`&PTR_FUN_007eb6f4` / `&PTR_FUN_007e4db8`), then `if (Buffer != 0 && IsAllocated) operator_delete(Buffer)`; zero Buffer and capacity.
6. Tail-calls what Ghidra labels `AbstractTypeClass__Constructor` — this is almost certainly the base destructor mislabeled (RTTI-labeler limitation).

The naming is a Ghidra project label, not a binary-level fact. Do NOT implement a "ReadINI_part1" — it does not exist.

---

## 6. `AttachedParticleSystem` — Indirect Lookup

ReadINI resolves `AttachedParticleSystem=` differently from `Warhead=` and `Projectile=` (which use `FindOrAllocate` returning a pointer). Instead:

```
char name[20] = "";
CCINIClass::ReadString(section, "AttachedParticleSystem", "", name, 20);
iVar3 = FUN_00644630(name);   // ParticleSystemTypeClass::FindIndexByName, returns -1 if missing
if (iVar3 != -1) {
    *(int*)(this + 0x11C) = *(int*)(DAT_00a83d6c + iVar3 * 4);
}
```

- `FUN_00644630` is the ParticleSystemTypeClass name-to-index lookup (returns `-1` for unknown). Xrefs confirm it is invoked from `ParticleSystemTypeClass::Constructor` and a handful of other particle-system call sites.
- `DAT_00a83d6c` is the `ParticleSystemTypeClass*` array base (the global `VectorClass` buffer).
- If the INI value is empty or the lookup returns `-1`, the field retains its previous value (the ctor default `NULL`), **NOT explicitly cleared**. This means re-reading a weapon section without `AttachedParticleSystem=` does not reset it.

This lookup pattern is unique among the WeaponType pointer fields.

---

## 7. Consumer Offset Map — `TechnoClass::Fire_At` (`0x006fdd50`)

All offsets the main firing path reads from the weapon, in execution order. This is a **second source of truth** for the layout — every field here agrees with the 2026-04-06 doc.

| Offset | Field | Role in `Fire_At` |
|--------|-------|-------------------|
| `0x9C` | Burst | `CurrentBurstIndex % weapon->Burst` (wrap), and `(8 / Burst)` for drift calc |
| `0xA0` | Projectile | Passed to `BulletClass::Allocate` and `BulletClass::Init` |
| `0xA4` | Damage | Passed to bullet via `uStack_a4 = weapon->Damage` after veterancy scaling |
| `0xA8` | Speed | Stored on bullet: `bullet[0x44] = weapon->Speed` when projectile uses driver |
| `0xAC` | Warhead | Passed to `BulletClass::Allocate` / `Init` |
| `0xCC` | Report.ActiveCount | Gates `VocClass::PlayAt` call (only if > 0) |
| `0xF8` | Anim.Buffer | Indexed impact anim selection |
| `0x104` | Anim.ActiveCount | `== 8` triggers directional (per-facing) anim lookup; `> 0` falls back to first entry |
| `0x110` | OccupantAnim | Used when shooter is a garrison occupant |
| `0x118` | OpenToppedAnim | Used when shooter fires from open-topped transport (veteran check precedes) |
| `0x11C` | AttachedParticleSystem | `ParticleSystemClass::Constructor(weapon->AttachedParticleSystem, ...)` when `UseFireParticles`/`UseSparkParticles`/`IsRailgun` triggers are set |
| `0x129` | UseFireParticles | If set and particle already spawned, early-returns; otherwise spawns fire particle |
| `0x12A` | UseSparkParticles | Mirror of `UseFireParticles` for spark particles |
| `0x12D` | IsRailgun | Mirror for railgun particle spawn |
| `0x12F` | Bright | Passed as 5th arg to `BulletClass::Allocate`/`Init` — controls bullet bright-illumination flag |
| `0x130` | IsSonic | Early-return if shooter already has an active `Wave`; also spawns Wave (type 0) |
| `0x131` | Spawner | Takes the SpawnManager path: sets target on SpawnManagerClass and returns early (no direct bullet spawn) |
| `0x132` | LimboLaunch | Post-fire: if set, triggers limbo-launch side effect on self + target |
| `0x135` | FireOnce | Post-fire cleanup: invokes vtable+0x3c8 (unlimbo/destroy-self path) |
| `0x137` | RevealOnFire | Calls `MapClass::RevealShroud` + `UpdateFogBorder` around firing position (human-only) |
| `0x142` | DrainWeapon | Early-return: diverts into `BuildingClass::EnterTransport` (drain mechanic) and skips bullet spawn |
| `0x144` | Suicide | Early-return: applies Rules.SuicideWeaponDamage to self and stops |
| `0x149` | IsLaser | Spawns laser visual via `TechnoClass::SpawnLaser` |
| `0x14A` | DiskLaser | Spawns `DiskLaserClass` (size `0x40`), increments `CurrentBurstIndex`, registers with `BulletAnimTracker`, returns early (no bullet) |
| `0x14D` | IsHouseColor | When laser spawned, sets laser color code to 2 (house color) |
| `0x150` | AreaFire | Takes cell-center target instead of object target |
| `0x151` | IsElectricBolt | Spawns electric bolt effect (not a laser); gated exclusive-else after `IsLaser` |
| `0x154` | IsRadBeam | Spawns rad beam; uses warhead byte at `warhead + 0x15A` (`Temporal`-adjacent flag) to select beam-type |
| `0x155` | IsRadEruption | Spawns rad eruption effect |
| `0x15C` | IsMagBeam | Spawns Wave (type 3, magnetron) if no existing Wave and target is not a cell |

**Observation:** The laser/bolt/rad family forms an **exclusive if-else chain** (see `Fire_At` ~line `0x006ff1a0`): `IsLaser` → `IsElectricBolt` → `IsRadBeam` → `IsRadEruption` → `IsMagBeam`. Only one visual fires per shot; the first match short-circuits the rest. Our Rust impl should mirror this exclusivity.

---

## 8. INI-Key Coverage Summary (Retail YR `rulesmd.ini`)

Verified by grep against `ini/rulesmd.ini` (counts are distinct weapon-section uses):

- **Hot path (>100 uses):** `Damage`, `ROF`, `Range`, `Speed`, `Projectile`, `Warhead`, `Report`
- **Common (10–100 uses):** `Burst`, `Anim`, `Bright`, `MinimumRange`, `CellRangefinding`, `OmniFire`, `FireOnce`, `AssaultAnim`, `OccupantAnim`, `FireInTransport`, `IsElectricBolt`, `IsLaser`
- **Occasional (1–9 uses):** `DecloakToFire`, `IsRadBeam`, `AreaFire`, `Spawner`, `RadLevel`, `Lobber`, `LimboLaunch`, `AmbientDamage`, `TurboBoost`, `Suicide`, `OpenToppedAnim`, `AttachedParticleSystem`, `IsMagBeam`, `SabotageCursor`, `IsSonic`, `IsBigLaser`, `IsAlternateColor`, `DiskLaser`, `DisguiseFireOnly`, `DisguiseFakeBlinkTime`, `Charges`, `UseSparkParticles`, `UseFireParticles`, `TerrainFire`, `NeverUse`, `MigAttackCursor`, `LaserOuterColor`, `LaserInnerColor`, `LaserDuration`, `IsRailgun`, `IsRadEruption`, `InfiniteMindControl`, `FireWhileMoving`, `DrainWeapon`, `LaserOuterSpread`, `IsHouseColor`
- **Never set in retail YR (parsed but modder-only):** `DistributedWeaponFire`, `DrawBoltAsLaser`, `IsLine`, `IonSensitive`, `DownReport`, `RevealOnFire` (rarely, defaults true), `Supress` (typo), `Camera`

**No additional keys** beyond the 63 in `ReadINI` appear in retail weapon sections — the parser is complete.

---

## 9. Current Rust Implementation Status

The struct `WeaponType` at [src/rules/weapon_type.rs:34](../src/rules/weapon_type.rs#L34) represents one weapon. Parsing at [src/rules/weapon_type.rs:180-276](../src/rules/weapon_type.rs#L180) reads all 63 INI keys. All 39 boolean flags, 9 numeric fields, 3 RGB triplets, and the list/pointer fields are already represented.

Consumers:
- [src/sim/combat/combat_weapon.rs](../src/sim/combat/combat_weapon.rs) — weapon selection (Primary/Secondary/IFV)
- [src/sim/combat/mod.rs:767-1204](../src/sim/combat/mod.rs#L767) — firing loop using `burst`, `rof`, `speed`
- [src/sim/combat/mod.rs:320-335](../src/sim/combat/mod.rs#L320) — death weapon AoE fallback

**Likely gaps vs. `Fire_At` binary behavior (out-of-scope for this report — implementation-side):**
- The exclusive visual-effect chain (`IsLaser` → `IsElectricBolt` → `IsRadBeam` → `IsRadEruption` → `IsMagBeam`) — needs verification in Rust render path.
- `AttachedParticleSystem` spawn-on-fire wired to `UseFireParticles`/`UseSparkParticles`/`IsRailgun` (triple-gate, and each has a per-weapon single-instance guard `this->field_0x304/0x308/0x314`).
- `RevealOnFire` post-fire `RevealShroud` + `UpdateFogBorder` (human-only).
- `LimboLaunch`, `FireOnce`, `Suicide` post-fire lifecycle hooks.
- Drain weapon path (`DrainWeapon=yes` diverts into `BuildingClass::EnterTransport`).
- Anim DVC 8-way directional facing selection.

These are observations only — no implementation is proposed here (per skill gate).

---

## 10. Open Questions

1. **DVC `+0x18` (extra field)** — The fourth copied dword from `ReadSoundList`'s return struct (Report `0xD4`, DownReport `0xF0`, Anim `0x10C`). Suspected cursor or ZoneVector-style reserved slot. Not read by `Fire_At`. No other consumer traced yet.
2. **`Bright` flag downstream** — Passed as the 5th arg to `BulletClass::Allocate` and `BulletClass::Init`. Need to trace what those calls do with it (likely sets a bullet-side bit that gates `IlluminationFlash` on impact), but that's inside `BulletClass` and out of scope for this report.
3. **`Camera=yes` consumer** — Not read by `Fire_At`. Field is parsed but the consumer was not traced in this pass. Suspected to live in warhead-detonate or map-reveal code (camera superweapon-like effect per MODEnc). Worth investigating if a `Camera=yes` weapon is implemented.
4. **Constructor variant at `0x00771f00`** — A second tiny constructor (body 0x50 bytes) exists. Not decompiled in this pass. Likely a minimal "allocate without list registration" path or a copy constructor.

---

## Sources

- **Ghidra functions decompiled** (verified in this pass):
  - `0x00771c70` `WeaponTypeClass::Constructor`
  - `0x00771f50` `WeaponTypeClass::~Destructor` (mislabeled `ReadINI_part1`)
  - `0x00772080` `WeaponTypeClass::ReadINI`
  - `0x00772fa0` `WeaponTypeClass::FindOrAllocate`
  - `0x006fdd50` `TechnoClass::Fire_At` (consumer cross-check)
- **Ghidra data:**
  - `0x0088756C` / `0x00887570` / `0x00887578` — global WeaponType array buffer/count/capacity
  - `0x00a83d6c` — ParticleSystemTypeClass global array base
  - `0x00817474`, `0x00817694` — reserved names blacklisted in FindOrAllocate
  - `0x00849440` — `"Supress"` string literal (typo confirmed; no `Suppress` exists)
- **Prior reports cross-checked (no contradictions found):**
  - `ra2-rust-game-docs/WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md` (2026-04-06)
  - `ra2-rust-game-docs/READINI_FIELD_MAPS.md`
  - `ra2-rust-game-docs/BURST_WEAPON_FIRING_GHIDRA_REPORT.md`
  - `ra2-rust-game-docs/FIRE_AT_PIPELINE_GHIDRA_REPORT.md`
  - `ra2-rust-game-docs/FIRE_AT_ANALYSIS.md`
  - `ra2-rust-game-docs/TECHNOCLASS_COMBAT_WEAPON_SYSTEMS_REPORT.md`
  - `ra2-rust-game-docs/GATTLING_WEAPON_STAGE_SYSTEM_GHIDRA_REPORT.md`
  - `ra2-rust-game-docs/MAGNETRON_SYSTEM_GHIDRA_REPORT.md`
- **INI files checked:**
  - `ini/rulesmd.ini` (weapon sections, retail YR 1.001)
  - `ini/rules.ini` (base RA2)
  - `ini/artmd.ini` (no weapon-specific keys beyond animation refs)
