# Skirmish Status Side/Color String Availability - Ghidra Research Report

**Address(es):** `0x004E38A0`, `0x004E42A0`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** exact stock YR CSF/STT key spelling and availability for side/country and color status strings returned by the native skirmish side/color status helpers.
**Non-Scope:** full hover/status resolver ordering, combo item-data population, owner-draw rendering, localized display text contents, non-stock mods, and online lobby behavior beyond shared helper support.
**Confidence:** High for exact key spelling and stock YR resource availability.
**Active in YR:** Yes for the helper families and ordinary side/color key families; Conditional for observer rows because the helpers support observer item data, while standard offline Skirmish row insertion is covered by separate reports.

## 0. Working Notes Seed

- Target question: Do all stock YR side/country and color status keys used by native skirmish side/color status helpers exist in the stock string resources, with exact spelling?
- Non-goals: Do not re-investigate full status update ordering, row population, owner-draw visuals, or Rust implementation edits.
- Evidence needed to mark COMPLETE: Ghidra helper branch-to-key proof plus stock `langmd.mix` CSF label-entry proof for every key, including Yuri/observer/color edge rows.
- Stop conditions: Stop after exact key availability and spelling are verified; list row-population and resolver-order gaps as out-of-scope rather than expanding.

## 1. Overview

The native side/country status helper maps item data `-2`, `-3`, and `0..9` to exact `STT:PlayerSide*` keys. The native color status helper maps item data `-2` and `0..8` to exact `STT:PlayerColor*` keys. Every key returned by these two helpers exists in stock YR `langmd.mix` as a CSF `LBL` entry with one string pair, so Rust can use the exact keys without falling into missing-key behavior for stock YR.

## 2. Verified Side/Color Status Keys

### Side/Country Helper `0x004E38A0`

Active in YR: Yes. `FUN_006AE3F0` handles parent message `0x4E9`; after `FUN_004E3830` recognizes side combo control ids, it calls `FUN_004E4170(param_4[1])` and then `0x004E38A0` before writing status text through `FUN_007B6880`.

| Item data | Exact key | Binary evidence | CSF resource evidence | Active in YR |
|---:|---|---|---|---|
| `-2` | `STT:PlayerSideRandom` | `0x004E38A0`, branch at `0x004E38A0`, string ptr `0x008229A8` | `langmd.mix:469057`, `LBL`, pairs `1`, len `20` | Yes |
| `-3` | `STT:PlayerSideObserver` | `0x004E38BC`, string ptr `0x00822990` | `langmd.mix:518660`, `LBL`, pairs `1`, len `22` | Conditional; helper-supported observer item data |
| `0` | `STT:PlayerSideAmerica` | `0x004E38D8`, string ptr `0x00822978` | `langmd.mix:469155`, `LBL`, pairs `1`, len `21` | Yes |
| `1` | `STT:PlayerSideKorea` | `0x004E38F3`, string ptr `0x00822964` | `langmd.mix:469260`, `LBL`, pairs `1`, len `19` | Yes |
| `2` | `STT:PlayerSideFrance` | `0x004E390F`, string ptr `0x0082294C` | `langmd.mix:469369`, `LBL`, pairs `1`, len `20` | Yes |
| `3` | `STT:PlayerSideGermany` | `0x004E392B`, string ptr `0x00822934` | `langmd.mix:469519`, `LBL`, pairs `1`, len `21` | Yes |
| `4` | `STT:PlayerSideBritain` | `0x004E3947`, string ptr `0x0082291C` | `langmd.mix:469636`, `LBL`, pairs `1`, len `21` | Yes |
| `5` | `STT:PlayerSideLibya` | `0x004E3963`, string ptr `0x00822908` | `langmd.mix:469737`, `LBL`, pairs `1`, len `19` | Yes |
| `6` | `STT:PlayerSideIraq` | `0x004E397F`, string ptr `0x008228F4` | `langmd.mix:469848`, `LBL`, pairs `1`, len `18` | Yes |
| `7` | `STT:PlayerSideCuba` | `0x004E399B`, string ptr `0x008228E0` | `langmd.mix:469942`, `LBL`, pairs `1`, len `18` | Yes |
| `8` | `STT:PlayerSideRussia` | `0x004E39B7`, string ptr `0x008228C8` | `langmd.mix:470036`, `LBL`, pairs `1`, len `20` | Yes |
| `9` | `STT:PlayerSideYuriCountry` | `0x004E39D3`, string ptr `0x008228AC` | `langmd.mix:524676`, `LBL`, pairs `1`, len `25` | Yes |

Out-of-range side item data returns `0` from `0x004E38A0`. Active in YR: Yes, as the helper's default branch.

### Color Helper `0x004E42A0`

Active in YR: Yes. `FUN_006AE3F0` handles parent message `0x4E9`; after `FUN_004E4230` recognizes color combo control ids, it calls `FUN_004E4E20(param_4[1])` and then `0x004E42A0` before writing status text through `FUN_007B6880`.

| Item data | Exact key | Binary evidence | CSF resource evidence | Active in YR |
|---:|---|---|---|---|
| `-2` | `STT:PlayerColorRandom` | `0x004E42A0`, string ptr `0x00822AC4` | `langmd.mix:470138`, `LBL`, pairs `1`, len `21` | Yes |
| `0` | `STT:PlayerColorGold` | `0x004E42BC`, string ptr `0x00822AB0` | `langmd.mix:470235`, `LBL`, pairs `1`, len `19` | Yes |
| `1` | `STT:PlayerColorRed` | `0x004E42D7`, string ptr `0x00822A9C` | `langmd.mix:470324`, `LBL`, pairs `1`, len `18` | Yes |
| `2` | `STT:PlayerColorBlue` | `0x004E42F3`, string ptr `0x00822A88` | `langmd.mix:470406`, `LBL`, pairs `1`, len `19` | Yes |
| `3` | `STT:PlayerColorGreen` | `0x004E430F`, string ptr `0x00822A70` | `langmd.mix:470491`, `LBL`, pairs `1`, len `20` | Yes |
| `4` | `STT:PlayerColorOrange` | `0x004E432B`, string ptr `0x00822A58` | `langmd.mix:470579`, `LBL`, pairs `1`, len `21` | Yes |
| `5` | `STT:PlayerColorSkyBlue` | `0x004E4347`, string ptr `0x00822A40` | `langmd.mix:470670`, `LBL`, pairs `1`, len `22` | Yes |
| `6` | `STT:PlayerColorPurple` | `0x004E4363`, string ptr `0x00822A28` | `langmd.mix:470770`, `LBL`, pairs `1`, len `21` | Yes |
| `7` | `STT:PlayerColorPink` | `0x004E437F`, string ptr `0x00822A14` | `langmd.mix:470861`, `LBL`, pairs `1`, len `19` | Yes |
| `8` | `STT:PlayerColorObserver` | `0x004E439B`, string ptr `0x008229FC` | `langmd.mix:518559`, `LBL`, pairs `1`, len `23` | Conditional; helper-supported observer color, normal offline insertion covered elsewhere |

Out-of-range color item data returns `0` from `0x004E42A0`. Active in YR: Yes, as the helper's default branch.

## 3. Resource Availability Findings

- Active in YR: Yes. `langmd.mix` contains all 22 exact side/color status keys as CSF label entries; each inspected key has preceding marker ` LBL`, pair count `1`, and a label length matching the exact key string.
- Active in YR: Yes. Base `language.mix` also contains the RA2-era side/color keys, but `STT:PlayerSideYuriCountry` is present in `langmd.mix`; stock YR code and resources must use the YR language archive path.
- Active in YR: Yes. The exact Yuri side key spelling is `STT:PlayerSideYuriCountry`; raw resource search for `STT:PlayerSideYuri[A-Za-z]*` in `langmd.mix` returns only that key.
- Active in YR: Yes. The exact sky-blue color spelling is `STT:PlayerColorSkyBlue`, not `STT:PlayerColorLightBlue`, `STT:PlayerColorCyan`, or a spaced variant.
- Active in YR: Conditional. `STT:PlayerColorObserver` and `STT:PlayerSideObserver` both exist in stock `langmd.mix` and are returned by the helpers for observer item data, even if a particular standard offline dropdown population path does not normally show an observer row.

## 4. Current Rust Implementation Status

Current Rust status help in `C:/Users/enok/Documents/ra2-rust-game/src/ui/skirmish_shell/state/hit_test.rs` returns generic combo help for side/color controls: `STT:SkirmishComboCountry` and `STT:SkirmishComboColor` at lines `196..203`. It has AI-row item-specific keys at lines `206..212`, but no side/country or color item-data-specific status key table yet. Active in YR: mismatch for open side/color dropdown item help when native item-specific helpers apply.

## 5. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Side status helper key mapping | verified | `0x004E38A0..0x004E39EF`; string ptrs `0x008228AC..0x008229A8` | none for key spelling/availability |
| Color status helper key mapping | verified | `0x004E42A0..0x004E43B7`; string ptrs `0x008229FC..0x00822AC4` | none for key spelling/availability |
| Stock YR resource availability | verified | `langmd.mix` offsets listed above; every entry marker ` LBL`, pairs `1` | none |
| Helper call path from status resolver | verified | `FUN_006AE3F0` parent message `0x4E9` side/color branches; `FUN_004E3830`, `FUN_004E4230` control-id recognizers; `FUN_004E4170`, `FUN_004E4E20` item-data readers | resolver ordering details are slot 3's target, not repeated here |
| Normal row population / observer visibility | deferred | prior color/side population reports | out-of-scope for this string-availability slot |
| Localized display text values | deferred | resource key availability is sufficient for this target | out-of-scope; would require decoding values, not needed for missing-key risk |

## 6. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-001 - Is `STT:PlayerSideYuriCountry` the exact Yuri side status key? -> Yes; `0x004E39D3` maps item data `9` to string ptr `0x008228AC`, and `langmd.mix:524676` is a CSF `LBL` for `STT:PlayerSideYuriCountry`.` (evidence: `0x004E39D3`; `langmd.mix:524676`)
- `[RESOLVED] OQ-002 - Do all ten country rows have stock side status keys? -> Yes; item data `0..9` maps to America, Korea, France, Germany, Britain, Libya, Iraq, Cuba, Russia, YuriCountry, and each key exists in `langmd.mix`.` (evidence: `0x004E38D8..0x004E39E9`; `langmd.mix:469155..524676`)
- `[RESOLVED] OQ-003 - Do Random and Observer side status keys exist? -> Yes; `-2` maps to Random and `-3` maps to Observer, and both are CSF `LBL` entries in `langmd.mix`.` (evidence: `0x004E38A0..0x004E38D2`; `langmd.mix:469057`, `langmd.mix:518660`)
- `[RESOLVED] OQ-004 - Do all color status keys, including Observer, exist? -> Yes; item data `-2`, `0..8` maps to Random, Gold, Red, Blue, Green, Orange, SkyBlue, Purple, Pink, Observer, and every key is in `langmd.mix`.` (evidence: `0x004E42A0..0x004E43B1`; `langmd.mix:470138..518559`)
- `[RESOLVED] OQ-005 - Does helper support imply all helper keys are safe to hand to Rust localization? -> Yes for stock YR resources; all helper-returned labels are present as exact CSF labels, so no stock missing-key fallback is expected from these keys.` (evidence: `langmd.mix` CSF `LBL` marker checks)
- `[DEFERRED] OQ-006 - Which observer rows are normally inserted in standard offline Skirmish?` (category: `out-of-scope`; reason: row population is covered by adjacent swarm slots/reports; next-step-if-pursued: verify side/color population helpers, not CSF availability)
- `[DEFERRED] OQ-007 - Does hovered dropdown row text beat generic combo face help in every resolver branch?` (category: `out-of-scope`; reason: assigned to slot 3; next-step-if-pursued: trace `0x4E8 -> 0x4E9 -> FUN_006040B0`)

## 7. Negative Facts / Do Not Do

- Do not spell the Yuri status key as `STT:PlayerSideYuri`; stock `langmd.mix` search for `STT:PlayerSideYuri[A-Za-z]*` returns only `STT:PlayerSideYuriCountry`, and `0x004E39D3` points to `0x008228AC`.
- Do not replace item-specific side/color status with the visible combo row labels; native helpers return `STT:PlayerSide*`/`STT:PlayerColor*` status keys, not `GUI:*` row labels. Evidence: `0x004E38A0`, `0x004E42A0`.
- Do not omit observer status keys from the mapper just because standard offline row population may hide observer rows; helpers explicitly support side `-3` and color `8`, and both keys exist in `langmd.mix`. Evidence: `0x004E38BC`, `0x004E439B`, `langmd.mix:518660`, `langmd.mix:518559`.
- Do not invent a missing-key fallback for these exact stock keys; every helper-returned key has a stock YR CSF `LBL` entry. Evidence: marker/pair checks at all listed `langmd.mix` offsets.
- Do not source Yuri-side availability from base `language.mix` alone; `STT:PlayerSideYuriCountry` is YR-specific and verified in `langmd.mix`. Evidence: `langmd.mix:524676`.

## 8. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Native side status helper maps item data `-2`, `-3`, `0..9` to exact side keys listed above, all present in stock YR resources | `0x004E38A0..0x004E39E9`; `langmd.mix` CSF `LBL` offsets | missing | `src/ui/skirmish_shell/state/hit_test.rs` status help resolution or a new side item-data mapper | Open side dropdown item status uses exact item-specific `STT:PlayerSide*` keys, including `STT:PlayerSideYuriCountry` | deterministic test covers every side item-data value and confirms each mapped key exists in loaded stock YR CSF; proposed test `test_skirmish_status_side_color_keys_exist_in_stock_yr` | Do not substitute generic `STT:SkirmishComboCountry` when a hovered/selected row item data is available |
| Native color status helper maps item data `-2`, `0..8` to exact color keys listed above, all present in stock YR resources | `0x004E42A0..0x004E43B1`; `langmd.mix` CSF `LBL` offsets | missing | `src/ui/skirmish_shell/state/hit_test.rs` status help resolution or a new color item-data mapper | Open color dropdown item status uses exact item-specific `STT:PlayerColor*` keys, including `SkyBlue` and `Observer` | deterministic test covers every color item-data value and confirms exact key spelling; proposed test `test_skirmish_status_color_item_data_maps_to_stock_yr_keys` | Do not rename `SkyBlue`, omit observer, or map item data by display order guesses |
| Out-of-range side/color item data returns null from the native helper, leaving fallback to the wider resolver rather than fabricating a key | `0x004E39EF`, `0x004E43B7` | unchecked/missing | status resolver around side/color dropdown item-data handling | Invalid item data should not produce a made-up `STT:PlayerSide*` or `STT:PlayerColor*` key | resolver test passes invalid side/color item data and expects fallback path behavior; proposed test `test_skirmish_status_side_color_invalid_item_data_falls_back` | Do not build dynamic keys from country/color names |

## 9. Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/units/structures/YACNST.md`: no replacement needed for the existing `STT:PlayerSideYuriCountry` wording; this report confirms that exact key spelling.
- `C:/Users/enok/Documents/ra2-rust-game-docs/LOBBY_SESSION_HOUSE_CREATION_GHIDRA_REPORT.md`: no replacement needed for the listed color key spelling/order as a key-availability claim; this report confirms the exact stock keys including `STT:PlayerColorObserver`.

## 10. Remaining Uncertainty

None for stock YR key availability and spelling. Row insertion and resolver precedence remain intentionally out-of-scope for this slot.

## Sources

- Ghidra decompile/assembly contexts: `0x004E38A0`, `0x004E42A0`.
- Ghidra activation path spot-check: `FUN_006AE3F0`, `FUN_004E3830`, `FUN_004E4230`, `FUN_004E4170`, `FUN_004E4E20`.
- Ghidra string search: `STT:PlayerSide*` at `0x008228AC..0x008229A8`; `STT:PlayerColor*` at `0x008229FC..0x00822AC4`.
- Stock YR resource search: `C:/Users/enok/Documents/Command and Conquer Red Alert II/langmd.mix`, exact offsets listed above.
- CSF label marker verification: each listed `langmd.mix` offset is preceded by ` LBL`, pair count `1`, and exact label length.
- Prior reports referenced narrowly: `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_0X102_STATUS_HELP_FULL_MAPPING_CURRENT_RUST_GHIDRA_REPORT.md`, `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_COLOR_COMBO_POPULATION_AND_SWATCH_ORDER_GHIDRA_REPORT.md`.
