# AMCV Arrival-Stop → Deploy Handoff Retrace — 2026-07-29

**Scenario:** stock YR `[AMCV]`, human player, driving east across flat clear Temperate ground toward an
ordered destination of `(60,50)`; the player presses **D** (or clicks Deploy) while the MCV is still
moving, one to two cells short. Bounded to the locomotion side of the handoff: arrival braking and
stop, the locomotor query the deploy path gates on, the wait-for-stop loop, and the resting cell /
sub-cell / body facing the ConYard inherits. Foundation validation, `CanDeployAtLocation`, EVA
feedback, and the MCV→building conversion are out of slot.

**Status:** **RED / NOT PARITY.** The native handoff is a four-stage mechanism — the deploy *event*
nulls the owner destination and calls the Drive locomotor's `Stop_Moving` (which clamps speed and
drops the destination but deliberately leaves the committed track running), the queued Unload mission
is held by `ReadyToCommence` until the locomotor's `Is_Moving_Now` slot goes false, the drive finishes
its committed cell step and comes to rest, and only then does the mission FSM's stopped-check fire
`UnitClass::Deploy`. Rust implements none of that for `Command::DeployMcv`: the command converts the
MCV to a ConYard **inside the same command-apply phase, at whatever cell the MCV's centre currently
occupies**, with no locomotor query, no stop, and no wait. The building lands one cell off the
native anchor whenever the player deploys mid-move, which is an ordinary opening-minute action in
every match.

**Verdict tally:** **PASS 6 · FAIL 5 · UNCHECKED 2 · NOT-IMPLEMENTED 6** (19 bounded stages). A PASS
certifies only the named row.

## Scope, freshness, and evidence discipline

- Investigation only. No Rust, INI, or asset file was edited; no Cargo command was run; Ghidra access
  was strictly read-only (no rename, no comment, no `save_program`) because four sibling agents were
  reading the same program concurrently. This report is the sole written artifact.
- Program: `gamemd.exe` in project `testProsjekt`, image base `0x00400000`, 10036 functions
  (`get_current_program_info`). Repo tree clean at `ce096b3f`.
- Prior deploy reports (`MCV_DEPLOY_GHIDRA_REPORT.md`, `UNIT_MISSION_DEPLOY_BUILDING_GHIDRA_REPORT.md`,
  `MISSION_DEPLOY_BUILDING_0x73D630_STATE_MACHINE_GHIDRA_REPORT.md`,
  `AMCV_CANDEPLOY_PREDICATE_GHIDRA_REPORT.md`) were used as navigation only. Three of their
  load-bearing claims did not survive re-verification; the corrections are in §Top root findings and
  are derived from vtable slot bytes and callsite argument flow, not from labels.
- Local Ghidra labels are navigation hints. `UnitClass__ShouldIdle @ 0x00744270` and
  `Force_MCV_Deploy @ 0x004fc060` are both stale names; their verified roles are recorded below.
- No literal frame series, resting-cell series, or lepton series was executed against an oracle. Every
  such row is `UNCHECKED` and nothing was promoted from static plausibility.

## Retail inputs and command handoff

- `ini/rulesmd.ini:6969..7010` — `[AMCV]`: `DeploysInto=GACNST`, `Speed=4`, `ROT=5`, `Crusher=yes`,
  `MovementZone=Normal`, `Locomotor={4A582741-9839-11d1-B709-00A024DDAFD1}` (Drive). **No `Turret=`**,
  so body facing is the entire visual. **No `DeployToFire=`** and no `Enslaves=`/slave-miner keys.
- `ini/rulesmd.ini:3613` — `DeployToFire` appears only as a commented default (`def=no`); it is set
  nowhere in stock `rulesmd.ini`.
- `ini/rulesmd.ini:11622..11631` — `[GACNST]`, `ConstructionYard=yes`, `UndeploysInto=AMCV`.
- `ini/rulesmd.ini:30553..30558` — MissionControl `[Unload]`: `Recruitable=no`, `Retaliate=no`,
  `Scatter=no`, **`Rate=.016`**. This is the entry the deploy FSM's re-poll delay is computed from.
- `ini/artmd.ini:773..776` — `[MCV] Voxel=yes`, `Remapable=yes`.
- Rust command entry: `src/app_input.rs:1045`, `1085..1086` and `src/app_context_order.rs:559..562`
  emit `Command::DeployMcv { entity_id }` for any selected unit whose type has `deploys_into`.
- Rust execution: `src/sim/world/world_commands.rs:514..532` → `src/sim/world/world_spawn.rs:667..787`
  (`deploy_mcv`), applied inside `apply_due_commands`
  (`src/sim/world/mod.rs:1880..1927`) in the same tick the command is due.

## Native identity fixes that the rest of this trace rests on

The UnitClass vtable base is **`0x007F5C70`**, not `0x007F5C74`. Verified three ways:
`read_memory 0x007F5C60` shows a non-code pointer `0x0080CC68` at `0x007F5C6C` (the RTTI complete-object
locator that precedes a vtable); `get_xrefs_to 0x0073D630` returns the single data reference
`0x007F5EAC`, i.e. **`+0x23C`**; and `read_memory 0x007F60F0` puts `0x00738970` at `0x007F60F4`, i.e.
`+0x484`, which is exactly the `UnitClass` OnArrival override slot named in commit `26ef9d2a`
(`git show 26ef9d2a`).

With that base, the slots this trace uses are:

| Slot | Address | Verified role | How verified |
|---|---|---|---|
| `+0x1E8` | `0x005B35E0` | `Queue_Mission(mission, immediate)` | `disassemble_function 0x005b35e0`: reads `[ESP+4]`/`[ESP+0xC]`, `RET 0x8`; writes queued `+0xB4`, clears `+0xB8`, optionally `ReadyToCommence`→`Commence` |
| `+0x1EC` | `0x005B3570` | `Commence()` — promote queued→current | `disassemble_function 0x005b3570`: no stack args, `RET`; `+0xAC = +0xB4`, `+0xB4 = -1`, `+0xBC = 0`, `+0xD0 = 0` |
| `+0x200` | `0x00744270` | `ReadyToCommence` (label `UnitClass__ShouldIdle` is stale) | `decompile_function 0x00744270`: calls locomotor slot `+0x80` and returns 0 while moving |
| `+0x23C` | `0x0073D630` | `UnitClass::Mission_Deploy_Building` | `get_xrefs_to 0x0073D630` → `0x007F5EAC` |
| `+0x480` | `0x00741970` | `TechnoClass::Set_Destination` | `read_memory 0x007F60F0`; `decompile_function 0x00741970` |
| `+0x484` | `0x00738970` | OnArrival / queued-destination advance | `read_memory 0x007F60F0`; commit `26ef9d2a` |

**`MissionClass::Mission_Dispatch @ 0x005B3060` case `0x10` calls `vtable+0x23C`**
(`decompile_function 0x005B3060`). So the MCV deploy FSM runs under **mission 16 (Unload)**, not
mission 13. This is self-consistent with the FSM's own `if (queued != -1 && queued != 0x10)` guard at
`0x0073DE20..0x0073DE2E` ("a queued mission that isn't another Unload") and with the stock `[Unload]`
MissionControl section supplying its re-poll rate.

The Drive locomotor's ILocomotion vtable base is **`0x007E7EB0`** (`read_memory 0x007E7E80`, 208
bytes: an 8-slot `IPersistStream` vtable at `0x007E7E90` preceded by COL `0x007FFDD0`, then COL
`0x007FFDE8` at `0x007E7EAC` and the ILocomotion vtable from `0x007E7EB0`). Slots used here:

| Slot | Address | Role |
|---|---|---|
| `+0x10` | `0x004AFB80` | `Is_Moving` |
| `+0x44` | `0x004AFD40` | `Head_To_Coord` |
| `+0x48` | `0x004AFE00` | `Stop_Moving` (confirmed by `0x004AFE00`'s own cascade calling `+0x48` on chained owners' `+0x674` locomotors) |
| `+0x4C` | `0x004B0EF0` | `Do_Turn` (confirmed by `0x0073D86C` passing `facing << 0xD`) |
| `+0x80` | `0x004AFC20` | `Is_Moving_Now` |

**Interface-pointer trap:** the ILocomotion methods receive `this = object + 4` (they read the owner at
`this+0x8`), while `Process_Drive_Track` receives the real base (owner at `this+0xC`). Every Drive
field offset below is stated in **real-object** coordinates: destination `+0x34/+0x38/+0x3C`, head-to
`+0x40/+0x44/+0x48`, head-to-valid byte `+0x63`, speed double `+0x50`, budget `+0x4C`, track number
`+0x58`, track point index `+0x5C`. `FootClass+0x674` is the ILocomotion pointer (proved from
`0x004AFE00`'s chain walk, which calls `+0x48` on `*(chained + 0x674)`).

## Stage 1 — `Is_Moving` as an authority, and the predicate that actually gates the deploy

`decompile_function 0x004afb80` (`DriveLocomotionClass::Is_Moving`, ILocomotion `+0x10`), in real
offsets:

```text
if (destination{+0x34,+0x38,+0x3C} != NullCoord)          return 1;
if (head_to{+0x40,+0x44,+0x48} == NullCoord)              return 0;
if (head_to.X == owner.X && head_to.Y == owner.Y)         return 0;   // exact leptons, not cells
return 1;
```

So `Is_Moving` is true for **an outstanding locomotor destination, or an aim point the object has not
exactly reached**. It never reads the speed scalar `+0x50`, the residual budget `+0x4C`, the track
number `+0x58`, or the track point index `+0x5C`. "Nonzero speed" and "active track" are *not* its
truth conditions.

The FSM's use of it is at `0x0073DDA2..0x0073DDC2`
(`disassemble_function 0x0073D630`): `MOV EAX,[ESI+0x674]` / `CALL [ECX+0x10]` / `TEST AL,AL` /
`JNZ 0x0073DE20`. `[ESI+0x674]` is `param_1[0x19d]`, the ILocomotion pointer — so this is exactly
`DriveLocomotionClass::Is_Moving` for an AMCV. Prior doc text saying "`ILocomotion[4]`" is right
(index 4 = byte `+0x10`); the same report's `§3A` text "vtable+0x4" is wrong and is drift.

**But `Is_Moving` is not the gate a player experiences.** The queued Unload mission is held earlier by
`ReadyToCommence @ 0x00744270`, whose moving branch calls locomotor slot **`+0x80`**
(`decompile_function 0x00744270`: `CALL [locomotor_vtable + 0x80]`, `return 0` when true under the
height/mission/MissionComplete conditions). `decompile_function 0x004afc20`
(`DriveLocomotionClass::Is_Moving_Now`, interface-relative fields shown in real offsets):

```text
if (turn_timer_remaining != 0)                                   return true;
if (Is_Moving() /* slot +0x10 */ && head_to{+0x40,+0x44,+0x48} != NullCoord) {
    if (owner->vtable+0x538 /* current speed */ > 0)             return true;
}
return false;
```

Rust's `LocomotorReadyState::Drive` is `turning_active || (slot_moving && head_to_nonnull && owner_speed > 0)`
(`src/sim/movement/locomotor_ready.rs:71..82`) — a structural match for `Is_Moving_Now`, and
`src/sim/mission/readiness.rs:145..205` (`unit_ready_to_commence`) reproduces `0x00744270`'s branch
order. What Rust does **not** have is any query shaped like `Is_Moving` (`+0x10`) itself: the raw state
exists (`DriveLocomotionRuntime { destination, head_to, … }`,
`src/sim/components.rs:380..407`) but nothing reduces it to the destination-or-unreached-aim-point
predicate, so `slot_moving` has no derivation. And `LocomotorState::mission_ready_state`
(`src/sim/movement/locomotor.rs:121`) is written **only** by the test-only setter at
`locomotor.rs:381..383`; every production construction site sets it to `None`
(`air_movement.rs:744,780`, `droppod_movement.rs:220`, `jumpjet_movement.rs:286`, `locomotor.rs:273,347`,
`movement_bridge.rs:517`, `parachute_descent.rs:156`, `tunnel_movement.rs:328`,
`bridge_orchestrator.rs:1617`, `in_range.rs:397`). The readiness authority therefore falls back to
`DEGRADED_NOT_MOVING` (`src/sim/mission/authority.rs:199..226`), whose own comment states the
moving-defer branch never defers.

Most decisively for this scenario, **`Command::DeployMcv` consults none of it.** `deploy_mcv`
(`src/sim/world/world_spawn.rs:667..787`) reads type/foundation/facing, checks footprint occupancy and
terrain, and then either turns or despawns-and-spawns. There is no locomotor read anywhere in it.

## Stage 2 — the wait loop's cadence

`Mission_Dispatch @ 0x005B3060` gates re-entry on `+0xC8` (last dispatch frame) and `+0xD0` (delay in
frames) and stores the handler's **return value** into `+0xD0`. The deploy FSM's common exit is
`0x0073DE3A..0x0073DE64`:

```text
CALL 0x005B3A00              ; MissionClass::GetMissionTimerEntry
FLD  double ptr [EAX + 0x10] ; MissionControl[CurrentMission].Rate
FMUL double ptr [0x007E27F8] ; 900.0
CALL Math__ftol
PUSH 2 / PUSH 0 / CALL Random__RandomRanged
ADD  EAX, ESI                ; delay = ftol(Rate * 900) + Random(0,2)
```

`decompile_function 0x005b3a00` confirms `&g_MissionControl_Array + CurrentMission * 8` (32-byte stride,
`Rate` at entry `+0x10`), keyed on `+0xAC` = **current** mission. Current mission here is 16, so the
row is `[Unload] Rate=.016` (`ini/rulesmd.ini:30558`): `0.016 * 900 = 14.4` → `ftol` → `14`, plus
`Random(0,2)` ⇒ **the wait-for-stop state re-polls every 14–16 native frames**, not every frame.

In practice the MCV rarely spends time there: because `ReadyToCommence` already blocked commencement
on `Is_Moving_Now`, by the time state 1 first runs the drive has usually already come to rest, and
state 0 falls straight through into the state-1 check inside the *same* call (`0x0073DD82` sets
`+0xBC = 1` and drops into `0x0073DDA2` with no intervening return). The per-frame half of the wait is
the `Commence` promotion, which `UnitClass::AI` performs at `0x00736473` and `0x007366FD`
(`search_instructions` for `CALL dword ptr [… + 0x1ec]`).

Rust has no wait state at any cadence. The command converts on the tick it is due, so the 45 Hz
sim-tick / 15 Hz drive-budget vs ~62.5 native logic-frame calibration question does not even arise for
this row — there is no interval to compare. (`SIM_TICK_HZ = 45`, `src/util/fixed_math.rs:51`;
`SIM_TICK_MS = 1000/45 = 22`, `src/app_types.rs:25..27`.)

## Stage 3 — where the MCV actually stops (highest-stakes row)

The player's Deploy is network event **case 9** in `EventClass__Execute @ 0x004C6CB0`
(`decompile_function 0x004C6CB0`). After liveness/mind-control/`+0x504` gates and a
not-Construction/not-Selling/not-a-Building check, it does exactly four things:

```text
(**(code **)(*obj + 0x274))(3);        ; clear target/threat scan
(**(code **)(*obj + 0x480))(0, 1);     ; Set_Destination(NULL, 1)     <-- owner destination cleared
(**(code **)(*obj + 0x3c8))(0);        ; Assign_Target(NULL)
(**(code **)(*obj + 0x1e8))(0x10, 0);  ; Queue_Mission(Unload=16, immediate=0)
```

`Set_Destination(NULL)` reaches `FootClass::Set_Destination_Internal @ 0x004D94B0`
(`decompile_function 0x004D94B0`), whose null branch is:

```text
NavCom_Aux(+0x5A0) = NULL
NavCom(+0x5A4)     = NULL
if (NavCom == 0) {  ... aircraft-attack exception ...
    (**(code **)(*locomotor + 0x48))(locomotor);   // ILocomotion::Stop_Moving
}
```

And `DriveLocomotionClass::Stop_Moving @ 0x004AFE00` (`decompile_function 0x004afe00`), in real
offsets:

```text
if (head_to{+0x40,+0x44,+0x48} != NullCoord)  → cascade Stop_Moving down the +0x6C8 chain
                                                 (gated by Type+0xC94 and owner+0x6D0)
if (0.3 <= speed_double{+0x50}) speed_double = 0.3
destination{+0x34,+0x38,+0x3C} = NullCoord
```

It clears the **destination only**. `head_to`, `+0x63`, the track number `+0x58`, and the track point
index `+0x5C` all survive. Therefore `Is_Moving` stays true (destination null, head-to non-null and not
yet exactly reached), `Is_Moving_Now` stays true while speed > 0, and the MCV **cannot stop mid-cell** —
it drives out the track step it was already committed to, at a speed fraction now clamped to ≤ 0.3.

When that track ends, `Process_Drive_Track @ 0x004B0F20`'s end-of-track block clears `head_to`, sets
`+0x63 = 0`, `+0x58 = -1`, `+0x5C = 0`. The path-head reset and the arrival advance are conditional on
a flag whose only setter is at `0x004B21B1` (`get_assembly_context 0x004b2200`): `MOV byte ptr
[ESP+0x13], 0x1` sits **inside** the `NavCom != 0` arrival branch, after the NavCom-target cell match
and the `|Δz| < 2 * [0x008A07D0]` tolerance test. With NavCom already NULL that whole branch is
skipped, so at `0x004B223A..0x004B2275` the byte reads 0 and the engine does **not** call
`FootClass::Stop_Moving (0x004DF0D0)`, does **not** write `owner+0x5E0 = -1`
(`search_instructions … "0x5e0"` → `0x004B224A`), and does **not** fire the Move-gated OnArrival
(`CALL [vtable+0x184]` / `CMP EAX,0x2` / `CALL [vtable+0x484]` at `0x004B2259..0x004B226D`).

The drive does not restart itself from the surviving path: the top of `0x004B0F20` bails immediately
(`speed budget = 0; return 0`) when `+0x63 == 0` **or** `+0x58 == -1` and the path head is not the tube
sentinel `8`. A new track is only ever armed by `Head_To_Coord` (`+0x44`), and the only caller of that
is `Set_Destination_Internal` with a non-null destination — which never happens under mission 16.

Net native behaviour: **the ConYard is anchored on the cell the MCV was already committed to entering,
not on the ordered destination `(60,50)`.** Pressing D two cells short does not cause the MCV to finish
its route; it costs at most the remainder of one committed cell step.

Rust, by contrast, deploys on the spot. `deploy_mcv` reads `entity.position.rx/ry`
(`world_spawn.rs:678..682`) — the cell the sprite centre is currently in mid-step — feeds it to
`deploy_origin_from_center`, and (facing permitting) calls `self.uninit(stable_id)` and spawns the yard
at that anchor (`world_spawn.rs:766..784`). The MCV never stops, never finishes the step, and the
already-correct primitives are simply not wired in: `set_destination_internal_null`
(`src/sim/movement/navcom.rs:79..87`) and `drive_stop_moving` (`navcom.rs:216..224`) exist and model the
native pair faithfully, but `Command::DeployMcv` calls neither.

**Frequency clause:** every match, every player, usually inside the first thirty seconds — a player who
sends the MCV toward a spot and presses D during the drive is the ordinary opening, not an edge case.
A one-cell anchor error relocates the entire base, every adjacency radius, and every subsequent
placement.

## Stage 4 — sub-cell position and body facing at rest

Native: the resting position is the last point of the drive track's point series (`Transform_Track_Coords`
→ `vtable+0x1B4` Set_Location inside `0x004B0F20`), with the leftover motion budget kept in `+0x4C`;
the cell-centre landing is a property of the track tables, not of a snap. The FacingClass value is
whatever the final track step's direction byte produced — nothing in the mission-16 MCV branch resets
it, and the branch contains no `Do_Turn` (`+0x4C`) call at all (the only `+0x4C` calls in `0x0073D630`
are at `0x0073D86C` and `0x0073DFAD`, both in the transport-unload and harvester-approach paths). Any
turn to the yard's deploy facing therefore happens inside `UnitClass::Deploy @ 0x007393C0`, which is
deploy-side and out of this slot.

Rust's arrival cleanup is native-shaped where it runs: `finalize_finished_entities`
(`src/sim/movement/movement_tick.rs:1963..2013`) calls `finish_drive_navigation`, clears
`movement_target`, `drive_track`, and `body_facing`, snaps `sub_x/sub_y` to `subcell_dest` or the
sub-cell offset, and resets phase/wobble. In the deploy-while-moving case none of that executes,
because the entity is uninit'd before it ever finishes. `deploy_mcv` discards `position.sub_x/sub_y`
outright — the anchor is the raw cell index.

Body facing is a second, separate drift. Because `[AMCV]` has no turret, body facing is the whole
visual. Rust's facing branch (`world_spawn.rs:757..764`) does
`entity.facing_target = Some(deploy_facing); entity.facing = deploy_facing; entity.movement_target = None; return true;`
— it **snaps** the hull to the deploy facing with zero turn time (`ROT=5` is ignored), clears the move,
and requires the player to press D a second time. The deploy-side native rule is covered by
`AMCV_DEPLOY_FACING_RULE_GHIDRA_REPORT.md`; what belongs to this slot is that the instantaneous facing
write bypasses the locomotor turn mechanism entirely.

## Stage 5 — order-queue interaction across the arrival tick

Two distinct native paths share the end-of-track block, and only one of them is the deploy path.

- **Arrival at a live NavCom destination** (an ordinary completed Move): flag `[ESP+0x13] = 1` at
  `0x004B21B1`, then liveness-gated (`+0x90` set, `+0x81` clear, `+0x8D` clear) it calls
  `FootClass::Stop_Moving @ 0x004DF0D0`, writes `owner+0x5E0 = -1`, and fires
  `vtable+0x484` **only when `GetCurrentMission() == 2` (Move)**, popping the next queued waypoint.
  This is exactly what Rust `finish_drive_arrival` (`src/sim/movement/navcom.rs:156..170`) implements:
  `foot_stop_moving`, drive-runtime reset, and a `MissionType::Move`-gated `nav_queue` pop — landed in
  commit `26ef9d2a`, which cites the same addresses.
- **The deploy path**: NavCom is already NULL when the track ends, so the flag stays 0, the path head
  is not reset, `FootClass::Stop_Moving` is not called again, and OnArrival never fires. The queued
  Unload mission is promoted separately, by `Commence` from `UnitClass::AI`, on the first frame
  `ReadyToCommence` returns true. Mission 16's own state 0 is what finally writes `+0x5E0 = -1`
  (`0x0073DD8E`: `MOV dword ptr [ESI + 0x5E0], 0xFFFFFFFF`) before falling through to the `Is_Moving`
  check in the same call.

So the answer to "is the queued Deploy consumed same-tick as arrival, or one tick later" is neither:
in native it is consumed **one `UnitClass::AI` frame after the locomotor's `Is_Moving_Now` goes false**,
via the commence promotion, and the FSM then deploys on its first dispatch. The generic-case finding in
`DRIVE_ARRIVAL_QUEUED_ORDER_LIFECYCLE_TRACE_20260527.md` still holds for a Move arrival; it does not
describe the deploy variant, which takes the NavCom-null branch. Rust has no equivalent of the
NavCom-null variant because it never reaches an arrival at all.

## Stage 6 — `DeployToFire` and `IsSlaveMiner` reachability

- **`DeployToFire`** is `Unit+0x68C` (`0x0073D6BC`: `MOV AL, byte ptr [ESI+0x68C]`; also
  `0x0073DDD5`). It selects mission-state 2 after a successful-looking deploy and gates the state-2
  body at `0x0073DD5B`. `ini/rulesmd.ini:3613` shows the key defaults to `no` and stock `rulesmd.ini`
  never sets it, so for a stock AMCV/SMCV/PCV **state 2 is unreachable**. Out of scope; do not
  implement. (Correction: the state-2 fallback at `0x0073DD6A` tests `Unit+0x5A4` — the **NavCom** —
  not a slave-miner flag, contrary to the prior report's `§3C` wording.)
- **The slave-miner / non-`DeploysInto` branch** at `0x0073DE6E` is only reached when
  `UnitTypeClass+0x404` (`DeploysInto`) is **zero** (`0x0073D694`: `CMP dword ptr [EAX+0x404], EBX` /
  `JZ 0x0073DE6E`). `[AMCV] DeploysInto=GACNST` is non-zero, so it is unreachable for this fixture in
  principle, not merely by data. Rust already routes `Enslaves`-bearing units to
  `slave_miner::deploy_slave_miner` before `deploy_mcv` (`world_commands.rs:519..530`). Out of scope.

## Stage verdicts

| # | Stage | Verdict | Bounded result |
|---:|---|---|---|
| 1 | Retail `[AMCV]`/Drive/`[GACNST]` bindings | PASS | Type, locomotor CLSID, `DeploysInto`, no-turret, `Speed=4`, `ROT=5` all agree with the fixture. |
| 2 | Deploy command clears the owner destination | FAIL | Native event 9 calls `Set_Destination(NULL,1)`; Rust `DeployMcv` never touches `nav_com`/`drive_locomotion`. |
| 3 | Deploy command queues mission 16 instead of converting | NOT-IMPLEMENTED | Rust has no Unload/deploy mission FSM; the command converts inline. |
| 4 | Drive `Stop_Moving` semantics (clear destination, keep head-to, 0.3 clamp) | PASS | `navcom.rs:216..224` matches `0x004AFE00`; native `if (0.3 <= s) s = 0.3` and Rust `if (s > 0.3) s = 0.3` are algebraically identical over all inputs. |
| 5 | Drive `Is_Moving` (`+0x10`) predicate | NOT-IMPLEMENTED | No Rust query reduces `destination`/`head_to` to the native truth condition. |
| 6 | Drive `Is_Moving_Now` (`+0x80`) predicate | PASS | `LocomotorReadyState::Drive` reproduces `turn-timer OR (Is_Moving AND head-to AND speed>0)`. |
| 7 | Live producer for the readiness state | NOT-IMPLEMENTED | `mission_ready_state` is written only by a `#[cfg(test)]` setter; production always `None`. |
| 8 | `ReadyToCommence` gate on the deploy path | FAIL | `unit_ready_to_commence` mirrors `0x00744270`, but `DeployMcv` bypasses the mission layer entirely. |
| 9 | Mission state-1 wait-for-stop loop | NOT-IMPLEMENTED | No wait state exists in Rust. |
| 10 | Wait-loop cadence (`[Unload] Rate=.016` → `14 + rand(0,2)` frames) | NOT-IMPLEMENTED | No interval to compare; the conversion is immediate. |
| 11 | Per-frame `Commence` promotion site | UNCHECKED | Native site identified (`UnitClass::AI @ 0x00736473`, `0x007366FD`); the Rust promotion path was not traced for per-tick unit coverage. |
| 12 | Where the MCV stops (mechanism) | FAIL | Native finishes the committed track step; Rust deploys at the current mid-step cell. |
| 13 | Literal resting cell for a 1–2 cell short press | UNCHECKED | Requires executing the fixture; track chaining within one process call can extend the committed step. |
| 14 | Sub-cell position at rest | FAIL | Native ends on the track's terminal point series; Rust discards `sub_x/sub_y` and never reaches its own arrival snap. |
| 15 | Body facing at rest / turn to deploy facing | FAIL | Rust snaps `entity.facing = deploy_facing` with zero turn time and demands a second command; native has no facing write in this branch and turns via the locomotor. |
| 16 | Generic Move-arrival same-tick clear + Move-gated queue advance | PASS | `finish_drive_arrival` matches the `0x004B2242..0x004B226D` sequence and the `GetCurrentMission == 2` gate. |
| 17 | NavCom-null arrival variant (the deploy case) | NOT-IMPLEMENTED | Native skips the path-head reset and OnArrival; Rust has no NavCom-null end-of-track path because it never arrives. |
| 18 | `DeployToFire` branch reachability | PASS | Unreachable for stock AMCV in both: key defaults `no` and is unset in `rulesmd.ini`; Rust has no such branch. |
| 19 | Slave-miner / no-`DeploysInto` branch reachability | PASS | Gated on `DeploysInto == 0`; unreachable for AMCV, and Rust routes `Enslaves` units elsewhere first. |

## Top root findings

1. **Critical, every match, every player: the ConYard anchor is wrong whenever D is pressed mid-move.**
   Native nulls the destination, lets the committed cell step finish under a ≤0.3 speed clamp, and
   deploys at that cell. Rust converts inside the command-apply phase at the cell the sprite is
   currently over. One cell of error relocates the whole base.
2. **High, structural: the deploy command bypasses the mission layer.** `Command::DeployMcv` is a
   direct world mutation, not `Queue_Mission(Unload)`. Everything downstream — the stop, the
   `ReadyToCommence` hold, the `Is_Moving` re-check, the `Guard`/`Hunt` fallback on a failed deploy —
   has no place to attach. Rust already owns the three primitives this needs
   (`set_destination_internal_null`, `drive_stop_moving`, `unit_ready_to_commence`); none is wired.
3. **High, correctness of every readiness claim: `mission_ready_state` has no production writer.**
   The exact `ReadyToCommence`/`Is_Moving_Now` port at `readiness.rs`/`locomotor_ready.rs` is dead
   code behind `DEGRADED_NOT_MOVING`. Any parity statement resting on it is currently vacuous.
4. **Doc drift, load-bearing: the MCV deploy FSM is mission 16 (Unload) at UnitClass vtable `+0x23C`,
   base `0x007F5C70`** — not "mission 13 at `+0x238`, base `0x007F5C74`" as
   `MISSION_DEPLOY_BUILDING_0x73D630_STATE_MACHINE_GHIDRA_REPORT.md:15..19,203` states. The old base is
   one slot high, which also mislabels `+0x1E8`/`+0x1EC` as `SetMission`/`QueueMission` when they are
   `Queue_Mission(mission, immediate)` / `Commence()`.
5. **Doc drift: `param_1[0x24]` after `UnitClass::Deploy` is the liveness byte `+0x90`, not a
   "deploy succeeded" flag.** `0x0073DDCB` reads `byte [ESI+0x90]`, the same byte `Mission_Dispatch`
   and `Process_Drive_Track` use as the alive gate. The `Guard`/`Hunt` assignment therefore runs when
   the unit is **still alive after Deploy** — i.e. when the deploy *failed* — which inverts the reading
   in `UNIT_MISSION_DEPLOY_BUILDING_GHIDRA_REPORT.md:249..261`.
6. **Doc drift, minor: `UnitClass__Mission_Deploy @ 0x006AFD60` is not an MCV path.** Its only caller is
   `SlaveManagerClass__AI @ 0x006AF638` (`get_xrefs_to 0x006AFD60`). `MCV_DEPLOY_GHIDRA_REPORT.md`
   presents it as "Path 1: MCV → Construction Yard". Likewise `Force_MCV_Deploy @ 0x004FC060` is a
   rally-point/flag helper called only from `ScenarioClass__Generate_Random_Units`.

## Smallest decisive follow-up

Route `Command::DeployMcv` through the two primitives that already exist instead of converting inline:
on the command, call `navcom::set_destination_internal_null(entity)` (which already performs the
NavCom clear plus the Drive destination drop and 0.3 clamp) and record a pending-deploy intent; then
convert on the first tick the mover has come to rest. That single change fixes stage 12 — the
highest-frequency, highest-blast-radius row — without touching foundation validation, EVA, or the
conversion itself. The natural acceptance test is a headless fixture that orders an AMCV from `(55,50)`
to `(60,50)`, issues `DeployMcv` on the tick it leaves `(57,50)`, and asserts the yard anchor derives
from `(58,50)` rather than `(57,50)`.
