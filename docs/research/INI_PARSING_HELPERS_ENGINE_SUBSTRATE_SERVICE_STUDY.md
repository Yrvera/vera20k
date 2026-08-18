# INI Parsing Helpers (CCINIClass / INIClass) — Engine Substrate Service Study & Replacement-Boundary Design

**Status:** STUDY + DESIGN (not an approved implementation plan). Read-only research; no Rust written.
**Date:** 2026-06-04 (Pass 2 verify-and-expand 2026-06-04)
**Rule:** Rust-native structure, gamemd-native semantics.
**Confidence posture / provenance:** **PASS 2 re-decompiled the ENTIRE accessor family live this session** — every accessor previously tagged DOC-ONLY has now been read out of Ghidra and is promoted to VERIFIED (see §9 ledger + §Pass-2). Live-verified this run: `ReadInt 0x005276D0`, `ReadBool 0x005295F0`, `ReadDouble 0x005283D0`, `ReadString 0x00528A10`, the enum-by-name helper `FUN_00474DA0`, `ReadMovementZone 0x00474E40`, `ReadAction 0x00474EE0`, `ReadCLSID 0x00527920`, `Read3Int 0x00529CA0`, `ReadMinMax 0x00529880`, `ReadPoint/Size 0x00529A30`, `ReadRect 0x00527F20`, `ReadLayer 0x00477050`, `ReadSpeedType 0x00476FC0`, `ReadSoundList 0x00525430`, `RulesClass::ReadGeneral 0x0066D530`, `strtrim 0x00727CF0`, the case-insensitive compare `FUN_007c8d20`, plus a **newly-found** family member `ReadColorRGB 0x00474B50`. All format-string + constant bytes re-read from memory this run. The remaining DOC-ONLY items are only the INIClass field offsets (§2b) and ctor/dtor — none of which are part of the load-bearing parse contract. Per CLAUDE.md the default verdict for any unproven parse difference is **DRIFT** — there is no internal-only escape hatch for a parse rule, because a wrong percent/hex/bool parse silently shifts every dependent stat to the last decimal. This is a **load-time substrate** (rules/assets layer), not a tick system; it has no RNG and no per-tick timer, so the "RNG/timer visibility" axis collapses to "the parsed value is identical bit-for-bit." Several boundary behaviors of the current Rust parser are flagged **DRIFT**; the `ReadDouble`-percent precision (S0) remains the one **UNCHECKED / blocking-gate** for the Rust f32→×0.01→SimFixed conversion (the binary arithmetic itself is now fully pinned — see Pass 2 + shared-gate fold-in below).

**Companion:** the in-flight engine-substrate program. Master TODO: `docs/plans/2026-05-29-core-engine-substrate-todo.md`. The INI accessor service is the **load-time data substrate** that feeds every other substrate (map/cell, object lifecycle, combat/projectile, factory/house). It slots in *below* `sim/` in the existing `rules/` layer; it does **not** invent a parallel architecture and it does **not** become a tick system.

> **2026-07-13 active-binary correction:**
> `disassemble_function(address="0x005283d0", program="gamemd.exe")` proves
> generic ReadDouble has two explicit f64 store boundaries: the unconditional
> f32→f64 spill/reload at `0x0052855d..0x00528569`, and, when `%` is present,
> the post-multiply f64 spill at `0x0052857a..0x00528584`. Retaining an Ext80
> temporary across either store is not the native mechanism.
> `disassemble_function(address="0x0075d590", program="gamemd.exe")`,
> `read_memory(address="0x00847c40", length=128, program="gamemd.exe")`,
> `decompile_function(address="0x00528a10", program="gamemd.exe")`, and
> `disassemble_function(address="0x007caf30", program="gamemd.exe")` further
> prove the Warhead Verses-specific contract: 0x80-byte bounded ReadString,
> eleven-`100%%` missing fallback, present-trimmed-empty skip, exactly 11 stores,
> native `strtok` empty-token collapse, and a null-dereference fault for an
> exhausted nonempty token list. These details supersede the older “unbounded
> Rust string/debug assert” recommendation and compact Verses prose below.

---

## Table of Contents

- §1. Verified active-YR responsibilities of the INI-parsing helper family
- §2. Full inventory (typed accessors, enum helpers, lookup core, globals, tables, vtable/COM slots, legacy)
- §3. Active vs inactive/legacy (TS) split
- §4. Comparison against the current Rust architecture
- §5. gamemd-native behavior contract (testable statements P1–P18)
- §6. Rust-native replacement boundary
- §7. Old ad hoc Rust logic to retire/fold
- §8. Migration slices + acceptance tests (S0–S6)
- §9. Sources & verification ledger

---

## 1. Verified active-YR responsibilities of the INI-parsing helper family

This is what the INI accessor family **owns** in a standard YR skirmish. CCINIClass is the engine's typed INI reader, sitting on top of INIClass (raw CRC-hashed section/entry store). Every TypeClass `ReadINI` virtual calls these accessors; every `[General]`/`[CombatDamage]`/`[AudioVisual]` constant flows through them; map files (`[Header]`, `[Basic]`, etc.) use their own CCINIClass. The observable contract is **the exact value each key resolves to**, because that value becomes a unit stat, a build time, a damage multiplier, a foundation extent, a facing byte.

| # | Responsibility (what it owns) | Active-YR | Evidence |
|---|---|---|---|
| R1 | **Typed value resolution by (section, key)** with default-on-miss for int / bool / double / string / 2-int / 3-int / CLSID / sound-list. | VERIFIED | `ReadInt 0x005276D0`, `ReadBool 0x005295F0`, `ReadDouble 0x005283D0`, `ReadString 0x00528A10` (verified this session); the rest (CCINICLASS doc §4). |
| R2 | **Integer hex notation**: `$xx` (prefix) and `xxh` (suffix, case-insensitive) parse as hex; everything else via C `atoi` (leading-numeric, lenient). | VERIFIED | `ReadInt 0x005276D0` (verified this session): `if *value=='$'` → sscanf `"$%x"`; else `tolower(last char)==0x68 'h'` → sscanf `"%xh"`; else `atoi`. |
| R3 | **Boolean by first character only**: `toupper(first char)` ∈ {`1`,`T`,`Y`} → true; ∈ {`0`,`F`,`N`} → false; anything else → the default. | VERIFIED | `ReadBool 0x005295F0` (verified this session): `switch(toupper(*value))` cases `0x31/0x54/0x59`→1, `0x30/0x46/0x4e`→0, default→param. |
| R4 | **Double/float with percent**: `sscanf "%f"` to f32, mandatory spill/reload to f64, then if the source contains `%`, multiply the reloaded f64 by `0.01` and spill/reload f64 again. | VERIFIED | `disassemble_function 0x005283D0`: stores at `0x0052855d..0x00528569` and `0x0052857a..0x00528584`. |
| R5 | **String read with trim + default-on-miss**: copy found value (or default) via `strncpy` into a caller buffer, force-terminate at `buf[size-1]`, then `strtrim` both ends (chars ≤0x20 leading, <0x21 trailing), return final strlen. Returns 0 if buffer null / size<2 / section null / key null. | VERIFIED | `ReadString 0x00528A10` (verified this session); strtrim body `0x00727CF0` (CCINICLASS doc §5). |
| R6 | **Comma-tokenized multi-value reads**: `Read3Int "%d,%d,%d"`, `ReadMinMax "%d,%d"`, `ReadPoint/Size "%d,%d"`, `ReadRect "%d,%d,%d,%d"`, `ReadColorRGB "%d,%d,%d"`→[u8;3], `ReadSoundList` strtok-on-`,` → VocClass index lookup, `RulesClass::ReadGeneral` comma-tokenize → FindOrAllocate-per-token → DynamicVectorClass. **All COMMA-delimited.** | VERIFIED LIVE (Pass 2) | Read3Int `0x00529CA0`, ReadMinMax `0x00529880`, ReadPoint `0x00529A30`, ReadRect `0x00527F20`, ReadColorRGB `0x00474B50`, ReadSoundList `0x00525430`, ReadGeneral `0x0066D530` — all `decompile_function` this run (§Pass-2 A/B). |
| R7 | **Enum-by-name resolution** (round-trip name↔id): ReadString into a fixed buffer with the default's *name* as the miss-default, then linear case-insensitive compare against a static `{name, id}` table; return the matched id or table-default. Used by Foundation, SpeedType, MovementZone, Action, Layer. | VERIFIED | `FUN_00474DA0` (verified this session): ReadString(...,table[idx].name,buf,0x20) then loop `_stricmp`-style compare → return id, default `return 0`. MovementZone/Action/Layer (CCINICLASS doc §4). |
| R8 | **CRC-hashed, binary-searched, lazily-sorted lookup with one-entry section cache** — the storage/lookup engine under all of the above. Section/key names CRC-32 hashed; sorted `{crc, ptr}` arrays; binary search; one-entry section cache keyed by **pointer identity** of the section-name string. | VERIFIED (mechanism) | Cache check + FindSection/FindEntry seen inline in all four accessors this session (`param_1+4`/`+8` cache, `CRCEngine__AddData`, `FindSection_BinarySearch`, `FindEntry_BinarySearch`); offsets (CCINICLASS doc §3). |
| R9 | **Authoritative merge order**: `rulesmd.ini` (YR) patches `rules.ini` (base RA2); `artmd.ini` patches `art.ini`. Merge is section+key override (later wins), case-insensitive, via find-or-allocate. | VERIFIED (behavior) | CCINICLASS doc §7 "merged (YR overrides RA2)"; `reference_mission_control_ini_reset_per_entry.md` (reset-per-entry default class). |
| R10 | **Reset-per-entry vs carry-forward defaults** for the MissionControl-style indexed sub-objects: each indexed entry resets to documented defaults rather than inheriting the previous entry's parsed values; 32 slots; `AARate` absent/0 copies `Rate`. | VERIFIED (doc) | `reference_mission_control_ini_reset_per_entry.md` (MEMORY index). |

**The single sentence that matters for parity:** the accessor family's only observable output is *the resolved value*. If `Strength=$190` (hex int) or `Crewed=yep` (first-char bool) resolves to a different number/bool than gamemd, every downstream system inherits that drift — and unlike a tick bug it is invisible until you diff a stat. *(Reviewer note: `Verses=100%,...` is NOT one of the broken cases — `parse_verses` already handles trailing `%`. The verified gaps are hex int, first-char bool, atoi-leniency, and `%`-suffixed `PercentAtMax`.)*

---

## 2. Full inventory

### 2a. Typed accessor methods (CCINIClass)

| Name | Address | Parse rule | Active-in-YR | Evidence |
|---|---|---|---|---|
| ReadString | `0x00528A10` | strncpy(default-on-miss) → force-terminate `buf[size-1]` → strtrim → return strlen; 0 on null buf / size<2 / null section / null key | YES | VERIFIED this session |
| ReadInt | `0x005276D0` | `$`→`"$%x"`; last-char tolower=='h'→`"%xh"`; else `atoi`; default if section/key null | YES | VERIFIED this session |
| ReadBool | `0x005295F0` | `toupper(first char)`: `1/T/Y`→true, `0/F/N`→false, else default | YES | VERIFIED this session |
| ReadDouble | `0x005283D0` | `"%f"` (single-precision), ×0.01 if value contains `%`; returns float widened to double | YES | VERIFIED this session |
| Read3Int / ReadInt3 | `0x00529CA0` | **64-byte buf** (`local_40[63]`, `_strncpy(...,0x40)`); strtrim; `"%d,%d,%d"` sscanf (fmt `0x008189B0`); copies 3 defaults on miss (or on stack-canary `0x40` guard) | YES | **VERIFIED this session** (`decompile_function 0x00529CA0`: `_strncpy(local_40,local_60,0x40)`; `s__d__d__d_008189b0`) |
| ReadMinMax | `0x00529880` | **64-byte buf**; strtrim; `"%d,%d"` sscanf (fmt `0x0081C000`); copies 2 defaults on miss | YES | **VERIFIED this session** (`decompile_function 0x00529880`: `_strncpy(local_40,local_60,0x40)`; `s__d__d_0081c000`) |
| ReadCLSID | `0x00527920` | **128-byte ANSI buf** (`local_180[127]`, `_strncpy(...,0x80)`) → `MultiByteToWideChar(...,0x80)` into a 128-WCHAR buf (`local_100[128]`) → `CLSIDFromString`; default by value on miss/HRESULT<0 | YES (locomotors) | **VERIFIED this session** (`decompile_function 0x00527920`: `_strncpy(local_180,local_1a8,0x80)`, `MultiByteToWideChar(...,local_100,0x80)`, `CLSIDFromString`) |
| ReadPoint / ReadSize | `0x00529a30` | **64-byte buf**; strtrim; `"%d,%d"` sscanf — **COMMA-separated** (fmt `0x0081C000`); section/key args fetched via `FUN_007b5440` (varargs); copies 2 defaults on miss | YES | **VERIFIED this session** (`decompile_function 0x00529a30`: `_strncpy(local_40,param_3,0x40)`; fmt `s__d__d_0081c000`; `read_memory 0x0081C000` = `25 64 2c 25 64 00` = `"%d,%d"`) |
| ReadRect | `0x00527f20` | **64-byte buf**; strtrim; `"%d,%d,%d,%d"` sscanf — **COMMA-separated** (fmt `0x00825bbc`); default literal `"0,0,0,0"` (`0x00825bc8`) seeds the sscanf so missing fields keep the default's component | YES | **VERIFIED this session** (`decompile_function 0x00527f20`: `_strncpy(local_40,local_70,0x40)`; fmt `s__d__d__d__d_00825bbc`; default `s_0_0_0_0_00825bc8`; `read_memory 0x00825bbc` = `"%d,%d,%d,%d"`, `0x00825bc8` = `"0,0,0,0"`) |
| ReadSoundList | `0x00525430` | **128-byte buf** (`ReadString(...,0x80)`); strtok on `,` (`0x00817f70`=`","`) → VocClass FindPtrByName→FindIndexByPtr → DynamicVectorClass (init cap 10, `local_88=10`) | YES | **VERIFIED this session** (`decompile_function 0x00525430`: `ReadString(...,local_80,0x80)`, `strtok(local_80,&DAT_00817f70)`, `local_88=10`) |
| ReadSpeedType | `0x00476FC0` | **128-byte buf** (`ReadString(...,0x80)`) → `SpeedType__FromName`; default `param_3` on miss (ReadString len==0) | YES | **VERIFIED this session** (`decompile_function 0x00476FC0`: `ReadString(...,local_80,0x80)`; `return param_3` on miss) |
| ReadMovementZone | `0x00474E40` | **32-byte buf** (`ReadString(...,0x20)`); linear case-insensitive (`FUN_007c8d20`) scan of 13-entry table @`0x0081BA88..0x0081BABC`; **-1** on miss | YES | **VERIFIED this session** (`decompile_function 0x00474E40`: `ReadString(...,local_20,0x20)`, loop `< 0x81babc`, `return -1`) |
| ReadAction | `0x00474EE0` | **32-byte buf**; linear scan of 73-entry table @`0x007E4C50..0x007E4D74` (stride 1); **0** on miss | YES | **VERIFIED this session** (`decompile_function 0x00474EE0`: `ReadString(...,local_20,0x20)`, loop `< 0x7e4d74`, `return 0`) |
| ReadLayer | `0x00477050` | **128-byte buf** (`ReadString(...,0x80)`) → `Layer_From_Name`; default `param_3` on miss | YES | **VERIFIED this session** (`decompile_function 0x00477050`: `ReadString(...,local_80,0x80)`; `return param_3` on miss) |
| **ReadColorRGB** (NEW) | `0x00474B50` | **64-byte buf**; default formatted as `"%d,%d,%d"` (fmt `0x008189B0`) before the read; `ReadString(...,0x40)`; `sscanf "%d,%d,%d"` → packs 3 bytes into a u8[3] RGB; default RGB on miss. **COMMA-delimited.** | YES (`[Colors]`/tint triplets) | **VERIFIED this session — NEW family member** (`decompile_function 0x00474B50`: `ReadString(...,local_40,0x40)`, `sscanf(local_40,s__d__d__d_008189b0,...)`) |
| Enum-by-name helper (Foundation pattern) | `FUN_00474DA0` | **32-byte buf** (`ReadString(...,default=table[idx].name,0x20)`) → linear case-insensitive (`FUN_007c8d20`=`_stricmp`) compare against `{name,id}` table @`0x0081b9d8..0x0081ba88` (stride 2 dwords) → id at `0x0081b9dc`; default `return 0` (=1x1) | YES | **VERIFIED this session** (`decompile_function 0x00474DA0`; compare helper `decompile_function 0x007c8d20`) |
| RulesClass::ReadGeneral | `0x0066D530` | **128-byte buf** (`ReadString(...,0x80)`) → strtok on `,` → `FindOrAllocate` per token (e.g. `AnimTypeClass__FindOrAllocate` for DamageFireTypes) → DynamicVectorClass (type-registry list build) | YES | **VERIFIED this session** (`decompile_function 0x0066D530`: `ReadString(...,local_88,0x80)`, `strtok(local_88,&DAT_00817f70)`, `AnimTypeClass__FindOrAllocate`) |

### 2b. Lookup / storage core (INIClass)

| Name | Address | Role | Active-in-YR | Evidence |
|---|---|---|---|---|
| INIClass::FindSection (binary search) | `0x0052B620` | CRC-keyed binary search on sorted `{crc,ptr}` 8-byte records; lazy qsort on first access | YES | CCINICLASS doc §3 (mechanism seen inline this session) |
| INIClass::FindEntry (binary search) | `0x0052B4F0` | same, within a section's sorted entry array | YES | CCINICLASS doc §3 |
| INIClass::FindSectionCached | `0x0052B390` | cache check (pointer-equality on section-name string) then binary search | YES | CCINICLASS doc §3 (seen inline this session) |
| CCINIClass::Constructor | `0x00535B30` | init empty INIClass + GenericList/GenericNode list | YES | CCINICLASS doc §7 |
| INIClass::Constructor | `0x00535AA0` | base init | YES | CCINICLASS doc §7 |
| INIClass::Destructor | `0x005256F0` | free linked list + sorted array | YES | CCINICLASS doc §7 |
| strtrim | `0x00727CF0` | trim ≤0x20 leading, <0x21 trailing, in place | YES | CCINICLASS doc §5 |
| CRCEngine::AddData | (referenced) | CRC-32 of `(name, strlen(name))` for section/key hashing | YES | seen inline this session |

**INIClass field layout (CCINICLASS doc §2, DOC-ONLY offsets):** `+0x04` cached_section_name_crc/ptr-identity, `+0x08` cached_section_ptr, `+0x0C..+0x24` GenericList/Node, `+0x28` sorted_sections_array, `+0x2C` section_count, `+0x30` capacity, `+0x34` sorted_flag, `+0x38` last_find_result, `+0x3C` entry cursor. INISection: `+0x00` name CRC, `+0x04` data node (→`+0x28` sorted entries, `+0x30` count, `+0x34` sorted flag, `+0x3C` cached entry). INIEntry: `+0x00` key CRC, `+0x04` data node (→`+0x10` value `char*`). **All field offsets here are DOC-ONLY; the *parse* semantics in §2a are the load-bearing part and are live-verified.**

### 2c. Static tables / format-string constants

| Table / constant | Address | Role | Evidence |
|---|---|---|---|
| `"$%x"` | `0x00825BB8` | hex with `$` prefix | **VERIFIED** `read_memory 0x00825BB8` = `24 25 78 00` = `"$%x"` |
| `"%xh"` | `0x00825BB4` | hex with `h` suffix | **VERIFIED** `read_memory 0x00825BB4` = `25 78 68 00` = `"%xh"` |
| `"%f"` | `0x00825BD8` | float (single-precision) | **VERIFIED** `read_memory 0x00825BD8` = `25 66 00` = `"%f"` |
| `"%d,%d,%d"` | `0x008189B0` | Read3Int / ReadColorRGB | **VERIFIED** `read_memory 0x008189B0` = `25 64 2c 25 64 2c 25 64 00` = `"%d,%d,%d"` |
| `"%d,%d"` | `0x0081C000` | ReadMinMax / ReadPoint / ReadSize | **VERIFIED** `read_memory 0x0081C000` = `25 64 2c 25 64 00` = `"%d,%d"` |
| `"%d,%d,%d,%d"` | `0x00825BBC` | ReadRect | **VERIFIED** `read_memory 0x00825BBC` = `25 64 2c 25 64 2c 25 64 2c 25 64 00` = `"%d,%d,%d,%d"` |
| `"0,0,0,0"` (ReadRect default literal) | `0x00825BC8` | ReadRect miss default (seeds sscanf) | **VERIFIED** `read_memory 0x00825BC8` = `30 2c 30 2c 30 2c 30 00` = `"0,0,0,0"` |
| `","` (strtok delimiter) | `0x00817F70` | ReadSoundList / ReadGeneral token split | **VERIFIED** `read_memory 0x00817F70` = `2c 00` = `","` |
| double `0.01` | `0x007E3808` (= `_g_ImpassableSpeedThreshold_0_01`) | percent multiplier | **VERIFIED** `read_memory 0x007E3808` = `7b 14 ae 47 e1 7a 84 3f` = IEEE-754 double `0.01` |
| Enum-by-name table (Foundation/etc.) | `0x0081b9d8` (name) / `0x0081b9dc` (id), stride 2 dwords, range `0x0081b9d8..0x0081ba88` | name↔id pairs | **VERIFIED** (`FUN_00474DA0` loop bound `< 0x81ba88`) |
| MovementZone table (13) | `0x0081BA88..0x0081BABC` | Normal..CrusherAll | **VERIFIED** (`ReadMovementZone` loop bound `< 0x81babc`; 0x34 bytes / 4 = 13) |
| Action table (73) | `0x007E4C50..0x007E4D74` | mission action names | **VERIFIED** (`ReadAction` loop bound `< 0x7e4d74`; 0x124 bytes / 4 = 73) |
| `&DAT_00889f64` (empty-string default sentinel) | `0x00889F64` | the `""` default many accessors pass when no explicit default | seen inline this session (ReadSoundList/ReadGeneral/3Int default = `&DAT_00889f64`) |

### 2d. Singleton / global INI instances

| Instance | Role | Active-in-YR | Evidence |
|---|---|---|---|
| Global rules CCINIClass | `rules.ini`+`rulesmd.ini` merged (YR overrides) | YES | CCINICLASS doc §7 |
| Global art CCINIClass | `art.ini`+`artmd.ini` merged | YES | CCINICLASS doc §7 |
| Per-map CCINIClass | `.map`/`.mmx` `[Header]`/`[Basic]`/triggers | YES | CCINICLASS doc §7 |
| Scenario CCINIClass ctor | `0x00599650` (full), `0x005981F0` (RMG) | YES | CCINICLASS doc §7 |

### 2e. Vtable / COM slots

| Slot | Class | Role | Evidence |
|---|---|---|---|
| vtable @+0x00 | CCINIClass (overrides INIClass) | typed-reader override surface | CCINICLASS doc §2 |
| (none gameplay-load-bearing for the *parse* contract) | — | The COM/vtable plumbing of INIClass/CCINIClass is **not** part of the observable parse contract; Rust does not reproduce it. | per translation rule |

### 2f. Legacy / dormant TS paths in this surface

| Item | Status | Evidence |
|---|---|---|
| `Subterranean` (MovementZone idx 6) in the SpeedType/MovementZone table | TS-LEGACY string present in the table but the **zone is not reachable for any stock-YR unit** (no YR unit declares `MovementZone=Subterranean`); parser must still resolve the *name* (idx 6) if a value asks for it, but no YR data does. Do not design special handling. | CCINICLASS doc §4 table; `feedback_no_tunnel_subterranean` |
| INI **write** methods (scenario editor) | DORMANT for a read-only game client; not investigated, not reproduced | CCINICLASS doc §9 |
| CRC-collision "first match wins" silent shadowing | A theoretical TS-era artifact of the CRC store; never occurs with stock RA2/YR key names. Rust's string-keyed map is immune; this is **not** a behavior to reproduce (it would be reproducing a latent bug). | CCINICLASS doc §8 |
| Fog-of-war / SpecialFlags-gated INI keys | The *parser* reads them like any key; the **gating** is a sim concern, not a parse concern. No parser-side TS handling. | per CLAUDE.md TS section |

---

## 3. Active vs inactive/legacy (TS) split

### ACTIVE-YR — the parse contract a Rust replacement MUST reproduce

| Item | One-line rationale |
|---|---|
| `$xx` / `xxh` hex int parse (R2) | Stock rules/art use hex for facing bytes, masks, color values; a missed hex parse silently zeroes the stat. |
| First-char bool parse (R3) | `yes/no/true/false/1/0` and any first-char-T/Y/F/N/1/0 string resolve deterministically; full-word matching diverges on odd values. |
| `%`→×0.01 double parse (R4) | `Verses=`, `PercentAtMax=`, `ConditionRed/Yellow=`, many `[General]` percents. Wrong handling shifts damage/economy to the decimal. |
| strtrim ≤0x20 / default-on-miss string read (R5) | Determines trimming of names/IDs and the value substituted when a key is absent. |
| Comma tokenization (R6) | Prereq lists, sound lists, paradrop lists, 3-int/min-max/point/rect tuples. **All COMMA-delimited** — Point/Size/Rect corrected from the draft's "space" claim (see P9). |
| Enum-by-name case-insensitive table match, table-default on miss (R7) | Foundation extents (building footprints), SpeedType/MovementZone, Layer; default id 0 = `1x1` for Foundation. |
| YR-over-base merge order (R9) | `rulesmd` patches `rules`; getting load order wrong corrupts every overridden key. |
| Reset-per-entry MissionControl defaults (R10) | Indexed sub-objects reset to documented defaults, not carry-forward. |
| Case-insensitive section/key lookup (R8 observable) | All lookups are case-insensitive; CRC-hash internals are NOT observable and need not be reproduced. |

### INACTIVE / LEGACY (TS) — do NOT reproduce as default

| Item | One-line rationale |
|---|---|
| CRC-hash store + lazy qsort + binary search + pointer-identity section cache (R8 internals) | Internal performance mechanism. Output (which value a key resolves to) is what matters; a Rust `HashMap`/`BTreeMap` is equivalent and immune to CRC collisions. Do NOT port the CRC engine. |
| CRC-collision first-match shadowing | Latent TS artifact; reproducing it would reproduce a bug. Skip. |
| INI write/save methods | Read-only client; dormant. |
| `Subterranean` movement-zone reachability | TS legacy; name resolvable but unused by stock YR data. |
| Fixed-size buffers (per-accessor: **32** for enum/MovementZone/Action, **64** for Read3Int/ReadMinMax/ReadPoint/ReadRect/ReadColorRGB, **128** for ReadCLSID/ReadSpeedType/ReadLayer/ReadSoundList/ReadGeneral and Warhead Verses) | A caller-supplied native parse boundary that Rust must model. `ReadString` has no universal built-in cap—the size is `param_6`—but every caller's 32/64/128 choice, `buf[size-1]=0`, and post-truncation trim are semantic. Over-length stock prevalence affects priority only; unbounded parsing is DRIFT for any input crossing the active caller cap. |

---

## 4. Comparison against the current Rust architecture

The current parser is `src/rules/ini_parser.rs`: `IniFile { sections: HashMap<String, IniSection> }` with lowercase-keyed sections and entries, an insertion-ordered `key_order`, a line-based `from_str`, and a `merge` for the md-over-base patch. Typed reads are **methods on `IniSection`** (`get`, `get_i32`, `get_f32`, `get_light_f32`, `get_percent`, `get_bool`, `get_list`, `get_values`). There is **no central typed accessor with gamemd semantics** — each consumer calls `section.get_*()` and then frequently re-implements its own enum/percent/hex/default logic inline (852 accessor calls + 85 raw `parse::`/`strip_suffix`/`from_str_radix`/`to_lowercase` re-implementations across `src/rules/`, this session's grep).

### 4.1 Accessor-by-accessor parity (default DRIFT)

| gamemd rule | Current Rust | Verdict | Player-visible? | Trigger frequency |
|---|---|---|---|---|
| **R2 hex `$xx`/`xxh`** | `get_i32` = `self.get(key)?.trim().parse::<i32>().ok()` (`ini_parser.rs:70`). Rust `parse::<i32>` **rejects** `$190`, `FFh`, and even `5cells`/`100 ; cmt`-style leading-numeric that gamemd's `atoi` accepts. No hex path at all. | **DRIFT** | YES — any hex-valued key (facing, mask, color) reads `None`→falls to the call-site default instead of the hex value | rare in stock rules but **silent** when it fires; also `atoi` leniency diverges on any non-pure-int value |
| **R3 first-char bool** | `get_bool` matches whole lowercased words `yes/true/1`→true, `no/false/0`→false, else `None` (`ini_parser.rs:114`). gamemd checks **only first char** (T/Y/1, F/N/0). | **DRIFT** | YES — `Crewed=yep`, `=T`, `=Y`, `=Nope` resolve to the gamemd value but Rust returns `None`→call-site default | uncommon stock values but the contract differs; any modded/odd value diverges |
| **R4 `%`→×0.01 double** | `get_percent` strips a **single trailing** `%` and divides by 100 (`ini_parser.rs:100`); `get_f32` does plain `parse::<f32>` with **no** percent handling (`ini_parser.rs:77`). gamemd's ReadDouble multiplies by 0.01 if **any** `%` appears. | **DRIFT** | YES — but NARROWER than the draft claimed (see correction) | `PercentAtMax` with a `%` suffix (uncommon); `Verses` is already `%`-safe |
| | **CORRECTION (reviewer):** The draft claimed `Verses` is read via `get_f32` and would be "100× wrong, hitting every damage calc." **That is FALSE.** `warhead_type.rs:115` reads `Verses` via a dedicated `parse_verses` (`warhead_type.rs:197`), which strips a trailing `%` and stores a u8 percentage (0–200) — it does NOT use `get_f32` and is NOT 100× wrong. The genuine `get_f32`-no-percent gap is only: `CellSpread` (`:118`, never `%`-valued in stock — a cell radius) and `PercentAtMax` (`:122`, read via `get_f32` then `×100`; a `%`-suffixed value fails `parse::<f32>` → falls to `unwrap_or(100)`). Residual `parse_verses` DRIFT: it strips only a *trailing* `%` vs gamemd's `%`-anywhere, and `parse::<f32>` rejects atoi-lenient junk where gamemd's sscanf would take the leading number. *(grep `warhead_type.rs:115,197`; `decompile_function 0x005283D0`.)* | — | — | — |
| **R5 strtrim + default-on-miss** | `from_str` trims the whole line and the value (`raw_line.trim()`, value `.trim()`), and `get` returns the stored trimmed value; default is the call-site `.unwrap_or(...)`. Rust `.trim()` strips Unicode whitespace; gamemd strtrim is ≤0x20/<0x21 (ASCII control). | **OK on stock ASCII** (no Unicode in RA2 INI), DRIFT on non-ASCII control chars | borderline | never in stock data |
| **R6 tokenization** | `get_list` splits on `,` and trims (`ini_parser.rs:128`). Matches comma case. **No dedicated Point/Size/Rect accessor**, but gamemd's Point/Rect are ALSO comma-delimited (P9 corrected), so `get_list`-style comma tokenization is the RIGHT base — only a thin `[i32]`-tuple wrapper is missing. | OK for comma lists; thin Point/Rect tuple wrapper MISSING | borderline (consumers parse the comma tuple ad-hoc today) | depends on consumers (e.g. `[Header]` map coords) |
| **R7 enum-by-name** | `foundation.rs` fixed 22-entry table, case-insensitive `eq_ignore_ascii_case`, default id 0 = `1x1` — **matches gamemd exactly** (verified `FUN_00474DA0` default `return 0`). `object_type.rs:60/88/118/1097` use inline `match value.trim().to_ascii_lowercase()` blocks for `BuildCategory`/`PipScale`/`FactoryType`. | **OK** (foundation), structurally-scattered (per-enum inline matches) | no (output matches) | n/a |
| **R8 case-insensitive lookup** | lowercase-keyed `HashMap`. Functionally equivalent to CRC hashing; immune to CRC collision. | **OK** | no | n/a |
| **R9 merge order** | `IniFile::merge(patch)` overrides base keys, adds new sections (`ini_parser.rs:304`). Caller `load_rules_ini` (`app_init_helpers.rs:247`) loads `rules.ini` base then merges `rulesmd.ini` on top. | **OK — CONFIRMED base-then-md** (reviewer: `app_init_helpers.rs:262-271`) | YES if reversed (it is not) | every load |
| **R10 reset-per-entry defaults** | Not modeled by the parser (it's a per-system concern); MissionControl-style defaults live in `reference_mission_control_ini_reset_per_entry.md` and are applied by whichever system consumes them. | **N/A to parser**, but no shared helper exists | n/a | n/a |

### 4.2 `get_light_f32` — the one place Rust already reproduces a gamemd quirk

`get_light_f32` (`ini_parser.rs:86`) deliberately reproduces the gamemd behavior that the numeric read stops before a comma, so `LightGreenTint=0,01` reads as `0`. This is the **correct shape** — but it is a *one-off ad hoc accessor* bolted onto `IniSection` rather than a property of a typed `ReadDouble`-equivalent. It proves the project already knows the accessors must mirror gamemd's parse, just not yet in one place.

### 4.3 What is MISSING outright

- **No single typed accessor that reproduces ReadInt's hex+atoi, ReadBool's first-char, ReadDouble's `%`×0.01.** These three are the load-bearing parity gaps.
- **No dedicated Point/Size/Rect tuple reader** — but gamemd's Point/Size/Rect are **comma-delimited** `"%d,%d"` / `"%d,%d,%d,%d"` (P9 corrected), so this is a thin comma-tuple wrapper, not a new delimiter.
- **No shared enum-by-name helper** — every enum re-implements `match lowercased`. Foundation has its own table; `object_type` has inline matches; `locomotor_type`, `terrain_rules`, `radar_event_config` each roll their own.
- **No central per-key default registry** — defaults are `.unwrap_or(x)` at 852 call sites; a wrong default at any one is invisible.
- **No load-order assertion** that `rules` precedes `rulesmd` (and `art` precedes `artmd`) in the merge.

---

## 5. gamemd-native behavior contract (testable statements)

Each is a TESTABLE invariant the Rust accessor service must satisfy. These are the acceptance-test targets of §8.

**P1 — Hex `$` prefix.** `ReadInt(section,key,default)` where value is `$1A` returns `26`; `$FF`→255; `$0`→0. Parse via hex. *(ReadInt `0x005276D0`, verified this session: `if *value=='$'` → `sscanf "$%x"`.)*

**P2 — Hex `h` suffix (case-insensitive).** Value `1Ah`→26, `FFh`→255, `0FFH`→255 (tolower of last char == 'h'). *(ReadInt verified this session: `tolower(value[strlen-1])==0x68` → `sscanf "%xh"`.)*

**P3 — `atoi` fallback leniency.** Non-hex value parses via C `atoi`: `100`→100, `-50`→-50, `5cells`→5, `  7 `→7, `abc`→0, empty→0. (atoi reads leading optional-sign digits and stops; Rust `parse::<i32>` rejects all of `5cells`/`abc`.) *(ReadInt verified this session: `atoi` fallback branch.)* — **the leniency is part of the contract**; gamemd never returns "None" from a present-but-nonnumeric value, it returns the atoi result (0 for non-numeric leading).

**P4 — Default only on absent key (not on unparseable value).** If the key is **present**, ReadInt/ReadBool/ReadDouble return the parsed result (which for int may be `atoi`=0); the `default` is returned **only** when section/key is null/absent. *(All four accessors verified this session: default return is on the find-miss path; the parse path always returns a parsed value.)* This is a sharp divergence from Rust's `get_i32(...).unwrap_or(default)` which falls to default on *parse failure too*.

> **2026-07-21 `ReadInt` no-conversion correction (live Ghidra recheck):**
> P4 is exact for the decimal/`atoi` branch, including present-empty and
> present-nonnumeric values returning `0`. The two hexadecimal `sscanf`
> branches are narrower: `ReadInt` passes the current/default value itself as
> the output slot, and `sscanf` leaves that slot unchanged when `%x` converts
> no digits. Therefore `$`, `$junk`, `h`, and other hex-selected inputs with no
> conversion return the supplied current/default value, not `0`. The `%x`
> scanner accepts optional leading whitespace, sign, and `0x`/`0X`, and its
> 32-bit shift/add accumulator wraps modulo `2^32`; the embedded `atoi` body
> likewise uses a 32-bit multiply/add accumulator and wrapping negation.
> Evidence: `decompile_function` + `disassemble_function` for
> `CCINIClass::ReadInt @ 0x005276D0`, `CRT__atoi @ 0x007C9B72`, and the `%x`
> path in `CRT` scanner body `0x007D170D`; `read_memory 0x00825BB0` confirmed
> formats `"$%x"` and `"%xh"`. This correction supersedes the `$`/`h`
> no-digit expectations in any older Rust-only test; retail capture is still
> required for full Oracle certification.

**P5 — String trim + default-on-miss + buffer-cap.** ReadString copies value-or-default via `_strncpy(buf, src, size)`, force-terminates at `buf[size-1]='\0'`, runs `strtrim` in place, returns final length; 0 on null out-buffer / size<2 / null section / null key. *(ReadString `0x00528A10`, strtrim `0x00727CF0` — BOTH verified live.)* Both ends strip bytes ≤0x20. The cap is caller-specific: 32, 64, or 128 in the verified families, with Warhead Verses explicitly using 128. Rust must apply the exact byte cap and trim before downstream parsing; a debug assertion or unbounded owned String is not equivalent. Whether stock data crosses a cap is only a fix-priority scan, not the parity verdict.

**P6 — Bool first-char.** `toupper(value[0])` ∈ {`1`,`T`,`Y`}→true; ∈ {`0`,`F`,`N`}→false; else default. So `yes`,`Y`,`true`,`T`,`1`,`on`?(o→default!) — note `on`/`off` are **NOT** recognized (o is neither); `off`→default, **not** false. *(ReadBool `0x005295F0`, verified this session.)*

**P7 — Double `%`→×0.01, single precision plus explicit f64 stores.** ReadDouble parses `"%f"` (`0x00825BD8`) into f32, loads it and unconditionally spills/reloads f64 at `0x0052855d..0x00528569`, then multiplies that f64 by `0.01` (`0x007E3808`) iff `strchr(value, '%')`. The percent product is spilled/reloaded f64 again at `0x0052857a..0x00528584`. The `0x2525` search argument truncates to byte `0x25`, so any `%` triggers. A Rust port that parses directly to f64, multiplies in f32, or retains Ext80 across either f64 store is DRIFT. *(Re-verified with `disassemble_function(address="0x005283d0", program="gamemd.exe")`.)*

> **Shared-gate fold-in (cross-family, Phase 1, cited):** This ReadDouble float32-narrowing is the **INI side of the project-wide "CCINI ReadDouble → SimFixed precision boundary" gate**. Generic `ReadDouble` consumers carry only **float32 mantissa** precision (~7 sig digits) even though the return type is `float10`/double, because of the `(double)(float)` round-trip (`decompile_function 0x005283D0`; fmt `"%f"` = `read_memory 0x00825bd8` = `25 66 00`). **EXCEPTION — `Verses` does NOT use ReadDouble.** The Verses loop (`decompile_function 0x0075d3a0`/`0x0075d590`, Phase 1) is hand-rolled: `ReadString` the whole `"100%%,..."` line then `strtok` on `,`; per token, no-`%` → `strtod` (`FUN_007c9d66`, **full f64**), has-`%` → `atoi(token) * 0.01` in **double**. Verses retains **double**, NOT float32 → `WarheadTypeClass+0xA0` is a `double[11]`. So the Rust port must split: generic `read_double` round-trips through **f32 first** then ×0.01 in f64; `Verses` parses each token in **full f64** (or `atoi as f64 * 0.01`). — Also tied to the **ftol truncation gate**: ReadDouble itself does **no ftol** (it returns a double); truncation toward zero happens at the *consuming* `Math__ftol` @ `0x007c5f00` (CW `0x00822d80` = `0x0E7F`, RC=11 = round-toward-zero — `read_memory 0x00822d80`, Phase 1). So a Rust `read_double` must keep the parsed value un-truncated and let the downstream consumer truncate (`.to_num::<i32>()` round-toward-zero), NOT `.round()`, NOT truncate at read time.

**P8 — Read3Int / ReadMinMax delimiters.** `Read3Int` parses `"%d,%d,%d"` (comma); `ReadMinMax` parses `"%d,%d"` (comma); both copy all defaults on miss. *(Read3Int `0x00529CA0`, ReadMinMax `0x00529880`; CCINICLASS doc §4.)*

**P9 — Point/Size/Rect delimiters are COMMAS (CORRECTED).** `ReadPoint/ReadSize` parse `"%d,%d"` (comma); `ReadRect` parses `"%d,%d,%d,%d"` (comma) — **same delimiter as the comma readers, NOT spaces.** The original draft claimed spaces; that was WRONG. *(CORRECTED by reviewer this session: `decompile_function 0x00529a30` → fmt at `0x0081C000`; `read_memory 0x0081C000` = `"%d,%d"`. `decompile_function 0x00527f20` → fmt at `0x00825bbc`; `read_memory 0x00825bbc` = `"%d,%d,%d,%d"`.)* — A Rust `read_point`/`read_rect` SHOULD reuse the same comma tokenization as `get_list` (split on `,`, parse each via the atoi-lenient int rule), not a space split. Both readers also strtrim the value, then sscanf; an extra-token suffix is ignored (sscanf stops after the format's fields).

**P10 — Enum-by-name case-insensitive + table-default.** ReadString into a fixed buffer with default = table[default_idx].name, then case-insensitive whole-string compare against the static `{name,id}` table; return matched id, else the table default (Foundation → id 0 = `1x1`; MovementZone → -1; Action → 0). *(`FUN_00474DA0` verified this session; per-table defaults: MovementZone -1 / Action 0 from CCINICLASS doc §4.)* The compare is **whole-string** case-insensitive (a substring does not match).

**P11 — Sound-list tokenize.** `ReadSoundList` ReadStrings (128 buf), strtok on `,`, resolves each token through VocClass name→index, builds a vector (init cap 10). *(ReadSoundList `0x00525430`; CCINICLASS doc §4.)*

**P12 — ReadGeneral type-registry build.** Comma-tokenize the value, FindOrAllocate per token, append to a DynamicVectorClass — this is how `[InfantryTypes]`-style ordered registries are built. *(ReadGeneral `0x0066D530`; task anchor.)*

**P13 — Case-insensitive section & key.** `[General]`==`[GENERAL]`==`[general]`; `Cost`==`COST`. *(CRC over the bytes, but stock keys never collide; observable = case-insensitive.)*

**P14 — Merge: YR patches base, later-wins, additive.** Load `rules.ini` then merge `rulesmd.ini`: existing sections gain/override keys (md wins), new md sections are added. Same for `art`/`artmd`. *(R9; CCINICLASS doc §7.)*

**P15 — Duplicate in-file section merges (later key wins).** Two `[General]` blocks in one file merge into one; later key wins, first-appearance order preserved. *(matches current Rust `from_str`; CCINICLASS doc §7 merge model.)*

**P16 — Inline `;` comment strip at load.** `Cost=1000 ; credits`→`1000`; `Image=USELESS;cmt`→`USELESS`. The `%%`/`%` percent marker is **distinct** from `;` — a value like `Verses=100%` has no `;` so the `%` survives to ReadDouble. *(CCINICLASS doc §8 "comments handled at load, not at Read"; current Rust strips `;` in `from_str`.)*

**P17 — Reset-per-entry defaults (MissionControl class).** Indexed sub-objects reset to documented defaults each entry (NOT carry-forward); 32 slots; `AARate` absent/0 copies `Rate`. *(`reference_mission_control_ini_reset_per_entry.md`.)*

**P18 — Value present-but-empty.** `Empty=` (present, empty value): ReadString returns the empty string (length 0) — **not** the default (key exists). ReadInt(`Empty=`)→atoi("")=0. ReadBool(`Empty=`)→first char is `\0`→default. *(derived from P4/P3/P6, verified this-session find-path returns the entry; the entry's value string is empty.)* — a real divergence from Rust `get(...).filter(|s|!s.is_empty())` idioms scattered in consumers.

**P19 — ReadSpeed value transform (NEW, Pass 2).** `ReadSpeed(section,key,default)` calls `ReadInt(default=-1)`; if the raw int is `-1` (i.e. absent) → caller `default`; else **clamp to 100**, compute `(value << 8) / 100` with round-toward-zero integer division (sign-corrected), then **clamp to 255**. So `Speed=100`→`(100*256)/100`=256→clamped **255**; `Speed=50`→128; `Speed=0`→0; `Speed=7`→17 (`1792/100`=17, truncated). A consumer that reads a `Speed=` key as a plain int instead of via this transform is **DRIFT**. *(`decompile_function 0x00474810`, verified this session.)*

**P20 — ReadRange value transform via ftol (NEW, Pass 2).** `ReadRange(section,key,default)` calls `ReadDouble(default=-1.0)`; if it equals the -1.0 sentinel (`_DAT_007e4900`) → caller `default`; else `Math__ftol()` → **truncate toward zero to int**. So `Range=5.9`→5, `Range=5`→5. This is the INI accessor instance of the project-wide **ftol truncation gate** (`Math__ftol` @ `0x007c5f00`, control word `0x00822d80`=`0x0E7F`, RC=11=round-toward-zero, Phase 1). Rust must truncate (`.to_num::<i32>()`), never `.round()`. *(`decompile_function 0x00474620`, verified this session.)*

**P21 — ReadColorRGB triplet (NEW, Pass 2).** `ReadColorRGB(section,key,&default_rgb)` formats the default as `"%d,%d,%d"`, `ReadString(...,64-byte buf)`, then `sscanf "%d,%d,%d"` into three bytes packed as a u8[3] (R,G,B); default RGB on miss/parse-fail. **COMMA-delimited**, same family as Read3Int. Each component is an `sscanf %d` (so it is **not** atoi-lenient like ReadInt — `%d` stops at first non-digit, no `$`/`h` hex). *(`decompile_function 0x00474B50`, verified this session.)*

---

## 6. Rust-native replacement boundary

A cohesive **typed INI accessor service** that mirrors the gamemd parse contract with clean Rust, lives in `rules/`, and respects the layering invariant (it depends only on `assets/`+`util/`; nothing in `sim/`/`render/`/etc. changes its design). It does **not** reproduce the CRC engine, COM vtables, or fixed buffers — it reproduces the *values*.

### 6.1 Ownership / module placement

```
src/rules/
  ini_parser.rs        // KEEP: IniFile/IniSection raw store, from_str, merge (the "INIClass" analog)
  ini_value.rs   (NEW) // the typed-accessor service: the "CCINIClass ReadX" analog
  ini_enum.rs    (NEW) // generic enum-by-name table helper (folds foundation + the inline matches)
```

- `IniFile`/`IniSection` stay the raw, case-insensitive section/entry store (the INIClass role). No CRC, no binary search — `HashMap` is the equivalent.
- `ini_value.rs` adds **free functions or `IniSection` methods** that reproduce ReadInt/ReadBool/ReadDouble/ReadString semantics **exactly**, returning gamemd-faithful values, with the gamemd default-on-*miss* (not default-on-parse-fail) distinction encoded.
- `ini_enum.rs` provides one `enum_by_name(value, table, default_id)` matching `FUN_00474DA0`, used by Foundation, MovementZone, SpeedType, Layer, and the `object_type` inline matches.

### 6.2 Surface sketch (signatures, not implementations)

```rust
// ini_value.rs — gamemd ReadX-equivalent typed reads on IniSection.
// "present" = key exists (even if empty). gamemd returns the parsed value
// for a present key; the default is returned ONLY when the key is absent.
impl IniSection {
    /// ReadInt: $xx / xxh hex, else C-atoi leniency. Default ONLY on absent key.
    fn read_int(&self, key: &str, default: i32) -> i32;          // P1–P4, P18
    /// ReadBool: toupper(first char) in {1,T,Y}=true / {0,F,N}=false / else default.
    fn read_bool(&self, key: &str, default: bool) -> bool;       // P6
    /// ReadDouble: "%f" via f32, then ×0.01 iff value contains '%'.
    /// Returns the gamemd double; sim callers convert via a single pinned path.
    fn read_double(&self, key: &str, default: f64) -> f64;       // P7 (precision pinned in S0)
    /// ReadString: byte-bounded copy, forced NUL, trim, and default-on-absent.
    /// Capacity is a required call-site argument, not a debug-only assertion.
    fn read_string_bounded(&self, key: &[u8], default: &[u8], capacity: usize)
        -> Vec<u8>; // P5, P18
    /// Read3Int / ReadMinMax: comma "%d,%d,%d" / "%d,%d", all-defaults on miss.
    fn read_3int(&self, key: &str, default: [i32; 3]) -> [i32; 3]; // P8
    fn read_minmax(&self, key: &str, default: [i32; 2]) -> [i32; 2]; // P8
    /// ReadPoint/Size/Rect: COMMA-separated "%d,%d" / "%d,%d,%d,%d" (P9 CORRECTED — same comma tokenize as get_list).
    fn read_point(&self, key: &str, default: (i32, i32)) -> (i32, i32);          // P9
    fn read_rect(&self, key: &str, default: (i32,i32,i32,i32)) -> (i32,i32,i32,i32); // P9
    /// ReadColorRGB: COMMA "%d,%d,%d" → [u8;3] (plain %d per-component, NOT atoi/hex). (P21)
    fn read_color_rgb(&self, key: &str, default: [u8; 3]) -> [u8; 3];           // P21
    /// ReadSpeed: read_int(-1) → clamp 100 → (v<<8)/100 trunc → clamp 255. (P19)
    fn read_speed(&self, key: &str, default: i32) -> i32;                       // P19
    /// ReadRange: read_double(-1.0) → ftol truncate-toward-zero. (P20)
    fn read_range(&self, key: &str, default: i32) -> i32;                       // P20
}

// A C-atoi-equivalent leading-numeric parse (shared by read_int's fallback).
fn atoi_lenient(s: &str) -> i32;                                  // P3

// ini_enum.rs — the FUN_00474DA0 round-trip helper.
pub struct EnumByName { pub name: &'static str, pub id: i32 }
pub fn enum_by_name(value: &str, table: &[EnumByName], default_id: i32) -> i32; // P10
```

### 6.3 What stays / what moves

- **Stays in sim-fixed space:** `read_double` returns a `f64` that mirrors gamemd's `(double)(float)x [×0.01]`. The **single** `f64`→`SimFixed` conversion stays in `util/fixed_math` (`sim_from_f32`/equivalent), so all parse→fixed math goes through one pinned path (determinism). No `f32`/`f64` enters `sim/` — only the converted `SimFixed`.
- **Per-key defaults** stay at the call sites (they ARE the per-field semantics), but the *parse* moves into `read_*`. This kills the "parse fail → unwrap_or" drift (P4) without centralizing 852 defaults.

### 6.4 Layering check

`ini_value.rs` / `ini_enum.rs` import only `std`, `crate::rules::ini_parser`, and (for the conversion at call sites) `crate::util::fixed_math`. No `sim/`/`render/`/`ui/`/`audio/`/`net/` dependency. ✔ #1 invariant preserved.

---

## 7. Old ad hoc Rust logic to RETIRE / fold into the service

Cite `file:symbol` — these re-implement (often divergently) what the service should own:

- **`src/rules/ini_parser.rs:get_i32`** — `parse::<i32>` with no hex, no atoi leniency, default-on-parse-fail. RETIRE → `read_int` (P1–P4).
- **`src/rules/ini_parser.rs:get_bool`** — whole-word match. RETIRE → `read_bool` first-char (P6).
- **`src/rules/ini_parser.rs:get_percent`** — single trailing `%`. FOLD into `read_double` (`%`-anywhere ×0.01, P7).
- **`src/rules/ini_parser.rs:get_f32`** — plain parse, no percent. The genuine `%`-affected caller is `warhead_type.rs:122 PercentAtMax` (read via `get_f32` then `×100`; a `%`-suffix fails the parse). `warhead_type.rs:118 CellSpread` is a cell radius — never `%`-valued in stock — so its `get_f32` is fine for stock data but should still route through `read_double` for atoi/leniency parity. **NOTE (reviewer):** `Verses` is NOT a `get_f32` caller — it uses dedicated `parse_verses` (`:197`) which already handles trailing `%`; only its `%`-anywhere / atoi-leniency edge differs. RETIRE/repoint the `get_f32` callers.
- **`src/rules/ini_parser.rs:get_light_f32`** — the comma-stop quirk; KEEP the behavior but **express it as a documented `read_*` variant** rather than a bespoke accessor (it is the gamemd "%f stops at comma" sub-case).
- **`src/rules/terrain_rules.rs:354`** — `trim_end_matches('%')` trailing-only percent. FOLD → `read_double` (`%`-anywhere ×0.01, P7).
- **`src/rules/warhead_type.rs:218`** — second `strip_suffix('%')` trailing-only percent. FOLD → `read_double` (P7).
- **`Speed=`/`Range=` consumers (audit)** — any consumer reading these keys as a raw int/double instead of via the gamemd transform (P19 `read_speed`, P20 `read_range`) is DRIFT. Grep `Speed`/`Range`/`MinimumRange` int/double reads across `rules/*_type.rs` in S4/S5 and repoint. (NEW, Pass 2.)
- **`src/rules/object_type.rs:58 BuildCategory::from_ini`, `:86 PipScale::from_ini`, `:115 FactoryType::from_ini`, `:1097` (locomotor/inline match)** — inline `match lowercased` enum tables. FOLD → `enum_by_name` (P10) with per-enum `&[EnumByName]` tables.
- **`src/rules/foundation.rs:foundation_def`** — already correct (matches `FUN_00474DA0` default-0). KEEP, but re-express the table as an `EnumByName` consumer so all enum-by-name share one helper. (Behavior-preserving refactor; lowest priority.)
- **The 85 raw `parse::`/`from_str_radix`/`strip_suffix`/`to_lowercase` sites** across `object_type.rs`(7), `warhead_type.rs`(5), `terrain_rules.rs`(5), `weapon_type.rs`(3), `projectile_type.rs`(3), `locomotor_type.rs`(3), `particle_type.rs`(7), `particle_system_type.rs`(6), `art_data.rs`(17), etc. — AUDIT each: any that parse an int/bool/double/percent/enum should route through the service; those parsing already-clean derived data (e.g. `art_data` frame math) may stay. This is the long tail; do it per-system, not in one bulk change (per CLAUDE.md change-management).

---

## 8. Migration slices + acceptance tests

Shadow-first, dependency-ordered, each independently shippable. This is a **load-time** substrate, so the "shadow → invert → authoritative → SNAPSHOT_VERSION bump → parity harness" rhythm adapts: there is no per-tick hash, so the analog of "shadow" is **add the new accessor + assert it equals the old accessor on the entire stock `rulesmd.ini`/`artmd.ini` corpus**, then flip consumers, then delete the old accessor. The state-hash relevance is **indirect**: a changed parsed value changes a unit stat which changes `state_hash` — so the global parity harness (deterministic replay vs baseline) is the end gate.

### S0 — BLOCKING RESEARCH GATE: pin ReadDouble precision (P7)
Before any consumer flips to `read_double`, re-decode and pin the exact arithmetic: gamemd computes `(double)(float)sscanf("%f")` then `× 0.01(double)`. Decide the Rust path that is **bit-identical after `SimFixed` conversion** across the boundary set {`0`,`1`,`100%`,`50%`,`12.5%`,`0.016`,`.9`,`Verses` values}. **Acceptance:** `test_read_double_precision_matches_gamemd` — a table of (string, expected `SimFixed`) computed from the pinned f32→×0.01→fixed path; must include negative and `%`-with-decimal cases. **Until S0 passes, `Verses`/percent consumers stay on the current accessor.** (DRIFT acknowledged, gated.)

### S1 — Introduce the service (additive, shadow)
Add `ini_value.rs` (`read_int/bool/double/string/3int/minmax/point/rect`, `atoi_lenient`) and `ini_enum.rs` (`enum_by_name`). No consumer changes yet.
**Acceptance:**
- `test_read_int_hex` — `$1A→26`, `1Ah→26`, `0FFH→255`, `$0→0`. (P1/P2)
- `test_read_int_atoi_leniency` — `5cells→5`, `abc→0`, `-50→-50`, present-empty→0, absent→default. (P3/P4/P18)
- `test_read_bool_first_char` — `yes/Y/T/true/1→true`, `no/N/F/false/0→false`, `off→default`, `xyz→default`, present-empty→default. (P6/P18)
- `test_read_double_percent` — `50%→0.5`, `100%→1.0`, `7→7.0`, bare `0.5→0.5`. (P7, after S0 pins precision)
- `test_read_string_trim_default` — trims ≤0x20 both ends; absent→default; present-empty→"". (P5/P18)
- `test_read_point_comma` — `"3,5"→(3,5)` (COMMA-delimited, P9 corrected); a 4-tuple `read_rect` parses `"1,2,3,4"→(1,2,3,4)`. (P9)
- `test_enum_by_name` — Foundation `3x3refinery→9`, unknown→0; whole-string (substring no-match). (P10)

### S2 — Corpus equivalence harness (the "shadow assert")
A test that loads stock `rulesmd.ini` + `artmd.ini`, and for **every** key currently read via `get_i32/get_bool/get_percent/get_f32`, asserts `read_*` produces a value whose `SimFixed`/int/bool equals the old accessor **OR** documents the intended divergence (hex/first-char/percent) with the gamemd-correct expected value.
**Acceptance:** `test_ini_accessor_corpus_parity` — enumerates divergences; every divergence row is either (a) a gamemd-correct fix (old was wrong) with a cited expected value, or (b) zero (identical). No silent diffs.

### S3 — Flip the enum consumers (lowest-risk, output-equivalent)
Repoint `object_type` `BuildCategory/PipScale/FactoryType` and `locomotor_type`/`terrain_rules` inline matches to `enum_by_name`; re-express `foundation.rs` over the shared helper.
**Acceptance:** existing `object_type`/`foundation` tests still green; add `test_factory_type_case_insensitive_via_service`.

### S4 — Flip int/bool/string consumers
Repoint `get_i32→read_int`, `get_bool→read_bool`, `get`/string reads → `read_string` across `rules/*_type.rs`. Per-system, not bulk.
**Acceptance:** per-system tests green; add targeted tests for any key with a hex or odd-bool stock value found in S2.

### S5 — Flip the percent/double consumers (gated on S0)
Repoint `warhead_type.rs:118 CellSpread`/`:122 PercentAtMax` (`get_f32`) and any `get_percent` caller to `read_double`. **Reviewer correction:** `Verses` is NOT a `get_f32` caller — it uses `parse_verses` (`:197`), which already handles trailing `%`; fold `parse_verses` into the shared comma-tokenize + `read_double` path so its `%`-anywhere / atoi-leniency edges also match. Player-visibility is MODERATE (a `%`-suffixed `PercentAtMax` is uncommon in stock), not the "every damage calc" the draft claimed.
**Acceptance:** `test_warhead_percentatmax_percent_parity` — a warhead with `PercentAtMax=100%` resolves identically to `PercentAtMax=1.0` after the `read_double` path; `test_verses_percent_anywhere` — `Verses=10%0` (% not trailing) resolves to the gamemd value; `/disparity-scan combat` shows no warhead drift.

### S6 — Retire old accessors + global parity gate
Delete `get_i32`/`get_bool`/`get_percent` (keep `get`/`get_list`/`get_values` as raw helpers if still used by non-typed callers). Run the **global parity harness** (deterministic replay vs baseline, per `Slice 8 T6`); the state hash for a stock skirmish replay must be unchanged unless a fix in S4/S5 corrected a real drift (in which case re-baseline with a documented reason).
**Acceptance:** `cargo test -p vera20k` green; global replay parity harness passes (or re-baselined with a one-line cited reason per changed value).

### S7 — Value-transforming accessors (NEW, Pass 2): ReadSpeed / ReadRange / ReadColorRGB
Two accessors do NOT just read — they transform, and the transform IS the observable contract:
- `read_speed(section,key,default)` — `read_int(default=-1)`; `-1`→default; else `min(v,100)`, `(v<<8)/100` (round-toward-zero), `min(result,255)`. (P19)
- `read_range(section,key,default)` — `read_double(default=-1.0)`; `==-1.0`→default; else **truncate toward zero** to i32 (the ftol gate). (P20)
- `read_color_rgb(section,key,default_rgb)` — comma `"%d,%d,%d"` (plain `%d`, NOT atoi-lenient/hex) → `[u8;3]`. (P21)
**Acceptance:** `test_read_speed_clamp` (`100→255`, `50→128`, `7→17`, absent→default); `test_read_range_truncates` (`5.9→5`, never rounds); `test_read_color_rgb` (`"12,34,56"→[12,34,56]`, miss→default). Any current Rust consumer that reads a `Speed=`/`Range=` key as a raw int/double MUST be repointed (audit in S4/S5).

---

## Pass 2 — Expansion (verify-and-expand, 2026-06-04)

Pass 1 was time-boxed; Pass 2 re-decompiled the **entire** accessor family live and ran the consumer/global sweep. Everything below was read out of Ghidra THIS run.

### A. Gate resolutions (JOB A)

| Gate | Verdict | Evidence (this run) |
|---|---|---|
| 128-char buffer behavior | **VERIFIED — corrected to PER-ACCESSOR caps, NOT a flat 128.** ReadString has no built-in cap; size is the caller's `param_6`. Caps: **32** (enum/MovementZone/Action), **64** (Read3Int/ReadMinMax/ReadPoint/ReadRect/ReadColorRGB), **128** (ReadCLSID-ANSI+128-WCHAR / ReadSpeedType / ReadLayer / ReadSoundList / ReadGeneral). Over-length → silent `_strncpy` truncation + force-NUL at `buf[size-1]`, THEN strtrim — so a mid-token truncation silently alters a comma-list member. | `decompile_function 0x00528A10` (`_strncpy(param_5,...,param_6); pcVar3[param_6-1]='\0'`); per-accessor `0x40`/`0x80`/`0x20` strncpy sizes read from each accessor body |
| ReadMovementZone 0x00474E40 | **VERIFIED** (was DOC-ONLY). 32-byte buf, case-insensitive scan, miss=-1, 13-entry table `0x0081BA88..0x0081BABC`. | `decompile_function 0x00474E40` |
| ReadMinMax 0x00529880 | **VERIFIED** (was DOC-ONLY). 64-byte buf, `"%d,%d"`. | `decompile_function 0x00529880` |
| ReadCLSID 0x00527920 | **VERIFIED** (was DOC-ONLY). 128-ANSI/128-WCHAR, `MultiByteToWideChar`→`CLSIDFromString`. | `decompile_function 0x00527920` |
| Read3Int/ReadInt3 0x00529CA0 | **VERIFIED** (was DOC-ONLY). 64-byte buf, `"%d,%d,%d"`. | `decompile_function 0x00529CA0` |
| ReadPoint/ReadSize 0x00529A30 | **VERIFIED** (was task-anchor). 64-byte buf, COMMA `"%d,%d"`. | `decompile_function 0x00529A30`; `read_memory 0x0081C000` |
| ReadRect 0x00527F20 | **VERIFIED** (was task-anchor). 64-byte buf, COMMA `"%d,%d,%d,%d"`, default literal `"0,0,0,0"`. | `decompile_function 0x00527F20`; `read_memory 0x00825bbc`/`0x00825bc8` |
| ReadLayer 0x00477050 | **VERIFIED** (was task-anchor). 128-byte buf, `Layer_From_Name`, default `param_3` on miss. | `decompile_function 0x00477050` |
| ReadSpeedType 0x00476FC0 | **VERIFIED** (was DOC-ONLY). 128-byte buf, `SpeedType__FromName`, default `param_3`. | `decompile_function 0x00476FC0` |
| ReadSoundList 0x00525430 | **VERIFIED** (was DOC-ONLY). 128-byte buf, strtok `,`, VocClass, vector cap 10. | `decompile_function 0x00525430`; `read_memory 0x00817f70`=`","` |
| ReadAction 0x00474EE0 | **VERIFIED** (was DOC-ONLY). 32-byte buf, 73-entry table `0x007E4C50..0x007E4D74`, miss=0. | `decompile_function 0x00474EE0` |
| strtrim 0x00727CF0 thresholds | **VERIFIED** (was DOC-ONLY). Both ends strip bytes ≤0x20 (lead break `0x20 < byte`; trail zero while `byte < 0x21`). | `decompile_function 0x00727CF0` |
| ReadGeneral 0x0066D530 | **VERIFIED** (was task-anchor). 128-byte buf, strtok `,`, FindOrAllocate→DynamicVector. | `decompile_function 0x0066D530` |
| $xx/xxh hex (ReadInt) | **VERIFIED (re-confirmed live).** `$`→`"$%x"`(`0x00825BB8`); `tolower(last)==0x68 'h'`→`"%xh"`(`0x00825BB4`); else `atoi`. tolower helper `FUN_007caff4` is ASCII `A-Z+0x20` (case-insensitive `h`). | `decompile_function 0x005276D0`, `0x007caff4`; `read_memory 0x00825BB8`/`0x00825BB4` |
| trailing-%% ⇒ *0.01 (ReadDouble) | **VERIFIED (re-confirmed live).** `strchr(value, '%')` matches `%` ANYWHERE (the `0x2525` arg truncates to byte `0x25`); ×`0.01`(`0x007E3808`); value narrowed via `(double)(float)`. | `decompile_function 0x005283D0`; `read_memory 0x007E3808`=`0.01` |
| first-char T/Y/1 bool (ReadBool) | **VERIFIED (re-confirmed live).** `toupper(*value)` switch: `0x30/0x46/0x4e`→0, `0x31/0x54/0x59`→1, else default. | `decompile_function 0x005295F0` |
| Rust ad-hoc parse re-impls | **MAPPED.** NO hex parse exists anywhere in `src/rules/` (no `from_str_radix`/`strip_prefix('$')`/`ends_with("h")`) → R2 gap is total. Trailing-only percent re-impls: `ini_parser.rs:102` (`get_percent` `strip_suffix('%')`), `terrain_rules.rs:354` (`trim_end_matches('%')`), `warhead_type.rs:201` + `:218` (`strip_suffix('%')`). `get_bool` (`:114`) whole-word, no first-char. 192 raw `parse::`/`0x`/`%` occurrences across 19 `src/rules/*.rs` files (heaviest: `weapon_type.rs`=52, `projectile_type.rs`=37, `warhead_type.rs`=26). | Grep `src/rules/`; Read `ini_parser.rs:60-134` |

### B. NEW items the sweep found (not in Pass 1 inventory)

| Item | Address | Role / parity note | Status |
|---|---|---|---|
| `CCINIClass__ReadColorRGB` | `0x00474B50` | typed accessor → `[u8;3]` from `"%d,%d,%d"`; used by ReadAudioVisual, ObjectType, ParticleType/System, WeaponType, TerrainType. NOT in Pass-1 inventory. | VERIFIED (B) → §2a row + P21 + S7 |
| `CCINIClass__ReadSpeed` | `0x00474810` | typed accessor with **value transform**: `ReadInt(-1)` → clamp100 → `(v<<8)/100` trunc → clamp255. `Speed=` keys. | VERIFIED → P19 + S7 |
| `CCINIClass__ReadRange` | `0x00474620` | typed accessor with **ftol-truncate transform**: `ReadDouble(-1.0)` → `Math__ftol` (round-toward-zero). `Range=`/`MinimumRange=` keys. | VERIFIED → P20 + S7 |
| `Math__ftol` (consumed by ReadRange) | `0x007c5f00` | the project-wide truncate-toward-zero float→int (CW `0x00822d80`=`0x0E7F`, RC=11). ReadDouble itself does NO ftol; ReadRange does. | VERIFIED (Phase 1 + this run via ReadRange) |
| `FUN_007caff4` (tolower) | `0x007caff4` | ASCII tolower used by ReadInt's `h`-suffix check → hex suffix is case-insensitive. | VERIFIED |
| `FUN_007c8d20` (_stricmp) | `0x007c8d20` | case-insensitive whole-string compare used by ALL enum-by-name helpers (Foundation/MovementZone/Action). | VERIFIED |
| `&DAT_00889f64` empty-default sentinel | `0x00889F64` | the `""` default many accessors pass; on miss with this default, ReadString returns `""` (len 0), and enum/list readers then take their own table/empty default. | seen inline (ReadSoundList/ReadGeneral/Read3Int) |
| ReadRect default literal `"0,0,0,0"` | `0x00825BC8` | seeds the sscanf so missing rect fields keep `0`. | VERIFIED `read_memory 0x00825BC8` |
| strtok delimiter `","` | `0x00817F70` | shared by ReadSoundList + ReadGeneral. | VERIFIED `read_memory 0x00817F70` |

### C. Consumer sweep (get_function_callers / get_xrefs_to)

- **ReadInt callers** (`get_function_callers 0x005276D0`): every `*TypeClass__ReadINI` (Aircraft/Anim/Building/Bullet/Infantry/Object/ParticleType/ParticleSystem/Superweapon/Techno/Terrain/Tiberium/VoxelAnim/Warhead/Weapon/HouseType), plus `RulesClass__Read{General,AudioVisual,CombatDamage,CrateRules,Difficulty,Elevation,IQ,Radiation,…}`, `OptionsClass__ReadFromINI`, `HouseClass__Read_Scenario_INI`, `Read_Map_Section_And_IsoMapPacks`, `Read_Theater_TileSets_INI`, and the thin wrappers `ReadSpeed`/`ReadRange`. → the parse contract feeds the entire type-load surface.
- **ReadDouble callers** (`get_function_callers 0x005283D0`): same TypeClass set + `MissionControlClass__Read_INI`, `ScenarioClass__Read_INI_Basic`, `Voc/Vox/VoxelAnim ReadINI`, and `WarheadTypeClass__ReadINI_Body 0x0075d3a0` (the Verses host — but Verses itself bypasses ReadDouble, see below).
- **ReadColorRGB callers** (`get_xrefs_to 0x00474B50`): `RulesClass__Process`, `RulesClass__ReadAudioVisual` (×4), `RulesClass__ReadRadiation`, `ObjectTypeClass__ReadINI` (×2), `ParticleType`/`ParticleSystemType`, `TerrainTypeClass__ReadINI_Full`, `WeaponTypeClass__ReadINI` (×3) — broad, load-bearing.

### D. Verses cross-family fold-in (CORROBORATED this run)

`disassemble_function 0x0075d590` shows the Verses path directly:
`ReadString(...,0x80, default=0x00847c40)` → fixed 11-token loop. The missing
fallback is eleven `100%%` tokens; present trimmed-empty skips the loop; other
inputs are bounded to 127 payload bytes then tokenized by native `strtok`, which
collapses empty fields. Each token uses `strtod` full f64 without `%`, or
`atoi * 0.01` with `%`, and stores f64 at `WarheadTypeClass+0xA0` stride 8. A
nonempty list that exhausts tokens before iteration 11 faults in `strchr(NULL,
'%')`. **Verses does not route through ReadDouble**; Rust must preserve this
separate precision, buffer, default, token-count, and termination contract.

### E. Re-applied burden of proof to own doc

- The §3 / §5 claim "no stock value exceeds the buffer" stays **UNCHECKED** (not downgraded) — not proven across the full corpus; the binding cap is now known to be as low as **32** (enum/zone/action), tightening the risk window. Surfaced, not triaged.
- S0 ReadDouble→SimFixed precision stays **BLOCKING/UNCHECKED** for the Rust side: the binary arithmetic is fully pinned (`(double)(float)x ×0.01`), but the Rust f32→×0.01→SimFixed bit-identity is unproven — next query is the boundary-spanning test in S0, not more Ghidra.
- No claim previously marked "equivalent/internal-only" was found that lacks proof except the CRC-store-is-equivalent claim (§3 INACTIVE) — that one is genuinely a storage-mechanism swap whose OUTPUT (which value a key resolves to) is identical for non-colliding stock keys; it stays INTERNAL-ONLY but is explicitly scoped to "no CRC collision in stock data" (a real, bounded condition), not hand-waved.

### F. Remaining UNCHECKED / blocking after Pass 2

- **S0 ReadDouble→SimFixed bit-identity** (BLOCKING for percent/Verses consumers). Next: the boundary-spanning Rust test in S0/S5.
- **P5 buffer truncation across full corpus** (low-risk, smallest cap 32). Next: scan stock `rulesmd.ini`/`artmd.ini` for any enum/zone/action value > 31 chars and any 128-cap list value > 127 chars.
- **INIClass field offsets (§2b)** remain DOC-ONLY (not load-bearing for the parse contract; the Rust port replaces the store with a `HashMap`, so these never need reproduction).

---

## 9. Sources & verification ledger

### Verified LIVE this session (Ghidra MCP `decompile_function`)
- `0x005276D0` CCINIClass::ReadInt — `$`→`"$%x"` (`0x00825BB8`); `tolower(last char)==0x68`→`"%xh"` (`0x00825BB4`); else `atoi`; default on null section/key. (P1–P4)
- `0x005295F0` CCINIClass::ReadBool — `switch(toupper(*value))`: `0x31/0x54/0x59`→1, `0x30/0x46/0x4e`→0, default. (P6)
- `0x005283D0` CCINIClass::ReadDouble — `sscanf "%f"` (`0x00825BD8`) into float; `(double)(float)`; `strchr(value,'%')`→`× _g_ImpassableSpeedThreshold_0_01` (`0x007E3808` = 0.01). (P7)
- `0x00528A10` CCINIClass::ReadString — null/size guards; `strncpy`; `buf[size-1]='\0'`; `strtrim()`; return strlen; default-on-miss path. (P5)
- `FUN_00474DA0` enum-by-name helper — `ReadString(buf=0x20, default=table[idx].name)` then linear case-insensitive compare against `{name,id}` table @`0x0081b9d8` (stride 2); return id at `0x0081b9dc`; default `return 0`. (P10)

### Re-verified LIVE this session by the adversarial reviewer (Ghidra MCP)
- `0x005276D0` ReadInt, `0x005295F0` ReadBool, `0x005283D0` ReadDouble, `0x00528A10` ReadString, `0x00474DA0` enum helper — all re-decompiled; parse semantics confirmed as stated in §2a/§5.
- `read_memory 0x00825BB8`=`"$%x"`, `0x00825BB4`=`"%xh"`, `0x008189B0`=`"%d,%d,%d"`, `0x0081C000`=`"%d,%d"`, `0x00825bbc`=`"%d,%d,%d,%d"`, `0x00825bc8`=`"0,0,0,0"`, `0x007E3808`= IEEE-754 double `0.01`. All confirmed.
- `0x00529a30` (ReadPoint/Size) and `0x00527f20` (ReadRect) re-decompiled — **both COMMA-delimited** (draft's "space" claim corrected).
- **strchr nuance (verified):** ReadDouble calls `strchr(value, 0x2525)`. `strchr` truncates the int arg to a `char`, so it searches for byte `0x25` = `'%'` — functionally "contains any `%`", as stated. The `0x2525` literal is harmless. A Rust port must match on `'%'` anywhere, not just trailing.
- **Rust:** `warhead_type.rs:115` `Verses` via `parse_verses` (`:197`, NOT `get_f32`); `:118` `CellSpread`, `:122` `PercentAtMax` via `get_f32`. `app_init_helpers.rs:247-271` merge order base-then-md confirmed.

### Promoted DOC-ONLY → VERIFIED LIVE this session (Pass 2)
- Read3Int `0x00529CA0` (64-byte buf, `"%d,%d,%d"`), ReadMinMax `0x00529880` (64-byte buf, `"%d,%d"`), ReadCLSID `0x00527920` (128-ANSI/128-WCHAR → `CLSIDFromString`), ReadSoundList `0x00525430` (128-byte buf, strtok `,`, vector cap 10), ReadSpeedType `0x00476FC0` (128-byte buf), ReadMovementZone `0x00474E40` (32-byte buf, miss=-1, 13-entry table), ReadAction `0x00474EE0` (32-byte buf, miss=0, 73-entry table), ReadLayer `0x00477050` (128-byte buf), ReadGeneral `0x0066D530` (128-byte buf, strtok `,`, FindOrAllocate). **All re-decompiled this run** (citations in §Pass-2 A).
- strtrim `0x00727CF0` (both ends strip bytes ≤0x20) — **VERIFIED `decompile_function 0x00727CF0`**.
- Format strings `0x008189B0` `"%d,%d,%d"`, `0x0081C000` `"%d,%d"`, `0x00825BBC` `"%d,%d,%d,%d"`, `0x00825BC8` `"0,0,0,0"`, `0x00817F70` `","`; tables `0x0081BA88..0x0081BABC` (MovementZone, 13), `0x007E4C50..0x007E4D74` (Action, 73), `0x0081B9D8..0x0081BA88` (enum-by-name) — **all VERIFIED via read_memory / loop bounds this run**.

### NEW this session (Pass 2 — not in any prior doc)
- `CCINIClass__ReadColorRGB 0x00474B50` (64-byte buf, `"%d,%d,%d"`→`[u8;3]`).
- `CCINIClass__ReadSpeed 0x00474810` (ReadInt(-1) → clamp100 → `(v<<8)/100` trunc → clamp255).
- `CCINIClass__ReadRange 0x00474620` (ReadDouble(-1.0) → `Math__ftol` truncate).
- `FUN_007caff4` ASCII tolower (ReadInt `h`-suffix case-insensitivity); `FUN_007c8d20` `_stricmp` (enum compare).

### DOC-ONLY remaining (NOT load-bearing for the parse contract)
- FindSection `0x0052B620`, FindEntry `0x0052B4F0`, FindSectionCached `0x0052B390`; ctor `0x00535B30`/`0x00535AA0`; dtor `0x005256F0`.
- INIClass/INISection/INIEntry field offsets (§2b) — **all offsets DOC-ONLY**; the Rust port replaces the store with a `HashMap`, so these never need reproduction.

### Task-anchor / other-doc sourced (NOT verified this session)
- ReadPoint/ReadSize `FUN_00529a30`, ReadRect `0x00527f20`. **VERIFIED LIVE this session (reviewer):** both COMMA-delimited — `"%d,%d"` (`0x0081C000`) / `"%d,%d,%d,%d"` (`0x00825bbc`). The original draft's `"%d %d"` space claim was WRONG; corrected. (P9)
- RulesClass::ReadGeneral `0x0066D530` (task anchor). (P12)
- `reference_mission_control_ini_reset_per_entry.md` (reset-per-entry / 32 slots / AARate copies Rate). (P10/P17)
- `building-selection-brackets/FOUNDATION_PARSER_TABLE_BRACKET_EXTENTS_GHIDRA_REPORT.md`, `LAYER_ENUM_*`, `BULLETTYPECLASS_GHIDRA_REPORT.md §4`, `UIMD_ART800_LOADER_*`, `BRIDGEEXPLOSIONS_RULES_OFFSETS_*` (family ReadINI patterns; DOC).

### Rust evidence (this session, grep/read)
- `src/rules/ini_parser.rs` (the parser; `get_i32:70`, `get_f32:77`, `get_light_f32:86`, `get_percent:100`, `get_bool:114`, `get_list:128`, `get_values:142`, `merge:304`).
- `src/rules/ini_parser_tests.rs` (current coverage — NO hex/first-char-bool/double-percent/atoi-leniency tests).
- `src/rules/foundation.rs` (matches gamemd default-0).
- `src/rules/object_type.rs:58/86/115/1097` (inline enum matches).
- `src/rules/warhead_type.rs:118/122` (`CellSpread`/`PercentAtMax` via `get_f32`, no percent). `:115/197` `Verses` via dedicated `parse_verses` (DOES handle trailing `%` — the draft's "Verses via get_f32, 100× wrong" claim was FALSE; corrected by reviewer).
- `src/rules/ruleset.rs` (consumer `from_ini` patterns; `merge`/`merge_art_data` load path).
- Grep counts: 852 `get_*`/`.get(` accessor calls across 26 files; 85 raw `parse::`/`strip_suffix`/`from_str_radix`/`to_lowercase` re-implementations across 17 files.

### UNCHECKED / blocking-gate items
- **S0 ReadDouble precision** — the `(double)(float)x ×0.01` round-trip vs Rust f32/f64→`SimFixed` is unproven bit-identical; BLOCKING for percent consumers (Verses/PercentAtMax). DRIFT until pinned.
- **P5 buffer truncation** — no stock value is *proven* ≤128 chars across the full corpus; UNCHECKED (low risk, surfaced not triaged).
- **P9 Point/Size/Rect addresses** — ~~task-anchor/DOC~~ **RESOLVED this session (reviewer):** `decompile_function 0x00529a30` and `0x00527f20` both re-decompiled; format strings read out of memory. Delimiter is **COMMA** (`"%d,%d"` / `"%d,%d,%d,%d"`), NOT space — the draft was WRONG and is now corrected throughout (§2a, P9, §4.3, §6.2, S1). No longer blocking.
- **R9 load order** — ~~confirm before trusting P14~~ **RESOLVED this session (reviewer):** `app_init_helpers.rs:247-271` (`load_rules_ini`) loads `rules.ini` as base (step 1), then merges `rulesmd.ini` on top (step 2). Order is correct (base-then-md). P14 confirmed. *(grep `src/app_init_helpers.rs:247,262-271`.)*

---

## Reviewer follow-ups (adversarial audit, 2026-06-04)

**Verdict: YELLOW — two factual errors patched, design otherwise sound.**

Corrections applied in place (all cited above):
1. **P9 delimiter was WRONG (major).** Draft claimed Point/Size/Rect use space-separated `"%d %d"`; verified COMMA (`"%d,%d"` / `"%d,%d,%d,%d"`). Corrected in §2a, §1 R6, §3, §4.1 R6, §4.3, §6.2, S1, §9. The §6 implementation guidance was inverted ("must not reuse `get_list`") — now says Point/Rect SHOULD reuse comma tokenization.
2. **`Verses` "100× wrong" headline was FALSE (major).** `Verses` uses `parse_verses` (handles trailing `%`), not `get_f32`. The real `%` gap is `PercentAtMax` only (uncommon). Severity downgraded from "every damage calc" to MODERATE. Corrected in §1, §4.1 R4, §5/S5, §7, §9.

Residual UNCHECKED (carry to synthesis):
- **S0 ReadDouble precision (still BLOCKING)** — `(double)(float)sscanf("%f") × 0.01(double)` round-trip vs Rust f64→SimFixed not proven bit-identical. The binary arithmetic is fully pinned (`0x005283D0`, ×`0x007E3808`=0.01); the Rust-side fixed conversion is NOT pinned. Still gates S5. (Next step is the S0 boundary test, not more Ghidra.)
- **P5 buffer truncation** — still unproven that no stock value exceeds its accessor's cap; the binding cap is now known to be as low as **32** (enum/zone/action), tightening the window. Low risk, surfaced not triaged.

---

## Pass 2 follow-up (verify-and-EXPAND, 2026-06-04) — supersedes the reviewer DOC-ONLY list above

**Verdict: GREEN — the entire accessor family is now bit-VERIFIED live.** The reviewer's "Residual DOC-ONLY" list above is **RESOLVED**: Read3Int, ReadMinMax, ReadCLSID, ReadSoundList, ReadSpeedType, ReadLayer, ReadGeneral, ReadMovementZone, ReadAction were all re-decompiled THIS run and promoted to VERIFIED (§Pass-2 A + §9). The only remaining DOC-ONLY items are the INIClass field offsets / ctor / dtor (§2b), which are NOT part of the parse contract (the Rust port replaces the store with a `HashMap`).

What Pass 2 added beyond gate-closing:
- **NEW accessors:** `ReadColorRGB 0x00474B50`, `ReadSpeed 0x00474810`, `ReadRange 0x00474620` — the latter two **transform** the value (clamp/scale, ftol-truncate) so the transform is the contract (P19/P20/P21, S7).
- **Buffer caps corrected** from a flat 128 to per-accessor 32/64/128 (P5, §3).
- **ftol gate** wired in via ReadRange (`Math__ftol 0x007c5f00`, truncate-toward-zero); **Verses double-precision split** corroborated directly in `WarheadTypeClass__ReadINI 0x0075d590` (P7 fold-in, §Pass-2 D).
- **Rust gap mapped:** zero hex parsing exists in `src/rules/`; percent re-impls are all trailing-only at 4 sites (§Pass-2 A last row).

Still GREEN/unchanged: ReadInt/ReadBool/ReadDouble/ReadString parse rules, all format-string bytes, the `0.01` constant, strchr-`'%'`-anywhere, enum-helper case-insensitivity — all re-confirmed live this run.
