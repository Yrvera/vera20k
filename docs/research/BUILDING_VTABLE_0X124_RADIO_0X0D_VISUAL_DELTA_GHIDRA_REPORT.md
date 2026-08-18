# Building vtable+0x124 / radio 0x0D visual delta - Ghidra Research Report

**Address(es):** `0x005F5320`, `0x0043C2D0`, `0x0043F180`, `0x006F4A70`
**Investigation Mode:** exhaustive-slice, downgraded to static-evidence slice because this session had no live Ghidra MCP exposed
**Claimed Scope:** BuildingClass/ObjectClass receiver behavior for radio `0x0D`, the concrete BuildingClass vtable `+0x124` target, the `mode=2` static visual/animation effect, and the stock war-factory swallow implication for Rust.
**Non-Scope:** full `BuildingClass::SetMissionAndAnims @ 0x0043F180`, runtime screenshot capture, exact rendered frame delta, non-building `+0x124` targets, and any Rust implementation.
**Confidence:** High for receiver binding and war-factory swallow; Medium for exact visual effect because static evidence proves mark/attached-animation refresh is skipped, but no screenshot/runtime frame diff was captured.
**Active in YR:** Yes for ObjectClass receiver and stock war-factory swallow; Conditional for radio `0x0D` sending because sender requires `TechnoClass+0x418 != 0` and a successful mark/update.

## 0. Investigation Contract

**Target question:** When radio `0x0D` reaches `ObjectClass::Receive_Radio`, what BuildingClass/ObjectClass vtable `+0x124` target is invoked with `mode=2`, what static visual/animation state would it affect, and what should Rust do for `WeaponsFactory=yes` war factories that swallow this message?

**Non-goals:** Do not re-audit every radio message, do not decode the entire 2400-byte BuildingClass `+0x124` routine, do not implement Rust, do not edit in-repo docs, and do not require runtime screenshot capture.

**Evidence needed to mark COMPLETE:**

- Receiver decompile plus assembly range for ObjectClass `0x0D -> vtable+0x124(2)`.
- BuildingClass vtable binding evidence for `+0x124 -> 0x0043F180`.
- BuildingClass receive-radio decompile evidence that `WeaponsFactory=yes` returns `ROGER=1` before ObjectClass.
- INI/default evidence that stock land war factories set `WeaponsFactory=yes`.
- Rust surface scan showing where a future generic radio/animation effect would need to be suppressed.

**Stop conditions:** Stop once the static receiver/swallow effect is pinned down. Downgrade any screenshot-level visual claim to uncertainty if it requires runtime capture.

## 1. Overview

Radio `0x0D` is a contacted-object mark/animation refresh notification. `ObjectClass::Receive_Radio @ 0x005F5320` handles it by calling the receiver's vtable `+0x124` with argument `2` and returning `ROGER=1`.

For BuildingClass receivers, vtable `+0x124` resolves to `0x0043F180`, named in the current vtable report as `BuildingClass::SetMissionAndAnims`. Prior Ghidra-backed work records that `mode=2` refreshes building mark/attached-animation state, including propagation across the 21-slot building anim pointer block. `BuildingClass::Receive_Radio @ 0x0043C2D0` intercepts `0x0D` first: if the receiver's `BuildingType+0x16BD WeaponsFactory` flag is set, it returns `1` and does not call the generic ObjectClass path.

The player-facing interpretation is negative and implementation-critical: stock war factories intentionally do not use `0x0D` to reset, restart, or otherwise touch their building animation overlay state during produced-unit radio contact churn. Rust should preserve that no-op for `WeaponsFactory=yes` receivers.

## 2. Verified Binary Findings

| Finding | Evidence | Confidence | Active in YR |
|---|---|---:|---|
| `ObjectClass::Receive_Radio(0x0D)` calls `this->vtable+0x124(2)` and returns `1`; it does not inspect sender or payload. | Prior Ghidra decompile/disassembly in `RADIO_MSG_0X0D_SENDERS_ANIM_RESET_GHIDRA_REPORT.md`: `0x005F5320`, assembly `0x005F5370..0x005F537A` (`PUSH 2`, `CALL [EDX+0x124]`, `MOV EAX,1`). | High | Yes, when the message reaches ObjectClass |
| BuildingClass vtable `+0x124` binds to `0x0043F180`. | `BUILDINGCLASS_VTABLE_COMPLETE.md` slot 73: `0x124 -> 0x0043F180`; prior Ghidra vtable data xref `0x007E3FE0 -> 0x0043F180`. | High | Yes |
| `0x0043F180` is a BuildingClass mission/animation state routine; in this radio path it is invoked as mode `2`. | Vtable report names `BuildingClass::SetMissionAndAnims`; `RADIO_MSG_0X0D...` records ObjectClass arg `2` and xrefs into `0x0043F180`. | High for binding/arg; Medium for complete effect | Yes |
| Static effect of non-swallowed BuildingClass `0x0D`: refresh mark/attached-animation state, including the building's 21-slot `Anims_0` block. | `RADIO_MSG_0X0D...` section 3.3 records `0x0043F180` attached animation coordinate/state propagation; helper docs identify `Anims_0` base `+0x55C`, slot writers `0x00451890`, slot image helper `0x00451750`, and clearer `0x00451E40`. | Medium | Yes for buildings that do not swallow and receive `0x0D` |
| `BuildingClass::Receive_Radio(0x0D)` swallows the message for `WeaponsFactory=yes`: it returns `1` before the ObjectClass fallback. | Prior Ghidra decompile of `0x0043C2D0`; `BUILDINGCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md` case `0x0D`; `RADIO_MSG_0X0D...` section 3.4. | High | Yes for stock war factories |

## 3. Static Visual / Animation Delta

The static delta is:

1. Non-weapon-factory BuildingClass receiver, if `0x0D` reaches ObjectClass: `ObjectClass::Receive_Radio` invokes `BuildingClass::SetMissionAndAnims(2)`. Prior Ghidra-backed evidence says this refreshes mark and attached animation state for the building.
2. `WeaponsFactory=yes` BuildingClass receiver: `BuildingClass::Receive_Radio` returns `ROGER=1` immediately. The `ObjectClass` `+0x124(2)` call does not happen.
3. Therefore, the war-factory player-visible effect of `0x0D` itself is no animation event. It is a deliberate suppressor of the generic building mark/attached-animation refresh during produced-unit contact churn.

Stock war-factory art supports the "do not trigger production animation from `0x0D`" handoff:

- `GAWEAP` has `ActiveAnim=GAWEAP_A` and `ActiveAnimTwo=GAWEAP_B`, but no active `ProductionAnim` key; only `ProductionAnimX/Y/YSort` comments appear.
- `NAWEAP` has `ActiveAnim=NAWEAP_A`; `ProductionAnim=NAWEAP_A` is commented out.
- `YAWEAP` has `ActiveAnim=YAWEAP_A`; its `ActiveAnimTwo` and production-related lines are commented.

This does not prove that skipping `+0x124(2)` has zero pixels in every modded war factory. It does prove the stock `0x0D` path must not be the mechanism that starts, resets, or advances the visible war-factory production/door/roof animation.

## 4. INI Keys

| Key | Stock YR value | Effect in this slice | Evidence | Active in YR |
|---|---|---|---|---|
| `WeaponsFactory=` | `yes` on `GAWEAP`, `NAWEAP`, `YAWEAP` | Gates `BuildingClass::Receive_Radio(0x0D)` swallow | `rulesmd.ini:11775`, `12565`, `13309`; binary reads `BuildingType+0x16BD` | Yes |
| `Factory=` | `UnitType` on `GAWEAP`, `NAWEAP`, `YAWEAP` | Production category, not the radio `0x0D` swallow gate | `rulesmd.ini:11777`, `12567`, `13311` | Yes |
| `ActiveAnim*` | present on stock war factories | Normal building overlay art; can be affected by generic building animation refresh paths, but `0x0D` is swallowed for WFs | `artmd.ini:[GAWEAP]/[NAWEAP]/[YAWEAP]` | Yes |
| `ProductionAnim` | absent/commented for stock land war factories | Should not be started/reset by radio `0x0D`; stock WFs rely on other door/roof/active anim paths | `artmd.ini:1238..1240`, `1432..1436`, `1281..1286` context | No active stock WF key |

## 5. Integration Points

| Function / surface | Role | Evidence | Active in YR |
|---|---|---|---|
| `TechnoClass__ProcessCloakAndNotify @ 0x006F4A70` | Verified immediate sender: after successful mark/update and `+0x418 != 0`, calls `Transmit_Radio_ToFirst(0x0D)` | decompile plus assembly `0x006F4A81..0x006F4A91` in prior report | Conditional |
| `ObjectClass__Receive_Radio @ 0x005F5320` | Terminal receiver maps `0x0D` to `vtable+0x124(2)` | decompile plus assembly `0x005F5370..0x005F537A` in prior report | Yes |
| `BuildingClass__Receive_Radio @ 0x0043C2D0` | Intercepts `0x0D`; returns `1` for `WeaponsFactory=yes` | prior full-switch decompile | Yes |
| `BuildingClass::SetMissionAndAnims @ 0x0043F180` | BuildingClass vtable `+0x124` concrete target | vtable report; prior vtable xref | Yes |
| `BuildingClass::CreateAnimForSlot @ 0x00451890` / `SetAnimSlotImage @ 0x00451750` / `ClearAnimSlot @ 0x00451E40` | 21-slot attached animation helper family that establishes the likely visual state affected by generic refresh | `BUILDINGCLASS_UPDATE_ANIMATION_GHIDRA_REPORT.md`; `miner/REFINERY_DOCK_CELL_AND_ANIM_HELPERS_GHIDRA_REPORT.md` | Yes |

## 6. Current Rust Implementation Status

Rust has relevant pieces, but no generic `ObjectClass::Receive_Radio(0x0D)` abstraction:

- `src/sim/game_entity.rs` has `radio_contacts` and live-contact helpers.
- `src/sim/production/production_spawn.rs::mark_war_factory_spawn_contact` marks the produced vehicle as contacted to the factory after stock land war-factory spawn.
- `src/app_building_anim.rs` owns `BuildingAnimOverlays`, including one-shot `Active` / `Production` overlay triggering for building producers and refinery special anim events.
- `src/app_instances/shp.rs` renders `Active` and `Production` overlays from `building_anim_overlays`.

Current Rust delta for this slice: if a future generic radio receiver maps `0x0D` to a building animation refresh hook, it must explicitly skip `WeaponsFactory=yes` receivers. The current production contact model should remain sim-side/passability-focused and should not call into building overlay animation just because a produced unit has contact churn.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Target question / non-goals / stop conditions | verified | this report section 0 | none |
| Live Ghidra access in this slot | deferred | no Ghidra MCP tools/resources exposed in this session | fresh decompile could only improve confidence, not change recorded prior evidence |
| ObjectClass `0x0D` receiver | verified | prior decompile + assembly `0x005F5370..0x005F537A` | none |
| BuildingClass `+0x124` binding | verified | `BUILDINGCLASS_VTABLE_COMPLETE.md`; prior vtable xref `0x007E3FE0 -> 0x0043F180` | none |
| Full internals of `0x0043F180` | touched-not-exhausted | prior report says attached-animation refresh; vtable report names SetMissionAndAnims | exact branch-by-branch mode `2` write set |
| BuildingClass `0x0D` WeaponsFactory branch | verified | prior decompile `0x0043C2D0` | none |
| Stock war-factory INI gate | verified | `rulesmd.ini` stock `GAWEAP`/`NAWEAP`/`YAWEAP` | none |
| Stock war-factory art keys | verified | `artmd.ini` stock sections | none for static key presence |
| Screenshot-level visual delta | deferred | static evidence only | runtime capture/debugger trace |
| Rust affected surfaces | touched-not-exhausted | `rg` scan of `radio_contacts`, `mark_war_factory_spawn_contact`, `building_anim_overlays` | no code changes made |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-001 - What is the ObjectClass receiver action for radio 0x0D? -> It calls receiver vtable +0x124 with arg 2 and returns 1.` (evidence: prior Ghidra decompile/disassembly `0x005F5320`, `0x005F5370..0x005F537A`)
- `[RESOLVED] OQ-002 - What is the BuildingClass vtable +0x124 target? -> `0x0043F180`, named `BuildingClass::SetMissionAndAnims` in the vtable report.` (evidence: `BUILDINGCLASS_VTABLE_COMPLETE.md`; prior xref `0x007E3FE0 -> 0x0043F180`)
- `[RESOLVED] OQ-003 - What is the static visual/animation effect if a building does not swallow 0x0D? -> It enters the BuildingClass mark/attached-animation refresh path with mode 2.` (evidence: `RADIO_MSG_0X0D_SENDERS_ANIM_RESET_GHIDRA_REPORT.md` sections 3.2/3.3)
- `[RESOLVED] OQ-004 - Does a stock war factory run that generic effect? -> No. BuildingClass swallows 0x0D with return 1 when `WeaponsFactory=yes`.` (evidence: prior decompile `0x0043C2D0`; `rulesmd.ini:11775`, `12565`, `13309`)
- `[RESOLVED] OQ-005 - Is `Factory=UnitType` the swallow gate? -> No. It is present on stock WFs, but binary evidence points to `WeaponsFactory=` / `BuildingType+0x16BD`.` (evidence: `0x0043C2D0`; `rulesmd.ini`)
- `[RESOLVED] OQ-006 - Should Rust use radio 0x0D to start or reset stock war-factory production animation? -> No; the binary receiver path for WFs is a no-op ack.` (evidence: `0x0043C2D0`; ObjectClass fallback evidence)
- `[DEFERRED] OQ-007 - What exact pixel/frame delta would occur if the `WeaponsFactory` swallow were removed?` (category: `needs-runtime-debugger`; reason: static evidence proves the skipped call and broad attached-animation refresh category, but not the exact rendered frame; next-step-if-pursued: capture GAWEAP/NAWEAP/YAWEAP produced-unit exit with a breakpoint/patch that forces fallthrough to ObjectClass `+0x124(2)`)
- `[DEFERRED] OQ-008 - What is the exact branch-by-branch write set inside `0x0043F180(mode=2)`?` (category: `bounded-cost-too-high`; reason: full 2400-byte routine is outside this follow-up; next-step-if-pursued: separate exhaustive slice on `BuildingClass::SetMissionAndAnims(mode=2)`)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| ObjectClass radio `0x0D` is a generic receiver-side `vtable+0x124(2)` refresh, not contact teardown. | `0x005F5320`, `0x005F5370..0x005F537A` | missing generic radio fallback | future radio abstraction; `src/sim/game_entity.rs` contact state; building anim refresh surface | If modeled, represent `0x0D` as a mark/attached-anim refresh event for eligible non-WF buildings | `radio_0d_non_weapon_factory_invokes_building_mark_anim_refresh` | Do not model `0x0D` as `BREAK`; it does not clear contacts |
| Stock war factories swallow `0x0D` and return `ROGER=1`, skipping ObjectClass `+0x124(2)`. | `0x0043C2D0`; `rulesmd.ini` `WeaponsFactory=yes` | must be preserved if generic `0x0D` is added | `src/sim/production/production_spawn.rs`, building type flags, any radio receiver layer | Check `WeaponsFactory=yes` before invoking any generic building refresh hook | `war_factory_radio_0d_swallow_preserves_production_anim_state` | Do not trigger `building_anim_overlays` from produced-unit contact churn on `GAWEAP`/`NAWEAP`/`YAWEAP` |
| Stock land war-factory production/door visuals are not driven by radio `0x0D`; stock WFs lack active `ProductionAnim` keys in `artmd.ini`. | `artmd.ini:[GAWEAP]`, `[NAWEAP]`, `[YAWEAP]`; `0x0043C2D0` swallow | current Rust has separate overlay and production contact surfaces | `src/app_building_anim.rs`, `src/app_instances/shp.rs`, production spawn/contact code | Keep production visuals on their existing production/building-animation triggers; keep radio contact as sim/pathing state | Produce a Grizzly/Lasher from a stock WF: contact row-skip works, but no extra overlay restart/reset happens from radio `0x0D` | Do not substitute `Factory=UnitType`, `Bib=yes`, or contact presence for the binary `WeaponsFactory` swallow |

## 10. Negative Facts / Do Not Do

- Do not call BuildingClass vtable `+0x124(2)` for `WeaponsFactory=yes` receivers when handling radio `0x0D`; the binary returns before ObjectClass.
- Do not treat `0x0D` as `BREAK(0x03)` or `LEAVE_DOCK(0x19)`; it does not clear contact vectors or `+0x418`.
- Do not broadcast `0x0D` to all contacts; the verified sender uses `Transmit_Radio_ToFirst`.
- Do not key the swallow on `Factory=UnitType`, `Bib=yes`, `ExitCoord`, or `NumberImpassableRows`; the receiver branch uses `WeaponsFactory=yes`.
- Do not use `0x0D` as the trigger for stock war-factory production, door, roof, or crane animation. Static evidence says the WF receiver path is an ack/no-op.

## 11. Remaining Uncertainty

- Screenshot-level effect remains uncertain. Static evidence proves that `+0x124(2)` would run for non-swallowing buildings and is skipped for war factories, but this pass did not capture frames showing what would visibly change if the swallow were disabled.
- The exact internal write set of `BuildingClass::SetMissionAndAnims(mode=2)` remains deferred. The available report-level evidence is enough for the Rust handoff because the stock WF path does not call it.
- No fresh live Ghidra decompile was possible in this subagent slot because no Ghidra MCP tools were exposed. Claims above cite prior Ghidra-backed reports with their recorded decompile, assembly, and xref evidence.

## 12. Stale Docs / Follow-up Docs

- `BUILDINGCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md` should replace "fires for WeaponsFactory buildings when a manufactured unit disconnects" with: "acknowledges contacted-peer `0x0D` mark/attached-animation refresh notifications for `WeaponsFactory=yes` buildings, but swallows them before ObjectClass can invoke `vtable+0x124(2)`."
- Any Rust design doc that says radio `0x0D` should reset war-factory production or door animation should instead say: "For `WeaponsFactory=yes`, radio `0x0D` is an ack-only no-op; production/door visuals must come from the normal war-factory production animation path, not this radio message."

## Sources

- `docs/research/RADIO_MSG_0X0D_SENDERS_ANIM_RESET_GHIDRA_REPORT.md`
- `docs/research/BUILDINGCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md`
- `docs/research/BUILDINGCLASS_VTABLE_COMPLETE.md`
- `docs/research/BUILDINGCLASS_UPDATE_ANIMATION_GHIDRA_REPORT.md`
- `docs/research/miner/REFINERY_DOCK_CELL_AND_ANIM_HELPERS_GHIDRA_REPORT.md`
- `ini/rulesmd.ini`
- `ini/artmd.ini`
- Rust scan: `src/sim/game_entity.rs`, `src/sim/production/production_spawn.rs`, `src/app_building_anim.rs`, `src/app_instances/shp.rs`

## Status

COMPLETE for the static receiver/swallow/Rust-handoff slice. PARTIAL for screenshot-level visual frame delta, explicitly deferred as runtime-capture work.
