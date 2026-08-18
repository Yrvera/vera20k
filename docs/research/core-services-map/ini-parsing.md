# Core Service Profile — INI parsing helpers (CCINIClass / INIClass accessors)

**Slug:** `ini-parsing`
**Primary doc:** `docs/research/INI_PARSING_HELPERS_ENGINE_SUBSTRATE_SERVICE_STUDY.md` (STUDY+DESIGN, Pass 2 fully live-verified 2026-06-04)
**Layer:** data / load-time substrate (sits in `rules/`, below `sim/`). NOT a tick system.
**Date:** 2026-06-25

> **2026-07-13 correction:** active-binary rechecks add the explicit generic
> ReadDouble f64 stores (`disassemble_function 0x005283d0`,
> `0x0052855d..0x00528569` and `0x0052857a..0x00528584`) and the Warhead Verses
> call-site contract (`disassemble_function 0x0075d590`; `ReadString` cap 0x80,
> eleven-`100%%` missing fallback, present-empty skip, fixed 11 stores, native
> `strtok` empty-token collapse, short-list null fault). See the corrected primary
> INI and Verses reports for full `read_memory 0x00847c40`, ReadString, and
> `strchr 0x007caf30` evidence.

---

## Purpose

CCINIClass is gamemd's typed INI reader, layered on INIClass (the raw CRC-hashed
section/entry store). It resolves a `(section, key, default)` triple into a typed value
(int / bool / double / string / 2-int / 3-int / point / rect / RGB / CLSID / sound-list /
enum-by-name / value-transformed Speed/Range). **Its only observable output is the
resolved value** — that value becomes a unit stat, build time, damage multiplier,
foundation extent, or facing byte, so a wrong percent/hex/bool/atoi parse silently shifts
every dependent stat to the last decimal (default verdict for any parse difference =
DRIFT). This is the load-time data substrate that feeds every other service's `ReadINI`.

---

## Owns

- **Typed value resolution by (section, key)** with default-on-*miss* (not on parse-fail)
  for every scalar/tuple/enum form.
- **The parse rules themselves** (the load-bearing contract):
  - hex int `$xx` (prefix) / `xxh` (suffix, case-insensitive), else C-`atoi` leniency
  - bool by first char only: `toupper(c0)` ∈ {1,T,Y}→true / {0,F,N}→false / else default
  - double `"%f"` (single-precision narrow), ×0.01 iff value contains `%` anywhere
  - string strncpy(default-on-miss) → force-NUL at `buf[size-1]` → strtrim ≤0x20 both ends
  - comma-tokenized tuples (3int / minmax / point / rect / RGB) — all COMMA-delimited
  - enum-by-name case-insensitive whole-string table match, table-default on miss
  - value transforms: ReadSpeed `(v<<8)/100` clamp; ReadRange ftol-truncate
- **Per-accessor fixed buffer caps** (C-ABI artifact, Rust replaces with owned String): 32
  (enum/MovementZone/Action), 64 (3int/minmax/point/rect/RGB), 128
  (CLSID/SpeedType/Layer/SoundList/ReadGeneral).
- **Storage internals (INIClass)** — NOT observable, Rust replaces with HashMap: CRC-32
  hashed `{crc,ptr}` sorted arrays, lazy qsort, binary search, one-entry pointer-identity
  section cache. INIClass field offsets at `+0x04..+0x3C` (DOC-ONLY, not load-bearing).
- **Global INI instances:** merged rules CCINIClass (rules.ini+rulesmd.ini), merged art
  CCINIClass (art.ini+artmd.ini), per-map CCINIClass, scenario CCINIClass.
- **Merge order:** rulesmd patches rules (later-wins, additive, case-insensitive); same for
  art/artmd. Reset-per-entry MissionControl defaults (32 slots, AARate absent/0 copies
  Rate).

---

## Key functions & globals (addresses)

Typed accessors (all VERIFIED live, Pass 2):
- `ReadInt 0x005276D0` — `$`→`"$%x"`(`0x00825BB8`); tolower(last)=='h'→`"%xh"`(`0x00825BB4`); else `atoi`
- `ReadBool 0x005295F0` — switch(toupper(c0)): 0x31/0x54/0x59→1, 0x30/0x46/0x4e→0, else default
- `ReadDouble 0x005283D0` — sscanf `"%f"`(`0x00825BD8`)→(double)(float); ×0.01(`0x007E3808`) iff strchr(v,'%')
- `ReadString 0x00528A10` — strncpy → `buf[size-1]='\0'` → `strtrim 0x00727CF0` → strlen
- `Read3Int 0x00529CA0` (`"%d,%d,%d"` `0x008189B0`), `ReadMinMax 0x00529880` (`"%d,%d"` `0x0081C000`)
- `ReadPoint/Size 0x00529A30` (`"%d,%d"`), `ReadRect 0x00527F20` (`"%d,%d,%d,%d"` `0x00825BBC`, default `"0,0,0,0"` `0x00825BC8`)
- `ReadColorRGB 0x00474B50` (NEW) — `"%d,%d,%d"`→[u8;3] (plain `%d`, not atoi/hex)
- `ReadSpeed 0x00474810` (NEW, TRANSFORM) — ReadInt(-1)→clamp100→`(v<<8)/100` trunc→clamp255
- `ReadRange 0x00474620` (NEW, TRANSFORM) — ReadDouble(-1.0)→`Math__ftol` truncate-toward-zero
- `ReadCLSID 0x00527920` (MultiByteToWideChar→CLSIDFromString), `ReadSoundList 0x00525430` (strtok→VocClass)
- `ReadSpeedType 0x00476FC0`, `ReadMovementZone 0x00474E40` (miss=-1, 13-tbl), `ReadAction 0x00474EE0` (miss=0, 73-tbl), `ReadLayer 0x00477050`
- `enum-by-name helper FUN_00474DA0` — ReadString(default=table[idx].name)→`_stricmp FUN_007c8d20` scan → id, default 0
- `RulesClass::ReadGeneral 0x0066D530` — strtok `,` → FindOrAllocate per token → DynamicVectorClass

Storage core (INIClass, DOC-ONLY internals): `FindSection 0x0052B620`, `FindEntry 0x0052B4F0`,
`FindSectionCached 0x0052B390`, ctor `0x00535B30`/`0x00535AA0`, dtor `0x005256F0`,
`strtrim 0x00727CF0`, `tolower FUN_007caff4`.

Globals/constants: `0.01` @ `0x007E3808`; strtok `","` @ `0x00817F70`; empty-default
sentinel `&DAT_00889f64` @ `0x00889F64`; enum table `0x0081B9D8..0x0081BA88`; MovementZone
table `0x0081BA88..0x0081BABC` (13); Action table `0x007E4C50..0x007E4D74` (73).

---

## Tick / render position

**N/A — load-time substrate, runs entirely during init, not in the per-tick LogicClass
spine and not in the render pass.** It has no RNG stream and no per-tick timer; the
"RNG/timer visibility" parity axis collapses to "the parsed value is bit-identical." Its
only indirect tick relevance: a changed parsed value changes a unit stat, which changes
`state_hash` — so the deterministic-replay parity harness is the end gate, not a per-tick
assertion.

---

## Depends-on (outgoing edges)

- **`lookup-tables`** — via the static read-only enum/format-string tables this service
  reads at parse time: enum-by-name table `0x0081B9D8..0x0081BA88` (Foundation etc.),
  MovementZone table `0x0081BA88..0x0081BABC`, Action table `0x007E4C50..0x007E4D74`,
  format strings (`"$%x"`,`"%xh"`,`"%f"`,`"%d,%d…"`), and the `0.01` constant
  `0x007E3808`. Evidence: `FUN_00474DA0`/`ReadMovementZone`/`ReadAction` linear-scan these
  tables; `read_memory` of each address (doc §2c, Pass-2 §A).
- **`damage-helpers`** — via `Math__ftol 0x007C5F00` (the project-wide truncate-toward-zero
  float→int kernel, control word `0x00822D80`=`0x0E7F`, RC=11). `ReadRange 0x00474620`
  calls it directly to truncate the parsed double; this is the same ftol kernel the damage
  multiplier order and other systems consume (`GATE_DAMAGE_COUNTRY_ARMOR_ORDER...md`,
  `ZONE_ESTIMATE_SLOPE_COST...md`, `GGI_GHIDRA_REPORT.md §8.3`). Evidence:
  `decompile_function 0x00474620` (doc P20, Pass-2 §B); shared-kernel corroboration via
  `research_search Math__ftol`. NOTE: `ReadDouble` itself does NO ftol — only `ReadRange`
  does; the truncation kernel is a shared helper, classified here under damage-helpers as
  the canonical owner of that kernel.

(No outgoing edges to `logicclass`, `techno-foot`, `cell-map`, `factory-house`,
`pathfinding-helpers`, `target-scoring`, `drawing-helpers`, etc. — those are all CONSUMERS
of this service, i.e. incoming edges. The parser does not call up into any sim/render
service; it is the bottom of the data layer. `ReadSoundList`→VocClass and
`ReadGeneral`→FindOrAllocate touch type-registry build helpers that live in the same
rules/asset layer, not in a separate canonical service slug.)

---

## Used-by (incoming edges)

This service is read by essentially the **entire type-load surface** — consumer sweep via
`get_function_callers 0x005276D0` (ReadInt), `0x005283D0` (ReadDouble), `get_xrefs_to
0x00474B50` (ReadColorRGB), doc §Pass-2 C:

- **`rules-class`** — via `RulesClass::Read{General,AudioVisual,CombatDamage,CrateRules,
  Difficulty,Elevation,IQ,Radiation,...}` and `RulesClass::Process`/`ReadGeneral
  0x0066D530`. RulesClass is the largest consumer; every `[General]`/`[CombatDamage]`/
  `[AudioVisual]` tunable flows through these accessors. Evidence: ReadInt/ReadDouble/
  ReadColorRGB caller lists (doc §C).
- **`techno-foot`** — via every `*TypeClass::ReadINI` (Aircraft/Building/Infantry/Object/
  Techno/Unit type loads): Speed via `ReadSpeed`, MovementZone/SpeedType/Layer enum reads,
  Range/MinimumRange via `ReadRange`, Foundation via enum-by-name. Evidence: ReadInt/
  ReadSpeed/ReadRange/ReadMovementZone caller set (doc §1, §C).
- **`damage-helpers`** — via `WarheadTypeClass::ReadINI 0x0075d590` (Verses loop —
  note Verses BYPASSES ReadDouble and parses each token in full f64, but the host
  ReadINI uses ReadInt/ReadString from this service for the warhead's other fields).
  Evidence: doc §Pass-2 D, §C ReadDouble callers.
- **`factory-house`** — via `HouseClass::Read_Scenario_INI`, `HouseTypeClass::ReadINI`,
  and the build-time/economy tunables read through RulesClass. Evidence: ReadInt callers
  list (doc §C).
- **`random-scenario`** — via `ScenarioClass::Read_INI_Basic` and the scenario/map-section
  loaders (`Read_Map_Section_And_IsoMapPacks`, `Read_Theater_TileSets_INI`). Evidence:
  ReadDouble/ReadInt caller list (doc §C).
- **`cell-map`** — via map-section / theater-tileset loaders that build the cell grid from
  the per-map CCINIClass `[Header]`/`[Basic]`/IsoMapPack sections. Evidence:
  `Read_Map_Section_And_IsoMapPacks` in ReadInt callers (doc §C, §2d per-map instance).
- **`lookup-tables`** — bidirectional: ReadGeneral/ReadSoundList BUILD ordered
  type-registry vectors (`[InfantryTypes]`-style) via FindOrAllocate, populating the
  static-ish registry tables. Evidence: `ReadGeneral 0x0066D530` (doc P12).

(Effectively a hub dependency: nearly every other service depends on `ini-parsing` for its
load-time data, but `ini-parsing` depends on almost nothing above the data layer.)

Rust consumers (current): `src/rules/ini_parser.rs` (the INIClass analog —
`get_i32/get_f32/get_light_f32/get_percent/get_bool/get_list/get_values/merge`); merge
caller `src/app_init_helpers.rs:247-271` (`load_rules_ini`, base-then-md, CONFIRMED);
per-type `from_ini` in `object_type.rs`, `warhead_type.rs`, `foundation.rs`,
`terrain_rules.rs`, `weapon_type.rs`, etc. (852 accessor calls across 26 files).

---

## Open / unverified edges

- **S0 ReadDouble→SimFixed bit-identity (BLOCKING, UNCHECKED).** The binary arithmetic is
  fully pinned (f32 parse → f64 spill/reload → optional ×0.01 → f64
  spill/reload), but the Rust f32→f64-store→optional-scale→f64-store→SimFixed
  conversion is not proven bit-identical at the last ULP. Gates the
  percent/Verses consumer flip. Edge to `damage-helpers` (Verses/warhead) and
  `rules-class` (General percents) carries this risk. Next step: boundary-spanning Rust
  test, not more Ghidra.
- **P5 stock-cap occurrence (UNCHECKED; priority only).** The truncation mechanism
  is verified and binding; no stock value is yet proven to exceed its accessor's
  cap. Rust must still reproduce the cap for modded/edge inputs. Warhead Verses
  specifically binds at 0x80 bytes before tokenization.
- **`damage-helpers` ftol-kernel ownership** — `Math__ftol 0x007C5F00` is a cross-service
  shared kernel (damage, zone cost, build time, ReadRange). Assigned to `damage-helpers`
  here as the canonical owner, but it is genuinely a `lookup-tables`/util-substrate
  primitive; the edge is real either way (ReadRange calls it). Classification, not the
  existence of the edge, is the only ambiguity.
- **INIClass field offsets / ctor / dtor (§2b)** — DOC-ONLY, never re-verified live; NOT
  load-bearing (Rust replaces the store with HashMap). No cross-service edge depends on
  them.
