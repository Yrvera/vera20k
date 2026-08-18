# Sidebar CompareItems Exact Order - Ghidra Report

Date: 2026-05-27

Target: `SidebarClass__CompareItems` / `CompareItems @ 0x006A8420`

## Target Question

Decode `SidebarClass__CompareItems @ 0x006A8420` enough to implement native sidebar cameo ordering: operand order, return meaning, field reads, signedness, side/player match rules, superweapon subgroup ordering, string/name tiebreak, and Soviet fixture candidates.

## Non-Goals

- Do not re-investigate `SidebarClass::AddCameo` or `StripClass::InsertEntry` beyond confirming comparator argument order and return meaning.
- Do not decode every producer of the fields read by `CompareItems`.
- Do not modify Rust, INI, or existing published docs.
- Do not rename or mutate anything in Ghidra.

## Evidence Needed To Mark COMPLETE

- Fresh Ghidra decompile of `0x006A8420`.
- Fresh Ghidra decompile plus assembly context for the `StripClass::InsertEntry @ 0x006A8710` call to `0x006A8420`, proving operand order and return meaning.
- Assembly contexts for comparator branches that prove key field reads, signed comparison direction, and final string comparison.
- Rust-facing handoff with concrete test-name proposals.

## Stop Conditions

- Ghidra MCP unavailable.
- Comparator or caller function not resolvable read-only.
- Required evidence would require mutating Ghidra state.

## Verified Findings

### 1. Argument Order And Return Meaning

Active in YR: Yes, conditional on normal sidebar cameo insertion. `StripClass::InsertEntry @ 0x006A8710` is the active insert helper established by `SIDEBAR_ADDCAMEO_INSERTENTRY_ORDER_STATUS_GHIDRA_REPORT.md`; this slot re-confirmed its direct comparator call.

`CompareItems(new_rtti, new_type_index, existing_rtti, existing_type_index)` returns true when the new candidate should be inserted before the scanned existing entry. `InsertEntry` scans the inline entries from `strip + 0x58`, stops when the comparator returns true, then shifts later 0x34-byte entries and writes the new entry.

Evidence: `decompile_function 0x006A8710`; assembly context `0x006A872F..0x006A8749` shows pushes from `[ESI]`/`[ESI+4]` for existing entry and stack args for the new entry before `CALL 0x006A8420`, then `TEST AL,AL; JNZ 0x006A8760` enters the shift/write path. Fresh decompile of `0x006A8420` confirms the four-argument comparator body.

### 2. Type Resolution And Empty Sentinel

Active in YR: Yes, on every `InsertEntry` comparator probe.

The candidate and existing normal object type pointers are resolved through `RTTI_To_TypeArray @ 0x0048DCD0`. Supported normal RTTIs in that helper are `1/0x28 -> UnitTypeClass`, `2/3 -> AircraftTypeClass`, `6/7 -> BuildingTypeClass`, and `0x0F/0x10 -> InfantryTypeClass`; other RTTIs return null there and are handled separately only when they are in the superweapon group. If `existing_rtti == 0`, `CompareItems` returns true immediately, treating an empty existing slot as an insertion sentinel.

Evidence: `decompile_function 0x0048DCD0`; `decompile_function 0x006A8420`; assembly context `0x006A8426..0x006A845D` shows the two `RTTI_To_TypeArray` calls and `TEST EBP,EBP; JNZ ...; MOV AL,1; RET 0x10`.

### 3. Superweapon Tier And Suborder

Active in YR: Yes for superweapon cameos inserted into the defense strip.

RTTIs `{0x1F, 0x39, 0x20}` form the superweapon group. Superweapon candidates sort before ordinary non-super entries. When both operands are superweapons, the comparator reads `g_SuperWeaponTypeClass_Array[type_index]`, compares signed 32-bit field `+0xB0` ascending, and if equal compares wide strings at `+0x60` with `FUN_007CA5D3`. The final predicate is `wide_compare(candidate_name, existing_name) < 1`, so equal names also return true.

Evidence: `decompile_function 0x006A8420`; assembly context `0x006A8470..0x006A849F` detects the superweapon RTTI set; `0x006A84A5..0x006A84E1` loads `g_SuperWeaponTypeClass_Array`, reads `+0xB0`, uses signed `JGE`/fallthrough for ascending order, then jumps to the `+0x60` string tiebreak; `decompile_function 0x007CA5D3` proves a 16-bit lexicographic compare returning `-1/0/1`; `0x006A86F3..0x006A8705` calls it and returns true for result `< 1`.

### 4. Ordinary Side Match And Land/Air/Naval Category Tier

Active in YR: Yes for ordinary non-super entries.

For two ordinary non-super entries, side match is tested before unit category, tech level, cost, or name. The comparator reads the current player's house type side index from `g_PlayerPtr +0x34 -> +0xBC`, compares it to each operand's `TechnoTypeClass +0x6D0` (`AIBasePlanningSide`, inferred/corroborated by existing TypeClass docs), and puts a matching-side operand before a nonmatching-side operand. If neither or both match, ordering continues.

For category suborder, the comparator only reads `+0xD96` and `+0xCCE` when the operand RTTI is `0x28` or `3`. Semantic names are inferred from existing docs: `+0xD96` is `ConsideredAircraft`; `+0xCCE` is `Naval`. Category order is ordinary/land before `+0xD96` before `+0xCCE`, with superweapons before all three.

Evidence: `decompile_function 0x006A8420`; assembly context `0x006A84E6..0x006A8523` for `g_PlayerPtr`, `+0x34`, `+0xBC`, and operand `+0x6D0` comparisons; `0x006A853A..0x006A8585` and `0x006A858A..0x006A85D2` for `RTTI == 0x28 || RTTI == 3` gated reads of `+0xD96`/`+0xCCE` and plain-category boolean derivation; `0x006A85D6..0x006A8674` for the category precedence returns.

### 5. Scalar Tiebreaks: TechLevel, Cost-Like Virtual, Name

Active in YR: Yes for ordinary entries after the earlier tiers do not decide.

The remaining tiebreak order is signed ascending `TechnoTypeClass +0x634`, then signed ascending return from vtable `+0x84(g_PlayerPtr)`, then wide string compare on `TechnoTypeClass +0x60` with `<=` accepted. Semantic names are inferred/corroborated from existing docs: `+0x634` is `TechLevel`; vtable `+0x84(g_PlayerPtr)` is cost-like and is named `GetCost` in older sidebar docs, but this slot did not independently decode each concrete TypeClass vtable target.

Evidence: `decompile_function 0x006A8420`; assembly context `0x006A8682..0x006A86A9` reads `[candidate+0x634]` and `[existing+0x634]` and uses signed `JGE`/`JLE`; `0x006A86AC..0x006A86E9` calls `[vtable+0x84]` for each operand with `g_PlayerPtr` and uses signed comparison; `0x006A86ED..0x006A8705` reads `+0x60`, calls `FUN_007CA5D3`, and returns true for compare result `< 1`.

## Implementation Handoff

1. Verified behavior: `CompareItems` returns true for `new < existing` using tiers `empty sentinel -> superweapon -> side match -> land/air/naval category -> TechLevel -> cost-like virtual -> wide-name <=`. Rust delta: `src/sidebar/sidebar_view.rs::collect_build_entries` currently builds transient vectors from production views and only prepends superweapons; it has no native comparator or persistent `(RTTI,type_index)` entry metadata. Affected surface: visible sidebar ordering for Soviet and captured/mixed-side build palettes. Acceptance scenario: a Soviet player with NAPOWR, NAREFN, NAHAND, NAWEAP, NARADR, and a captured Allied/Yuri buildable sees matching-side entries first, then lower `TechLevel`, then lower cost/name according to `CompareItems`. Proposed test: `test_sidebar_compareitems_orders_soviet_buildings_by_native_tiers`. Risk: high screenshot/UI parity.

2. Verified behavior: category ordering is superweapon before ordinary land before `ConsideredAircraft` before `Naval` for operands that reach this tier. Rust delta: production/sidebar data needs a native comparable category, not just `ProductionCategory` or current tab. Affected surface: defense/vehicle strip ordering for naval and aircraft-like entries. Acceptance scenario: fixture entries with equal side/tech/cost/name except category sort in the exact native sequence. Proposed test: `test_sidebar_compareitems_category_tier_super_land_air_naval`. Risk: medium-high for naval/air sidebar parity.

3. Verified behavior: final name compare uses the raw `+0x60` wide string pointer and returns true on equality (`<=`), so equal-name nonduplicate entries can insert ahead of an existing one after all earlier tiers tie. Rust delta: do not replace the final tiebreak with stable append or Rust `Ord` over display names unless it matches the source string and equality behavior. Affected surface: modded build palettes and any stock duplicate-name edge. Acceptance scenario: two distinct `(RTTI,type_index)` entries with equal side/category/tech/cost and equal compare name insert in native new-before-existing order unless duplicate scan rejected the exact pair. Proposed test: `test_sidebar_compareitems_equal_name_inserts_new_before_existing_when_not_duplicate`. Risk: medium for mods, low for stock.

Stock Soviet fixture candidates from `ini/rulesmd.ini`: `NAPOWR` (`TechLevel=1`, `Cost=600`, `AIBasePlanningSide=1`), `NAREFN` (`TechLevel=1`, `Cost=2000`, `AIBasePlanningSide=1`), `NAHAND` (`TechLevel=2`, `Cost=500`, `AIBasePlanningSide=1`), `NAWEAP` (`TechLevel=2`, `Cost=2000`, `AIBasePlanningSide=1`), `NARADR` (`TechLevel=3`, `Cost=1000`, `AIBasePlanningSide=1`), `NAYARD` (`Naval=yes`, `TechLevel=2`, `AIBasePlanningSide=1`), `SUB` (`Naval=yes`, `TechLevel=2`, `Cost=1000`), `DRED` (`Naval=yes`, `TechLevel=6`, `Cost=2000`), `E2` (`TechLevel=1`, `Cost=100`), `FLAKT` (`TechLevel=1`, `Cost=300`), `SHK` (`TechLevel=5`, `Cost=500`), `IVAN` (`TechLevel=5`, `Cost=600`), `DESO` (`TechLevel=8`, `Cost=600`). Use these as fixture candidates only after mapping their native RTTI/type indices through the rules arrays.

## Negative Facts / Do Not Do

- Do not sort sidebar cameos only by rules file order, queue order, or `ProductionCategory`; native insertion calls `CompareItems` and stops at first true comparator result. Evidence: `0x006A872F..0x006A8749`, `0x006A8760..0x006A8789`.
- Do not put superweapons after normal defense cameos; a superweapon candidate returns true before ordinary existing entries. Evidence: `0x006A8499..0x006A8536` and `0x006A85D2..0x006A85EA`.
- Do not use `Cost`/name before player-side match; side match at `+0x6D0` is evaluated before category and scalar tiebreaks. Evidence: `0x006A84E6..0x006A8523`.
- Do not treat `Naval=yes` as globally read for every RTTI in this comparator; `+0xCCE` is read only after `RTTI == 0x28 || RTTI == 3` in this function. Evidence: `0x006A853A..0x006A8558`, `0x006A858A..0x006A85C2`.
- Do not make the final name tiebreak strict `<`; native returns true for compare result `-1` or `0`. Evidence: `0x006A86F3..0x006A8705` plus `FUN_007CA5D3`.

## Remaining Uncertainty

- Semantic name of vtable `+0x84(g_PlayerPtr)` remains inferred/corroborated as cost-like/GetCost from sibling docs; this slot proved the call, argument, signed compare, and ordering but did not decode each TypeClass vtable target.
- Full-strip overflow behavior remains inherited from `InsertEntry` and was not expanded here.
- The exact stock runtime order needs type indices from parsed rules arrays; this report provides comparator mechanics and fixture candidates, not a generated final stock Soviet list.
- `+0xD96` semantic naming is inferred from existing docs; this slot proved the byte read and branch use but did not re-decode `TechnoTypeClass::ReadINI`.

## Stale-Doc Wording

- `docs/research/SIDEBAR_STRIPS_TABS_CAMEOS_GHIDRA.md`, Cameo Sort Order: replace the simplified priority list with:

  `Order is determined by SidebarClass__CompareItems @ 0x006A8420. InsertEntry calls it as CompareItems(new_rtti, new_type_index, existing_rtti, existing_type_index), and true means insert the new entry before the existing slot. Existing RTTI 0 is an insertion sentinel. Superweapon RTTIs {0x1F,0x39,0x20} sort before ordinary entries and are ordered by SuperWeaponTypeClass+0xB0, then +0x60 wide-name <=. Ordinary entries first prefer current-player side matches via current HouseType+0xBC against TechnoTypeClass+0x6D0, then category-sort ordinary/land before +0xD96 before +0xCCE for RTTI 0x28/3 operands, then signed TechnoTypeClass+0x634, then signed vtable+0x84(g_PlayerPtr), then +0x60 wide-name <=.`

- `docs/research/SIDEBAR_SYSTEM_GHIDRA_REPORT.md`, unverified CompareItems note: replace the old "super-weapon-last, factory match, tech-level, GetCost, alphabetical" wording with the same text above, or mark the previous wording stale because this slot proves superweapons sort before ordinary entries and side/category tiers precede TechLevel/cost/name.

## Status

COMPLETE.
