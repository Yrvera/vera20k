# Manual Partial-Cargo Chrono Miner Return Order And Voice - Ghidra Research Report

**Address(es):** `0x0073FD50`, `0x004D74E0`, `0x006FFBE0`, `0x004C6CB0`, `0x004DF0E0`, `0x0043C2D0`, `0x00709020`, `0x00708FC0`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** stock YR `CMIN` with partial cargo ordered by object-click to an owned stock refinery (`GAREFN`/`NAREFN`): action/mission code, order voice, target binding, eligibility gates, and full-cargo requirement.
**Non-Scope:** full sidebar/input system, full refinery queue/dock/unload/exit behavior, non-stock refinery mods, and runtime debugger confirmation of keyboard-only hotkeys.
**Confidence:** High for object-click path; Medium for keyboard-only/generic return because no stock keyboard return entry was proven in this slice.
**Active in YR:** Yes for object-click on owned stock refinery; Conditional for keyboard/generic forced return (not proven as a stock gamemd hotkey in this slice).

## Target Question

When a player manually orders a stock `CMIN` with partial cargo to an owned refinery, does gamemd require full cargo, which action/mission is issued, which voice plays immediately, and is the clicked refinery preserved as the order target?

## Non-Goals

- Do not re-open the whole sidebar/input system.
- Do not re-investigate stock refinery unload, zero-link exit, `ReleaseDockedHarvester`, or `Force_Track(0x47)`.
- Do not replace settled dock-anchor findings; this report only needs the order entry into the dock/enter path.

## Evidence Needed To Mark COMPLETE

- Action code from hover/click for stock `CMIN` -> owned stock refinery.
- Mission argument emitted by `FootClass::ClickedAction_Object`.
- Immediate order voice slot selected by `TechnoClass::Player_Send_Command`.
- Proof that the clicked refinery object is encoded into the command event and restored as the mission target.
- Proof that storage fullness is not checked on the order acceptance path.

## Stop Conditions

- Stop after the object-click forced-return path reaches mission assignment and target binding.
- Stop if the next question requires full tactical hotkey enumeration or runtime debugger-only keyboard tracing.
- Stop before dock admission/unload/exit once `Mission Enter` / target binding is established.

## 1. Overview

For the standard object-click path, stock YR does not require a full Chrono Miner before accepting a refinery return order. The live path is: `UnitClass::What_Action_OnObject` returns action `3` for an owned available refinery after `BuildingClass::Receive_Radio(0x0F)` returns ROGER, `FootClass::ClickedAction_Object` sends mission `7` with the clicked refinery object, and `TechnoClass::Player_Send_Command` plays `VoiceEnter` (`ChronoMinerReturn` for `CMIN`).

The older trace's "action `0x1A`" wording is stale for this stock available-refinery case. Action `0x1A` exists, but if that fallback path is taken it sends mission `0x0B` and does not use the `VoiceEnter` branch.

## 2. Key Offsets / Values

| Field / value | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| action `3` | Owned-refinery enter/dock action after `0x0F` ROGER | `UnitClass__What_Action_OnObject @ 0x0073FD50`; `ClickedAction_Object @ 0x004D74E0` | Yes |
| mission `7` | Enter/dock mission sent for action `3` | `ClickedAction_Object @ 0x004D74E0`, case `3` calls vtable `+0x378(7, 0, target, 0)` | Yes |
| action `0x1A` | Alternate dock/enter action, not the stock available-refinery result proven here | `ClickedAction_Object @ 0x004D74E0`, case `0x1A` calls vtable `+0x378(0x0B, target, 0, 0)` | Conditional |
| `TechnoType+0x558` | `VoiceEnter` single sound index | `FUN_00709020 @ 0x00709020`; `TechnoTypeClass` layout doc | Yes |
| `TechnoType+0x468..0x480` | `VoiceMove` vector | `FUN_00708FC0 @ 0x00708FC0`; `TechnoTypeClass` layout doc | Yes |
| `[CMIN] VoiceEnter` | `ChronoMinerReturn` | `ini/rulesmd.ini:[CMIN]` | Yes |
| `[CMIN] Storage` | `20`; not read by order acceptance path | `ini/rulesmd.ini:[CMIN]`; no storage read in scoped functions | Yes |

## 3. Core Logic

### 3.1 Action Selection

`UnitClass__What_Action_OnObject @ 0x0073FD50` starts from the inherited object action, then for a human player clicking an allied building it probes the target with radio message `0x0F`:

- If the target building returns `1` (ROGER), the action is changed to `3`.
- If the target building returns `10`, a different repair/guard-active action can be returned.
- No cargo/fullness field is read in this decision block.

`BuildingClass__Receive_Radio @ 0x0043C2D0`, case `0x0F`, returns ROGER for a stock DockUnload refinery when:

- target is allied,
- target is not in construction/unload missions,
- auxiliary slot flag is present,
- sender passes the harvester check,
- naval/enter-building/power gates pass,
- target has `Type+0x16B3` (`DockUnload`) and sender is a unit with `UnitType+0xE0E` (`Harvester`),
- map editor is on or `building+0x118 == 0`.

No storage fullness or nonzero cargo check appears in this `0x0F` refinery branch.

### 3.2 Click Dispatch And Mission

`FootClass__ClickedAction_Object @ 0x004D74E0`, case `3`, verifies the selected foot object can accept orders, then calls the player command builder with mission `7` and the clicked object still supplied as a target argument.

The action `0x1A` case is real but separate. It calls the player command builder with mission `0x0B`. This matters because mission `0x0B` takes the default voice path in `0x006FFBE0`, not the `VoiceEnter` branch.

### 3.3 Voice Selection

`TechnoClass::Player_Send_Command @ 0x006FFBE0` chooses voice from the mission/action argument:

- `param_2 == 7` -> vtable `+0x358`, decompiled at `0x00709020`, which reads `TechnoType+0x558` (`VoiceEnter`) and falls back to `VoiceMove` only if `VoiceEnter == -1`.
- `param_2 == 2` or `0x1D` -> vtable `+0x368`, decompiled at `0x00708FC0`, which uses `VoiceMove`.
- default/other -> random `VoiceSpecialAttack` vector at `TechnoType+0x4A0..0x4B8` when present.

For stock `[CMIN]`, `VoiceEnter=ChronoMinerReturn` and `VoiceMove=ChronoMinerMove`. Therefore the object-click refinery return acknowledgement is `ChronoMinerReturn`, not `ChronoMinerMove`.

### 3.4 Event Execution And Target Binding

`TechnoClass::Player_Send_Command @ 0x006FFBE0` encodes the selected unit and target object through `FUN_006E6AB0` into event type `4` via `FUN_004C6860`/`FUN_00646E90`.

`EventClass__Execute @ 0x004C6CB0`, case `4/5`, restores:

- selected unit via `FUN_006E6F20`,
- target object/cell via `FUN_006E6E20` / `FUN_006E6FF0`,
- mission through `FootClass__Assign_Target_Command @ 0x004DF0E0`,
- destination/target through vtable `+0x3C8` and `+0x480`.

For mission `7`, this preserves the clicked refinery as the current target/destination. It does not collapse the order into "nearest refinery" at command time.

## 4. INI Keys

| INI | Value | Use in this slice | Active in YR |
|---|---|---|---|
| `rulesmd.ini:[CMIN] Dock=NAREFN,GAREFN` | stock dock list | Makes stock refineries valid dock targets downstream | Yes |
| `rulesmd.ini:[CMIN] Harvester=yes` | true | Required by refinery `0x0F` DockUnload branch (`UnitType+0xE0E`) | Yes |
| `rulesmd.ini:[CMIN] Storage=20` | 20 | Cargo capacity; not checked by order acceptance | Yes |
| `rulesmd.ini:[CMIN] VoiceMove=ChronoMinerMove` | move voice | Used by move orders and fallback if `VoiceEnter` absent | Yes |
| `rulesmd.ini:[CMIN] VoiceEnter=ChronoMinerReturn` | return/enter voice | Immediate owned-refinery order voice for mission `7` | Yes |
| `rulesmd.ini:[GAREFN]/[NAREFN] DockUnload=yes` | true | Enables stock refinery enter/dock radio branch | Yes |
| `rulesmd.ini:[GAREFN]/[NAREFN] Refinery=yes` | true | Stock refinery role; later dock chain | Yes |
| `rulesmd.ini:[GAREFN]/[NAREFN] NumberOfDocks=1` | 1 | Contact capacity; not a full-cargo gate | Yes |

## 5. Integration Points

Object-click path:

1. Determine object action through `UnitClass__What_Action_OnObject @ 0x0073FD50`.
2. Probe refinery eligibility through `BuildingClass__Receive_Radio(0x0F) @ 0x0043C2D0`.
3. Dispatch click through `FootClass__ClickedAction_Object @ 0x004D74E0`.
4. Build network/player command through `TechnoClass::Player_Send_Command @ 0x006FFBE0`.
5. Execute event through `EventClass__Execute @ 0x004C6CB0`.
6. Assign mission/target through `FootClass__Assign_Target_Command @ 0x004DF0E0`, then mission dispatch later reaches `FootClass__Mission_Enter @ 0x004D9290`.

Keyboard-only/generic return:

- No stock gamemd keyboard-only "return this miner to refinery" entry was proven in this bounded slice.
- The current Rust comment that `MinerReturn` is used by the `'D' key` is suspect because local Rust `KeyD` queues deploy/undeploy, and the binary evidence here only proves the object-click path.

## 6. Current Rust Implementation Status

Current Rust already has an explicit refinery target in `Command::MinerReturn` and seeds it into `miner.reserved_refinery`:

- `src/app_context_order.rs`: object click on friendly refinery queues `Command::MinerReturn { target_refinery_id: clicked_friendly_refinery_id }`.
- `src/sim/command.rs`: `MinerReturn` contains `target_refinery_id: Option<u64>`.
- `src/sim/world/world_commands.rs`: validates explicit refinery, stores it in `miner.reserved_refinery`, and sets `MinerState::ForcedReturn`.

Current Rust still appears voice-mismatched:

- `src/app_context_order.rs` emits `VoiceMove` for all non-attack orders after queuing commands.
- `src/app_input.rs::emit_order_voice` only handles `VoiceMove` and `VoiceAttack`; it does not parse/play `VoiceEnter`.
- `src/rules/object_type.rs` does not currently expose `VoiceEnter`.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Stock object action for `CMIN` -> owned refinery | verified | `0x0073FD50`, `0x0043C2D0` | none for available stock refinery |
| Click dispatch action `3` | verified | `0x004D74E0` | none |
| Alternate action `0x1A` | verified as non-primary/fallback | `0x004D74E0`, `0x006FFBE0` | exact UI situations that reach it are outside this slice |
| Immediate voice for mission `7` | verified | `0x006FFBE0`, `0x00709020`, `rulesmd.ini:[CMIN]` | none |
| Target object preservation | verified | `0x006FFBE0`, `0x006E6AB0`, `0x004C6CB0`, `0x004DF0E0` | none for object-click event shape |
| Full-cargo requirement | verified negative | no storage read in `0x0073FD50`, `0x0043C2D0` case `0x0F`, `0x004D74E0`, `0x006FFBE0`, `0x004C6CB0` | zero-cargo UX not separately runtime-tested |
| Keyboard-only forced return | deferred | Rust scan + bounded binary scope | hotkey system follow-up if needed |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-1 - What action does stock CMIN -> owned available refinery use? -> action 3 after refinery CAN_ENTER(0x0F) returns ROGER.` (evidence: `0x0073FD50`, `0x0043C2D0`)
- `[RESOLVED] OQ-2 - What mission is sent by the click handler? -> mission 7 for action 3.` (evidence: `0x004D74E0`)
- `[RESOLVED] OQ-3 - Which immediate voice slot plays? -> mission 7 calls vtable +0x358, which reads VoiceEnter at TechnoType+0x558; stock CMIN uses ChronoMinerReturn.` (evidence: `0x006FFBE0`, `0x00709020`, `rulesmd.ini:[CMIN]`)
- `[RESOLVED] OQ-4 - Is VoiceMove used for this object-click order? -> no, not on the verified action 3/mission 7 path; VoiceMove is used by mission 2/0x1D or VoiceEnter fallback if missing.` (evidence: `0x006FFBE0`, `0x00708FC0`, `0x00709020`)
- `[RESOLVED] OQ-5 - Is the clicked refinery preserved? -> yes, it is encoded as an object reference and restored by event execution before target/destination assignment.` (evidence: `0x006FFBE0`, `0x006E6AB0`, `0x004C6CB0`, `0x004DF0E0`)
- `[RESOLVED] OQ-6 - Is full cargo required? -> no full-storage or nonzero-storage check appears in the scoped order/action/voice path.` (evidence: `0x0073FD50`, `0x0043C2D0`, `0x004D74E0`, `0x006FFBE0`, `0x004C6CB0`)
- `[RESOLVED] OQ-7 - Is action 0x1A the stock available-refinery path? -> no for the verified available-refinery case; 0x1A is a separate fallback/alternate path that sends mission 0x0B.` (evidence: `0x004D74E0`, `0x006FFBE0`)
- `[DEFERRED] OQ-8 - Does stock gamemd have a keyboard-only generic miner return hotkey?` (category: `out-of-scope`; reason: would require broader hotkey/sidebar enumeration, explicitly excluded; next-step-if-pursued: run a small hotkey-only investigation from `HOTKEY_SYSTEM_GHIDRA_REPORT.md` entries)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Stock object-click refinery order plays `VoiceEnter`, not `VoiceMove`, for CMIN. | `0x004D74E0` action 3 -> mission 7; `0x006FFBE0`; `0x00709020`; `rulesmd.ini:[CMIN] VoiceEnter=ChronoMinerReturn` | mismatch: Rust emits `VoiceMove` for non-attack orders and lacks `VoiceEnter` parsing/playback | `src/app_context_order.rs`, `src/app_input.rs`, `src/rules/object_type.rs`, audio event types if needed | Friendly-refinery miner return should emit `VoiceEnter` with fallback to `VoiceMove` only if `VoiceEnter` is absent. | `test_partial_cargo_cmin_refinery_order_uses_return_voice_and_enters_return` | Do not special-case CMIN by name; use `VoiceEnter` data from rules. |
| Clicked refinery remains the target; command execution does not choose nearest refinery at click time. | `0x006FFBE0`, `0x006E6AB0`, `0x004C6CB0`, `0x004DF0E0` | likely fixed in current uncommitted Rust: `target_refinery_id` is stored in `reserved_refinery` | `src/app_context_order.rs`, `src/sim/command.rs`, `src/sim/world/world_commands.rs`, `src/sim/miner/miner_system.rs` | Preserve explicit refinery until invalid/rejected; only generic/keyboard orders may choose nearest. | `test_partial_cargo_cmin_refinery_order_uses_clicked_refinery_not_nearest` | Do not collapse object-click into generic `ForcedReturn` with no target. |
| Full cargo is not required for manual object-click refinery order. | no storage read in `0x0073FD50`, `0x0043C2D0` case `0x0F`, `0x004D74E0`, `0x006FFBE0`, `0x004C6CB0` | current Rust appears compatible if `MinerReturn` has no full-cargo gate | `src/sim/world/world_commands.rs`, miner return tests | Partial cargo must enter forced return/mission-enter flow and keep the clicked target. | `test_partial_cargo_cmin_refinery_order_uses_return_voice_and_enters_return` | Do not gate manual return on `storage == capacity`; automatic return and manual return are different entry conditions. |

## Negative Facts / Do Not Do

- Do not use action `0x1A` as the handoff-critical stock available-refinery object-click result; action `3` is the verified path here.
- Do not play `VoiceMove=ChronoMinerMove` for a successful object-click refinery return when `VoiceEnter` exists.
- Do not require full cargo for manual refinery return acceptance.
- Do not use the clicked refinery only as a hint for nearest-refinery search; it is an explicit target object.
- Do not infer keyboard/generic return behavior from the object-click proof without a separate hotkey investigation.

## Remaining Uncertainty

- Exact keyboard-only/generic forced-return behavior in stock gamemd was not proven. The current Rust comment mentioning `'D' key` should not be treated as binary evidence.
- The exact busy-refinery cursor/action branch when `0x0F` returns non-ROGER was touched but not exhausted; this report only claims the owned available stock refinery case.
- Runtime audio sample selection within `ChronoMinerReturn` was not debugger-tested; binary evidence proves the `VoiceEnter` slot, not which random sample index is picked.

## Stale Docs / Follow-Up Docs

- `miner/traces/MINER_MANUAL_ORDER_PARTIAL_CARGO_TO_REFINERY_TRACE.md`: replace "action `0x1A` calls vtable `+0x378` with the original target" for the stock available-refinery path with: "stock owned available refinery resolves to action `3`; `ClickedAction_Object` sends mission `7` with the clicked refinery object, causing `VoiceEnter` and target-preserving Mission Enter. Action `0x1A` is a separate fallback/alternate dock path that sends mission `0x0B`."
- Older sound docs that list `VoiceEnter` as "entering refinery to dump" are directionally correct for this slice, but should mention the concrete command path: action `3` -> mission `7` -> `Player_Send_Command` vtable `+0x358`.

## Sources

- Ghidra read-only decompile: `UnitClass__What_Action_OnObject @ 0x0073FD50`.
- Ghidra read-only decompile: `FootClass__ClickedAction_Object @ 0x004D74E0`.
- Ghidra read-only decompile: `TechnoClass::Player_Send_Command @ 0x006FFBE0`.
- Ghidra read-only decompile: `EventClass__Execute @ 0x004C6CB0`.
- Ghidra read-only decompile: `FootClass__Assign_Target_Command @ 0x004DF0E0`.
- Ghidra read-only decompile: `BuildingClass__Receive_Radio @ 0x0043C2D0`.
- Ghidra read-only decompile: `FUN_00709020`, `FUN_00708FC0`, `FUN_00709060`, `FUN_00708DC0`, `FUN_00708E00`, `FUN_007090A0`.
- `ini/rulesmd.ini` sections `[CMIN]`, `[GAREFN]`, `[NAREFN]`.
- Starting trace: `docs/research/miner/traces/MINER_MANUAL_ORDER_PARTIAL_CARGO_TO_REFINERY_TRACE.md`.
