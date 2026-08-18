---
name: BuildingClass Prerequisite Evaluation
description: Storage format for Prerequisite/PrerequisiteOverride, CanBuild algorithm, categorical-token resolution, invalidation handling.
type: reference
---

# BuildingClass Prerequisites — Ghidra Research Report

**Date:** 2026-04-24
**Scope:** Task 11 of BuildingClass full-decode plan. RESEARCH ONLY.
**Confidence:** HIGH
**Active in YR:** Yes (tech-tree gating is a live, core skirmish system)

---

## 1. Overview

`Prerequisite=` and `PrerequisiteOverride=` are stored on the **TechnoTypeClass** base
(so BuildingTypeClass, InfantryTypeClass, UnitTypeClass, AircraftTypeClass all share the
same fields), not on BuildingTypeClass proper. Each is a `DynamicVectorClass<int>` whose
elements are **signed integers** — non-negative values are BuildingType array indices and
negative values are categorical keyword IDs (POWER/FACTORY/BARRACKS/RADAR/TECH/PROC).

Evaluation happens per-house in `HouseClass::CanBuild` (0x4F7870). `Prerequisite=` is
pure AND semantics (every entry must be satisfied). `PrerequisiteOverride=` short-circuits
the AND chain: if the house owns any building listed, the entire Prerequisite check is
bypassed.

No separate owned-count is maintained per prerequisite group. Instead, the engine keeps a
dense `int[]` of "owned count per BuildingType index" on the house, so the inner
`HouseClass::CountOwnedInstances` call is O(1).

Cross-side prerequisites work *implicitly* — the PrerequisiteBarracks list in [General]
includes all three barracks (NAHAND, GAPILE, YABRCK), so a captured enemy barracks
satisfies `BARRACKS`. There is no explicit ALLIED/SOVIET/THIRD keyword.

---

## 2. Storage (TechnoTypeClass offsets)

Both fields live on TechnoTypeClass and are inherited by BuildingTypeClass.

| Byte Offset | INI Key | Storage | Default | Confidence |
|---|---|---|---|---|
| `+0x638` | `Prerequisite=` | `DynamicVectorClass<int>` — 12 bytes {ptr, count, capacity} | empty vector | HIGH |
| `+0x654` | `PrerequisiteOverride=` | `DynamicVectorClass<int>` — 12 bytes {ptr, count, capacity} | empty vector | HIGH |

**Verification:** Parser at `TechnoTypeClass::ReadINI` (0x00712170):
- `0x007141A0: LEA EDI, [EBP + 0x638]` followed by key string `"Prerequisite"` at 0x843DA8
- `0x00714210: LEA EDI, [EBP + 0x654]` followed by key string `"PrerequisiteOverride"` at 0x843D90

Each element in the vector is a 4-byte signed int with one of these meanings:

| Value | Meaning |
|---|---|
| `>= 0` | BuildingTypeClass array index (direct building reference) |
| `-1` | POWER (group) |
| `-2` | FACTORY (group) |
| `-3` | BARRACKS (group) |
| `-4` | RADAR (group) |
| `-5` | TECH (group) |
| `-6` | PROC (group) |

Tokens that don't match any keyword and don't resolve to a known BuildingType name (via
`FUN_0045E7B0`) are silently dropped (the parser only appends if `local_84 != -1`).

**Per-house owned-count array** (on HouseClass, used by CountOwnedInstances):
- `+0x64` — pointer to `int[]` of per-BuildingType owned counts (growable)
- `+0x58` — current size
- `+0x5D` — sentinel/grow flag
- `+0x60` — allocated capacity

A parallel list of active BuildingClass instances of a filtered sub-kind lives at
`+0x6C/+0x78/+0x70` (append-on-Unlimbo), but that's a pointer list, not a count.
CountOwnedInstances returns the count array entry directly.

---

## 3. Evaluation Function — `HouseClass::CanBuild` (0x004F7870)

**Signature (via decompilation):**

```c
// __thiscall; args read via in_stack_*
int HouseClass::CanBuild(
    this,                            // ECX — HouseClass*
    TechnoTypeClass* type,           // first stack arg (int*)
    char skip_prereqs,               // second stack arg
    char allow_in_production         // third stack arg
)
```

**Returns:**

| Value | Meaning |
|---|---|
| `1` | Can build |
| `0` | Cannot build (prerequisite/tech/side/stolen/notbuildable failed) |
| `-1` | At BuildLimit but one instance is queued in a factory (sidebar greyed) |

**Step order (verified from decompilation at 0x4F7870, body 0x4F7870–0x4F8363, 2803 bytes):**

1. If `skip_prereqs != 0` → jump straight to BuildLimit handling (Step 10).
2. **NotBuildable check** — `type[0x326]` (char) at byte offset `0x326 * 4 = 0xC98`.
   (param is `int*`, so subscripts multiply by 4. `TechnoTypeClass+0xC98` = NotBuildable.)
3. **PrerequisiteOverride scan** — copies `type+0x654` vec locally (via
   `FUN_004779E0`), iterates and for each index calls `CountOwnedInstances`. If any
   returns `> 0`, skip to BuildLimit (Step 10). Otherwise continue to normal flow.
4. **TechLevel check:**
   - `type+0x634` (`type[0x18d]`) is the type TechLevel.
   - If `== -1` → return 0 (unbuildable).
   - If `> this+0x1D4` (house TechLevel) → return 0.
5. **Stolen tech check** — three bool fields on TechnoTypeClass:
   - `+0xD9D` `RequiresStolenAlliedTech` vs `this+0x2BE`
   - `+0xD9C` `RequiresStolenSovietTech` vs `this+0x2BD`
   - `+0xD9B` `RequiresStolenThirdTech` vs `this+0x2BC`
   - Also the secondary bool at `type[0x367]` (= `+0xD9C` again under decomp) / 0x2BD.
6. **RequiredHouses check** — `type+0xDA0` (`type[0x368]`). If `!= -1`:
   - `side_bit = 1 << this->Type[0xB8]` (CountryType self-index).
   - OK if `(mask & side_bit)` or RTTI-matched acquired-tech bit:
     - Aircraft (RTTI 0x10): `this+0x2C4` acquired mask
     - Buildings (0x28): `this+0x2C8`
     - Infantry (3): `this+0x2CC`
     - Vehicles (7): `this+0x2D0`
7. **ForbiddenHouses check** — `type+0xDA4` (`type[0x369]`). If `!= -1` and
   `(mask & side_bit) != 0` → return 0.
8. **Naval Slave Miner deploy special** — if RTTI is Vehicle (7) and
   `type+0x5BC != -1` (deploys-into a building), scans `RulesClass+0x920`
   (stolen-tech building list) to allow/deny the deploy.
9. **AI shortcut** — if the house is not the local player and not in single-player
   controlled mode, AI skips Prerequisite check entirely (but TechLevel/RequiredHouses
   already gated above).
10. **Prerequisite scan** (`type+0x638` copied locally, iterated):
    - Each entry drives a switch on the int value. Cases `-1..-6` do a group scan;
      default (>= 0) does a single-type match.
    - Group check: iterate `RulesClass` array `+count` pairs (see table below);
      for each BuildingTypeClass in the group, call `CountOwnedInstances(type_index)`.
      If any > 0, group is satisfied; move to next prereq.
    - Specific-index check: if `BuildingTypeClass+0xE88` (IsPowersUpBuilding) is 0,
      simply `CountOwnedInstances(index) > 0`. Otherwise, scan `this+0x6C` building
      list for any building whose upgrade slots (`+0x17B..+0x17D`) contain this
      BuildingType — the "upgrade attached to tech building" pattern.
    - **Any failed entry → return 0.**
11. **BuildLimit check** (`type+0x3B8`, labelled `type[0xEE]`) — switch on RTTI,
    compute owned count, apply the 0/positive/negative semantics documented in
    `OWNER_BITMASK_TECH_PREREQUISITE_SYSTEM.md`.

---

## 4. AND/OR Semantics

| Construct | Semantics |
|---|---|
| Multiple entries in `Prerequisite=` | **AND** — every entry must pass |
| Keyword entry (e.g., `POWER`, `FACTORY`) | **OR across group members** — owning any member satisfies the keyword |
| Specific building entry (e.g., `GAWEAP`) | Exact match — must own at least one of this type (or have it as an upgrade on another building) |
| `PrerequisiteOverride=` has any match | **Short-circuit OR** — entire `Prerequisite=` check bypassed |
| `|` (pipe) in INI string | **NOT parsed as a special operator.** The parser uses `strtok` with `","` as delimiter (at `DAT_00817F70`). Modders who want OR use keyword groups or `PrerequisiteOverride=`. |

The engine **never** interprets `Prerequisite=A|B|C`. A pipe-separated token becomes a
single string that fails BuildingType name lookup and is silently dropped. (Seen
occasionally in mods as a commented-out experiment.)

---

## 5. Categorical Token Table

All six group keywords resolve against RulesClass global arrays populated from
`[General]` by `RulesClass::ReadGeneral` (0x0066E400). String compares use
`_stricmp` (`FUN_007C8D20`) so case is irrelevant.

| Keyword | Encoded ID | RulesClass array offset | RulesClass count offset | Vanilla members (rulesmd.ini) | Meaning in YR |
|---|---|---|---|---|---|
| `POWER` | -1 (0xFFFFFFFF) | `+0x35C` | `+0x368` | GAPOWR, NAPOWR, NANRCT, YAPOWR | Any power plant |
| `FACTORY` | -2 (0xFFFFFFFE) | `+0x378` | `+0x384` | GAWEAP, NAWEAP, YAWEAP | Any war factory |
| `BARRACKS` | -3 (0xFFFFFFFD) | `+0x394` | `+0x3A0` | NAHAND, GAPILE, YABRCK | Any barracks |
| `RADAR` | -4 (0xFFFFFFFC) | `+0x3B0` | `+0x3BC` | GAAIRC, NARADR, AMRADR, NAPSIS | Allied airforce command OR Soviet/Yuri radar |
| `TECH` | -5 (0xFFFFFFFB) | `+0x3CC` | `+0x3D8` | GATECH, NATECH, YATECH | Any battle lab |
| `PROC` | -6 (0xFFFFFFFA) | `+0x3E8` | `+0x3F4` | GAREFN, NAREFN, YAREFN | Any refinery |
| *(PROC special)* | (handled inside -6 branch) | `+0x400` (ptr to TypeClass) | — | SMIN (Slave Miner) | `PrerequisiteProcAlternate`. Satisfies PROC if owned AND SMIN's `type+0xDF8` (deploy-building ref) count > 0 |

**There is no ALLIED, SOVIET, THIRD, HELIPAD, or SHIPYARD keyword.** Cross-side
gating happens implicitly — the BARRACKS group lists all three barracks, so capturing
any enemy barracks satisfies BARRACKS for any nationality.

Naval/aircraft dependency is expressed by listing the actual building (e.g.,
`Prerequisite=GAYARD` for a naval unit, `Prerequisite=GAAIRC` for an Allied aircraft).
See `OWNER_BITMASK_TECH_PREREQUISITE_SYSTEM.md` §6 for factory-pointer separation.

---

## 6. PrerequisiteOverride Semantics

**INI key:** `PrerequisiteOverride=`
**Storage:** TechnoTypeClass+0x654 (DynVec<int>)
**Parsing:** Same `Prerequisite_INI_Parser` (0x4770E0) as `Prerequisite=` — so keyword
tokens work identically.

**Behavior in CanBuild:**

```text
if PrerequisiteOverride vector is non-empty:
    for each entry:
        if CountOwnedInstances(entry_index) > 0:
            SKIP Prerequisite scan (step 10) — proceed to BuildLimit
    // no match → fall through to normal Prerequisite evaluation
```

Note: **PrerequisiteOverride replaces-if-satisfied**, it does not *add*. Owning any
override building bypasses all Prerequisite entries. If no override is owned, the
normal Prerequisite chain runs; it's additive in that sense only ("alternate path").

**Gotcha:** PrerequisiteOverride entries go through the same keyword/direct-index
resolver, so `PrerequisiteOverride=RADAR` is legal but rarely used (cleaner to list
specific buildings).

**Use case in vanilla:** Navy SEAL / Tanya variants use `PrerequisiteOverride=CAWA2A,
CAWA2B,CAWA2C,CAWA2D` (capturable Pentagon buildings on map) — owning any Pentagon
unlocks SEALs regardless of normal prereqs.

---

## 7. Aggregate Counters

**On HouseClass** (for O(1) ownership queries):

| Offset | Meaning |
|---|---|
| `+0x64` | `int*` — array of per-BuildingType owned counts, indexed by BuildingType array index |
| `+0x58` | current array size (grows on first access to a new type index) |
| `+0x5D` | grow-allowed flag |
| `+0x60` | allocated capacity |
| `+0x68` | vtable-ish growable container ops |
| `+0x6C` | `BuildingClass**` — pointer list of active owned buildings (for scans like upgrade checking) |
| `+0x70` | current active-building-list size |
| `+0x78` | list capacity |

`HouseClass::CountOwnedInstances(param_1 = this, param_2 = buildingtype_index)` at
0x49FAE0 grows the array if needed (adds entries zeroed) and returns
`this->counts[type_index]`. The `+10` adjustment in the growth loop is `param_2 + 10`
— a small growth pad.

**Maintenance:**
- `BuildingClass::Unlimbo` (0x00440580) increments the per-type count when a new
  building lands on the map (uses the type-category flag fields 0x16A9/0x16AB/0x16AC/
  0x16AD/0x16AE/0x16AF/0x16B0/0x16CD/0x157B/0x16B7 on BuildingTypeClass to route into
  the right HouseClass sub-list — e.g., barracks list at +0x80/+0x84, radar list, etc.).
- `BuildingClass::OnDestroyed` (0x00445880) calls `HouseClass__Recount(this)` to
  refresh the counts, then sets `this->Owner[0x1FC] = 1` (ProductionDirty).

**Additional per-category lists maintained on Unlimbo** (append-on-construct,
relevant for `[Helipad]`/`[Radar]`/etc. scans):

| House offset | Built from BuildingType flag | Category |
|---|---|---|
| `+0x6C/+0x70/+0x78` | (always) | All buildings |
| `+0x80/+0x84/+0x90` | `+0x16A9` | Power plant? (InfantryOrHelipad-ish) |
| `+0x98/+0x9C/+0xA8` | `+0x16AD` | - |
| `+0xB0/+0xB4/+0xC0` | `+0x16AE` \|\| `+0x16AF` | - |
| `+0xC8/+0xCC/+0xD8` | `+0x16AB` | - |
| `+0xE0/+0xE4/+0xF0` | `+0x157B` && `TechLevel >= 0` | - |
| `+0xF8/+0xFC/+0x108` | `+0x16AC` | - |
| `+0x110/+0x114/+0x120` | `+0x16B0` | - |
| `+0x128/+0x12C/+0x138` | `BuildingTypeClass+0x170C > 0` | - |
| `+0x140/+0x144/+0x150` | `+0x16CD` | - |

Exact flag-to-name mapping is outside this report's scope — see per-flag research in
`BUILDING_SYSTEMS_GHIDRA_REPORT.md`.

---

## 8. Invalidation on Destroy

The central invalidation signal is `HouseClass+0x1FC` (**ProductionDirty** / "Production
Changed" byte flag, 1 byte).

**Set by:**
- `BuildingClass::Unlimbo` at completion of a new building.
- `BuildingClass::OnDestroyed` when a building dies.
- `BuildingClass::OnSpyInfiltrate` after stolen tech is recorded.
- Anywhere else the buildable sidebar must be rebuilt.

**Consumed by:**
- `HouseClass::Update` (the per-house tick at ~0x4F8440/4F8F70) tests `+0x1FC`. If set,
  clears it and runs `HouseClass::AI_ManageProduction` → `HouseClass::AI_ResumeProduction`.
- AI_ResumeProduction / FUN_00509140 re-checks whether currently queued items still
  pass `CanBuild`. Items failing CanBuild are **abandoned**: queued copies are removed
  from the factory via `FactoryClass::Abandon` / `AbandonProduction` and the factory
  is reset to idle.
- The player sidebar rebuild also runs off the same flag — entries whose prerequisites
  disappeared are re-greyed, and newly-valid entries appear.

**Net effect:** Destroying a prerequisite building *does* kick queued items out. The
kick is one tick late (not instantaneous, because the dirty flag is observed next tick)
— in practice this is invisible to the player.

**Confidence:** HIGH on the dirty-flag mechanism itself (verified from
FACTORYCLASS_PRODUCTION_DEEP_DIVE.md §3 and Unlimbo/OnDestroyed sets observed live).
MEDIUM on the exact abandon-queued-items code path inside FUN_00509140 — the full
decompilation chain is not traced here, deferred to FactoryClass deep-dive.

---

## 9. TechLevel Gating

`TechLevel=` combines with `Prerequisite=` via two separate checks, both required:

**Storage:**
- Type: `TechnoTypeClass+0x634` (int; default -1 = never buildable by players).
- House: `HouseClass+0x1D4` (int; default from `RulesClass+0x1254` = [General] TechLevel).

**Check sequence (from step 4 of CanBuild):**
```c
if (type->TechLevel == -1) return 0;      // unbuildable (civilian, internal, etc.)
if (type->TechLevel > house->TechLevel) return 0;
```

**In skirmish/multiplayer:** All houses are initialized with TechLevel=10 from the MP
dialog — so TechLevel on types effectively becomes a sort key (lower values appear
first in the sidebar). Prerequisites do the gating.

**In campaign:** Map INI sets per-house TechLevel, often low early in the mission and
raised by triggers. Then TechLevel *does* gate builds (e.g., TechLevel=3 forbids
TechLevel=9 buildings regardless of prerequisites).

**Triggers can modify TechLevel mid-mission** via Action 77 or similar.

---

## 10. Walkthroughs

### 10.1 AlliedBattleLab (GATECH)

**INI:**
```
Prerequisite=GAWEAP,RADAR,GACNST
TechLevel=8
Owner=British,French,Germans,Americans,Alliance,Russians,Confederation,Africans,Arabs,YuriCountry
```

**Parsed Prerequisite vector (+0x638):**
```
[0] = GAWEAP array index     (> 0, specific)
[1] = -4                      (RADAR keyword)
[2] = GACNST array index     (> 0, specific Construction Yard)
```

**Evaluation trace for Allied player at TechLevel 10, owning a GACNST + GAWEAP + GAAIRC:**

| Step | Result |
|---|---|
| NotBuildable? | no |
| PrereqOverride? | empty → skip |
| TechLevel: 8 ≤ 10 | OK |
| RequiresStolenTech? | none |
| RequiredHouses: -1 (all) | OK |
| ForbiddenHouses: -1 (none) | OK |
| Prereq[0] = GAWEAP | CountOwnedInstances(GAWEAP) = 1 → pass |
| Prereq[1] = -4 (RADAR) | scan {GAAIRC, NARADR, AMRADR, NAPSIS}; GAAIRC owned = 1 → pass |
| Prereq[2] = GACNST | CountOwnedInstances(GACNST) = 1 → pass |
| BuildLimit (0) | unlimited → return 1 |

**Result: buildable.** Destroying GAWEAP flips this back to unbuildable on the next tick
when ProductionDirty is processed.

### 10.2 SovietRadarTower (NARADR → NAHAND upstream)

NAHAND (Soviet Barracks):
```
Prerequisite=POWER,NACNST
TechLevel=2
```

Parsed vector: `[-1 (POWER), NACNST_index]`.

For a Soviet player with just NACNST and NAPOWR:
- POWER resolves to {GAPOWR, NAPOWR, NANRCT, YAPOWR}; NAPOWR owned = 1 → pass.
- NACNST owned = 1 → pass.
- Result: NAHAND buildable at TechLevel ≥ 2.

NARADR (next tier up):
```
Prerequisite=NAREFN,NACNST
TechLevel=3
```
Note: `NAREFN` is a direct building ref, not `PROC`. A player with captured GAREFN
(Allied refinery) does NOT satisfy this — NAREFN specifically is required.

**Gotcha:** If NARADR had used `Prerequisite=PROC,NACNST` instead, the PROC group
{GAREFN, NAREFN, YAREFN} would have allowed a captured Allied/Yuri refinery. The
vanilla choice of specific-NAREFN is intentional — it enforces same-side refinery for
Soviets regardless of capture.

### 10.3 YuriGrinder (YAGRND)

**INI:**
```
Prerequisite=YAWEAP,YACNST
TechLevel=9
Owner=British,French,Germans,Americans,Alliance,Russians,Confederation,Africans,Arabs,YuriCountry
```

**Parsed:** `[YAWEAP, YACNST]` — both specific, no keywords.

Evaluation trace for Yuri player at TechLevel 10 with YACNST + YAWEAP:
- TechLevel 9 ≤ 10 → OK.
- YAWEAP owned = 1 → pass.
- YACNST owned = 1 → pass.
- BuildLimit 0 → unlimited.
- Return 1.

**Yuri quirk:** `Owner=` includes every non-Yuri country, allowing capture-to-build.
But an Allied player who captures a YACNST + YAWEAP and meets TechLevel 9 *can* build
YAGRND. The two gates (`Owner=` vs `Prerequisite=`) are independent.

---

## 11. Magic Constants and Edge Cases

| Constant / Pattern | Meaning |
|---|---|
| `-1` / `0xFFFFFFFF` as keyword | POWER (also the default "any house" RequiredHouses) |
| `-2` / `0xFFFFFFFE` | FACTORY |
| `-3` / `0xFFFFFFFD` | BARRACKS |
| `-4` / `0xFFFFFFFC` | RADAR |
| `-5` / `0xFFFFFFFB` | TECH |
| `-6` / `0xFFFFFFFA` | PROC |
| Strtok delimiter | `","` (byte at `DAT_00817F70` is `,\0`) — no whitespace stripping beyond what ReadString does |
| Keyword comparison | `_stricmp` (`FUN_007C8D20`) — case-insensitive |
| Empty Prerequisite= | Zero iterations → Step 10 passes trivially. Any type without a `Prerequisite=` line is buildable the moment TechLevel/Owner allows. |
| Unknown token (not a keyword, not a BuildingType name) | Silently dropped; parser doesn't error. **This is how typos go undetected** — `Prerequisite=POWAR` behaves like `Prerequisite=` (empty). |
| Pipe `|` | Not parsed — entire pipe-joined token fails BuildingType lookup and is dropped. |
| SMIN (PrerequisiteProcAlternate) | Only case where a type owned elsewhere satisfies a keyword via a *deploy-building* reference (`type+0xDF8`). A player owning a deployed SMIN satisfies PROC even with no refinery. |
| IsPowersUpBuilding flag (`BuildingTypeClass+0xE88`) | When set, the specific-type prerequisite check switches to upgrade-slot scan: any building in `HouseClass+0x6C` whose slot `[+0x17B..+0x17D]` contains this type-ptr satisfies the prereq. |
| Wall/Tech center special handling | Section `(int)piVar4 + 0x81 != 0` or `piVar4[0x198] == 0` (Limbo flag, Unplaced?) skips candidates — the upgrade host must be alive and placed. Return `0x13` from vtable slot 0x184 also skips (RTTI filter, likely "not a valid building"). |
| Upgrade slot count | 3 slots per building (`[0x17B..+0x17D]`). Hardcoded; a single building can host at most 3 upgrades. |
| AI shortcut (Step 8) | AI players that are neither CurrentPlayer nor PlayerControl bypass the entire Prerequisite loop. AI still respects TechLevel, RequiredHouses, ForbiddenHouses, and stolen tech. This is *why AI can sometimes appear to build things without prereqs* in campaign/skirmish — the behavior is intentional. |
| `ProductionDirty` flag (`+0x1FC`) | Set by Unlimbo/OnDestroyed/OnSpyInfiltrate. Consumed next HouseClass::Update tick, triggering full rebuild. One tick of staleness is possible but invisible. |

---

## 12. Open Questions

1. **FUN_00509140** is labelled "UpdateFactoryPrereqs" by inference but not fully
   decompiled for kick-out semantics. Specifically: when a queued item's prereq
   vanishes while the item is N% built, does the house get a cash refund, or is the
   partial spend forfeited? **Deferred to FactoryClass deep-dive.**
2. The **per-category aggregate lists** on HouseClass (table in §7) use BuildingType
   flag bytes at `+0x16A9..+0x16B0` etc. Not every flag has a named INI key in our
   docs yet. A separate field-names pass on BuildingTypeClass `+0x16A0..+0x1710`
   would close this.
3. The **Slave Miner deploy check** in step 8 of CanBuild reads
   `*(int*)(*(int*)(this+0x258) + type[0x5BC]*4)+0x28)+0xE7` — this is an array on the
   house indexed by the deploy-target type, dereferenced to read a byte at +0xE7. The
   exact field at `+0xE7` on this deeper struct is not pinned; almost certainly a
   "can-deploy" / "has-deployed" flag on BuildingTypeClass.
4. **Does PrerequisiteOverride combine with PrerequisiteOverride from a parent
   TechnoType (via [AudioVisual]/[AI] INI inheritance)?** No inheritance exists for
   TechnoTypes in vanilla YR — the question is moot, but worth noting that the check
   is a direct field read with no fallback.
5. **Keyword interaction with `PrerequisiteOverride=`:** a keyword in the override
   list (e.g., `PrerequisiteOverride=TECH`) technically parses, but no vanilla type
   uses this. Behavior: **any** owned tech building satisfies the override. Mods using
   this pattern get expected OR-style behavior.

---

## Sources

**Decompiled (this pass):**
- `HouseClass::CanBuild` — 0x004F7870 (body 0x4F7870–0x4F8363)
- `Prerequisite_INI_Parser` — 0x004770E0
- `HouseClass::CountOwnedInstances` — 0x0049FAE0
- `BuildingClass::Unlimbo` — 0x00440580 (counter-increment paths only)
- `BuildingClass::OnDestroyed` — 0x00445880 (Recount + ProductionDirty set)
- `FUN_0050A490` — AI base-plan cleanup (not prereq invalidation)

**Assembly-context verified:**
- `Prerequisite` string xref at 0x007141AC → `LEA EDI,[EBP+0x638]`
- `PrerequisiteOverride` string xref at 0x00714229 → `LEA EDI,[EBP+0x654]`
- All 7 `Prerequisite*` string xrefs land in `RulesClass::ReadGeneral` (0x66E763..0x66F7A2)

**Existing reports cross-referenced (not re-verified):**
- `OWNER_BITMASK_TECH_PREREQUISITE_SYSTEM.md` (primary) — TechLevel, BuildLimit,
  stolen tech, per-RTTI acquired masks, Naval/Aircraft factory pointers
- `FACTORYCLASS_PRODUCTION_DEEP_DIVE.md` — ProductionDirty flag semantics and
  AI_ManageProduction/AI_ResumeProduction driver
- `BUILDINGCLASS_MASTER_GHIDRA_REPORT_V2.md` — NotBuildable offset +0xC98
- `HOUSECLASS_VERIFIED_FIELD_MAP.md` — HouseClass +0x1D4 TechLevel, +0x1FC ProductionDirty

**INI (source of truth for walkthroughs):**
- `ini/rulesmd.ini` §[General] (lines 484–490) — 7 Prerequisite* group keys
- `ini/rulesmd.ini` [GATECH] (line 11917), [NAHAND] (12482), [NARADR] (12601),
  [YAGRND] (13495) — walkthrough targets

**Tiberian-Sun legacy flag:** Prerequisite system is fully live in YR skirmish.
No TS-only branches detected in CanBuild's prerequisite scan.
