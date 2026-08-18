# SpecialFlags MCVDeploy Bit Conflict Recheck - Ghidra Research Report

**Address(es):** `0x006B8CA0`, `0x006B8B30`, `0x006B8AE0`, `0x006886B0`, `0x004FC060`, `0x00740DF0`, `0x00686890`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** exact `[SpecialFlags] MCVDeploy` parser/writer bit, null-mode `Generate_Random_Units` runtime mask before `Force_MCV_Deploy`, selected-vs-null distinction, and Rust parser implication.
**Non-Scope:** deployment mission mechanics, construction-yard conversion, full session packet packing, and selected-mode MCV placement internals.
**Confidence:** High for parser/writer bit `0x0100`, high for null-mode mask `0x10`, high for selected/null dispatch, medium for the semantic source of low bit `0x10` because full session-staging composition was not re-audited.
**Active in YR:** Conditional. SpecialFlags load/save are active in YR scenario handling. The `0x006886B0 -> 0x004FC060` call is active only on the null-mode startup branch when no selected MPModes object is installed; ordinary selected Skirmish does not use it.

## 0. Working Notes

- Target question: Is `[SpecialFlags] MCVDeploy` bit `0x0100` or `0x10`, and what exact flag does null-mode `Generate_Random_Units` test before calling `Force_MCV_Deploy`?
- Non-goals: Do not investigate deploy mission mechanics, selected-mode callback behavior, or full game-option packet/session packing beyond this bit conflict.
- Evidence needed to mark COMPLETE: Ghidra load/save mapping for key `MCVDeploy`; assembly context for `0x006886B0` runtime test and `0x004FC060` call; direct-call proof for `0x004FC060`; selected-vs-null dispatch proof; Rust parser scan.
- Stop conditions: stop once parser/writer mapping, null-mode mask, direct callsite inventory, selected/null distinction, and Rust parser implication are all evidenced.

## 1. Overview

The stale-doc conflict resolves into two different facts. `[SpecialFlags] MCVDeploy` is loaded and saved as bit 8, `0x0100`, in the active flags dword. The null-mode starting-unit generator does not test that bit before calling `Force_MCV_Deploy`; it loads the active flags pointer and tests byte bit `0x10` at `[ScenarioClass+0]`.

Therefore old wording saying "MCVDeploy is runtime `& 0x10`" is wrong for the `[SpecialFlags]` key. The correct wording is: parser/writer `MCVDeploy` is `0x0100`; the only observed null-mode `Force_MCV_Deploy` call is gated by `0x10`, a separate low/session flag known from sibling docs as `CaptureTheFlag`/session-staging state.

## 2. Class Layout / Key Offsets

| Offset / global | Type | Purpose | Evidence | Active in YR |
|---|---:|---|---|---|
| `DAT_00A8B230` | pointer to active flags dword | source read by `0x006886B0` | `0x00688BF2` loads pointer, `0x00688BF8` tests byte `[ECX]` | Conditional |
| bit 8 / `0x0100` | bool | `[SpecialFlags] MCVDeploy` | load `0x006B8CDB..0x006B8D00`; save `0x006B8BAC..0x006B8BBB` | Yes for load/save |
| byte bit `0x10` | bool | null-mode gate before `Force_MCV_Deploy`; not the parser bit | `0x00688BF2..0x00688C02` | Conditional, null-mode only |
| `RulesClass+0x14B2` | bool | `CaptureTheFlag` default source from `[MultiplayerDialogSettings]` | `0x006720D7..0x006720FC`; `rulesmd.ini:3035` | Conditional/default off |
| `DAT_00A8B23C` | selected mode pointer | selects null-mode generator vs selected callbacks | `0x00686890` | Yes |
| `House+0x53DC` | pointer | primary object/factory pointer written by helper | `0x004FC060` | Conditional |
| `Unit+0x81` | byte | limbo guard | `0x004FC060` | Conditional |
| `Unit[0x1B3]` | int field | deploy target set before queueing mission `2` | `0x00740DF0` | Conditional |

## 3. Core Logic

`FUN_006B8CA0` reads `[SpecialFlags] MCVDeploy` with default `(*flags >> 8) & 1`, then writes `(value & 1) << 8` after clearing old bit `0x0100` with `0xFFFFFEFF`. `FUN_006B8B30` saves `MCVDeploy` from `(*flags >> 8) & 1`. Active in YR: Yes. Evidence: `0x006B8CDB..0x006B8D00`, `0x006B8BAC..0x006B8BBB`.

`FUN_006B8AE0` applies `*flags = *flags & 0xFFF88088 | 0x8088`. This clears bit 8 (`0x0100`), so parsed `MCVDeploy` defaults off unless a later INI/session path sets it. Active in YR: Yes. Evidence: `0x006B8AE0..0x006B8AEC`.

After successful null-mode MCV placement, `ScenarioClass__Generate_Random_Units @ 0x006886B0` clears `House+0x53DC`/`House+0x53E0`, loads `DAT_00A8B230`, tests `TEST byte ptr [ECX], 0x10`, and calls `0x004FC060` only if that low-byte bit is set. This is not the parsed `MCVDeploy` bit: dword `0x0100` is byte offset `+1`, bit `0x01`, not byte `[flags] & 0x10`. Active in YR: Conditional. `0x006886B0` runs only when `Post_Map_Init` sees `DAT_00A8B23C == 0`. Evidence: `0x00686890`; assembly context `0x00688BF2..0x00688C02`.

The exact direct-call byte pattern for `CALL 0x004FC060` occurs once, at `0x00688C02`. This corroborates the prior selected-mode report: selected callbacks do not contain a direct helper call. Active in YR: Conditional/null-mode only for the observed direct call. Evidence: byte search `E8 59 34 E7 FF -> 0x00688C02`.

`0x004FC060` returns `0` for null unit or `Unit+0x81 != 0`. Otherwise it clears previous house primary state, calls `0x00740DF0`, stores the MCV pointer into `House+0x53DC`, and returns `1`. `0x00740DF0` only proceeds when deploy target is not `-1` and `Unit[0x1B3] == -1`; it then writes `Unit[0x1B3]` and queues mission `2` through vtable `+0x124`. Active in YR: Conditional if reached. Evidence: decompile `0x004FC060`, `0x00740DF0`.

## 4. INI Keys

| Key | Location | Native bit / field | Default / stock value | Effect in this slice | Evidence | Active in YR |
|---|---|---:|---|---|---|---|
| `MCVDeploy` | `[SpecialFlags]` | bit 8 / `0x0100` | reset default off | load/save only; no observed startup helper gate tests this bit | `0x006B8CA0`, `0x006B8B30`, `0x00688BF8` | Yes for load/save |
| `InitialVeteran` | `[SpecialFlags]` | bit 9 / `0x0200` | off | nearby parser key and starting-unit consumer, confirms bit sequence | `0x006B8D08..0x006B8D28`; `0x00688D4B` | Yes |
| `CaptureTheFlag` | `[MultiplayerDialogSettings]` | `RulesClass+0x14B2`, sibling docs map to low bit `0x10` | `no` | explains the attractive `0x10` mislabel; full staging audit deferred | `0x006720D7..0x006720FC`; `rulesmd.ini:3035`; `SPECIAL_FLAGS_SYSTEM.md` | Conditional/default off |
| `MCVRedeploys` | `[MultiplayerDialogSettings]` | lobby/session byte | `yes` | separate from `[SpecialFlags] MCVDeploy`; Rust `mcv_redeploy` belongs here | `rulesmd.ini:3041`; Rust scan | Yes |
| `Bases` | `[MultiplayerDialogSettings]` | `DAT_00A8B258` | `yes` | MCV creation gate before helper can matter | `0x006886B0`; `rulesmd.ini:3032` | Yes |

## 5. Integration Points

`ScenarioClass__Post_Map_Init @ 0x00686890` splits startup: `DAT_00A8B23C == 0` calls `ScenarioClass__Generate_Random_Units @ 0x006886B0`; `DAT_00A8B23C != 0` calls selected mode vtable `+0x84`, then `FUN_005D6D80`. Prior selected-mode evidence verifies selected callbacks do not auto-deploy startup MCVs. The parser/writer path is separate: `0x006B8CA0` and `0x006B8B30` own `[SpecialFlags]` INI bit mapping; `0x006886B0` tests a different low bit.

## 6. Current Rust Implementation Status

Rust parses only `TiberiumGrows`, `TiberiumSpreads`, and `DestroyableBridges` in `src/map/basic.rs::SpecialFlagsSection` and `parse_special_flags_section`; `src/map/map_file.rs` stores the parsed section.

Rust has `mcv_redeploy` in `src/sim/game_options.rs`, `src/skirmish_launch.rs`, and UI state. That maps to `MCVRedeploys`, not `[SpecialFlags] MCVDeploy`.

Rust selected Skirmish startup is `src/app_skirmish.rs::apply_skirmish_launch_session`; it should continue not to auto-deploy selected-mode MCVs from either `0x0100` or `0x10`. If a future null-mode generator is implemented, its low-bit gate needs a separate session-source model.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `MCVDeploy` load mapping | verified | `0x006B8CDB..0x006B8D00` | none |
| `MCVDeploy` save mapping | verified | `0x006B8BAC..0x006B8BBB` | none |
| SpecialFlags reset default | verified | `0x006B8AE0..0x006B8AEC` | none |
| null-mode runtime mask | verified | `0x00688BF2..0x00688C02` | semantic source of low bit beyond sibling docs |
| `0x004FC060` direct call inventory | verified | byte search `E8 59 34 E7 FF` -> `0x00688C02` | indirect calls not exhaustively proven |
| selected-vs-null dispatch | verified | `0x00686890` | none |
| selected-mode no-auto-deploy | verified by prior report | `SKIRMISH_SELECTED_MODE_MCVDEPLOY_START_FLAG_GHIDRA_REPORT.md` | none |
| `CaptureTheFlag` Rules read | verified for default source | `0x006720D7..0x006720FC`; `rulesmd.ini:3035` | full session-staging bit composition |
| Rust SpecialFlags parser | verified | `src/map/basic.rs:39`, `src/map/basic.rs:78` | missing `mcv_deploy` if full parser coverage is desired |
| Rust `mcv_redeploy` separation | verified | `src/sim/game_options.rs:26`, `src/skirmish_launch.rs:122`, `rulesmd.ini:3041` | none |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - Which investigation mode? -> exhaustive-slice for this bit conflict.` (evidence: scope)
- `[RESOLVED] OQ-02 - What bit does SpecialFlags load use for MCVDeploy? -> bit 8 / 0x0100.` (evidence: `0x006B8CDB..0x006B8D00`)
- `[RESOLVED] OQ-03 - What bit does SpecialFlags save use for MCVDeploy? -> bit 8 / 0x0100.` (evidence: `0x006B8BAC..0x006B8BBB`)
- `[RESOLVED] OQ-04 - What is reset default? -> bit 8 cleared, default off.` (evidence: `0x006B8AE0..0x006B8AEC`)
- `[RESOLVED] OQ-05 - What mask gates null-mode Force_MCV_Deploy? -> byte [active_flags] & 0x10.` (evidence: `0x00688BF2..0x00688C02`)
- `[RESOLVED] OQ-06 - Is that the parsed MCVDeploy bit? -> No, parsed MCVDeploy is dword 0x0100.` (evidence: `0x006B8CA0`; `0x00688BF8`)
- `[RESOLVED] OQ-07 - Is null-mode path active in ordinary selected Skirmish? -> No, selected mode object routes through +0x84 and FUN_005D6D80.` (evidence: `0x00686890`)
- `[RESOLVED] OQ-08 - How many direct callers reach 0x004FC060? -> one direct callsite, 0x00688C02.` (evidence: byte search `E8 59 34 E7 FF`)
- `[RESOLVED] OQ-09 - What does 0x004FC060 do? -> limbo/null guard, clear previous primary, queue deploy target via 0x00740DF0, store House+0x53DC.` (evidence: `0x004FC060`)
- `[RESOLVED] OQ-10 - Does 0x00740DF0 directly deploy? -> No, it sets Unit[0x1B3] and queues mission 2.` (evidence: `0x00740DF0`)
- `[RESOLVED] OQ-11 - Does Rust parse [SpecialFlags] MCVDeploy? -> No.` (evidence: `src/map/basic.rs:39`, `src/map/basic.rs:78`)
- `[RESOLVED] OQ-12 - Is Rust mcv_redeploy the same flag? -> No.` (evidence: `rulesmd.ini:3041`, `src/sim/game_options.rs:26`)
- `[RESOLVED] OQ-13 - Is stock CaptureTheFlag default on? -> No, CaptureTheFlag=no.` (evidence: `rulesmd.ini:3035`)
- `[DEFERRED] OQ-14 - Which exact startup/session writer puts RulesClass+0x14B2 into active low bit 0x10?` (category: bounded-cost-too-high; reason: parser/source proof and runtime mask conflict are complete without full staging audit; next-step-if-pursued: targeted `DAT_00A8E960` composer audit)
- `[DEFERRED] OQ-15 - Are there indirect calls to 0x004FC060?` (category: bounded-cost-too-high; reason: direct-call inventory and selected callback checks are enough for this handoff; next-step-if-pursued: whole-program pointer/xref sweep)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `[SpecialFlags] MCVDeploy` parses/saves as bit 8 / `0x0100`, default off. | `0x006B8CA0`, `0x006B8B30`, `0x006B8AE0` | missing parser field | `src/map/basic.rs`, `src/map/map_file.rs` | If full SpecialFlags coverage is added, parse/write `mcv_deploy` as key `MCVDeploy`, but do not use it to selected-mode auto-deploy. | Map `[SpecialFlags] MCVDeploy=yes` yields parsed `mcv_deploy == Some(true)` while default/reset remains false. | `parse_specialflags_mcvdeploy_uses_bit8_key_not_low_0x10`; risk: encoding it as bit `0x10`. |
| Null-mode `Generate_Random_Units` gates its only direct `Force_MCV_Deploy` call on byte bit `0x10`, not parsed bit `0x0100`. | `0x00688BF2..0x00688C02`; byte search | null-mode generator not modeled | future null-mode startup generator | Keep this as a separate low-flag/session-source issue if null-mode generation is implemented. | Null-mode fixture with only bit `0x0100` set does not take helper gate unless low `0x10` source is set. | `null_mode_mcvdeploy_helper_uses_low_0x10_gate_not_specialflags_bit8`; risk: driving helper from parsed MCVDeploy alone. |
| Ordinary selected Skirmish remains no-auto-deploy. | `0x00686890`; selected-mode report | Rust currently matches | `src/app_skirmish.rs::apply_skirmish_launch_session`, `src/skirmish_launch.rs` | Preserve no-auto-deploy for selected Battle/TeamGame unless a separate selected callback contradicts it. | Selected Battle with `[SpecialFlags] MCVDeploy=yes` still starts with undeployed MCV. | `skirmish_selected_battle_mcvdeploy_flag_does_not_auto_deploy_starting_mcv`; risk: copying null-mode evidence into selected launch. |
| `mcv_redeploy` is `MCVRedeploys`, not `[SpecialFlags] MCVDeploy`. | `rulesmd.ini:3041`; Rust fields | none for separation | `src/sim/game_options.rs`, `src/skirmish_launch.rs`, UI state | Keep names and behavior separate. | Turning off `mcv_redeploy` does not change parsed map `MCVDeploy`. | `mcv_redeploy_lobby_option_is_separate_from_specialflags_mcvdeploy`; risk: reusing the lobby option. |

### Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/MCVDEPLOY_START_FLAG_AUTO_DEPLOY_GHIDRA_REPORT.md`: replace the top Active-in-YR claim with `Active in YR: Conditional for the null-mode ScenarioClass__Generate_Random_Units path. The only direct Force_MCV_Deploy callsite tests low byte bit 0x10, not the parsed [SpecialFlags] MCVDeploy bit 0x0100. Ordinary selected Skirmish callbacks do not auto-deploy startup MCVs.`
- `C:/Users/enok/Documents/ra2-rust-game-docs/MCVDEPLOY_START_FLAG_AUTO_DEPLOY_GHIDRA_REPORT.md`: replace `Startup MCVDeploy is active bit 8 / 0x0100 in the final SpecialFlags word` with `Parser/writer MCVDeploy is bit 8 / 0x0100. The null-mode startup helper gate at 0x00688BF8 tests low byte bit 0x10, so previous auto-deploy handoffs must not be driven from parsed MCVDeploy until the low-flag/session source is deliberately modeled.`
- `C:/Users/enok/Documents/ra2-rust-game-docs/MCV_CREATION_STARTING_UNITS_DEEP_DIVE.md`, section 7: replace `| 4 | 0x10 | MCVDeploy | Yes -- forces MCV auto-deploy at game start |` with `| 4 | 0x10 | low session flag, sibling docs identify CaptureTheFlag | Conditional -- null-mode Generate_Random_Units tests this bit before Force_MCV_Deploy; this is not the [SpecialFlags] MCVDeploy parser bit |`.
- `C:/Users/enok/Documents/ra2-rust-game-docs/MCV_CREATION_STARTING_UNITS_DEEP_DIVE.md`, section 7: replace the layout-discrepancy note with `There is no endian/layout alias here: [SpecialFlags] MCVDeploy save/load uses bit 8 / 0x0100, while the null-mode Generate_Random_Units helper gate tests byte [flags] & 0x10. Treat these as separate flags unless a future session-staging audit proves an intentional transfer.`
- `C:/Users/enok/Documents/ra2-rust-game-docs/MCV_DEPLOY_GHIDRA_REPORT.md` and `GAME_START_INITIALIZATION.md`: replace any statement that `bit 8 MCVDeploy forces immediate starting MCV deploy` with `bit 8 is the [SpecialFlags] MCVDeploy parser/writer bit. The only verified startup Force_MCV_Deploy callsite in Generate_Random_Units is gated by low bit 0x10; selected Skirmish does not call it.`

## Negative Facts / Do Not Do

- Do not implement `[SpecialFlags] MCVDeploy` as bit `0x10`; load/save prove bit `0x0100` (`0x006B8CA0`, `0x006B8B30`).
- Do not claim null-mode `Generate_Random_Units` tests parsed `MCVDeploy`; it tests byte `[active_flags] & 0x10` (`0x00688BF8`).
- Do not wire selected Skirmish startup auto-deploy from either bit; selected mode bypasses `0x006886B0` when `DAT_00A8B23C != 0` (`0x00686890`).
- Do not reuse Rust `mcv_redeploy` for this; it maps to `MCVRedeploys`, not `[SpecialFlags] MCVDeploy` (`rulesmd.ini:3041`, `src/sim/game_options.rs:26`).
- Do not direct-spawn a Construction Yard for the helper path; `0x004FC060` calls `0x00740DF0`, which queues mission `2`.

## Remaining Uncertainty

- Exact session-staging writer from `RulesClass+0x14B2 CaptureTheFlag` or related game options into active low bit `0x10` was not fully re-audited.
- Indirect calls to `0x004FC060` were not exhaustively disproven.
- Runtime UX for external maps setting `[SpecialFlags] MCVDeploy=yes` while low bit `0x10` remains off needs a native runtime fixture if empirical confirmation is desired.

## Sources

- Ghidra decompiled/read-only: `0x006B8CA0`, `0x006B8B30`, `0x006B8AE0`, `0x006886B0`, `0x004FC060`, `0x00740DF0`, `0x00686890`.
- Ghidra assembly/read-only: `0x00688BF2..0x00688C02`, `0x006B8BAC..0x006B8BBB`, `0x006B8CDB..0x006B8D00`, `0x006720D7..0x006720FC`.
- Ghidra byte search: direct call pattern `E8 59 34 E7 FF` found only at `0x00688C02`.
- Prior docs referenced: `C:/Users/enok/Documents/ra2-rust-game-docs/SPECIAL_FLAGS_SYSTEM.md`, `C:/Users/enok/Documents/ra2-rust-game-docs/MCVDEPLOY_START_FLAG_AUTO_DEPLOY_GHIDRA_REPORT.md`, `C:/Users/enok/Documents/ra2-rust-game-docs/MCV_CREATION_STARTING_UNITS_DEEP_DIVE.md`, `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_SELECTED_MODE_MCVDEPLOY_START_FLAG_GHIDRA_REPORT.md`.
- INI checked: `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini`, `C:/Users/enok/Documents/ra2-rust-game/ini/rules.ini`.
- Rust scanned: `C:/Users/enok/Documents/ra2-rust-game/src/map/basic.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/map/map_file.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/app_skirmish.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/skirmish_launch.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/sim/game_options.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/world_commands.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/world_spawn.rs`.
