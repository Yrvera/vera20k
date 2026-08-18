# Skirmish BuildOffAlly First Consumer - Ghidra Research Report

**Address(es):** `0x006ACEE0` writer, `0x004A8EB0` first gameplay consumer, direct read at `0x004A8FFA`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** offline Skirmish `BuildOffAlly` packed byte `DAT_00A8B264`, its first verified gameplay consumer, and the placement/base-adjacency effect.  
**Non-Scope:** full building placement blocker taxonomy, command execution after placement command enqueue, online packet protocol beyond excluding non-gameplay xrefs.  
**Confidence:** High  
**Active in YR:** Yes, conditional on ordinary tactical building placement for the local player.

## 1. Overview

`BuildOffAlly` controls one part of building placement: whether allied buildings can count as build-area providers. The first verified gameplay consumer is `FUN_004A8EB0 @ 0x004A8EB0`, called by placement preview movement and final click handling. It does not change footprint blockers, terrain/buildability, command execution, house creation, or spawn placement.

## 2. Class Layout / Key Offsets

| Field / global | Meaning | Evidence | Active in YR |
|---|---|---|---|
| `DAT_00A8B264` | Session/global `BuildOffAlly` byte used by gameplay placement | `0x006AD803` writer; `0x004A8FFA` reader | Yes |
| `RulesClass + 0x14BA` | `[MultiplayerDialogSettings] BuildOffAlly` default/settings field | `RulesClass__ReadMultiplayerDialogSettings @ 0x006721C7..0x006721D4`; ctor `0x006672F3` | Yes |
| `Session/skirmish settings + 0x16` | persisted skirmish `BuildOffAlly` override byte | `SessionClass__ReadSkirmishSettings @ 0x00697FE7..0x00697FFB`; write `0x00699021..0x0069902D` | Yes |
| `BuildingType + 0xEB4` | `Adjacent` radius of the building being placed | parser `0x0045DEAC` default 3; read key at `0x0045FFE5`; consumer `0x004A8F3E` | Yes |
| `BuildingType + 0x154F` | `BaseNormal` self-owned provider gate | ctor `0x0045DFF2` default 1; read key `0x004601F0..0x004601FD`; consumer `0x004A8FE6..0x004A8FF5` | Yes |
| `BuildingType + 0x1550` | `EligibileForAllyBuilding` allied provider gate, typo preserved | ctor `0x0045DFF9` default 0; read key `0x00460203..0x00460217`; consumer `0x004A9017..0x004A9027` | Yes |

## 3. Core Logic

### Writer and defaults

Offline Skirmish Start reads checkbox control `0x69D` with `BM_GETCHECK (0xF0)` and writes `DAT_00A8B264 = (result == 1)`, then mirrors `DAT_00A8B3DA = DAT_00A8B264`. Evidence: `FUN_006ACEE0 @ 0x006AD7F7..0x006AD87E`.

Rules constructor seeds `Rules + 0x14BA = 1`; `RulesClass__ReadMultiplayerDialogSettings` only overrides it if a `BuildOffAlly` key exists. The supplied `rulesmd.ini` has no explicit `BuildOffAlly` under `[MultiplayerDialogSettings]`, so standard YR fallback is enabled. Evidence: ctor assembly `0x006672ED..0x006672F9`, reader assembly `0x006721B4..0x006721D4`, and `ini/rulesmd.ini:3017..3041`.

### First gameplay consumer

`FUN_004A8EB0` returns whether a candidate building placement is inside a valid build area. It is a build-area/adjoining-base validator, not the footprint blocker validator.

The helper performs the scan only when all gates pass:

- placing house index equals `g_PlayerPtr + 0x30`;
- map editor flag `g_IsMapEditor` is false;
- placement mode/object argument is non-null;
- target cell is not the invalid sentinel at `DAT_008A03F8`;
- object RTTI/vtable `+0x2C` returns `7` (building type).

If any gate fails, it returns success (`1`) without consulting `BuildOffAlly`. Active in YR: Conditional, because this is ordinary local tactical placement. Evidence: `0x004A8EB0..0x004A905C`.

For the live path, it computes a scan rectangle around the candidate building foundation:

- foundation width from `BuildingTypeClass__GetFoundationWidth @ 0x0045EC90`;
- foundation height from `BuildingTypeClass__GetFoundationHeight(0) @ 0x0045ECA0`;
- expansion radius is `placed_type.Adjacent + 1`, read from `BuildingType + 0xEB4`;
- cells inside the candidate foundation are skipped; only the surrounding expanded ring is tested.

For each ring cell, it calls `MapClass__Get_CellClass @ 0x005657A0`, then `Look_up_building_in_cell @ 0x0047C520`. Empty cells do not satisfy the build-area check.

Provider acceptance:

1. Self-owned provider path: if provider owner `House + 0x30` equals placing house index and provider type `BaseNormal` byte `+0x154F` is nonzero, the result becomes true. Evidence: `0x004A8FD7..0x004A8FF5`. Active in YR: Yes.
2. Allied provider path: if `DAT_00A8B264 != 0`, then `HouseClass__IsAlliedWith @ 0x004F9A50` must return true between provider owner and placing house, and provider type `EligibileForAllyBuilding` byte `+0x1550` must be nonzero. Evidence: `0x004A8FFA..0x004A9027`. Active in YR: Yes when option is enabled and houses are allied.

## 4. INI Keys

| Key | Section | Default | Binary read | Effect |
|---|---|---:|---|---|
| `BuildOffAlly` | `[MultiplayerDialogSettings]` | yes from constructor; absent in supplied `rulesmd.ini` | `0x006721C7..0x006721D4`; skirmish settings `0x00697FE7..0x00697FFB` | Enables allied-provider branch in build-area placement |
| `Adjacent` | building type sections | `3` in ctor; Rust also defaults 3 | `0x0045FFE5`; consumer `0x004A8F3E` | Radius of placed building's build-area scan, plus one for ring expansion |
| `BaseNormal` | building type sections | yes | `0x004601F0..0x004601FD` | Self-owned provider must have this true |
| `EligibileForAllyBuilding` | building type sections | no | `0x00460203..0x00460217` | Allied provider must have this true; typo is literal |

Standard YR `rulesmd.ini` sets `EligibileForAllyBuilding=yes` on `GACNST`, `NACNST`, `YACNST`, and `YACOMD` (`ini/rulesmd.ini:11650`, `12446`, `13120`, `13231`). It does not set it globally; most structures retain constructor default false.

## 5. Integration Points

`FUN_004A8EB0` has two verified callers:

- `FUN_004A91B0 @ 0x004A9480`: placement cursor/preview update. It writes the result to display state byte `+0x1180`, then calls `FUN_004A9070` for the separate footprint/blocker validity byte `+0x1181`. Active in YR: Yes.
- `DisplayClass__BandBox_LeftUp @ 0x004ABA59`: final placement click. If either `FUN_004A8EB0` or the footprint state is false, it plays the cannot-place feedback and does not enqueue the placement command. If both pass, it queues the placement command. Active in YR: Yes.

Non-gameplay xrefs to `DAT_00A8B264` were excluded from the "first gameplay consumer" claim: option string/status formatting (`0x005DBB60`, `0x0077E430`), option-change snapshotting (`0x005E32D0`), shell checkbox sync/init (`0x005B4EE0`, `0x006AE6E0`), and persistence/session copies (`0x005ED400`, `0x005ED5A0`, `0x00697FF0`, `0x00699025`).

## 6. Current Rust Implementation Status

Current Rust implements this scoped behavior. `src/sim/game_options.rs` defaults
`GameOptions::build_off_ally` to `true` and parses an explicit
`[MultiplayerDialogSettings] BuildOffAlly=` override.

`src/rules/object_type.rs` defaults `ObjectType::adjacent` to `3`, defaults
`base_normal` to `true`, and parses typo-preserved
`EligibileForAllyBuilding` with a default of `false`.

`src/sim/production/production_placement.rs::is_within_build_area` uses the
placed building's `Adjacent + 1` ring. Same-owner providers require
`BaseNormal`; other-owner providers require `BuildOffAlly`, a friendly house
relationship, and provider `EligibileForAllyBuilding`. Focused tests cover the
enabled, disabled, ineligible-provider, and own-provider cases.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Offline shell writer `FUN_006ACEE0` | verified | `0x006AD7F7..0x006AD87E` | none |
| Rules default and INI reader | verified | ctor `0x006672F3`; reader `0x006721C7..0x006721D4`; `rulesmd.ini` absent key | none |
| `DAT_00A8B264` xrefs | verified | `get_xrefs_to(0x00A8B264)` plus decompile spot checks | none for first gameplay consumer |
| First gameplay consumer `FUN_004A8EB0` | verified | decompile and assembly `0x004A8EB0..0x004A9063` | none |
| Placement preview caller `FUN_004A91B0` | verified | call at `0x004A9480`; decompile writes `+0x1180` | footprint helper internals out of scope |
| Final placement click caller `DisplayClass__BandBox_LeftUp` | verified | call at `0x004ABA59`; failure path before command enqueue | command execution after enqueue out of scope |
| `BaseNormal` parser and default | verified | ctor `0x0045DFF2`; parser `0x004601F0..0x004601FD` | none |
| `EligibileForAllyBuilding` parser and default | verified | ctor `0x0045DFF9`; parser `0x00460203..0x00460217` | none |
| Rust surfaces | verified current | `src/sim/game_options.rs`, `src/rules/object_type.rs`, `src/sim/production/production_placement.rs`, and focused production-placement tests; implemented by `a35f1cd4` with later session-state relocation in `a74be6430` | no remaining scoped implementation delta |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-1 - Is the writer still the offline Skirmish Start checkbox 0x69D? -> Yes, `FUN_006ACEE0` writes `DAT_00A8B264 = (BM_GETCHECK == 1)` and mirrors `DAT_00A8B3DA`.` (evidence: `0x006AD7F7..0x006AD87E`)
- `[RESOLVED] OQ-2 - What is the first gameplay reader? -> `FUN_004A8EB0`, direct read at `0x004A8FFA`.` (evidence: `get_xrefs_to(0x00A8B264)`, decompile/disassembly)
- `[RESOLVED] OQ-3 - Is the reader live in standard YR? -> Yes, it is called by tactical placement preview and final placement click.` (evidence: `0x004A9480`, `0x004ABA59`)
- `[RESOLVED] OQ-4 - Does BuildOffAlly affect startup house creation or spawn placement? -> No evidence of a gameplay reader there; the verified gameplay reader is tactical building placement.` (evidence: xref classification to `0x00A8B264`)
- `[RESOLVED] OQ-5 - What gates the allied path? -> option byte nonzero, `HouseClass__IsAlliedWith`, and provider BuildingType `+0x1550` nonzero.` (evidence: `0x004A8FFA..0x004A9027`)
- `[RESOLVED] OQ-6 - What gates the self-owned path? -> same house index and provider BuildingType `+0x154F` nonzero; BuildOffAlly is not consulted.` (evidence: `0x004A8FD7..0x004A8FF5`)
- `[RESOLVED] OQ-7 - What are `+0x154F` and `+0x1550`? -> `BaseNormal` and typo-preserved `EligibileForAllyBuilding`.` (evidence: `0x004601F0..0x00460217`)
- `[RESOLVED] OQ-8 - What is the default BuildOffAlly value in standard YR skirmish? -> Enabled unless rules/skirmish settings override it.` (evidence: ctor `0x006672F3`; `rulesmd.ini` no key)
- `[RESOLVED] OQ-9 - What is the default allied-provider type flag? -> false in constructor, true only where INI says `EligibileForAllyBuilding=yes`.` (evidence: `0x0045DFF9`; `rulesmd.ini:11650`, `12446`, `13120`, `13231`)
- `[RESOLVED] OQ-10 - Does the helper use provider `Adjacent`? -> Not in the verified consumer; it reads placed BuildingType `+0xEB4` and expands by `+1`.` (evidence: `0x004A8F3E..0x004A8F5E`)
- `[DEFERRED] OQ-11 - Exact visual palette/shape of placement preview cells` (category: out-of-scope; reason: not needed to prove BuildOffAlly consumer/effect; next-step-if-pursued: dedicated placement preview rendering trace)
- `[DEFERRED] OQ-12 - Full `FUN_004A9070` footprint/blocker formula` (category: out-of-scope; reason: BuildOffAlly does not feed it; next-step-if-pursued: full building placement validator investigation)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Standard YR skirmish defaults `BuildOffAlly` on unless overridden | `0x006672F3`; `0x006721C7..0x006721D4`; absent key in `rulesmd.ini` | implemented | `src/sim/game_options.rs`; skirmish option packing | preserve | Standard skirmish with default options allows build-area check near eligible allied ConYard | `build_off_ally_default_matches_yr_enabled` | Do not infer false from absent `rulesmd.ini`; constructor fallback is yes |
| Enabled `BuildOffAlly` lets allied eligible buildings satisfy build area; disabled does not | `0x004A8FFA..0x004A9027`; callers `0x004A9480`, `0x004ABA59` | implemented | `src/sim/production/production_placement.rs`; house alliance model | preserve | Player A can place a normal structure inside build range of allied Player B's `GACNST` when enabled, and cannot when disabled | `build_off_ally_enabled_accepts_allied_eligible_provider`, `build_off_ally_disabled_rejects_allied_eligible_provider` | Do not treat all allied structures as providers |
| Allied provider requires `EligibileForAllyBuilding=yes`, separate from `BaseNormal` | parser `0x00460203..0x00460217`; consumer `0x004A9017..0x004A9027` | implemented | `src/rules/object_type.rs`; placement validator | preserve | Allied `GACNST` allows; allied `GAPOWR` or other default-false provider does not | `build_off_ally_requires_eligibile_for_ally_building` | Do not reuse `BaseNormal` for allied eligibility |
| Self-owned providers remain governed by `BaseNormal` and do not depend on `BuildOffAlly` | `0x004A8FD7..0x004A8FF5` | implemented | `src/sim/production/production_placement.rs` | preserve | Player can place near own `BaseNormal=yes` ConYard with BuildOffAlly off | `build_off_ally_off_keeps_own_base_provider` | Do not make `build_off_ally=false` disable normal base expansion |

Stale Docs / Follow-up Docs:

- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_PACKED_OPTION_GLOBAL_CONSUMERS_GHIDRA_REPORT.md`: replace the BuildOffAlly deferred row with: "`BuildOffAlly` first verified gameplay consumer is `FUN_004A8EB0 @ 0x004A8EB0`, read at `0x004A8FFA`. It gates allied buildings as build-area providers during tactical building placement preview/final click, after `HouseClass__IsAlliedWith` and provider BuildingType `EligibileForAllyBuilding` (`+0x1550`) pass. Self-owned providers use `BaseNormal` (`+0x154F`) and do not depend on this option."
- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_BUILD_OFF_ALLY_FIRST_READER_GHIDRA_REPORT.md`: replace "Exact semantic names and INI sources for BuildingType bytes `+0x154F` and `+0x1550` were not traced in this slot" with: "`+0x154F` is `BaseNormal`, default true; `+0x1550` is typo-preserved `EligibileForAllyBuilding`, default false. Both are parsed in `BuildingTypeClass_ReadINI_Water @ 0x004601F0..0x00460217` and consumed by `FUN_004A8EB0`."

## Sources

- Ghidra read-only decompiled/disassembled: `FUN_006ACEE0 @ 0x006ACEE0`, `FUN_004A8EB0 @ 0x004A8EB0`, `FUN_004A91B0 @ 0x004A91B0`, `DisplayClass__BandBox_LeftUp @ 0x004AB9B0`, `BuildingTypeClass_ReadINI_Water @ 0x004601F0`, `RulesClass__ReadMultiplayerDialogSettings @ 0x006721C7`, `SessionClass__ReadSkirmishSettings @ 0x00697FF0`, `SessionClass__WriteSkirmishSettings @ 0x00699025`, `RulesClass` constructor `0x00665650`.
- Ghidra xrefs: `DAT_00A8B264`, `BuildOffAlly`, `BaseNormal`, `EligibileForAllyBuilding`.
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`.
- Rust scanned: `src/sim/game_options.rs`, `src/rules/object_type.rs`, `src/sim/production/production_placement.rs`.
