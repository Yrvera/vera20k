# Skirmish Rust Side/Color Item Data Alignment - Ghidra Research Report

**Address(es):** `0x004E3A00`, `0x004E4170`, `0x004E38A0`, `0x004E45A0`, `0x004E4E20`, `0x004E42A0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** current Rust `SkirmishComboItem::Country` / `ColorSentinel` / `Color` representation compared against the native item-data values needed by the standard offline Skirmish `0x102` side/color status mapping.  
**Non-Scope:** complete side/color STT key table, combo item source precedence, Choose Map, online/WOL observer rows, Rust patches, and launch/session packing beyond evidence needed for item-data alignment.  
**Confidence:** High for current Rust representation and standard offline side/color item-data alignment; Medium for observer-row handling because it is evidenced as native-addressable/conditional but is out of the standard offline visible row set.  
**Active in YR:** Yes for standard offline `0x102` side/color rows; Conditional for observer/restricted rows.

## Working Notes Gate

- Target question: Do current Rust `SkirmishComboItem::Country` and `SkirmishComboItem::Color*` values align with native item data needed by the side/color status mapping, and does Rust need value/model changes or just a mapper?
- Non-goals: Do not build the full STT table owned by slots 1/2, do not re-prove item-source precedence owned by slot 3, do not modify Rust, and do not expand into online/WOL observer UI behavior.
- Evidence needed to mark COMPLETE: exact current Rust enum/generation/selection/status lines; native side item-data values for Random/countries; native color item-data values for Random/colors/observer availability boundary; proof whether current Rust represents observer rows; implementation-facing handoff with acceptance tests.
- Stop conditions: stop after every Rust alignment point is classified as match, mapper-needed, model-gap, or out-of-scope/conditional, and after no Rust-facing uncertainty remains for standard offline `0x102`.

## 1. Overview

Current Rust has enough data to resolve standard offline side/color dropdown row status strings without changing the selection model. Side rows are semantic (`Random` or `SkirmishCountry`) and need an explicit native item-data mapper; color rows already carry the native normal item data (`-2`, `0..7`) in their variants.

Rust still lacks the item-specific side/color status resolver: non-AI `ComboItem` hovers fall back to the generic combo key. That is a resolver/mapping gap, not evidence that the side/color selection model must be replaced.

## 2. Current Rust Representation

| Rust surface | Current value/model | Alignment judgment | Active in YR |
|---|---|---|---|
| `SkirmishComboItem::Country(SkirmishCountryChoice)` | semantic enum, no numeric item-data field | Needs mapper to native `-2`/`0..9`; no model change required for standard offline rows | Rust-only |
| `SkirmishCountryChoice::Random` | first side row | Maps to native item data `-2` | Rust-only |
| `SkirmishCountry::ALL` | `America, Korea, France, Germany, GreatBritain, Libya, Iraq, Cuba, Russia, Yuri` | Order matches YR country item data `0..9` via `[Countries]` | Rust-only plus INI/native helper evidence |
| `SkirmishComboItem::ColorSentinel(-2)` | first color row | Already stores native random/no-color sentinel `-2` | Rust-only |
| `SkirmishComboItem::Color(usize)` | normal color rows `0..HOUSE_COLOR_COUNT` | With `HOUSE_COLOR_COUNT = 8`, maps directly to native normal color item data `0..7` | Rust-only plus native helper evidence |
| Observer side `-3` | no Rust side combo row variant in standard list | Not needed for standard offline normal side dropdown; future observer UI would need representation or special mapper branch | Conditional |
| Observer color `8` | omitted by current Rust test and normal list | Matches normal population; native status helper can map `8`, but normal `FUN_004E45A0` does not insert row `8` | Conditional |

Rust evidence: `SkirmishComboItem` variants at `src/ui/skirmish_shell/state.rs:95..103`; `combo_items` side/color generation at `src/ui/skirmish_shell/state/combos.rs:260..285`; side/color selection update at `src/ui/skirmish_shell/state/combos.rs:468..498`; country order at `src/ui/main_menu.rs:39..65`; color count at `src/skirmish_launch.rs:10..12`; current generic non-AI status fallback at `src/ui/skirmish_shell/state/hit_test.rs:137..142`.

## 3. Native Item-Data Evidence

Active in YR: Yes for standard offline `0x102`.

Side/country population (`FUN_004E3A00`) inserts Random first with item data `-2`, then inserts country rows whose item data is `HouseTypeClass+0xB8` when that value is in `-2..9` bounds and the country has a display name. The getter (`FUN_004E4170`) returns selected/hovered item data, accepts `-3..9`, and falls back to selected-mode default or `-2` only outside that range. The status helper (`FUN_004E38A0`) maps `-2`, `-3`, and `0..9` to side status strings.

The stock YR country order is the same zero-based sequence Rust uses: `Americans`, `Alliance`, `French`, `Germans`, `British`, `Africans`, `Arabs`, `Confederation`, `Russians`, `YuriCountry`. Evidence: `ini/rulesmd.ini:959..971`; native side report `SKIRMISH_SIDE_COUNTRY_TEAM_FINAL_WRITES_GHIDRA_REPORT.md`; direct read-only decompile of `0x004E3A00`, `0x004E4170`, `0x004E38A0`.

Color normal population (`FUN_004E45A0`) inserts sentinel item data `-2`, then iterates only normal rows `0..7`; the initialized observer/grey row `8` is addressable by status helper logic but is not inserted by the normal loop. The getter (`FUN_004E4E20`) returns item data through `CB_GETITEMDATA`; the status helper (`FUN_004E42A0`) maps `-2` and `0..8`. Evidence: `SKIRMISH_COLOR_COMBO_POPULATION_AND_SWATCH_ORDER_GHIDRA_REPORT.md`; direct read-only decompile of `0x004E45A0`, `0x004E4E20`, `0x004E42A0`.

## 4. Current Status Resolver Gap

Current Rust already carries the hovered row item in `SkirmishHoverTarget::ComboItem { id, item }` from `hit_test_dropdown_item` (`src/ui/skirmish_shell/state/hit_test.rs:30..37`). The resolver then special-cases only AI row items and sends every other combo item to the generic combo-face mapping (`src/ui/skirmish_shell/state/hit_test.rs:137..142`).

Active in YR: Conditional on open dropdown row hover. Native `0x4E8 -> 0x4E9` side/color item paths try item-specific text before generic fallback, as documented in `SKIRMISH_0X102_STATUS_HELP_FULL_MAPPING_CURRENT_RUST_GHIDRA_REPORT.md` and spot-checked through the helper functions above.

## 5. INI Keys

No INI key controls status help resolution. `rulesmd.ini` is only material here because its `[Countries]` order confirms the stock country index order used by native side item data and Rust `SkirmishCountry::ALL`.

| INI path | Scoped role | Active in YR |
|---|---|---|
| `ini/rulesmd.ini:959..971` `[Countries]` | zero-based stock country order `0..9` | Yes |
| `ini/rulesmd.ini:981..987` `[Sides]` | side grouping after country choice; not a separate dropdown item-data source | Yes, not directly used by status mapping |

## 6. Current Rust Implementation Status

Current Rust can add side/color item status mapping as a pure resolver helper over existing `SkirmishComboItem` values:

- Side: `Random -> -2`; each `SkirmishCountry` maps by the explicit `SkirmishCountry::ALL` stock index `0..9`.
- Color: `ColorSentinel(-2) -> -2`; `Color(0..7) -> same numeric item data`.
- Standard offline observer rows are not currently represented and should not be added to the normal dropdown solely because native status helpers know `-3`/`8`.

The main implementation risk is using enum discriminants or visible labels as native item data. Rust enums have no contract that their source order or display text is the native country id.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Rust `SkirmishComboItem` variants | verified | `src/ui/skirmish_shell/state.rs:95..103` | none |
| Rust side item generation | verified | `src/ui/skirmish_shell/state/combos.rs:275..282` | none |
| Rust color item generation | verified | `src/ui/skirmish_shell/state/combos.rs:283..285` | none |
| Rust side/color selection update | verified | `src/ui/skirmish_shell/state/combos.rs:468..498` | none for status mapping |
| Rust current status resolver | verified | `src/ui/skirmish_shell/state/hit_test.rs:137..142` | implement side/color item-specific mapping later |
| Rust country order | verified | `src/ui/main_menu.rs:39..65` | none for stock YR |
| Native side item data | verified | `0x004E3A00`, `0x004E4170`, `0x004E38A0`; `ini/rulesmd.ini:959..971` | full STT key table owned by slot 1 |
| Native color item data | verified | `0x004E45A0`, `0x004E4E20`, `0x004E42A0`; color combo report | full STT key table owned by slot 2 |
| Observer/online row representation | deferred | side/color helper evidence | out-of-scope for standard offline normal rows |
| Source precedence | deferred | parent slot 3 target | not re-investigated here |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Does Rust store native country item data directly? -> No; side rows store `SkirmishCountryChoice`, so item data must be derived.` (evidence: `src/ui/skirmish_shell/state.rs:89..99`)
- `[RESOLVED] OQ-02 - Does Rust side row order match stock native country item data `0..9`? -> Yes for stock YR; `SkirmishCountry::ALL` order matches `[Countries]` indices `0..9`.` (evidence: `src/ui/main_menu.rs:53..65`; `ini/rulesmd.ini:959..971`; `0x004E3A00`)
- `[RESOLVED] OQ-03 - Does Rust represent side Random? -> Yes, first row `SkirmishCountryChoice::Random`, mapping to native `-2`.` (evidence: `src/ui/skirmish_shell/state/combos.rs:275..282`; `0x004E3A00`)
- `[RESOLVED] OQ-04 - Does Rust represent side Observer `-3`? -> No; standard offline side list does not require it, but helper/status code can address it conditionally.` (evidence: `src/ui/skirmish_shell/state/combos.rs:275..282`; `0x004E4170`, `0x004E38A0`)
- `[RESOLVED] OQ-05 - Does Rust color Random/no-color sentinel align? -> Yes, `ColorSentinel(-2)` matches native sentinel item data `-2`.` (evidence: `src/ui/skirmish_shell/state/combos.rs:283..285`; `0x004E45A0`)
- `[RESOLVED] OQ-06 - Does Rust color normal row range align? -> Yes, `HOUSE_COLOR_COUNT = 8` yields `Color(0)..Color(7)`, matching normal native rows `0..7`.` (evidence: `src/skirmish_launch.rs:10..12`; `src/ui/skirmish_shell/state/combos.rs:283..285`; `0x004E45A0`)
- `[RESOLVED] OQ-07 - Should Rust add `Color(8)` for normal Skirmish status parity? -> No; native status helper maps `8`, but normal population omits row `8`.` (evidence: `0x004E45A0`; `SKIRMISH_COLOR_COMBO_POPULATION_AND_SWATCH_ORDER_GHIDRA_REPORT.md`; `src/ui/skirmish_shell/state/tests.rs:1458..1485`)
- `[RESOLVED] OQ-08 - Is current Rust status mapping complete for side/color rows? -> No; non-AI `ComboItem` falls through to generic combo status.` (evidence: `src/ui/skirmish_shell/state/hit_test.rs:137..142`)
- `[RESOLVED] OQ-09 - Does this require a selection-model rewrite? -> No for standard offline side/color status mapping; the hovered `item` is already available and can be mapped to native item data.` (evidence: `src/ui/skirmish_shell/state/hit_test.rs:30..37`; current enum/generation evidence above)
- `[DEFERRED] OQ-10 - Online/WOL observer/closed row model alignment.` (category: `out-of-scope`; reason: this slot is standard offline `0x102` side/color item-data alignment; next-step-if-pursued: investigate online/lobby row population and observer status rows separately)

## 9. Visual/UI Composition Ledger

No paint-path or asset composition was investigated in this slice. The visual surface is the status/help text chosen for open dropdown row hover; row drawing, swatches, and dropdown visuals are covered by sibling reports.

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Side status helper consumes native item data `-2`, `-3`, `0..9`; standard side population exposes Random `-2` then stock countries `0..9` | `0x004E3A00`, `0x004E4170`, `0x004E38A0`; `ini/rulesmd.ini:959..971` | mapper missing, model sufficient | `src/ui/skirmish_shell/state/hit_test.rs`; possible helper near `SkirmishComboItem` | Map `SkirmishCountryChoice::Random` to `-2` and stock `SkirmishCountry` to explicit `0..9` before resolving the slot-1 STT table | Open Side dropdown, hover Random and Great Britain; resolver returns item-specific side keys, not `STT:SkirmishComboCountry` | Do not cast enum discriminants or use visible labels; proposed test `test_skirmish_combo_items_expose_native_status_item_data` |
| Normal color status helper consumes `-2` and `0..8`, but normal population inserts only `-2` and `0..7` | `0x004E45A0`, `0x004E4E20`, `0x004E42A0`; color combo report | mapper partly implicit; resolver missing | `src/ui/skirmish_shell/state/hit_test.rs`; color combo item helper | Map `ColorSentinel(-2)` to `-2`; map `Color(0..7)` directly; keep observer `8` out of normal dropdown | Open Color dropdown and hover Random/Gold/Pink; resolver returns item-specific color keys; no normal row for observer grey | Do not add `Color(8)` to normal Skirmish dropdown just because the status helper can map it; proposed test `test_skirmish_color_status_item_data_omits_observer_row_8_in_normal_population` |
| Current hovered target already includes the concrete dropdown item | `src/ui/skirmish_shell/state/hit_test.rs:30..37`; generic fallback at `:137..142` | resolver-only delta | `status_help_key_for_hover` or a richer item status resolver | Resolve side/color `ComboItem` before the generic `status_help_key_for_combo` fallback | Hover AI row still uses existing AI item STT; hover side/color row uses new item-specific STT; hover combo face remains generic | Do not change selection state or launch packing for a status-only fix; proposed test `test_skirmish_status_resolver_prefers_side_color_item_rows_over_combo_face_key` |

### Negative Facts / Do Not Do

- Do not replace `SkirmishCountryChoice` with raw `i32` just for status help. Active in YR: Rust-only alignment; the current semantic model maps cleanly to native item data with an explicit table. Evidence: `src/ui/skirmish_shell/state.rs:89..99`; `src/ui/main_menu.rs:53..65`; `0x004E3A00`.
- Do not cast `SkirmishCountry` to a native id. Active in YR: Rust-only risk; Rust enum declarations do not define native item data. Evidence: `src/ui/main_menu.rs:39..65`; native `[Countries]` order at `ini/rulesmd.ini:959..971`.
- Do not add color row `8` to the normal offline color dropdown. Active in YR: No for normal population; `FUN_004E45A0` stops before row `8`, and Rust already tests omission. Evidence: `0x004E45A0`; `src/ui/skirmish_shell/state/tests.rs:1458..1485`.
- Do not use visible dropdown labels as status text. Active in YR: No; native status uses helper-loaded STT strings from item data. Evidence: `0x004E38A0`, `0x004E42A0`, `SKIRMISH_0X102_STATUS_HELP_FULL_MAPPING_CURRENT_RUST_GHIDRA_REPORT.md`.
- Do not change launch/session packing to fix hover status. Active in YR: Rust-only scope; launch uses separate selected values, while hover already carries `SkirmishComboItem`. Evidence: `src/ui/skirmish_shell/state/hit_test.rs:30..37`; `src/ui/skirmish_shell/state/combos.rs:468..498`.

## 11. Remaining Uncertainty

- Exact final STT key strings for each side/color item are owned by slots 1 and 2 of this swarm; this report only proves the Rust item-data alignment needed to feed those tables.
- Online/WOL observer/closed row representation is out-of-scope; standard offline Rust should not infer that model from the helper's conditional `-3` / `8` support.

## 12. Stale Docs / Follow-up Docs

No stale-doc replacement needed. `SKIRMISH_0X102_STATUS_HELP_FULL_MAPPING_CURRENT_RUST_GHIDRA_REPORT.md` still matches current Rust for this slot: non-AI side/color item-specific status is missing, while the generic status resolver and combo item hover target exist.

## Sources

- Read-only Ghidra spot-checks: `0x004E3A00`, `0x004E4170`, `0x004E38A0`, `0x004E45A0`, `0x004E4E20`, `0x004E42A0`.
- Existing reports: `docs/research/skirmish-ui/SKIRMISH_0X102_STATUS_HELP_FULL_MAPPING_CURRENT_RUST_GHIDRA_REPORT.md`; `docs/research/skirmish-ui/SKIRMISH_SIDE_COUNTRY_TEAM_FINAL_WRITES_GHIDRA_REPORT.md`; `docs/research/skirmish-ui/SKIRMISH_COLOR_COMBO_POPULATION_AND_SWATCH_ORDER_GHIDRA_REPORT.md`.
- Rust source: `src/ui/skirmish_shell/state.rs`; `src/ui/skirmish_shell/state/combos.rs`; `src/ui/skirmish_shell/state/hit_test.rs`; `src/ui/main_menu.rs`; `src/skirmish_launch.rs`.
- INI source: `ini/rulesmd.ini:959..987`.
