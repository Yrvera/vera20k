# Aircraft Mission Verb Override Family — Ghidra Report

**Date:** 2026-07-22  
**Binary:** active Yuri's Revenge `gamemd.exe` (image base `0x00400000`)  
**Mode:** exhaustive slice  
**Status:** **COMPLETE for the bounded five-slot Aircraft mission-verb family**  
**Rust changes:** none

## Target question

What are the exact `AircraftClass` vtable entries and active bodies at
`+0x1E8..+0x1F8` for Queue, Commence, Assign, Override, and Restore? In
particular:

- which old report labels are stale;
- which raw missions the Aircraft leaf filter protects;
- how `AircraftClass+0x294` changes that filter;
- whether arguments are preserved through the parent call chain;
- what the raw-current-`0x1E` Commence exception does; and
- what current Rust lacks before these verbs can become authoritative.

## Non-goals

- Full paradrop, spyplane, attack, radio, or `AirstrikeClass::AI` behavior.
- The full mission dispatcher or every caller of the five virtual slots.
- Re-research of the base Mission verb bodies beyond the immediate call-through
  and state/order facts needed to understand this Aircraft family.
- Rust implementation, contract refresh, or edits to older research documents.

## Prior state

| Source | Prior state | Use in this pass |
|---|---|---|
| `AIRCRAFTCLASS_GHIDRA_REPORT.md` sections 38/43 | Partial and contradictory; labels `0x0041B870` as Override, `0x0041BB30` as a NavCom-only helper, gives stale mission names, and calls `+0x294` an owner-building check | Enumerated as claims to retest, not accepted as authority |
| `READYTOCOMMENCE_AIRCRAFT_BUILDING_WRITER_LIFECYCLES_GHIDRA_REPORT.md` | Newer evidence correctly identifies `0x0041B870` as Commence | Reverified from raw vtable bytes and assembly |
| `MISSIONCLASS_VERB_API_GUARDS_OVERRIDE_RESTORE_SEMANTICS_GHIDRA_REPORT.md` | Base-slot/signature research | Used to seed the parent-chain check; load-bearing rows rechecked live |
| Current Rust | Partial mission-verb substrate | Read-only disparity scan |

## Evidence needed to call this slice complete

- [x] Prove the receiver vtable is `AircraftClass` through constructor and RTTI.
- [x] Read all five raw pointers at `+0x1E8..+0x1F8`.
- [x] Disassemble every Aircraft leaf body and every immediate parent body needed
  for argument/order semantics.
- [x] Read all six current/target classifier arrays used by Assign, Queue, and
  Override.
- [x] Prove the identity, initialization, and stock-data liveness of `+0x294`.
- [x] Check virtual callsites showing the family is active in YR.
- [x] Map every raw filter id through the current verified MissionType table.
- [x] Scan current Rust storage, APIs, and callers without editing source.
- [x] Reconcile the contradictory older prose with exact replacement wording.
- [x] Run a cold second pass over the raw vtable and the two most load-bearing
  bodies (`0x0041B870`, `0x0041BB30`) with zero new material questions.

## Verdict

The exact Aircraft family is:

| Vtable slot | Index | Raw pointer | Exact role | Leaf status | Active in YR |
|---:|---:|---:|---|---|---|
| `+0x1E8` | 122 | `0x0041BA90` | Queue | Aircraft override | **Yes** — constructor installs this vtable and live Aircraft code calls the slot |
| `+0x1EC` | 123 | `0x0041B870` | Commence | Aircraft override | **Yes** — reached directly and through base Queue's virtual promotion path |
| `+0x1F0` | 124 | `0x0041B9F0` | Assign | Aircraft override | **Yes** — active virtual command surface |
| `+0x1F4` | 125 | `0x0041BB30` | Override `(mission, combat_target, navcom)` | Aircraft override | **Yes** — `AircraftClass::Mission_SpyPlane` calls `+0x1F4` at `0x00417499` |
| `+0x1F8` | 126 | `0x004D8F80` | Restore | inherited `FootClass` body | **Yes** — `AircraftClass::Enter_Idle_Mode` calls it at `0x00417706` |

The raw 20 bytes at `0x007E248C` are
`90ba4100 70b84100 f0b94100 30bb4100 808f4d00`, which decode in order to
the five pointers above. **Active in YR: Yes.**

This resolves the central contradiction: **`0x0041B870` is Commence, not
Override. `0x0041BB30` is the three-argument Aircraft Override, not a standalone
Set-NavCom helper.**

## Receiver and inheritance proof

Read-only Ghidra evidence:

- `AircraftClass` constructor `0x00413D20` writes primary vtable
  `0x007E22A4` at `0x00413D87`.
- `0x007E22A0` points to Complete Object Locator `0x007FB4C0`; its TypeDescriptor
  is `0x00817B90`, whose RTTI name is `.?AVAircraftClass@@`.
- The sibling `FootClass` table at `0x007E8C94` has base Mission bodies at
  `+0x1E8..+0x1F0`, then `0x004D8F40`/`0x004D8F80` at
  `+0x1F4/+0x1F8`. Its RTTI resolves to `.?AVFootClass@@`.
- Aircraft replaces the first four entries but retains Foot Restore at
  `+0x1F8`.

**Active in YR: Yes.** These are constructor-installed primary vtables, not
orphan tables or TS-only remnants.

## Shared Aircraft filter used by Assign, Queue, and Override

### Exact protected set

All six classifier arrays are byte-identical. For raw missions `4..31`, each is:

```text
00 01 01 01 01 01 01 01 01 01 01 01 01 01
01 01 01 01 01 01 01 01 00 00 01 01 00 00
```

The zero entries are offsets `{0,22,23,26,27}` from raw id 4, therefore the
protected set is exactly:

| Raw id | Current verified `MissionType` name | Active in YR |
|---:|---|---|
| `4` | `Retreat` | **Conditional** — when an Aircraft is currently/targeted to this mission |
| `0x1A` (26) | `ParadropApproach` | **Yes/conditional** — stock paradrop phase |
| `0x1B` (27) | `ParadropOverfly` | **Yes/conditional** — stock paradrop phase |
| `0x1E` (30) | `SpyplaneApproach` | **Yes/conditional** — stock spyplane phase |
| `0x1F` (31) | `SpyplaneOverfly` | **Yes/conditional** — stock spyplane phase |

The old `{QMove, Open, Rescue, ParaDropApproach, ParaDropOverfly}` naming is
stale and must not be reused. In the verified 32-entry table, QMove/Open/Rescue
are raw `3/24/21`, not `4/26/27`.

### Exact branch semantics

Let `P = {4, 0x1A, 0x1B, 0x1E, 0x1F}`. Assign, Queue, and Override run this leaf
gate before their parent call:

```text
if current_mission in P and this.airstrike_manager_at_0x294 == null:
    if target_mission not in P:
        return without calling the parent
call the parent with the original argument(s)
```

| Case | Result | Active in YR |
|---|---|---|
| Current not in `P`, including values outside `4..31` | Parent call is allowed | **Yes** |
| Current in `P`, `+0x294 == null`, target in `P` | Parent call is allowed | **Yes/conditional** |
| Current in `P`, `+0x294 == null`, target not in `P` | Entire leaf verb is a no-op; parent is not called | **Yes/conditional**; this is the stock-aircraft protected case |
| Current in `P`, `+0x294 != null` | Gate is bypassed; any target reaches the parent | **Conditional** on an Airstrike manager existing |
| Protected/null current with target outside `4..31`, including `-1` | Blocked by the unsigned `JA` range branch | **Yes/conditional** |

There is no wildcard exception for `-1`, and there is no target normalization
before classification. The checks use full 32-bit mission values.

### `AircraftClass+0x294` identity and stock liveness

- `TechnoClass` construction initializes `+0x294` to null at `0x006F2E09`.
- `TechnoClass::Init_Managers @ 0x006F3F40` reads type field `+0x61C`; when it
  is positive, it allocates a `0x60`-byte object through `0x0041D380` and stores
  the returned pointer at `+0x294` (`0x006F41EE`).
- The allocated object's vtable `0x007E29A8` resolves through RTTI to
  `.?AVAirstrikeClass@@`.
- In the repo stock INIs, the only `AirstrikeTeam`/elite type/count/recharge
  keys occur in `[BORIS]` in `ini/rulesmd.ini`; Boris is infantry. No stock
  AircraftType defines the manager input.

Therefore `+0x294` is an **`AirstrikeClass*`**, not an owner-building pointer.
The exact verb condition is **null means apply the restriction; non-null means
bypass it**. **Active in YR: Yes** for the null test; **the non-null bypass is
Conditional** for mod/scenario aircraft data and is not normally present on
stock AircraftTypes.

## Exact five bodies

### `+0x1E8`: Aircraft Queue — `0x0041BA90`

Native stack contract: two 32-bit arguments, `(mission, commence_now)`, `RET 8`.

1. Read old current mission from `this+0xAC`.
2. Apply the shared Aircraft filter to the requested mission.
3. If allowed, reload the original second argument from the stack, push it, push
   the original mission, and call base Queue `0x005B35E0`.
4. If blocked, return without the base call or a leaf write.

The wrapper preserves both complete DWORD arguments and their order. Base Queue
reads only the low byte of `commence_now`; when nonzero it calls virtual
ReadyToCommence at `+0x200`, then virtual Commence at `+0x1EC` if ready. Thus an
Aircraft immediate Queue promotion dispatches back to `0x0041B870`.

**Active in YR: Yes.** `AircraftClass::Enter_Idle_Mode` calls Queue at
`0x00417B4B` and then performs the same Ready/Commence virtual sequence at
`0x00417B55..0x00417B63`.

### `+0x1EC`: Aircraft Commence — `0x0041B870`

Exact body:

```text
if dword[this + 0xAC] != 0x1E:
    byte[this + 0x6D2] = 0
tail-jump MissionClass::Commence @ 0x005B3570
```

There are no stack arguments. The raw-current comparison occurs **before** the
tail jump promotes the queued mission, so it observes the old current mission.
Raw `0x1E` is `SpyplaneApproach`, not `ParadropApproach`. If old current is
`SpyplaneApproach`, `+0x6D2` is preserved even when Commence installs a different
mission; every other old current clears it first.

The best evidence-backed field name remains `aircraft_action_latch`; the old
`IsStrafe` name is not established by this body.

**Active in YR: Yes.** The exception is **Conditional** on old current raw
`0x1E`; the branch belongs to the live stock spyplane mission family.

### `+0x1F0`: Aircraft Assign — `0x0041B9F0`

Native stack contract: one 32-bit mission argument, `RET 4`.

1. Apply the shared Aircraft filter.
2. If allowed, push the unchanged mission and call base Assign `0x005B2FD0`.
3. If blocked, return without a parent call or leaf write.

**Active in YR: Yes.** The blocking branch is **Conditional** on protected old
current plus null `+0x294` plus a target outside the protected set.

### `+0x1F4`: Aircraft Override — `0x0041BB30`

Native stack contract: exactly three 32-bit arguments,
`(mission, combat_target, navcom)`, `RET 0xC`.

1. Apply the shared Aircraft filter to `mission`.
2. If allowed, reload all three original DWORD arguments and call
   `FootClass::Override @ 0x004D8F40` unchanged.
3. If blocked, return before any mission, target, or NavCom save/write.

The allowed parent chain is load-bearing:

| Order | Body | Exact effect | Active in YR |
|---:|---:|---|---|
| 1 | Foot `0x004D8F40` | Save current NavCom `this+0x5A4` to suspended NavCom `this+0x5A8` | **Yes** |
| 2 | Techno `0x007013A0` | Save current combat target `this+0x2B4` to suspended target `this+0x2B8` | **Yes** |
| 3 | Mission `0x005B3650` | Apply base Override to the mission argument; its ABI still consumes all three stack arguments (`RET 0xC`) | **Yes** |
| 4 | Techno `0x007013A0` | Call virtual `+0x3C8` with the new combat target | **Yes** |
| 5 | Foot `0x004D8F40` | Call virtual `+0x480` with `(new_navcom, 1)` | **Yes** |

The target and NavCom arguments are not padding and are not discarded merely
because the base Mission body only reads the mission. The subclass layers own
them. Also, once the Aircraft leaf gate allows the call, the target/NavCom
setter calls occur after the base call even when a base Mission guard declines
the mission write; the wrappers do not branch on a base return value.

**Active in YR: Yes.** `AircraftClass::Mission_SpyPlane` contains a virtual
`+0x1F4` call at `0x00417499`.

### `+0x1F8`: inherited Foot Restore — `0x004D8F80`

Aircraft has no leaf Restore body and no Restore-side protected-mission or
`+0x294` filter. The inherited chain is:

1. Foot calls `TechnoClass::Restore @ 0x007013E0`.
2. Techno calls base `MissionClass::Restore @ 0x005B36B0`.
3. If base Restore returns false, Techno returns false without target restore;
   Foot returns false without NavCom restore.
4. If base Restore succeeds, Techno calls virtual `+0x3C8` with saved target
   `this+0x2B8`, then returns true.
5. Foot then calls virtual `+0x480` with `(this+0x5A8, 1)` and returns true.

Success order is therefore **mission -> combat target -> NavCom**. Failure is
an all-or-nothing boundary for the latter two setters. These wrappers do not
explicitly clear the saved target or saved NavCom slots after use.

**Active in YR: Yes.** `AircraftClass::Enter_Idle_Mode` invokes virtual Restore
at `0x00417706`.

## Activity and caller census

A read-only whole-program `CALL` operand scan found 259 virtual calls matching
`+0x1E8`, 60 matching `+0x1EC`, 29 matching `+0x1F0`, 10 matching `+0x1F4`, and
3 matching `+0x1F8`. Counts are navigation evidence, not a claim that every
caller is active or Aircraft-receiving.

The Aircraft-local callsites above, its constructor-installed vtable, and the
base Queue re-dispatch prove the bounded family itself is active in YR.
**Active in YR: Yes.**

## Current Rust scan

| Native requirement | Current Rust evidence | Verdict |
|---|---|---|
| Aircraft-specific filter on Assign/Queue/Override | `src/sim/mission/verb.rs:71-121` has generic verbs and only Selling/Deliberate guards | **DRIFT / missing** |
| Queue second `commence_now` argument and Ready->Commence chain | `queue_mission` at `verb.rs:82` accepts only a mission; `commence_queued` is separate | **DRIFT / missing** |
| Commence old-current-`0x1E` write to Aircraft `+0x6D2` equivalent | No authoritative Aircraft action-latch field or verb hook was found | **DRIFT / missing** |
| Three-argument Override | Rust `override_mission` accepts `(MissionCom, mission, now)` only | **DRIFT / missing** |
| Preserve/replace combat target during Override/Restore | `GameEntity` has `attack_target`, but no suspended-target partner or verb integration was found | **DRIFT / missing** |
| Preserve/replace NavCom during Override/Restore | `NavigationState::suspended_nav_com` exists at `src/sim/components.rs:300`, but direct search found no read/write user | **Partial storage only; behavior missing** |
| Restore success ordering and failure short-circuit | Rust Restore touches only `MissionCom` | **DRIFT / missing** |
| Airstrike-manager null/non-null gate input | No `AirstrikeTeam` parser or runtime `AirstrikeClass`-equivalent state was found; only the unrelated warhead `Airstrike=` boolean exists | **DRIFT / missing** |
| One mission authority | `GameEntity::aircraft_mission` at `src/sim/game_entity.rs:399` remains a separate Aircraft state machine while `MissionCom` is projected alongside it | **Parity risk; authority not unified** |

Additional exact-mechanism warning: Rust `override_mission` uses
`com.queued.take()`, while native base Override reads queued mission `+0xB4` as
the value to suspend when present but does not clear `+0xB4` in this body.
Rust Override/Restore also reset the Rust timer; the native bodies examined here
reset the mission stage byte but contain no timer reset. These are base-verb
contract concerns exposed by the Aircraft call-through, not new Aircraft-only
rules.

The Rust comments themselves state that Queue/Commence/Override/Restore had no
live callers when written (`src/sim/mission/verb.rs:16-24`). The direct current
scan still finds production callers only for Assign; the other verb references
are tests. **Current Rust parity: no.**

## Implementation Handoff

### Unit 1 — Aircraft leaf verb gate and Queue signature

- **Verified behavior:** Assign, Queue, and Override share the exact protected
  set and null-`AirstrikeClass*` gate above. Queue carries a second
  `commence_now` argument and may synchronously call Ready then virtual Commence.
- **Likely Rust surfaces:** `src/sim/mission/verb.rs`,
  `src/sim/mission/retask.rs`, `src/sim/game_entity.rs`, and Aircraft type/runtime
  initialization in `src/rules/object_type.rs` plus spawn setup.
- **Required design constraint:** pass an authoritative
  `has_airstrike_manager` fact; do not rename this to an owner-building or
  paradrop boolean. Stock false is not sufficient for mod-data parity.
- **Risk:** the filter must run before every parent write/call and must classify
  raw `-1`/out-of-range targets as blocked under a protected/null current.

### Unit 2 — Commence Aircraft latch hook

- **Verified behavior:** compare old current before promotion; clear the Aircraft
  action latch except when old current is raw `0x1E` (`SpyplaneApproach`), then
  run base Commence.
- **Likely Rust surfaces:** authoritative Aircraft runtime state in
  `src/sim/game_entity.rs` and the Queue/Commence owner in
  `src/sim/mission/verb.rs` or its simulation wrapper.
- **Risk:** testing the newly promoted mission instead of the old current changes
  the exception; calling the hook only from direct Commence misses immediate
  Queue promotion.

### Unit 3 — Coupled mission/target/NavCom Override and Restore

- **Verified behavior:** Override has three semantic arguments and save/write
  order `old NavCom -> old target -> mission override -> new target -> new
  NavCom`; successful Restore order is `mission -> target -> NavCom`, while a
  failed mission Restore runs neither later setter.
- **Likely Rust surfaces:** `src/sim/mission/verb.rs`,
  `src/sim/mission/retask.rs`, `GameEntity::attack_target`, and
  `NavigationState::{nav_com,suspended_nav_com}`.
- **Required storage:** a suspended combat target equivalent in addition to the
  already-declared suspended NavCom.
- **Risk:** do not implement `0x0041BB30` as a NavCom-only helper, do not reorder
  the saves/setters, and do not apply the Aircraft filter to Restore.

## Required tests

- `aircraft_mission_filter_exact_five_id_truth_table`
- `aircraft_mission_filter_maps_retreat_paradrop_and_spyplane_ids`
- `aircraft_mission_filter_blocks_none_and_out_of_range_target`
- `aircraft_airstrike_manager_nonnull_bypasses_leaf_filter`
- `aircraft_blocked_assign_queue_override_make_no_writes`
- `aircraft_queue_commence_now_runs_ready_then_aircraft_commence`
- `aircraft_commence_clears_action_latch_except_old_spyplane_approach`
- `aircraft_commence_exception_reads_old_current_before_promotion`
- `aircraft_override_preserves_three_arguments_and_native_order`
- `aircraft_override_base_guard_still_runs_target_and_navcom_setters`
- `aircraft_restore_failure_leaves_target_and_navcom_untouched`
- `aircraft_restore_success_orders_mission_target_navcom`
- `stock_aircraft_has_no_airstrike_manager_bypass`

For exact parity certification, at least the filter truth table and ordering
tests need gamemd-derived fixtures/traces; Rust-vs-Rust unit tests alone are
regression checks, not parity proof.

## Negative Facts

- `0x0041B870` is **not** Aircraft Override. It occupies `+0x1EC` and tail-jumps
  base Commence.
- `0x0041BB30` is **not** a two-argument or NavCom-only setter. It is the
  three-argument Aircraft Override and returns with `RET 0xC`.
- Raw `0x1E` is **not** ParadropApproach. It is SpyplaneApproach in the verified
  MissionType table.
- Raw `4` is **not** QMove; raw `0x1A/0x1B` are not Open/Rescue.
- `AircraftClass+0x294` is **not** an owner-building pointer in this gate. It is
  an `AirstrikeClass*`.
- Non-null `+0x294` does **not** strengthen the restriction; it bypasses it.
- Queue's second argument is **not** discarded by the Aircraft wrapper.
- Override's target/NavCom arguments are **not** unused padding.
- Restore has **no** Aircraft leaf filter and **no** `+0x294` check.
- Failed base Restore does **not** restore target or NavCom.
- The inherited restore wrappers do **not** explicitly clear saved target/NavCom
  storage after success.

## Stale-document replacement wording

### Replace `AIRCRAFTCLASS_GHIDRA_REPORT.md` section 38/43 claims with

> AircraftClass primary vtable `0x007E22A4` maps `+0x1E8` Queue to
> `0x0041BA90`, `+0x1EC` Commence to `0x0041B870`, `+0x1F0` Assign to
> `0x0041B9F0`, `+0x1F4` three-argument Override to `0x0041BB30`, and
> `+0x1F8` inherited Foot Restore to `0x004D8F80`. Assign, Queue, and Override
> apply the shared raw-mission set `{4,0x1A,0x1B,0x1E,0x1F}` =
> `{Retreat, ParadropApproach, ParadropOverfly, SpyplaneApproach,
> SpyplaneOverfly}` only when current is in the set and the
> `AirstrikeClass*` at `+0x294` is null; a requested mission outside the set is
> then blocked. `0x0041B870` is Commence, not Override: it clears byte `+0x6D2`
> unless old current is raw `0x1E`, then tail-jumps base Commence.
> `0x0041BB30` forwards all `(mission, combat_target, navcom)` arguments through
> Foot/Techno/Base Override. Restore succeeds in mission-target-NavCom order.

### Replace the stale caller wording in
`READYTOCOMMENCE_VTABLE_0X200_SUBCLASS_OVERRIDES_GHIDRA_REPORT.md`

> `0x0041B870` is the Aircraft Commence override at vtable `+0x1EC`.
> `AircraftClass::Queue @ 0x0041BA90` calls base Queue, whose nonzero commence
> path invokes virtual ReadyToCommence at `+0x200` and then virtual Commence at
> `+0x1EC`. The stale local `AircraftClass__Override_Mission` label at
> `0x0041B870` must not be used as semantic evidence; Aircraft Override is
> `0x0041BB30` at `+0x1F4`.

## Coverage Ledger

| Aspect | Coverage | Result |
|---|---|---|
| Receiver/vtable identity | Constructor, raw pointers, COL/RTTI | **Complete** |
| Inputs and ABI | All five entries; `RET 4/8/0xC`; parent stack reloads | **Complete** |
| Outputs and writes | Filter no-op, Commence latch, coupled target/NavCom state | **Complete** |
| Mission filters | Six raw classifier arrays and exact id mapping | **Complete** |
| Null/zero/sentinel edges | Constructor-null manager; target `-1`; Restore false | **Complete** |
| Ordering | Queue Ready/Commence; Override save/set; Restore success/failure | **Complete** |
| Stock-data liveness | Stock INI AirstrikeTeam census and Aircraft callsites | **Complete** |
| Determinism/RNG | No RNG, float, or iteration in these bodies | **Complete; no consumption** |
| Save/load/pause/replay | Field persistence outside these verb bodies | **Deferred as out of bounded scope** |
| Full Airstrike behavior | Only pointer identity/gate proven | **Deferred as explicit non-goal** |
| Full caller census | Representative Aircraft callsites plus program scan | **Sufficient for liveness; exhaustive semantic caller analysis is out of scope** |
| Rust touchpoints | Mission, Aircraft, target, NavCom, parser/runtime scan | **Complete for implementation handoff** |

## Stop conditions

The slice stops because all five pointers, bodies, signatures, filter tables,
manager-gate identity, parent ordering, edge branches, stock liveness, and Rust
touchpoints are resolved. A second cold read of `0x007E248C`,
`0x0041B870`, `0x0041BB30`, and `0x004D8F80` added no new material facts or
questions. Continuing into Airstrike AI, complete spyplane/paradrop handlers,
save/load, or the dispatcher would cross the approved boundary.

## Sources and exact live checks

### Live read-only Ghidra

- Raw memory/RTTI: `0x007E22A0`, `0x007E22A4`, `0x007E248C`,
  `0x007FB4C0`, `0x00817B90`, `0x007E8C90`, `0x007E8C94`.
- Leaf disassembly: `0x0041BA90`, `0x0041B870`, `0x0041B9F0`,
  `0x0041BB30`, `0x004D8F80`.
- Parent-chain disassembly: `0x004D8F40`, `0x007013A0`, `0x007013E0`,
  `0x005B35E0`, `0x005B3650`, `0x005B36B0`.
- Classifier memory: `0x0041BA48`, `0x0041BA6C`, `0x0041BAEC`,
  `0x0041BB10`, `0x0041BB90`, `0x0041BBB4` (28 bytes each).
- `+0x294` identity: Techno constructor `0x006F2B40`, manager init
  `0x006F3F40`, allocation constructor `0x0041D380`, vtable `0x007E29A8` and
  its `AirstrikeClass` RTTI chain.
- Active callsites: `0x00417499`, `0x00417706`,
  `0x00417B4B..0x00417B63`; whole-program virtual-call operand scans for
  `+0x1E8..+0x1F8`.

### Repo evidence

- `docs/research/READYTOCOMMENCE_AIRCRAFT_BUILDING_WRITER_LIFECYCLES_GHIDRA_REPORT.md`
- `docs/research/READYTOCOMMENCE_VTABLE_0X200_SUBCLASS_OVERRIDES_GHIDRA_REPORT.md`
- `docs/research/MISSIONCLASS_VERB_API_GUARDS_OVERRIDE_RESTORE_SEMANTICS_GHIDRA_REPORT.md`
- `docs/research/MISSIONCLASS_STATE_MACHINE.md`
- `docs/research/AIRCRAFTCLASS_0XA5_RADIO_GATE_WRITERS_GHIDRA_REPORT.md`
- `docs/research/AIRCRAFTCLASS_GHIDRA_REPORT.md`
- `src/sim/mission/verb.rs`, `src/sim/mission/mod.rs`,
  `src/sim/mission/retask.rs`, `src/sim/components.rs`,
  `src/sim/game_entity.rs`, `src/sim/aircraft/mod.rs`,
  `src/rules/object_type.rs`, `src/rules/warhead_type.rs`
- `ini/rules.ini`, `ini/rulesmd.ini`, `ini/art.ini`, `ini/artmd.ini`

## Remaining Uncertainty

No material uncertainty remains inside the bounded five-slot family.

The exact original C++ source method names are unavailable; the names in this
report are semantic roles proven by vtable position, ABI, body, parent chain,
and callers. Full Airstrike manager behavior, full mission-handler behavior,
serialization, and dispatcher authority remain deliberately unclaimed.

## Open Questions (final)

| ID | Question | Status | Resolution |
|---|---|---|---|
| OQ-01 | Is the table really AircraftClass? | **RESOLVED** | Constructor plus COL/RTTI prove it. |
| OQ-02 | What are the five slot identities? | **RESOLVED** | Queue, Commence, Assign, Override, inherited Restore. |
| OQ-03 | Is `0x0041B870` Override? | **RESOLVED** | No; raw slot `+0x1EC` and tail jump prove Commence. |
| OQ-04 | Is `0x0041BB30` only Set-NavCom? | **RESOLVED** | No; three args, `RET 0xC`, and full parent chain prove Override. |
| OQ-05 | What exact missions are filtered? | **RESOLVED** | `{4,26,27,30,31}` with current verified names listed above. |
| OQ-06 | Are the six classifiers identical? | **RESOLVED** | All six 28-byte reads match exactly. |
| OQ-07 | What is `+0x294`? | **RESOLVED** | Nullable `AirstrikeClass*`; null restricts, non-null bypasses. |
| OQ-08 | Does stock Aircraft normally have that manager? | **RESOLVED** | No stock AircraftType defines AirstrikeTeam; only Boris does. |
| OQ-09 | Does Queue preserve `commence_now`? | **RESOLVED** | Yes, full DWORD forwarded; base reads its low byte. |
| OQ-10 | When does Commence test raw `0x1E`? | **RESOLVED** | Before base promotion, against old current. |
| OQ-11 | Are Override's extra two args meaningful? | **RESOLVED** | Yes; Techno consumes target and Foot consumes NavCom. |
| OQ-12 | What is Restore order? | **RESOLVED** | Mission, target, NavCom; failed mission restore short-circuits both later setters. |
| OQ-13 | Does Restore share the Aircraft filter? | **RESOLVED** | No; Aircraft inherits Foot Restore unchanged. |
| OQ-14 | Are these slots active YR code? | **RESOLVED** | Constructor-installed vtable plus Aircraft-local virtual callsites. |
| OQ-15 | What is Rust's present parity state? | **RESOLVED** | Storage/API/authority gaps above; not parity-correct. |
| OQ-16 | How does full Airstrike AI behave? | **DEFERRED / non-goal** | Not needed to identify the pointer or exact verb gate. |
| OQ-17 | How are these fields serialized? | **DEFERRED / non-goal** | Outside the bounded verb bodies; must be handled by a later persistence slice. |
