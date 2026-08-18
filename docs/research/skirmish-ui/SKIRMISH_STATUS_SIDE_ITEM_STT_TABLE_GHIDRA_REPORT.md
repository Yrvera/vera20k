# Skirmish Status Side Item STT Table - Ghidra Research Report

**Address(es):** `0x004E3830`, `0x004E4170`, `0x004E38A0`, caller slice `0x006AE531..0x006AE5C9`, population helper `0x004E3A00`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** exact side/country combo item-data values that feed `FUN_004E38A0` and their `STT:PlayerSide*` status keys, including Random, Observer, and stock YR country rows, plus current Rust handoff.  
**Non-Scope:** color item status, full combo source precedence beyond the side branch, visual dropdown painting, online lobby observer-row population, CSF localized prose text, and Rust implementation.  
**Confidence:** High for item-data bounds, signed values, key table, standard offline population, and current Rust delta; Medium for observer row liveness outside standard offline Skirmish because this slot verified the mapper but did not trace the online lobby population owner.  
**Active in YR:** Yes for standard offline `0x102` side/country status rows that are populated by `FUN_004E3A00`; Conditional for the `-3` Observer mapping, which is present in YR dialog support code but not inserted by the standard offline country population helper.

## 0. Working Notes Gate

- Target question: What exact side/country dropdown item-data values map to which `STT:PlayerSide*` keys in native YR status help?
- Non-goals: Do not expand color rows, generic static fallback, Choose Map, visual dropdown rendering, online lobby observer population, or patch Rust.
- Evidence needed to mark COMPLETE: live parent `0x4E9` caller proof into the side helper chain, side-control recognizer table, item-data retrieval bounds and fallback, exact signed item-data to `STT:PlayerSide*` key table with assembly, standard offline population proof for Random and country rows, Rust surface scan, negative facts, and implementation handoff.
- Stop conditions: stop after `-3`, `-2`, `0..9`, out-of-range, and item `-1` status behavior are resolved or explicitly deferred, and after the handoff can name the Rust mapper/tests.

## 1. Overview

The side/country dropdown item-specific status path is live under the Skirmish dialog `0x102` parent `0x4E9` handler. When the hovered combo row supplies a non-`-1` row index, `FUN_006AE3F0` recognizes side controls with `FUN_004E3830`, reads the row item data with `FUN_004E4170`, and maps the signed item data through `FUN_004E38A0`.

The native table is not based on visible row text. It is a fixed signed item-data table: `-2` is Random, `-3` is Observer, and stock country data `0..9` maps to America, Korea, France, Germany, Britain, Libya, Iraq, Cuba, Russia, and YuriCountry. Standard offline Skirmish population inserts Random and country rows; it does not insert Observer in the scoped helper.

## 2. Side Control Recognizer

Active in YR: Yes. `FUN_006AE3F0` gets the hovered child control id and calls `FUN_004E3830` for non-AI combo controls. `FUN_004E3830` maps the side/country control family to row slots and returns `-1` for non-side controls.

| Control id | Recognizer result | Role | Evidence | Active in YR |
|---:|---:|---|---|---|
| `0x6A1` | `0` | local player side/country combo | `FUN_004E3830`; assembly `0x004E3830..0x004E383B` | Yes |
| `0x510` | `1` | AI row side/country combo | `0x004E383C..0x004E3848` | Yes |
| `0x513` | `2` | AI row side/country combo | `0x004E3849..0x004E3855` | Yes |
| `0x51E` | `3` | AI row side/country combo | `0x004E3856..0x004E3862` | Yes |
| `0x514` | `4` | AI row side/country combo | `0x004E3863..0x004E386F` | Yes |
| `0x51F` | `5` | AI row side/country combo | `0x004E3870..0x004E387C` | Yes |
| `0x520` | `6` | AI row side/country combo | `0x004E387D..0x004E3889` | Yes |
| `0x521` | `7` | AI row side/country combo | `0x004E388A..0x004E3898` | Yes |
| any other id | `-1` | not a side combo for this helper | same computed fallback at `0x004E388A..0x004E3898` | Yes |

## 3. Core Logic

### 3.1 Caller and Item `-1` Gate

Active in YR: Yes. In the `0x4E9` branch, `FUN_006AE3F0` first clears the output string holder, rejects null hovered child handles, and rejects item index `-1` before any item-specific helper is called. For side controls, it then calls `FUN_004E3830`, `FUN_004E4170`, and `FUN_004E38A0`.

Material details:

- Item-specific side status is attempted only when parent message payload item/index is not `-1`; evidence: `0x006AE4C0..0x006AE4CE`.
- The side branch is selected before color/start helper branches; evidence: `0x006AE531..0x006AE547`.
- The side branch passes the combo control id in `EDX`, parent hwnd in `ECX`, and the hovered item index on the stack to `FUN_004E4170`; evidence: `0x006AE5A9..0x006AE5B1`.
- If `FUN_004E4170` returns `-1`, no `FUN_004E38A0` lookup happens and the branch falls through with an empty item-specific result; evidence: `0x006AE5B6..0x006AE5C9`.
- Active in YR: Yes. This is the standard `0x102` dialog proc installed by the offline Skirmish launcher at `0x006AE31C..0x006AE328`, and the common handler reaches this parent `0x4E9` path as verified by the prior status report.

Consequence for face/selected fallback: the parent item-specific path does not use selected row data when the incoming status item is `-1`. `FUN_004E4170` can read the current selection if directly called with item `-1`, but the `0x4E9` status branch blocks `-1` before calling it. Combo-face hover therefore falls back to generic `STT:SkirmishComboCountry`, not the selected country's `STT:PlayerSide*` key. Active in YR: Yes, with evidence `0x006AE4CA..0x006AE4CE` plus `0x004E417D..0x004E419D`.

### 3.2 Item Data Retrieval and Bounds

Active in YR: Yes. `FUN_004E4170(hwnd, control_id, item_index)` reads combo item data with message `0x150` (`CB_GETITEMDATA`). If the requested item index is `-1`, the helper first reads current selection with message `0x147` (`CB_GETCURSEL`), but again the status caller normally prevents this path for `0x4E9`.

Bounds and fallback are signed:

- Valid native side status item-data range is inclusive `-3..9`; evidence: `CMP EAX,-0x3` then `JL fallback`, `CMP EAX,0x9` then `JLE return` at `0x004E419F..0x004E41AA`.
- Out-of-range values first call selected-mode/default-country provider `DAT_00A8B23C` vtable `+0x28` when non-null; evidence: `0x004E41AC..0x004E41BB`.
- If no provider exists, out-of-range falls back to `-2` Random; evidence: `0x004E41BE`.
- Active in YR: Yes. These branches are reached by the live side helper path and by launch/session code that also calls `FUN_004E4170`.

### 3.3 Exact Side Item Data to STT Key Table

Active in YR: Yes for all mapper branches when such item data is supplied. Standard offline `FUN_004E3A00` supplies `-2` and `0..9`; `-3` is a supported YR mapper branch but not inserted by the scoped offline population helper.

| Item data | Source line id | Embedded key pointer | STT key | Stock row meaning | Evidence | Active in YR |
|---:|---:|---:|---|---|---|---|
| `-3` | `0xEF` | `0x00822990` | `STT:PlayerSideObserver` | Observer | `FUN_004E38A0`; assembly `0x004E38BC..0x004E38D7`; `gamemd.exe` string block contains exact key | Conditional |
| `-2` | `0xED` | `0x008229A8` | `STT:PlayerSideRandom` | Random country | `0x004E38A0..0x004E38BB`; `FUN_004E3A00` inserts item data `-2` at `0x004E3A3E..0x004E3A68` | Yes |
| `0` | `0xF1` | `0x00822978` | `STT:PlayerSideAmerica` | Americans / America | `0x004E38D8..0x004E38F2`; `[Countries] 0=Americans` in `rulesmd.ini:960` | Yes |
| `1` | `0xF3` | `0x00822964` | `STT:PlayerSideKorea` | Alliance / Korea | `0x004E38F3..0x004E390E`; `rulesmd.ini:961` | Yes |
| `2` | `0xF5` | `0x0082294C` | `STT:PlayerSideFrance` | French / France | `0x004E390F..0x004E392A`; `rulesmd.ini:962` | Yes |
| `3` | `0xF7` | `0x00822934` | `STT:PlayerSideGermany` | Germans / Germany | `0x004E392B..0x004E3946`; `rulesmd.ini:963` | Yes |
| `4` | `0xF9` | `0x0082291C` | `STT:PlayerSideBritain` | British / Great Britain | `0x004E3947..0x004E3962`; `rulesmd.ini:964` | Yes |
| `5` | `0xFB` | `0x00822908` | `STT:PlayerSideLibya` | Africans / Libya | `0x004E3963..0x004E397E`; `rulesmd.ini:966` | Yes |
| `6` | `0xFD` | `0x008228F4` | `STT:PlayerSideIraq` | Arabs / Iraq | `0x004E397F..0x004E399A`; `rulesmd.ini:967` | Yes |
| `7` | `0xFF` | `0x008228E0` | `STT:PlayerSideCuba` | Confederation / Cuba | `0x004E399B..0x004E39B6`; `rulesmd.ini:968` | Yes |
| `8` | `0x101` | `0x008228C8` | `STT:PlayerSideRussia` | Russians / Russia | `0x004E39B7..0x004E39D2`; `rulesmd.ini:969` | Yes |
| `9` | `0x103` | `0x008228AC` | `STT:PlayerSideYuriCountry` | YuriCountry / Yuri | `0x004E39D3..0x004E39EE`; `rulesmd.ini:971`; `langmd.mix` string block contains exact key | Yes |
| any other post-fallback value that remains unmapped | none | none | null/empty item-specific result | `0x004E39EF..0x004E39F1` | Yes |

Key spelling evidence: ASCII strings in `gamemd.exe` at the `GDlgSupp.cpp` literal block include exactly `STT:PlayerSideYuriCountry`, `STT:PlayerSideRussia`, `STT:PlayerSideCuba`, `STT:PlayerSideIraq`, `STT:PlayerSideLibya`, `STT:PlayerSideBritain`, `STT:PlayerSideGermany`, `STT:PlayerSideFrance`, `STT:PlayerSideKorea`, `STT:PlayerSideAmerica`, `STT:PlayerSideObserver`, and `STT:PlayerSideRandom`. The retail string files contain the same keys split across RA2 base and YR override data: `language.mix` contains Random through Russia and `langmd.mix` contains `STT:PlayerSideYuriCountry` and Observer-family YR additions.

### 3.4 Standard Offline Population

Active in YR: Yes for standard offline Skirmish side/country combos.

`FUN_004E3A00(hwnd, control_id)` clears the combo, sets max visible rows to `7`, inserts `GUI:RandomEx` with item data `-2`, then loops `g_HouseTypeClass_Array`. A country row is inserted only when the country pointer is non-null, selectable byte `+0x1A5` is nonzero, self/index field `+0xB8` is greater than `-3` and less than `10`, and UI name pointer `+0x60` points to a non-empty string.

This means the standard offline helper can insert item data `-2` and stock country indices `0..9`; it does not insert a row whose item data is `-3` because the condition is `-3 < index`, not `-3 <= index`. Evidence: `FUN_004E3A00` decompile and assembly `0x004E3A3E..0x004E3A68` for Random, `0x004E3A6A..0x004E3AC8` for the country loop.

The stock YR `[Countries]` order supplies the country item data used by the mapper: `0=Americans`, `1=Alliance`, `2=French`, `3=Germans`, `4=British`, `5=Africans`, `6=Arabs`, `7=Confederation`, `8=Russians`, `9=YuriCountry` (`ini/rulesmd.ini:959..971`). The user-facing row labels are not the status keys; the status keys use the geopolitical display names in the mapper table above.

## 4. INI Keys

No INI key controls the `STT:PlayerSide*` status mapping. INI only supplies the stock country order/data that becomes `CountryTypeClass+0xB8` and therefore the combo item data.

| INI source | Value | Effect in this slice | Evidence | Active in YR |
|---|---|---|---|---|
| `[Countries]` | `0..9` stock YR country list | Matches item data accepted by `FUN_004E38A0` and inserted by `FUN_004E3A00` | `ini/rulesmd.ini:959..971`; binary reader/consumer `CountryTypeClass+0xB8` in `FUN_004E3A00` | Yes |
| Country sections `UIName=` | non-empty UI label pointers | `FUN_004E3A00` requires non-empty `+0x60` before inserting row | `ini/rulesmd.ini:3219..3332`; `0x004E3A6A..0x004E3AC8` | Yes |

## 5. Integration Points

| Point | Role | Evidence | Active in YR |
|---|---|---|---|
| `FUN_006AE3F0` | Standard Skirmish `0x102` dialog proc; routes parent `0x4E9` side item status | `0x006AE31C..0x006AE328`, `0x006AE4B4..0x006AE5C9` | Yes |
| `FUN_004E3830` | side/country control family recognizer | `0x004E3830..0x004E3898` | Yes |
| `FUN_004E4170` | side combo item-data reader, selected/default fallback helper | `0x004E4170..0x004E41C3` | Yes |
| `FUN_004E38A0` | signed item-data to `STT:PlayerSide*` status-key mapper | `0x004E38A0..0x004E39F1` | Yes/Conditional by row |
| `FUN_004E3A00` | standard side/country combo population | `0x004E3A00..`; decompile plus assembly for Random and loop | Yes |

## 6. Current Rust Implementation Status

Current Rust models side combo items as enum values, not native numeric item data. It generates Random plus `SkirmishCountry::ALL` in stock order and selected state in `src/ui/skirmish_shell/state/combos.rs`, while `status_help_key_for_hover` maps non-AI `ComboItem` side hovers to generic `STT:SkirmishComboCountry` in `src/ui/skirmish_shell/state/hit_test.rs`.

Rust therefore has enough semantic data to build a mapper without changing the country model for stock rows, but it does not yet expose item-specific `STT:PlayerSide*` status keys. Observer is not currently a stock skirmish country item and should not be added to the standard offline side dropdown just to satisfy the mapper.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `0x102` parent side item status branch | verified | `0x006AE531..0x006AE5C9` | none |
| side control recognizer ids | verified | `FUN_004E3830`, `0x004E3830..0x004E3898` | none |
| item `-1` status behavior | verified | `0x006AE4CA..0x006AE4CE`; `0x004E417D..0x004E419D` | runtime message cadence belongs to slot 3 |
| item-data signed range `-3..9` | verified | `0x004E419F..0x004E41AA` | none |
| out-of-range fallback to mode default or Random | verified | `0x004E41AC..0x004E41BE` | exact selected-mode provider return values out of scope |
| `FUN_004E38A0` table | verified | `0x004E38A0..0x004E39F1`; embedded key strings | none |
| standard offline population Random/countries | verified | `FUN_004E3A00`, `rulesmd.ini:959..971` | none |
| Observer row population in online/lobby UI | deferred | mapper branch exists, offline helper excludes `-3` | separate online/lobby investigation |
| current Rust side combo item generation | verified | `src/ui/skirmish_shell/state/combos.rs`; `src/ui/main_menu.rs` scan | implementation remains |
| current Rust status mapping | verified | `src/ui/skirmish_shell/state/hit_test.rs` scan | implementation remains |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - Is the side item status branch active in standard YR 0x102? -> Yes, `FUN_006AE3F0` is installed for dialog `0x102` and routes parent `0x4E9` side controls through the helper chain.` (evidence: `0x006AE31C..0x006AE328`, `0x006AE531..0x006AE5C9`)
- `[RESOLVED] OQ-02 - Which controls are side/country controls? -> `0x6A1`, `0x510`, `0x513`, `0x51E`, `0x514`, `0x51F`, `0x520`, `0x521`.` (evidence: `FUN_004E3830`)
- `[RESOLVED] OQ-03 - What happens for status item index `-1`? -> The status caller exits item-specific handling before `FUN_004E4170`, so generic combo fallback wins for face/selected hover.` (evidence: `0x006AE4CA..0x006AE4CE`)
- `[RESOLVED] OQ-04 - Is `FUN_004E4170` signed? -> Yes, valid bounds compare against signed `-3` and `9`.` (evidence: `0x004E419F..0x004E41AA`)
- `[RESOLVED] OQ-05 - What is Random item data? -> `-2` and key `STT:PlayerSideRandom`.` (evidence: `0x004E38A0..0x004E38BB`, `0x004E3A3E..0x004E3A68`)
- `[RESOLVED] OQ-06 - What is Observer item data? -> `-3` and key `STT:PlayerSideObserver`, but not inserted by standard offline population.` (evidence: `0x004E38BC..0x004E38D7`, `0x004E3A6A..0x004E3AC8`)
- `[RESOLVED] OQ-07 - What are country item data values? -> Stock YR country self/index values `0..9` map to the exact table in section 3.3.` (evidence: `0x004E38D8..0x004E39EE`, `rulesmd.ini:959..971`)
- `[RESOLVED] OQ-08 - Is Yuri included in stock YR side status? -> Yes, item data `9` maps to `STT:PlayerSideYuriCountry`.` (evidence: `0x004E39D3..0x004E39EE`, `rulesmd.ini:971`, `langmd.mix` key string)
- `[RESOLVED] OQ-09 - Are visible row labels the same as status keys? -> No, row labels come from Random/UIName strings, while status comes from `STT:PlayerSide*`.` (evidence: `FUN_004E3A00`, `FUN_004E38A0`)
- `[RESOLVED] OQ-10 - Does out-of-range item data fall directly to no text? -> Not first; `FUN_004E4170` tries selected-mode provider `+0x28`, else Random `-2`.` (evidence: `0x004E41AC..0x004E41BE`)
- `[RESOLVED] OQ-11 - Does current Rust already use item-specific side status keys? -> No, it falls back to `STT:SkirmishComboCountry` for non-AI combo items.` (evidence: `src/ui/skirmish_shell/state/hit_test.rs` search)
- `[RESOLVED] OQ-12 - Can Rust map existing stock enum values without changing dropdown order? -> Yes for Random plus `SkirmishCountry::ALL`, whose order matches stock YR `0..9`.` (evidence: `src/ui/main_menu.rs`, `src/ui/skirmish_shell/state/combos.rs`, `rulesmd.ini:959..971`)
- `[DEFERRED] OQ-13 - Which online/lobby owner inserts Observer `-3` into side combos?` (category: `out-of-scope`; reason: this slot only claims the mapper and standard offline population; next-step-if-pursued: investigate online/lobby observer combo population path)
- `[DEFERRED] OQ-14 - Exact selected-mode provider `DAT_00A8B23C +0x28` returns for every mode when item data is invalid.` (category: `out-of-scope`; reason: standard populated side rows are in range; next-step-if-pursued: selected-mode default-country callback report)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Open side/country dropdown row hover maps signed item data to exact `STT:PlayerSide*` keys before generic combo fallback | `0x006AE5A9..0x006AE5C9`; `0x004E38A0..0x004E39F1` | missing | `src/ui/skirmish_shell/state/hit_test.rs` or a richer status resolver near combo hover handling | Add a side item status mapper for Random and countries in the exact native table | Open Side dropdown, hover Random/America/Yuri, and get `STT:PlayerSideRandom`, `STT:PlayerSideAmerica`, `STT:PlayerSideYuriCountry` rather than generic `STT:SkirmishComboCountry`; proposed test `test_skirmish_status_side_item_keys_match_native` | Do not derive status keys from visible labels or enum debug names |
| Combo face/item `-1` status does not use selected country item-specific text | `0x006AE4CA..0x006AE4CE`; `0x004E417D..0x004E419D` | likely generic behavior already matches for face hover | `src/ui/skirmish_shell/state/hit_test.rs` | Keep collapsed side combo hover on generic `STT:SkirmishComboCountry`; only open row item hovers use `STT:PlayerSide*` | Select Great Britain, close dropdown, hover collapsed side combo, and still get generic country-combo help; proposed test `test_skirmish_status_side_face_uses_generic_country_key` | Do not show selected-country tooltip on the closed combo face |
| Standard offline side dropdown population includes Random `-2` and country values `0..9`, not Observer `-3` | `0x004E3A3E..0x004E3A68`; `0x004E3A6A..0x004E3AC8`; `rulesmd.ini:959..971` | current stock row order is aligned; observer absent is aligned | `src/ui/skirmish_shell/state/combos.rs`, `src/ui/main_menu.rs` | Add the mapper without adding an Observer row to standard offline Skirmish | Row count remains 11: Random plus 10 countries; proposed test `test_skirmish_side_dropdown_stock_rows_exclude_observer` | Do not add Observer to offline Skirmish side choices just because the status mapper supports `-3` |

### Negative Facts / Do Not Do

- Do not use `STT:SkirmishComboCountry` for open side dropdown rows. Active in YR: Yes. Evidence: side branch calls `FUN_004E4170 -> FUN_004E38A0` at `0x006AE5A9..0x006AE5C9`.
- Do not use visible row labels (`America`, `Great Britain`, `Yuri`) as status text or key derivation. Active in YR: Yes. Evidence: row labels are inserted by `FUN_004E3A00`, while status keys are fixed in `FUN_004E38A0`.
- Do not map item data `4` to `STT:PlayerSideGreatBritain`; the exact key is `STT:PlayerSideBritain`. Active in YR: Yes. Evidence: `0x004E394C..0x004E395D`, embedded key pointer `0x0082291C`.
- Do not add an Observer row to standard offline Skirmish side dropdown. Active in YR: Yes for offline population. Evidence: `FUN_004E3A00` inserts Random separately and only country indices with `-3 < index < 10`; `-3` fails that condition.
- Do not implement item-specific selected-country help for collapsed side combo face hover. Active in YR: Yes. Evidence: parent `0x4E9` item-specific path rejects item `-1` before calling `FUN_004E4170`.

### Stale Docs / Follow-up Docs

- `docs/research/skirmish-ui/SKIRMISH_0X102_STATUS_HELP_FULL_MAPPING_CURRENT_RUST_GHIDRA_REPORT.md`: replace "exact numeric id -> every country key table if needed" with "Resolved by `SKIRMISH_STATUS_SIDE_ITEM_STT_TABLE_GHIDRA_REPORT.md`: side item data maps `-3 -> STT:PlayerSideObserver`, `-2 -> STT:PlayerSideRandom`, and `0..9 -> STT:PlayerSideAmerica/Korea/France/Germany/Britain/Libya/Iraq/Cuba/Russia/YuriCountry`; standard offline population inserts `-2` and `0..9`, not Observer."
- `docs/research/skirmish-ui/SKIRMISH_STATUS_CHILD_0X695_TEXT_SOURCE_GHIDRA_REPORT.md`: replace "This report did not expand those helper families" for side/country with "The side/country helper family is now expanded in `SKIRMISH_STATUS_SIDE_ITEM_STT_TABLE_GHIDRA_REPORT.md`; use its exact item-data table for side dropdown item-specific status."

## Sources

- Ghidra decompile: `FUN_004E3830`, `FUN_004E4170`, `FUN_004E38A0`, `FUN_004E3A00`, `FUN_006AE3F0`.
- Ghidra assembly contexts: `0x006AE31C..0x006AE328`, `0x006AE4C0..0x006AE5C9`, `0x004E3830..0x004E3898`, `0x004E38A0..0x004E39F1`, `0x004E4170..0x004E41C3`, `0x004E3A3E..0x004E3AC8`.
- Retail string evidence: ASCII string blocks in `gamemd.exe`, `language.mix`, and `langmd.mix` for `STT:PlayerSide*` keys.
- INI data: `ini/rulesmd.ini:959`, `:960..971`, country sections `:3219..3332`.
- Prior docs: `SKIRMISH_0X102_STATUS_HELP_FULL_MAPPING_CURRENT_RUST_GHIDRA_REPORT.md`, `SKIRMISH_STATUS_CHILD_0X695_TEXT_SOURCE_GHIDRA_REPORT.md`, `SKIRMISH_SIDE_COMBO_OPEN_SCROLL_SELECT_SOUND_TRACE.md`.
- Rust scan: `src/ui/main_menu.rs`, `src/ui/skirmish_shell/state/combos.rs`, `src/ui/skirmish_shell/state/hit_test.rs`.
