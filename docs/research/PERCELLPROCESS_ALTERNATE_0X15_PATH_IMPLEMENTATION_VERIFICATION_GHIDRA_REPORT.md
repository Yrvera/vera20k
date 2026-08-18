# PerCellProcess Alternate 0x15 Path Implementation Verification - Ghidra Report

**Address(es):** `0x00739EC0`, `0x004D9290`, `0x005B3060`, `0x004DA530`, `0x006F9E50`, `0x00737430`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Verify whether the `UnitClass::PerCellProcess` contact-flag adjacent-building `0x15` sender can bypass the standard `Mission_Enter -> Building 0x0E -> Unit 0x16` retry in normal stock HARV/CMIN refinery unloading, and compare that with the current Rust `FaceSync -> MissionQueued` implementation.  
**Non-Scope:** Re-decoding accepted `NW+(3,1)`, stock `GetDockCoord`, `QueueingCell`, receiver-side `0x15` unload effects, full drive locomotor internals, or editing Rust.  
**Confidence:** High for static order, branch predicates, and Rust delta; Medium for concrete live-frame first-source winner because that still needs runtime logging of per-cell callbacks after `+0x418`.  
**Active in YR:** Conditional yes. The branch is live in standard YR unit per-cell processing, and stock HARV/CMIN refinery handshakes can set `+0x418`; the branch only fires if a later per-cell callback occurs while its predicates still hold.

## 0. Working Notes

Target question: Does the `UnitClass::PerCellProcess` alternate contact-flag adjacent-building `0x15` path bypass, replace, or merely supplement the later `Mission_Enter -> Building 0x0E -> Unit 0x16` retry in normal stock refinery unloading?

Non-goals: Do not re-prove `NW+(3,1)` versus `GetDockCoord`, do not re-decode `0x15` receiver unload side effects, do not inspect unrelated per-cell branches, and do not edit Rust.

Evidence needed to mark COMPLETE: Decompile plus assembly for the alternate `0x15` branch; decompile plus assembly/xref evidence for mission dispatch and FootClass AI order; decompile evidence for Unit `0x16`; stock INI activity proof for HARV/CMIN/refineries; current Rust scan of `FaceSync`/`MissionQueued`.

Stop conditions: Stop once the report can answer whether this branch can fire before the retry that sets `+0x418`, whether it can beat a due same-tick Mission_Enter pass, what Rust currently omits, and what exact behavior not to implement.

## 1. Overview

The alternate `UnitClass::PerCellProcess` branch is not the mechanism that gets a miner from accepted-cell arrival into the first `0x18/0x16` handshake. It requires `Techno+0x418` to already be set, so it cannot fire before a successful later `Mission_Enter` retry has already caused the building to send `0x18`.

Static tick order also shows that a due `Mission_Enter` pass runs before locomotor/per-cell processing in the same unit AI tick. Therefore this alternate branch cannot bypass a currently due Mission_Enter retry. It is a supplemental per-cell/cell-process `0x15` source: if a per-cell callback happens after `+0x418` is set and before a later retry sends `0x16 -> 0x15`, it can send `0x15`; otherwise the ordinary later retry path wins.

## 2. Class Layout / Key Offsets

| Offset / slot | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| `Techno+0x418` | Contact-entered flag; required before alternate branch continues | `0x0073A558..0x0073A560`; writer is Techno radio `0x18` from prior verified docs | Conditional yes |
| Foot destination / NavCom | Must be non-null and point to a building | `0x0073A566..0x0073A578` | Conditional yes |
| vtable `+0x184` | Current mission getter; requires mission `7` (`Enter`) | `0x0073A57D..0x0073A588`; mission 7 dispatch via `0x005B3110` | Yes |
| vtable `+0x1B8` | Current occupied cell getter, read at PerCellProcess entry | `0x00739ECD..0x00739EE2` in prior report; decompile `0x00739EC0` | Yes |
| Cell `(current_x, current_y - 1)` | Adjacent-building lookup cell | `0x0073A58A..0x0073A5B7` | Conditional yes |
| vtable `+0x278` | Directed radio send; sends message `0x15` to destination building | `0x0073A5C3..0x0073A5C8` | Conditional yes |
| `MissionClass+0xC8/+0xD0` | Mission dispatch start frame and duration gate | `0x005B307A..0x005B30A1`, `0x005B3116..0x005B3126` | Yes |
| Foot locomotor `+0x674`, vtable `+0x40` | Locomotor process call after mission dispatch | `0x004DA86E..0x004DA87A` | Yes |

## 3. Core Logic

### 3.1 Alternate branch predicates

The `UnitClass::PerCellProcess @ 0x00739EC0` branch has these exact gates:

1. `Unit+0x418 != 0`.
2. Current destination pointer is non-null.
3. Destination `WhatAmI()` returns `6` (building).
4. Unit current mission is `7`.
5. The cell one row north of the unit's current cell contains the same destination building pointer.

Assembly evidence:

- `0x0073A558`: reads byte `[EBP+0x418]`; `0x0073A55E..0x0073A560` exits if zero.
- `0x0073A566..0x0073A56C`: loads and null-checks destination.
- `0x0073A572..0x0073A578`: calls destination vtable `+0x2C`, requires type `6`.
- `0x0073A57D..0x0073A588`: calls unit vtable `+0x184`, requires mission `7`.
- `0x0073A58A..0x0073A599`: constructs `(current_x, current_y - 1)`.
- `0x0073A5B0..0x0073A5B7`: calls `MapClass::Get_CellClass` then `Look_up_building_in_cell`.
- `0x0073A5BC..0x0073A5BE`: requires destination pointer equality with the looked-up building.

Active in YR: Conditional yes. These are live UnitClass per-cell instructions; stock refinery radio can set `+0x418`, but the branch still requires a per-cell invocation after that point.

### 3.2 Send and return handling

If all gates pass, the branch sends directed radio `0x15` to the destination building:

- `0x0073A5C3`: push destination building.
- `0x0073A5C4`: push `0x15`.
- `0x0073A5C8`: call unit vtable `+0x278`.

Receiver return `1` or `5` is treated as non-fallback:

- `0x0073A5CE`: `DEC EAX`; original return `1` exits.
- `0x0073A5D1..0x0073A5D4`: subtracts `4`; original return `5` exits.
- Other returns call vtable `+0x174` with `DAT_00B1CFE8, 1, 0` at `0x0073A5D6..0x0073A5E4`.

Active in YR: Conditional yes. The branch can send `0x15`, but it does not itself snap position, mark pad occupancy, start cargo drain, play sound, or force unload animation; receiver-side `0x15` effects are covered by the separate `RADIO_0X15_START_UNLOAD_SIDE_EFFECTS_GHIDRA_REPORT.md`.

### 3.3 Interaction with Mission_Enter retry

`FootClass::AI @ 0x004DA530` calls `TechnoClass::AI_Update` first:

- `0x004DA539`: `CALL 0x006F9E50`.

`TechnoClass::AI_Update @ 0x006F9E50` calls `MissionClass::Mission_Dispatch` before returning to FootClass AI:

- `0x006FA655`: `CALL 0x005B3060`.

Only later does `FootClass::AI` call the active locomotor `Process`:

- `0x004DA86E`: load locomotor pointer from `+0x674`.
- `0x004DA877`: call locomotor vtable `+0x40`.
- `0x004DA87A`: post-call alive check.

`MissionClass::Mission_Dispatch @ 0x005B3060` gates mission calls by `+0xC8/+0xD0`:

- `0x005B307A..0x005B30A1`: load start and duration; dispatch only when elapsed is greater than or equal to duration.
- `0x005B3110`: mission 7 calls vtable `+0x240`.
- `0x005B3116..0x005B3126`: after handler return, stores current frame to `+0xC8` and handler return duration to `+0xD0`.

`FootClass::Mission_Enter @ 0x004D9290` is the stock mission-7 handler. It sends one `0x0E` per dispatch at `0x004D92B2..0x004D92BF`, then returns `ftol([Enter] Rate * 900) + RandomRanged(0,2)` at `0x004D946C..0x004D9497`.

Active in YR: Yes. HARV/CMIN are UnitClass objects, tick through this AI path, use mission 7 for Enter, and stock `[Enter] Rate=.016` gives a 14..16 frame retry.

### 3.4 Can the alternate branch bypass the retry?

No for the retry that first creates the contact state. The alternate branch requires `+0x418`, and `+0x418` is set by the `0x18` radio handshake that occurs only after a successful already-there `Mission_Enter -> Building 0x0E` pass. Accepted-cell movement arrival alone does not set `+0x418`, so the branch cannot turn the first arrival into an immediate `0x15`.

No for a due Mission_Enter pass in the same unit AI tick. The due mission dispatch runs before locomotor/per-cell processing in FootClass AI. If both are possible in one unit tick, Mission_Enter gets first opportunity.

Conditional for later handoff after `+0x418` is already set. If a per-cell callback occurs while the unit remains mission 7, destination is still the refinery, and `(current_x,current_y-1)` contains that refinery, the branch can send `0x15` before the next mission-timer retry. Static evidence proves this possible source but does not prove that a stopped accepted-cell HARV/CMIN gets a fresh no-movement per-cell callback before the next Enter timer. In the normal stopped accepted-cell path, the verified primary source remains the later Mission_Enter retry causing Unit `0x16`, which can then send `0x15` once facing is already synchronized.

Active in YR: Conditional yes. The branch is active; the bypass of a later retry is conditional on an actual post-`+0x418` PerCellProcess invocation.

## 4. INI Keys

| Key | Stock YR value | Role | Evidence | Active in YR |
|---|---:|---|---|---|
| `[Enter] Rate` | `.016` | Mission retry base: `ftol(.016 * 900) == 14`, plus `RandomRanged(0,2)` | `ini/rulesmd.ini:30507..30510`; `0x004D946C..0x004D9497` | Yes |
| `[CMIN] Dock` | `NAREFN,GAREFN` | Chrono Miner can target stock refineries | `ini/rulesmd.ini:7351..7361` | Yes |
| `[HARV] Dock` | `NAREFN,GAREFN` | War Miner can target stock refineries | `ini/rulesmd.ini:8215..8225` | Yes |
| `[GAREFN] DockUnload` / `Refinery` | `yes` / `yes` | Allied refinery participates in stock dock-unload path | `ini/rulesmd.ini:11722..11727` | Yes |
| `[NAREFN] DockUnload` / `Refinery` | `yes` / `yes` | Soviet refinery participates in stock dock-unload path | `ini/rulesmd.ini:12515..12520` | Yes |

## 5. Integration Points

| Function / point | Role | Evidence | Active in YR |
|---|---|---|---|
| `UnitClass::PerCellProcess @ 0x00739EC0` | Owns the alternate adjacent-building `0x15` send | decompile and assembly `0x0073A558..0x0073A5E4` | Conditional yes |
| `FootClass::Mission_Enter @ 0x004D9290` | Ordinary mission-7 retry source that sends `0x0E` | `0x004D92B2..0x004D92BF`, `0x004D946C..0x004D9497` | Yes |
| `MissionClass::Mission_Dispatch @ 0x005B3060` | Passive start-frame/duration gate for Mission_Enter | `0x005B307A..0x005B3126` | Yes |
| `TechnoClass::AI_Update @ 0x006F9E50` | Calls mission dispatch before FootClass locomotor processing resumes | `0x006FA655` | Yes |
| `FootClass::AI @ 0x004DA530` | Orders Techno AI before locomotor `Process` | `0x004DA539`, `0x004DA877` | Yes |
| `UnitClass::Receive_Radio @ 0x00737430`, case `0x16` | Later repeat-radio path can send `0x15` after facing sync/current checks | `0x007376BF..0x00737780` | Conditional yes |

## 6. Current Rust Implementation Status

Current Rust now has the corrected broad split:

- `src/sim/miner/mod.rs:107` defines `FaceSync` as contact/facing sync with no unload, sound, pad snap, or on-pad side effects.
- `src/sim/miner/mod.rs:111` defines `MissionQueued` as radio `0x15` having queued mission `0x10` only.
- `src/sim/miner/miner_dock_sequence.rs:645` gates `phase_mission_enter` on `enter_retry_due`.
- `src/sim/miner/miner_dock_sequence.rs:699..707` marks contact and enters `FaceSync` only when stopped at the accepted cell and the Mission_Enter-like pass is due.
- `src/sim/miner/miner_dock_sequence.rs:741..768` keeps `FaceSync` waiting for the Enter retry and facing acceptance before moving to `MissionQueued`.
- `src/sim/miner/miner_dock_sequence.rs:772..773` advances `MissionQueued` to `Pivoting`.
- `src/sim/miner/miner_dock_sequence.rs:805..830` starts unload-active Rust effects only in `start_unload_deploy`, reached through `phase_pivoting`, not in `MissionQueued`.

Current Rust does not explicitly implement the PerCellProcess alternate `(current_x,current_y-1)` branch as a separate source. For the normal stopped accepted-cell HARV/CMIN path, this is probably acceptable relative to the verified primary path: Rust waits for the later Enter retry and then queues `MissionQueued` from `FaceSync`. For exact parity, it remains a gap for cases where a real per-cell callback occurs after `+0x418` but before the next Mission_Enter retry.

Two stale comments remain in current files:

- `src/sim/miner/miner_dock_sequence.rs:1..6` still describes the sequence as "link onto the pad".
- `src/sim/miner/miner_tests.rs:3878` and `:4481` still mention older `Linked`/`phase_linked` wording in comments.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Alternate branch predicates | verified | `0x0073A558..0x0073A5BE` | none |
| Alternate `0x15` send/return handling | verified | `0x0073A5C3..0x0073A5E4` | none |
| Requirement that `+0x418` already be set | verified | `0x0073A558..0x0073A560`; `0x18` writer in prior report | none |
| Mission dispatch before locomotor/per-cell processing | verified | `0x004DA539`, `0x006FA655`, `0x004DA877` | none |
| Mission timer gate and storage | verified | `0x005B307A..0x005B3126` | none |
| `0x16` later path can send `0x15` | verified | `0x007376BF..0x00737780` | none for this slice |
| Normal stopped accepted-cell first winner | touched-not-exhausted | static order plus `+0x418` gate | runtime logging would prove exact frame/source |
| Current Rust `FaceSync/MissionQueued` split | verified | source scan listed above | branch-specific per-cell trigger missing |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - What is the exact investigation mode? -> exhaustive-slice for the alternate PerCellProcess 0x15 path interaction with Mission_Enter retry.` (evidence: user scope and report header)
- `[RESOLVED] OQ-02 - Can this branch fire before contact flag setup? -> No; it exits if `+0x418` is zero.` (evidence: `0x0073A558..0x0073A560`)
- `[RESOLVED] OQ-03 - What sets the required contact flag in the stock refinery path? -> radio `0x18` sets it, reached after an already-there Mission_Enter/Building 0x0E handshake per prior parent reports.` (evidence: `RADIO_0X18_CONTACT_FLAG_LIFECYCLE_GHIDRA_REPORT.md`; `MISSIONENTER_RETRY_TIMER_STORAGE_AND_DISPATCH_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-04 - Can accepted-cell arrival alone cause this alternate 0x15? -> No; accepted arrival before the later successful retry does not have `+0x418`.` (evidence: `0x0073A558`; Mission_Enter timer report)
- `[RESOLVED] OQ-05 - Does a due Mission_Enter pass run before FootClass locomotor Process in the same tick? -> Yes.` (evidence: `0x004DA539`, `0x006FA655`, `0x004DA877`)
- `[RESOLVED] OQ-06 - Can PerCellProcess beat a currently due Mission_Enter retry in that same unit tick? -> No; mission dispatch is earlier in the tick.` (evidence: same as OQ-05)
- `[RESOLVED] OQ-07 - Can the branch send 0x15 without `GetDockCoord`? -> Yes; this branch uses one-row-north building lookup and pointer equality, not `GetDockCoord`.` (evidence: `0x0073A58A..0x0073A5C8`)
- `[RESOLVED] OQ-08 - What receiver returns avoid fallback? -> returns `1` and `5`; other returns call vtable `+0x174` with fallback args.` (evidence: `0x0073A5CE..0x0073A5E4`)
- `[RESOLVED] OQ-09 - Is the branch active for stock HARV/CMIN/refineries? -> Conditional yes; HARV/CMIN dock to NAREFN/GAREFN, refineries are DockUnload/Refinery, and the unit branch is live, but it needs post-contact per-cell invocation.` (evidence: `ini/rulesmd.ini` lines listed; `0x00739EC0`)
- `[RESOLVED] OQ-10 - Does current Rust model the ordinary retry path? -> Yes, in the new split it waits in `FaceSync` for Enter retry due/facing acceptance before `MissionQueued`.` (evidence: `miner_dock_sequence.rs:645..768`)
- `[RESOLVED] OQ-11 - Does current Rust model the alternate one-row-north PerCellProcess source? -> No explicit source was found.` (evidence: source scan of `miner_dock_sequence.rs` and `miner_dock.rs`)
- `[RESOLVED] OQ-12 - Is omission of the alternate branch a normal-path blocker? -> Not for the verified stopped accepted-cell primary path; it is still an exact-parity gap for post-contact per-cell callback cases.` (evidence: static order plus current Rust scan)
- `[DEFERRED] OQ-13 - Does a stopped accepted-cell HARV/CMIN receive a no-movement UnitClass::PerCellProcess callback before the next Enter retry in a concrete retail run?` (category: `needs-runtime-debugger`; reason: static evidence does not prove callback occurrence without a cell-cross event; next-step-if-pursued: runtime trace `0x00739EC0`, `0x004D9290`, `0x00737430`, `+0x418`, current cell, and `+0xC8/+0xD0`)
- `[DEFERRED] OQ-14 - Exact drive-locomotor cell-cross callback source after `+0x418`.` (category: `requires-different-system-context`; reason: this report verifies interaction, not full locomotor callback internals; next-step-if-pursued: bounded DriveLocomotion per-cell callback slice)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Alternate PerCellProcess `0x15` cannot create the first contact; it requires `+0x418` already set. | `0x0073A558..0x0073A560`; prior `0x18` report | Rust matches the primary shape by setting contact during `MissionEnter`/`FaceSync`, not on raw arrival. | `miner_dock_sequence.rs::phase_mission_enter`, `phase_face_sync` | Preserve the rule that accepted-cell arrival alone does not queue `MissionQueued`. | Miner reaches accepted cell with no contact flag and not due for Enter retry; no `MissionQueued`, no unload side effects. | Do not add an arrival shortcut from `AwaitingAcceptedCell` to `MissionQueued`. |
| A due Mission_Enter pass runs before locomotor/per-cell processing in the same unit AI tick. | `0x004DA539`, `0x006FA655`, `0x004DA877` | Rust broadly matches by using `FaceSync` retry timing for normal handoff. | `miner_dock_sequence.rs::phase_face_sync`; miner tick ordering | Keep later `0x16 -> 0x15` handoff tied to Enter retry eligibility, not movement_target clearing. | Due retry and possible per-cell event same tick: retry path gets first opportunity. | Do not let a simulated PerCellProcess branch run before a due Enter retry for the same miner. |
| Alternate branch is a separate post-contact source: destination building must be one row north of current cell and pointer-equal to destination. | `0x0073A58A..0x0073A5C8` | Missing as explicit branch. | Future miner per-cell/contact integration, likely near movement callback handling rather than `phase_mission_enter` alone | If Rust later models post-contact per-cell callbacks, add a source-aware `0x15` attempt with these exact gates. | With contact flag set, mission 7, destination refinery north of current cell, and a per-cell callback before next retry, Rust can enter `MissionQueued`; with wrong destination or no contact, it cannot. | Do not fake this by moving/snap-setting the miner to `GetDockCoord` or by using `refinery_pad_cell` equality. |
| `0x15` still only queues mission `0x10`; unload-active effects are later. | `RADIO_0X15_START_UNLOAD_SIDE_EFFECTS_GHIDRA_REPORT.md`; branch send `0x0073A5C8` | Current Rust now has `MissionQueued` and starts deploy effects in `Pivoting`/`start_unload_deploy`. | `RefineryDockPhase::MissionQueued`, `phase_mission_queued`, `phase_pivoting` | Preserve no snap/sound/pad/cargo side effects in `MissionQueued`, regardless of whether the source was `0x16` or PerCellProcess. | `MissionQueued` tick has no `DockDeploy`, no display override, no cargo drain; deploy effects start only after mission 0x10 gate. | Do not make alternate PerCellProcess `0x15` a direct unload start. |
| Return values `1` and `5` are accepted by the alternate branch; other returns trigger fallback mission/action. | `0x0073A5CE..0x0073A5E4` | Not modeled because branch is missing. | Future branch-specific radio result handling | If implemented, handle refusal/fallback distinctly. | Building receiver returns non-1/non-5 from the alternate source; miner does not proceed to unload and takes fallback. | Do not treat every attempted branch send as success. |

## 10. Negative Facts / Do Not Do

- Do not implement the alternate branch as a bypass for the initial accepted-cell arrival. It requires `+0x418`, and accepted-cell arrival does not set that flag.
- Do not run a simulated PerCellProcess `0x15` before a due Mission_Enter retry in the same unit tick. Mission dispatch is earlier than locomotor/per-cell processing.
- Do not collapse `FaceSync` and the alternate branch into a generic "linked" event with snap, sound, or pad occupancy.
- Do not use `GetDockCoord` or `refinery_pad_cell` equality for this alternate branch. It checks `(current_x,current_y-1)` building pointer equality.
- Do not treat the missing explicit branch as evidence that the new Rust split is wrong for the normal stopped accepted-cell path. The static evidence supports the retry path as the ordinary handoff source there.
- Do not ignore the missing branch forever: exact parity still needs it for any post-contact per-cell callback case that occurs before the next Mission_Enter retry.

## 11. Sources

- Ghidra read-only decompile: `UnitClass::PerCellProcess @ 0x00739EC0`.
- Ghidra read-only assembly context: `0x0073A558..0x0073A5E4`.
- Ghidra read-only decompile: `FootClass::AI @ 0x004DA530`.
- Ghidra read-only assembly context: `0x004DA539`, `0x004DA86E..0x004DA87A`.
- Ghidra read-only decompile: `TechnoClass::AI_Update @ 0x006F9E50`.
- Ghidra read-only assembly context: `0x006FA655`.
- Ghidra read-only decompile/assembly: `MissionClass::Mission_Dispatch @ 0x005B3060`, `0x005B307A..0x005B3126`.
- Ghidra read-only decompile/assembly: `FootClass::Mission_Enter @ 0x004D9290`, `0x004D92B2..0x004D92BF`, `0x004D946C..0x004D9497`.
- Ghidra read-only decompile/assembly: `UnitClass::Receive_Radio @ 0x00737430`, `0x007376BF..0x00737780`.
- `docs/research/UNITCLASS_PERCELLPROCESS_CONTACT_FLAG_ADJACENT_BUILDING_0X15_BRANCH_GHIDRA_REPORT.md`.
- `docs/research/UNITCLASS_PERCELLPROCESS_CALLER_TICK_ORDER_GHIDRA_REPORT.md`.
- `docs/research/MISSIONENTER_RETRY_TIMER_STORAGE_AND_DISPATCH_GHIDRA_REPORT.md`.
- `docs/research/RADIO_0X15_START_UNLOAD_SIDE_EFFECTS_GHIDRA_REPORT.md`.
- INI checked: `ini/rulesmd.ini`.
- Current Rust scanned: `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/mod.rs`, `src/sim/miner/miner_dock.rs`, `src/sim/miner/miner_tests.rs`.
