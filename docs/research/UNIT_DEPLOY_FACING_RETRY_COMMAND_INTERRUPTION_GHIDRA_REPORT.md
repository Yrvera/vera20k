# Unit Deploy-Facing Retry Command Interruption — Ghidra Research Report

**Address(es):** `0x0055D360` (`Main_Tick`), `0x00647260` / `0x0064C380` (command-queue processing), `0x00700600` / `0x006FFEC0` (cell/object action classification), `0x004D7D50` / `0x004D74E0` (cell/object clicked-action execution), `0x006FFBE0` (MegaMission producer), `0x004C6860` (`EventClass::BuildMegaMissionEnvelope`), `0x004C6CB0` (`EventClass::Execute`), `0x004DF0E0` (mission-byte assignment), `0x005B3060` (`MissionClass::Mission_Dispatch`), `0x007447A0` (Unit mission-1 thunk), `0x007393C0` (`UnitClass::Deploy`), `0x0073D630` (`UnitClass::Mission_Unload`, generic deploy-building branch), `0x00739EC0` (`UnitClass::PerCellProcess`), `0x004D4DC0` (`FootClass::Mission_Attack`), `0x004D5690` (attack approach/destination helper; current `Greatest_Threat_Scan` label is too broad), `0x004B0500` (`DriveLocomotionClass::Process`), `0x004B0F20` (`DriveLocomotionClass::Process_Drive_Track`), `0x00738970` (`UnitClass::Enter_Idle_Mode`)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** active-retail UnitClass `+0x68C` deploy-facing retry behavior when a player issues Move, Stop, ordinary object Attack, Ctrl force-fire on an object, or Ctrl force-fire on a cell while the unit is turning or carrying the retry through an Attack approach; action-to-MegaMission convergence; command execution order; locomotor callbacks; target loss; conversion/destruction; deterministic RNG effects; and the exact Rust correction seam.
**Non-Scope:** full building placement-validator internals, arbitrary mod-only deployers, `DeployToFire=yes` simple-deployer behavior, full combat/ROF/turret timing during a long approach, pathfinder cell-fan equivalence inside `0x004D5690`, malformed/network-full command rings, and unrelated transport Unload behavior.
**Confidence:** High for stock clear-path Move, Stop-during-facing, accepted ordinary/force-fire Attack convergence, in-range Attack, out-of-range Attack through arrival, command ordering, and `+0x68C` writers. Medium for Stop issued during a blocked far-approach before any Drive track is committed and for exact candidate-cell selection at path/LOS boundaries.
**Active in YR:** Yes. Stock `AMCV`, `SMCV`, `PCV`, and `SMIN` use `DeploysInto=`. Of these, only stock `SMIN` is armed and therefore admits ordinary Attack plus object/cell force-fire during this slice.

## 1. Verdict

`UnitClass+0x68C` is a pending deploy-facing retry/progress byte, not a generic command lock and not the `DeployToFire` INI flag.

The actual `DeployToFire` type flag is UnitType offset `+0xE11`. Stock YR has no active `DeployToFire=` assignment (only a commented example near `rulesmd.ini:3613`), so it is not an alternate explanation for any stock AMCV/SMCV/PCV/SMIN path below.

Retail resolves commands against that byte by normal mission and locomotor order:

1. Object AI runs before the frame's command queue. A command received this frame first affects that object's AI on its next live-object visit.
2. A different-cell **Move** installs NavCom. On the next current-Unload state-2 visit, `UnitClass::Deploy` fails because the locomotor is moving and the deploy-building branch of `Mission_Unload` clears `+0x68C` at `0x0073DD76`. The unit then moves. Move cancels the deploy retry.
3. **Stop** executes immediately, broadcasts radio BREAK, clears destination and target, and normally does not queue or commence a Stop mission. During the original facing turn, current Unload remains authoritative and retries deploy next visit. During an already-running far Attack approach, Drive's Stop operation truncates movement to the committed track head; the eventual `PerCellProcess(2)` retries deploy there.
4. Effective **Attack** is a staged MegaMission. Ordinary object Attack, Ctrl object force-fire, and Ctrl cell force-fire all emit mission 1 with a target token and a null destination token; the event retains object-vs-cell target kind but no command-origin discriminator. When the facing turn finishes, queued Attack is promoted before `FootClass::Mission_Attack`; its approach helper runs before the unconditional `RandomRanged(0,2)` cadence jitter. A fireable in-range target produces no destination, so the rotation-finished `PerCellProcess(0)` deploys the unit before `UnitClass::Fire_At_Target`. An out-of-range/unfireable target produces a destination, so that callback sees movement and leaves `+0x68C` set; the unit approaches and retries deploy from a later cell/arrival callback.

At the investigation cutoff, Rust checkpoint `f5d052a9759edfc8d24c68ff1f9ec319eb53ba37` plus the then-uncommitted mechanism patch modeled Move cancellation and ordinary-Attack ordering, but had introduced `forward_deploy_attack_eligible` as a command-origin discriminator. Active retail has no such discriminator after the MegaMission is emitted. That bit incorrectly excluded object/cell force-fire from the same Attack owner and its authoritative approach/cadence/PerCell sequence. Section 22 records the subsequent builder resolution; the binary verdict itself is unchanged.

## 2. Active Stock Carrier Inventory

| Source unit | Target building | Armed while mobile | Relevant stock facts | Command-interruption relevance |
|---|---|---:|---|---|
| `AMCV` | `GACNST` | No | `DeploysInto=GACNST`, `ROT=5`; target building supplies the deploy-facing rule | Move and Stop are live; ordinary Attack is not |
| `SMCV` | `NACNST` | No | `DeploysInto=NACNST`, `ROT=5`; target building supplies the deploy-facing rule | Move and Stop are live; ordinary Attack is not |
| `PCV` | `YACNST` | No | `DeploysInto=YACNST`, `ROT=5`; target building supplies the deploy-facing rule | Move and Stop are live; ordinary Attack is not |
| `SMIN` | `YAREFN` | Yes | `Primary=20mmRapid`, `ElitePrimary=20mmRapidE`, `Turret=yes`, `OpportunityFire=yes`, `ROT=5`, `StupidHunt=yes`, `DeploysInto=YAREFN`; target `[YAREFN] DeployFacing=0` | Only stock carrier that exercises the Attack split |

Stock evidence: `ini/rulesmd.ini:6969..6977`, `7838..7845`, `8826..8834`, `9042..9105`, and `13234..13299`. The stock SMIN weapon is `[20mmRapid] Damage=30, ROF=20, Range=5.5, Projectile=InvisibleLow, Warhead=HARVWH` at `ini/rulesmd.ini:22966..22974`; elite range is `5.75` at `25199..25207`.

`UnitClass::Deploy` passes the `DeploysInto` target building type to the deploy-facing accessor. For SMIN this is `YAREFN`, whose explicit value is 0. The matching `[SMIN] DeployFacing=0` line is not the source of this conversion gate; it is coincidental for the stock pair.

## 3. Identity and Field Proof

### 3.1 UnitClass and DriveLocomotion vtables

The UnitClass primary vtable is `0x007F5C70`. Its complete-object locator at base-minus-four (`0x007F5C6C`) points to `0x0080CC68`, whose type descriptor is `0x00842D80` and string is `.?AVUnitClass@@`.

Load-bearing Unit slots in this slice:

| Slot | Address | Verified role |
|---:|---:|---|
| `+0x18C` | `0x00739EC0` | `UnitClass::PerCellProcess` |
| `+0x1E8` | `0x005B35E0` | queue/assign pending mission (`commence_now=false` does not immediately promote it) |
| `+0x1EC` | `0x005B3570` | `MissionClass::Commence` / pending-to-current promotion |
| `+0x200` | `0x00744270` | `Is_Ready_To_Commence`; defers promotion while `Is_Moving_Now` |
| `+0x280` | `0x0065ACE0` | `RadioClass::Broadcast_Radio_ToAll` |
| `+0x378` | event producer | MegaMission producer |
| `+0x480` | `0x00741970` | UnitClass destination override ending in `FootClass::Set_Destination_Internal` |
| `+0x484` | `0x00738970` | `UnitClass::Enter_Idle_Mode`, not OnArrival and not ForceScatter |
| `+0x4A4` | `0x004DF0E0` | action assignment from event payload |
| `+0x53C` | `0x004D5690` | attack target/approach-destination decision helper |

The DriveLocomotion vtable is `0x007E7EB0`; its locator is `0x007FFDE8`, whose type descriptor string is `.?AVDriveLocomotionClass@@`. Its load-bearing entries include `Process @ 0x004B0500`, `Is_Moving @ 0x004AFB80`, `Is_Moving_Now @ 0x004AFC20`, `Set_Destination @ 0x004AFD40`, and `Stop_Moving @ 0x004AFE00`.

### 3.2 `+0x68C` instruction census and meaning

The bounded instruction census found 28 references to the byte. The active writers that govern this slice are:

| Site | Write | Meaning |
|---|---|---|
| Unit construction/initialization | clear | starts with no pending retry |
| `UnitClass::Deploy @ 0x00739650` | set to 1 | facing mismatch requested a body turn and conversion remains pending |
| `UnitClass::Deploy @ 0x00739573` | clear | footprint/placement rejection terminates the retry and queues fallback mission work |
| `UnitClass::Deploy @ 0x00739AA7` | clear | late target-building Unlimbo failure terminates retry |
| `Mission_Unload @ 0x0073DD76` | clear | deploy-building state 2 observed a failed deploy while NavCom was non-null; a Move has taken ownership |

No Move, Stop, Attack, Deploy-event, or generic EventClass handler writes `+0x68C` directly. The byte's fate is an effect of later mission/deploy/locomotor execution.

`FootClass::ComputeChecksum @ 0x004DBD1A` reads the byte, so its persistence is lockstep-authoritative even when the immediate screen result is unchanged. This corroborates retaining the field in Rust snapshots/state hash and makes the skipped in-range Attack RNG call a real deterministic divergence rather than cosmetic ordering.

`UnitClass::Enter_Idle_Mode @ 0x00738970` reads this same byte as `char param_1[0x1A3]`. When nonzero, its ordinary no-target/no-destination branch exits without assigning Guard, Hunt, Harvest, or Unload. This is why target loss does not itself erase the pending deploy transaction.

## 4. Frame and Command Order

`Main_Tick @ 0x0055D360` runs LogicClass/object AI before `Process_Command_Queues_For_Frame @ 0x00647260` (call site near `0x0055DE40`). Therefore a command created for frame N is applied after the unit's frame-N AI/locomotor work and is first visible to that unit's AI in frame N+1.

`FUN_0064C380` processes eligible primary-ring records in house/ring order. Non-MegaMission events execute during that scan. Opcode 4 MegaMissions are copied into a 256-record secondary ring and drained FIFO only after the house's primary scan.

Consequences:

- Stop is opcode 6 and executes before same-house Move/Attack MegaMissions in the same command frame, regardless of insertion order.
- Multiple opcode-4 MegaMissions retain their FIFO order; later events overwrite the queued mission/target/destination fields written by earlier ones.
- Command order does not race object AI inside the same frame. If the final facing callback already converted and removed the source during object AI, a command later in that frame cannot rescue or retask it.

Rust mirrors the immediate-then-staged envelope split in `src/sim/world/mod.rs:6190..6237` and applies the drained commands after object logic at `src/sim/world/mod.rs:6392..6402`.

## 5. Baseline Deploy-Facing Retry

`UnitClass::Mission_Unload @ 0x0073D630` owns the generic `DeploysInto=` state 0/1/2 branch (older docs call this whole function `Mission_Deploy_Building`). In state 1 it checks `ILocomotion::Is_Moving`, which observes translation but not a pure FacingClass turn, then calls `UnitClass::Deploy`.

If the body facing does not match the `DeploysInto` target building's facing, `UnitClass::Deploy`:

1. requests the target heading,
2. writes `UnitClass+0x68C = 1`, and
3. returns without allocating the building.

Mission state advances to 2 while the byte is live. State 2 behaves as follows:

```text
if +0x68C == 0:
    Enter_Idle_Mode / commence pending mission
else:
    UnitClass::Deploy()
    if Deploy returned false and FootClass+0x5A4 NavCom != NULL:
        +0x68C = 0                 // 0x0073DD76
```

`UnitClass::PerCellProcess @ 0x00739EC0` independently calls `UnitClass::Deploy` when its argument is 0 or 2 and `+0x68C != 0`. It checks source liveness after the callback because successful conversion consumes the Unit.

Two ordinary Drive callers matter:

- `DriveLocomotionClass::Process @ 0x004B0500`: while the body is rotating, sets Drive byte `+0x5E`; on the first tick no longer rotating, clears that byte and calls `PerCellProcess(0)` before later movement work.
- `DriveLocomotionClass::Process_Drive_Track @ 0x004B0F20`: at a committed cell/terminal track contact, clears the finished Drive destination/head as appropriate and calls `PerCellProcess(2)` before checking whether the Unit survived.

This makes retry ownership event-driven as well as mission-driven. A command can change what the next PerCell callback sees even though it never writes `+0x68C` itself.

## 6. Move During the Facing Retry

An ordinary different-cell Move is a MegaMission action 2. `EventClass::Execute`:

1. assigns action/mission Move,
2. queues it with `commence_now=false`,
3. assigns the event target token (normally null for a cell move), and
4. calls the Unit destination slot with the event's cell token.

The Unit destination override ends in `FootClass::Set_Destination_Internal @ 0x004D94B0`, which writes Foot/NavCom `+0x5A4` and calls the active locomotor's `Set_Destination @ 0x004AFD40`. Drive's primary destination becomes non-null.

On the next Unit AI visit, current mission is still Unload state 2. Its retry calls `UnitClass::Deploy`; the Drive moving predicate rejects conversion. Because NavCom is now non-null, state 2 clears `+0x68C` at `0x0073DD76`. Queued Move can then become authoritative and the source moves.

**Retail verdict:** a normal different-cell Move cancels a pending deploy-facing conversion before the source translates.

At the investigation cutoff Rust did not do this. `retry_forward_deploy_on_unit_ai` returned false while translating, then `unit_techno_bracket` saw the still-live latch and returned before mission promotion/dispatch (`src/sim/world/world_spawn.rs:2192..2228`, `src/sim/world/techno_ai.rs:734..750`). The unit could retain a permanently suppressing latch while its Move path advanced. This cutoff mismatch is resolved in the builder state summarized in Section 22.

Same-cell clicks and mod-only no-path/no-locomotor cases are not interchangeable with this verified path. They may fail to install NavCom and should not be used as the acceptance fixture.

## 7. Stop During the Facing Retry

`StopCommandClass::Execute @ 0x00730EA0` calls the selected object's event-producing slot with opcode 6. `EventClass::Execute @ 0x004C6CB0`, case 6:

1. validates the actor,
2. performs team/path cleanup where applicable,
3. calls `vtable+0x280(3)` — `RadioClass::Broadcast_Radio_ToAll(BREAK)`,
4. calls destination `(NULL, 1)`,
5. clears assigned target,
6. clears spawn/passenger targets where applicable, and
7. only for a Unit type carrying the harvester flag and current mission Harvest (10) or Return (12), queues Guard and commences it immediately.

There is no general native Stop mission assignment. A deploy-facing Unit is on current mission Unload (16), so the harvester exception does not run. Stop also does not write `+0x68C`.

During the initial body turn, next-frame Unload state 2 calls Deploy again with no NavCom. The retry remains authoritative: it either continues turning, converts, or terminates through a real placement/Unlimbo rejection.

Rust currently calls `queue_mission_with_teardown(... MissionType::Stop ...)` at `src/sim/world/world_commands.rs:746`. The navigation and target cleanup is close to native, including retaining only an already-committed Drive/Ship head segment at `:748..805`, but the synthetic queued Stop mission is not native. It can become player-visible after a footprint rejection because native Deploy queues its Guard fallback while Rust may later promote Stop instead.

Rust also does not currently perform the native radio broadcast at this command site. Its teardown path cancels selected reservation bookkeeping (`src/sim/mission/retask.rs:45..66`, `src/sim/world/world_commands.rs:2280..2338`), while the actual all-contact BREAK authority is `src/sim/radio/mod.rs:69..110`. Stop must call that authority rather than treating dock-reservation cleanup as equivalent to a RadioClass broadcast.

## 8. Attack During the Facing Retry

### 8.1 Envelope and promotion

`FootClass::ClickedAction_Object @ 0x004D74E0` emits a MegaMission whose mission byte is `1` (Attack), whose target is the clicked object token, and whose destination token is null. `EventClass::Execute` case 4/5 queues Attack with `commence_now=false`, installs the target, and calls Set_Destination with null. It does not write `+0x68C`. Mission byte `1` must not be confused with a tactical UI action number; several accepted UI Attack actions converge on that same mission byte.

While FacingClass remains rotating, `Is_Ready_To_Commence @ 0x00744270` defers the queued mission. On the first actor visit when rotation is complete, queued Attack is promoted and `FootClass::Mission_Attack @ 0x004D4DC0` runs before Drive's rotation-finished `PerCellProcess(0)` callback.

Mission_Attack calls Unit slot `+0x53C` when a target exists, then unconditionally computes its mission cadence and calls `Random::RandomRanged(0,2)` using the Scenario RNG. The call is unconditional across target/no-target and approach/no-approach branches. `RandomRanged` may use rejection sampling, so the portable claim is one `RandomRanged(0,2)` invocation, not exactly one raw XOR-state advance.

### 8.2 In-range / fireable target

At `0x004D5690`, the initial `vtable+0x3A8` call is `TechnoClass::CanFireAt`. For a fireable target, a playfield-resident non-Aircraft actor (`TechnoClass+0x3D5 != 0`) reaches the early return around `0x004D5A1A` without installing a destination.

For stock SMIN on a clear map with the target safely inside `20mmRapid` range:

1. Attack is promoted.
2. The approach helper leaves NavCom/destination null.
3. Mission_Attack invokes `RandomRanged(0,2)`.
4. Drive reaches its rotation-finished `PerCellProcess(0)` callback.
5. `UnitClass::Deploy` sees a stopped actor and converts it to YAREFN if placement succeeds.
6. `FootClass::AI` / `UnitClass::AI` checks source liveness before the later Unit fire step, so the consumed SMIN does not fire at the clicked target on that successful conversion tick.

The player-visible result may look like the current Rust result, but the deterministic state does not: Rust converts before Attack promotion and therefore skips Attack's RNG call. The six already-pinned YAREFN/slave-manager constructor draws then start from the wrong Scenario RNG state.

### 8.3 Out-of-range / currently unfireable target

If `CanFireAt` is false, `0x004D5690` scans candidate approach cells and may install one through the Unit destination slot. On a clear, non-boundary fixture well outside range:

1. the helper installs NavCom/Drive destination,
2. Mission_Attack invokes `RandomRanged(0,2)`,
3. rotation-finished `PerCellProcess(0)` calls Deploy,
4. Deploy observes `Is_Moving_Now` and returns false, and
5. `+0x68C` stays set because this callback is not Mission-Unload state 2 and has no NavCom-clear tail.

The SMIN begins ordinary Drive movement with Attack current, target retained, and the deploy retry retained. At committed cell contacts, `PerCellProcess(2)` tries Deploy again. At the terminal approach cell, Drive clears its primary destination before the callback. If body facing already matches YAREFN's facing, conversion can occur there. Otherwise Deploy requests another facing turn and a later rotation-finished `PerCellProcess(0)` performs the conversion.

Stock SMIN has a turret and `OpportunityFire=yes`. During a multi-cell approach, normal per-frame combat/ROF/turret gates may permit shots before conversion once the target enters range. The exact number of shots is geometry- and timing-dependent and is not owned by Mission_Attack's cadence body. Do not encode a blanket “far target remains undamaged until deploy” rule.

Exact candidate-cell fan, LOS, path-cost, and range-boundary equivalence inside `0x004D5690` remains outside this slice. Acceptance tests must use a flat clear grid, a comfortably near target and a comfortably far target, and a bearing that isolates the known deploy-facing turn.

### 8.4 Target loss while turning

If the clicked target is detached or destroyed before Attack dispatch, Mission_Attack takes its no-target `+0x484` path. `UnitClass::Enter_Idle_Mode` sees `+0x68C` and refuses to replace the mission. Mission_Attack still invokes `RandomRanged(0,2)`. The subsequent stopped rotation/cell callback remains able to deploy.

Target loss therefore does not cancel the deploy transaction and does not remove Attack's cadence RNG invocation once Attack has been promoted.

### 8.5 Ctrl object/cell force-fire converges before Mission_Attack

The prior revision excluded force-fire and cell Attack because the original trace started from an ordinary object click. That exclusion was not evidence of different native ownership. The bounded extension resolves the actual modifier, action, envelope, event, and dispatch chain:

1. The active tactical modifier globals are `DAT_00A8EC00/04 = Ctrl`, `DAT_00A8EC08/0C = Alt`, and `DAT_00A8EBF8/FC = Shift`. Both `TechnoClass::What_Action_OnObject @ 0x006FFEC0` and the cell classifier at `0x00700600` read those exact groups. Ctrl alone is the force-fire input; the Ctrl+Alt and Ctrl+Shift combinations are separate verbs.
2. Accepted object Attack actions `5`, `0x35`, `0x3F`, `0x40`, and `0x47` share one `FootClass::ClickedAction_Object @ 0x004D74E0` case. After the attack-capability gate at vtable `+0x9C`, the common tail calls vtable `+0x378` with mission `1`, the clicked object as the target, and both remaining target/destination arguments null. Raw jump-table bytes at `0x004D7D04` map action `5` and `0x3F` to the same body at `0x004D769F`; assembly `0x004D76B1..0x004D76C1` pushes `0, 0, clicked-object, 1` before the common call.
3. Ctrl empty-cell force-fire is action `5` on the ordinary armed-unit path. `FootClass::ClickedAction_Cell @ 0x004D7D50` groups actions `5` and `0x3F`. It first accepts a live object found at the cell as an object target; otherwise it converts the clicked `CellClass` into the target token. Both arms call vtable `+0x378` with mission `1` and null destination/auxiliary tokens (`0x004D80CD..0x004D80D8` for the object-at-cell arm; `0x004D80E9..0x004D80FF` for the cell-target arm).
4. The vtable `+0x378` producer at `0x006FFBE0` writes only the mission and target/destination tokens into the opcode-4 MegaMission envelope. `EventClass::BuildMegaMissionEnvelope @ 0x004C6860` stores the mission at `+0x0C`, target token at `+0x0E/+0x12`, and destination token at `+0x13/+0x17`; it has no modifier or ordinary-vs-force origin field.
5. `EventClass::Execute @ 0x004C6CB0`, shared cases 4/5, resolves the actor, validates the optional tokens, obtains the mission through vtable `+0x4A4`, queues it through `+0x1E8(..., false)`, assigns the resolved target through `+0x3C8`, and assigns the resolved destination through `+0x480`. `FootClass::Assign_Target_Command @ 0x004DF0E0` returns the stored mission byte unchanged for mission `1`; only mission `0x1D` has special remapping. No branch can recover whether mission 1 began as ordinary Attack or force-fire.
6. `MissionClass::Mission_Dispatch @ 0x005B3060` dispatches mission `1` only through object vtable `+0x210`. The UnitClass binding at `0x007F5E80` is `0x007447A0`, a single thunk to `FootClass::Mission_Attack @ 0x004D4DC0`. Mission_Attack distinguishes target kind only inside its target/approach work; it still calls slot `+0x53C` for any non-null target and then unconditionally invokes `RandomRanged(0,2)`.
7. `0x004D5690` has an explicit `What_Am_I == 0x0B` cell-target arm. A cell target therefore participates in the same approach-destination production as an object target, subject to cell-specific geometry. Exact boundary candidate selection remains the already-deferred approach-helper scope; the ownership, null-destination input, and cadence call do not.

**Retail verdict:** once an accepted object or cell force-fire has emitted mission 1, the deploy-facing retry must select the same Attack owner as ordinary Attack. Target kind remains authoritative; command provenance does not. A target that later disappears still follows Mission_Attack's no-target arm and cadence draw, so the owner must be derived from the effective queued/current Attack mission, not from the continued presence of a live target and not from a Rust-only origin bit.

The native null destination is independently load-bearing. ForceAttack object/cell must clear the uncommitted owner destination at command execution while preserving only a genuinely committed locomotor head, exactly as the ordinary Attack event path does. Merely clearing Rust's high-level `movement_target` leaves stale NavCom/Drive state and is not equivalent.

## 9. Stop During an Existing Far Attack Approach

Once far Attack has installed a destination and begun a Drive track, Stop clears Foot/NavCom and calls the active locomotor's Stop operation. `DriveLocomotionClass::Stop_Moving @ 0x004AFE00` clears only the primary destination coordinates and clamps speed; it deliberately leaves the committed track index/head-to fields intact.

`DriveLocomotionClass::Process` therefore finishes the already-committed track segment rather than following the discarded full route. `Process_Drive_Track` reaches its normal cell contact and calls `PerCellProcess(2)`, which retries deploy with `+0x68C` still set. Target is null, current Attack may call Enter_Idle_Mode in the meantime, but that selector's `+0x68C` gate refuses an unrelated idle replacement.

**Verified clear-path result:** Stop truncates the far approach to its committed head/cell and the pending deploy retries there; it does not directly cancel the retry and does not synthesize a native Stop mission.

While that retained segment is active, current Attack remains current. If its mission timer is due after target clear, Mission_Attack enters the idle selector, the live `+0x68C` gate refuses an idle replacement, and Mission_Attack still invokes `RandomRanged(0,2)`. The retained movement segment therefore does not suspend the authoritative Attack cadence stream.

**Bounded uncertainty:** if no Drive track/head was ever committed (for example a blocked mod/pathfinder edge), Stop leaves no proven immediate PerCell trigger. The latch is still not cleared, but exact time-to-next retry depends on a later Facing/PerCell event. This edge is not needed for the stock clear-path correction.

Rust's retained-head Stop representation at `src/sim/world/world_commands.rs:748..805` is directionally correct for this locomotor behavior. Its queued `MissionType::Stop` is the separate mismatch.

Rust's Drive stop-speed handling is also incomplete here. `src/sim/movement/navcom.rs:253..273` clears the destination without applying native's `min(old_speed, 0.3)`-style clamp when a head remains, and `src/sim/movement/movement_tick.rs:2014..2032` can recompute an ordinary terrain target on the next Drive tick. The retained segment needs a stored stop-clamped target that survives until that head retires; the analogous Ship seam at `src/sim/movement/drive_locomotion.rs:171..183` is a useful architecture reference, not binary evidence for Drive.

## 10. Same-Frame Command Combinations

| Commands submitted for one actor in one frame | Native execution order | Result relevant to `+0x68C` |
|---|---|---|
| Stop + Move | Stop immediate, then staged Move | final NavCom is Move's; next current-Unload state-2 retry clears the latch |
| Stop + Attack | Stop immediate, then staged Attack | final target/queued mission is Attack; near/far geometry split occurs after promotion |
| Move + Attack | both staged FIFO | later Attack clears/replaces destination according to its null event token; Attack helper decides near/far next visit |
| Attack + Move | both staged FIFO | later Move clears target and installs NavCom; next current-Unload state-2 retry clears latch |
| Attack A + Attack B | both staged FIFO | B is the final target; only the effective Attack dispatch consumes cadence RNG |

Exceptional full secondary rings, malformed records, and network delay can defer execution. They do not change the ordinary FIFO semantics and are not implementation authority for stock local/skirmish tests.

## 11. Source Destruction and Failed Conversion

Successful `UnitClass::Deploy` removes/UnInits the source. `ObjectClass::UnInit @ 0x005F65F0` clears the alive state and schedules/delegates final removal; it does not need to clear `+0x68C` because the owning Unit no longer participates in AI or serialization as a live source.

If the source is killed by another mechanism before the callback, `MissionClass::Mission_Dispatch`, Foot AI, Unit AI, and locomotor callback liveness gates prevent a dead Unit from continuing the retry.

If placement fails after facing is correct, `UnitClass::Deploy` clears `+0x68C` and queues its fallback mission. If target-building Unlimbo fails, it also clears the byte. These are transaction failures, not command cancellation. Rust must preserve the native fallback mission's authority; a synthetic queued Stop must not overwrite it later.

## 12. Rust Delta at the Investigation Cutoff (`f5d052a` plus the then-uncommitted patch)

### Correct at the investigation cutoff

- `forward_deploy_retry` exists in `GameEntity`, defaults false, participates in snapshot round-trip and deterministic state hash.
- The first facing mismatch sets the latch; successful conversion and rejection clear it.
- A translating entity does not deploy.
- Rust's command phase is after object logic and its immediate/staged event ordering mirrors the relevant native split.
- Different-cell Move now reaches the Unit-AI state-2-equivalent retry/clear seam.
- Ordinary object Attack now promotes before its approach/cadence work, preserves the retry through a far approach, and retries after the owner-local locomotor bracket.
- Stop retains only a genuinely committed Drive/Ship head segment while clearing the longer route; an eagerly prepared but unprocessed Drive curve is discarded rather than mistaken for a committed head.
- Newly promoted far Attack revalidates deploy placement before committing its first approach segment.

### Blocking mismatch at the investigation cutoff

The cutoff patch introduced `forward_deploy_attack_eligible` as an ordinary-object-Attack command-origin carrier. `Command::ForceAttack` and `Command::ForceAttackCell` explicitly set it false, and `ForwardDeployAiOwner` required it before selecting Attack. The active MegaMission/Event/Mission chain has no such origin bit. This made accepted object/cell force-fire skip the verified Attack approach, cadence, and owner-local PerCell sequence even though the effective mission was Attack. The carrier was also serialized and hashed, turning the unsupported distinction into authoritative state. Section 22 records its removal and the replacement production proof.

## 13. Required Rust Correction Shape

The correction must preserve per-object order, not merely add command-specific latch clears at command tail.

1. **Move:** when a latch-owning Unit is still in the current Unload retry state and a real navigation destination exists, model the state-2 Deploy attempt and clear the latch before Move becomes authoritative. A direct command-tail clear would be one frame early relative to native and would skip the final failed Deploy attempt; prefer the Unit-AI state seam.
2. **Attack promotion:** allow a latch-owning Unit with effective queued/current Attack to pass Ready/Commence instead of returning at the blanket latch gate.
3. **Attack approach before cadence:** factor or expose one-object pursuit/approach resolution so it runs after promotion and before `dispatch_supported_foot_mission_cadence`. Preserve the native order `approach decision -> RandomRanged`.
4. **Post-locomotor retry:** invoke the pending deploy retry after that object's movement/turn processing, matching `PerCellProcess(0/2)`. If a destination was installed and movement remains active, retain the latch. If stopped, convert/reject there and propagate source consumption plus `spawned_entities` accounting.
5. **Enter-idle gate:** pass `forward_deploy_retry` into mission-handler evaluation and make the Attack no-target idle selector decline replacement while it is set. This permits the due Attack RNG call without inventing Guard.
6. **Stop:** remove the general queued Stop mission for ordinary Units or otherwise prevent it from becoming a mission authority. Call the all-contact BREAK authority, retain navigation/target teardown and committed-head truncation, and carry native's stop-speed clamp through that final segment. Keep the separately verified Harvest/Return-to-Guard exception.
7. **No global preemption:** do not globally move all pursuit before all object movement. Native ownership is per object, and unrelated object RNG/order must remain stable.
8. **Force-fire convergence:** select the Attack owner from the effective queued/current Attack mission while `forward_deploy_retry` is live, not from command provenance or continued target liveness. Route object and cell force-fire through the same null-destination teardown used by ordinary Attack, preserving only a genuinely committed locomotor head. Retain `TargetKind::Entity` versus `TargetKind::Cell`; remove the synthetic command-origin authority from snapshot/hash state.

The exact helper factoring is a Rust architecture choice, but these observable orders are mandatory.

## 14. Acceptance Tests

| Test | Fixture | Required assertions |
|---|---|---|
| `forward_deploy_retry_move_cancels_on_next_unit_ai_before_translation` | mis-faced AMCV/SMIN with live latch; issue different-cell Move after AI | next actor visit performs failed retry, clears latch, preserves queued Move/nav; no building; later movement proceeds |
| `forward_deploy_retry_stop_keeps_unload_and_deploys_after_turn` | mis-faced AMCV/SMIN; issue Stop during initial facing | radio/nav/target cleanup; no native Stop mission authority; latch remains until turn callback; target building created or real placement rejection clears it |
| `forward_deploy_retry_attack_in_range_draws_jitter_before_conversion` | armed SMIN; clear flat grid; target comfortably within range on controlled bearing | Attack becomes current; one `RandomRanged(0,2)` call occurs before the six known constructor draws; source consumed; one YAREFN; clicked target receives no shot from consumed source |
| `forward_deploy_retry_attack_out_of_range_approaches_with_latch` | identical setup except target comfortably beyond range | after first Attack visit: current Attack, target retained, movement/NavCom live, source exists, no YAREFN, latch true; cadence RNG invoked; next still-moving visit still does not convert |
| `forward_deploy_retry_attack_converts_on_clear_path_arrival` | continue prior far fixture until terminal approach cell | primary destination clears before retry; source converts immediately if facing matches, otherwise latch requests final turn and converts on its completion |
| `forward_deploy_retry_force_attack_object_uses_attack_owner_and_rng` | mis-faced armed SMIN; Ctrl object force-fire on a comfortably near target | queued/current mission is Attack; no origin discriminator; object target retained; one `RandomRanged(0,2)` precedes conversion and constructor draws; source does not fire after consumption |
| `forward_deploy_retry_force_attack_cell_uses_attack_owner_and_rng` | same fixture with Ctrl empty-cell force-fire comfortably near | cell target retained; same Attack owner/cadence; null destination teardown; conversion occurs after cadence rather than through DeployBuilding preemption |
| `forward_deploy_retry_force_attack_object_approaches_with_latch` | Ctrl object force-fire comfortably beyond range | first Attack visit installs movement/NavCom, retains object target and retry, and does not create YAREFN; terminal approach/cell callback owns conversion |
| `forward_deploy_retry_force_attack_cell_approaches_with_latch` | Ctrl cell force-fire comfortably beyond range | identical owner/order with `TargetKind::Cell`; approach destination replaces the null command destination; conversion occurs at the approached cell, not the original cell |
| `forward_deploy_retry_force_attack_clears_uncommitted_destination` | same-frame Move then object/cell force-fire before Drive consumes the eager curve | stale Move destination/route/reservation is discarded; no false committed-head retention; Attack helper alone decides the next destination |
| `forward_deploy_retry_target_loss_still_draws_attack_jitter_and_deploys` | target detached after Attack queue but before promotion | target null; Attack cadence RNG still invoked; EnterIdle does not overwrite while latch set; stopped callback deploys |
| `forward_deploy_retry_stop_during_attack_finishes_committed_head_then_deploys` | far Attack with committed Drive head; issue Stop | full route/target cleared; only head segment retained; latch true through segment; deploy retry at cell contact; no synthetic Stop mission wins |
| `forward_deploy_retry_footprint_rejection_clears_latch_and_keeps_native_guard` | block target footprint at final retry | no conversion; latch false; Guard fallback remains authoritative even if Stop was issued earlier |
| `forward_deploy_retry_same_frame_stop_attack_uses_immediate_then_staged_order` | submit both same frame | Stop teardown occurs first; Attack is final queued mission/target; near/far branch follows geometry |
| `forward_deploy_retry_snapshot_and_hash_cover_command_interruption_state` | snapshot during Move cancellation and far Attack approach | round-trip retains latch, mission, target/nav/track and reproduces next hash/RNG outcome |

For RNG assertions, compare Scenario RNG state against a reference that invokes `RandomRanged(0,2)` followed by the six pinned constructor draws. Do not assert a fixed count of raw PRNG core advances because ranged rejection sampling is allowed.

The Stop-during-approach fixture must additionally assert all staged radio contacts receive BREAK, current Attack remains current with queued mission NONE, the stored Drive target is clamped to at most native `0.3`, due Attack dispatch still consumes jitter while the head is live, and conversion occurs on the exact tick the retained head retires rather than one frame later.

## 15. Adversarial Review

1. **Could Stop secretly clear `+0x68C` through the event handler?** No. Case 6 has no write; the bounded byte-reference census locates command-independent writers only.
2. **Could ordinary Attack carry a cell destination in its event envelope?** No. `ClickedAction_Object` passes clicked object as target and zero as the destination token; Event Execute then applies those distinct tokens.
3. **Could in-range Attack fire before deploy?** No on the successful same-tick conversion path. Mission_Attack does not itself fire; Drive's PerCell callback consumes the source, and Unit AI checks liveness before `Fire_At_Target`.
4. **Could far Attack clear the latch merely because NavCom exists?** Not from `PerCellProcess`. The NavCom clear is specifically in Mission-Unload state 2; once Attack is current, locomotor callbacks call Deploy without that tail.
5. **Could Stop overwrite Attack because it was clicked later?** Not within one command frame. Opcode 6 drains immediately before staged opcode 4. A later-frame Stop can clear an already-running Attack target/path, but does not assign a general native mission.
6. **Could a destroyed target cancel the deploy retry?** No direct path. EnterIdle refuses ordinary reassignment while `+0x68C` is set; the stopped PerCell callback can still deploy.
7. **Could successful conversion still allow an SMIN shot?** No. The source liveness guard lies between locomotor processing and the Unit fire step.
8. **Could all stock MCVs exercise Attack?** No. The active stock AMCV/SMCV/PCV definitions have no mobile weapon. SMIN is the stock Attack carrier.
9. **Could source `[SMIN] DeployFacing=0` be the gate?** No. Deploy passes target `YAREFN`; its explicit `DeployFacing=0` happens to match.
10. **Could the current Rust result be accepted because the screen looks right for near Attack?** No. It skips an authoritative Scenario RNG call, shifting all later lockstep randomness.
11. **Could global pursuit be moved earlier wholesale?** That risks reordering unrelated objects. Native performs mission, RNG, and locomotion in the object's own AI bracket.
12. **Could a blocked/path-boundary far target be used to prove the clear-path rule?** No. The helper's candidate-cell fan is richer than Rust's range-only pursuit; use comfortably separated clear fixtures.
13. **Could Ctrl force-fire remain distinguishable after event creation?** No. The MegaMission envelope stores mission and target/destination tokens, not Ctrl state or command provenance.
14. **Could cell force-fire use a different mission owner because its target is a cell?** No. It emits mission `1`; target kind changes the approach helper's geometry arm, not Mission_Dispatch ownership or the unconditional cadence call.
15. **Could a missing/detached force-fire target justify dropping Attack ownership?** No. Mission_Attack's no-target arm still reaches the cadence draw, and the live deploy retry prevents ordinary idle replacement.
16. **Could Rust retain the origin bit only for serialization compatibility?** Not in the unshipped v116 mechanism patch: the bit asserts a native distinction that is absent and changes the authoritative hash. Preserve the real retry state, not unsupported provenance.
17. **Could clearing only `movement_target` satisfy the null destination token?** No. Rust can still carry NavCom, eager Drive curve/head state, and reservations. The native destination setter reaches the locomotor destination authority.
18. **Could every prepared Drive curve be preserved as a committed head?** No. Only a track already consumed/committed by locomotor processing survives Stop/null-destination teardown; a same-frame eager curve is still uncommitted command state.

## 16. Cold Spot Checks and Zero-Add Pass

Independent cold reads performed after the main trace:

1. Re-decompiled the deploy-building state 2 in `Mission_Unload @ 0x0073D630` and re-confirmed the `+0x68C` clear is conditional on failed Deploy plus non-null NavCom at `0x0073DD76`.
2. Re-decompiled `UnitClass::PerCellProcess @ 0x00739EC0` and `DriveLocomotionClass::Process @ 0x004B0500`; re-confirmed arguments 0 and 2 trigger retry, rotation completion invokes argument 0, and source liveness is checked after callback.
3. Re-decompiled `EventClass::Execute @ 0x004C6CB0` case 6; re-confirmed BREAK, null destination/target, no `+0x68C`, and only the Harvest/Return Unit exception queues+commences Guard.
4. Re-decompiled `FootClass::ClickedAction_Object @ 0x004D74E0` plus Event Execute case 4/5; re-confirmed ordinary object Attack has target token plus null destination token.
5. Re-decompiled `FootClass::Mission_Attack @ 0x004D4DC0` and the `0x004D5690` helper; re-confirmed approach precedes the unconditional ranged RNG call and the fireable/playfield/non-Aircraft no-destination return.
6. Re-decompiled `DriveLocomotionClass::Stop_Moving @ 0x004AFE00` and `Process @ 0x004B0500`; re-confirmed Stop clears primary destination but preserves committed track/head state.
7. Re-decompiled `UnitClass::Enter_Idle_Mode @ 0x00738970`; re-confirmed the live `+0x68C` gate and corrected vtable identity.
8. Re-decompiled `DriveLocomotionClass::Process_Drive_Track @ 0x004B0F20` after a conflicting doc scan. At terminal track contact it clears track/head state, conditionally clears the primary destination when the NavCom cell is reached, then calls owner vtable `+0x18C(2)` before the later, separate `+0x504` call. The `+0x18C` callback is the load-bearing deploy retry.
9. Re-decompiled both tactical action classifiers (`0x006FFEC0`, `0x00700600`) and re-confirmed their Ctrl/Alt/Shift global reads plus accepted Attack action returns; no classifier-origin state is passed into the mission layer.
10. Re-read `FootClass::ClickedAction_Object @ 0x004D74E0` and `FootClass::ClickedAction_Cell @ 0x004D7D50`, including their raw switch mappings and call-site assembly; object action `5`/`0x3F` and cell action `5`/`0x3F` converge on mission `1`, target token, and null destination/auxiliary tokens.
11. Re-decompiled the active MegaMission producer `0x006FFBE0`, envelope builder `0x004C6860`, Event Execute cases 4/5, `FootClass::Assign_Target_Command @ 0x004DF0E0`, Mission Dispatch, and the Unit mission-1 thunk. No modifier/origin field or recovery branch was found between accepted click and `Mission_Attack`.
12. Re-decompiled `0x004D5690` specifically for cell targets and re-confirmed the `What_Am_I == 0x0B` arm. Cell-target geometry is variant-specific, but Attack ownership and cadence are shared.

Zero-add pass: no additional active-retail command writer, `+0x68C` clear, Attack envelope destination, force-fire origin discriminator, or pre-fire path was found. A final independent reread of the two action classifiers and the entire envelope-to-dispatch chain added no question. Newly observed malformed/full-ring/network deferrals, mod-only ROT=0 behavior, and exact attack approach-cell fan are recorded as deferred rather than promoted into this transaction.

## 17. Coverage Ledger

| Area / branch | Status | Evidence | Remaining uncertainty |
|---|---|---|---|
| Active stock DeploysInto carrier set | verified | stock `rulesmd.ini`; target-building facing reports | none for stock list |
| Unit/Drive vtable identities | verified | COL/type descriptors and raw slot pointers | none |
| `+0x68C` active writers | verified | bounded 28-reference instruction census; Deploy/Mission state sites | constructor exact symbolic field name remains annotation-only |
| Object AI before commands | verified | `Main_Tick @ 0x0055D360`; command call near `0x0055DE40` | network latency outside local execution |
| Immediate Stop vs staged MegaMission order | verified | `FUN_0064C380`; primary and secondary ring drains | full-ring exceptional drop/defer out of scope |
| Move cancellation | verified | Event case 4/5; destination setters; Mission-Unload state 2 | same-cell/no-path edge deferred |
| Stop during initial facing | verified | Event case 6; current Unload; no direct byte write | none for clear stock path |
| Stop during active far approach | verified for committed track | Drive Stop/Process/Process_Drive_Track | blocked/no-committed-track timing remains Medium |
| Stop all-contact radio and speed clamp | verified native; Rust mismatch located | Event case 6; `0x0065ACE0`; `0x004AFE00`; Rust radio/navcom/movement source | focused implementation/tests pending |
| Ordinary object Attack envelope | verified | `0x004D74E0`; `0x006FFBE0`; `0x004C6860`; Event Execute | none for mission/target/destination fields |
| Ctrl object force-fire convergence | verified | classifiers `0x006FFEC0`; ClickedAction object common switch body; envelope/event/dispatch chain | none for accepted stock SMIN path |
| Ctrl cell force-fire convergence | verified | classifier `0x00700600`; `0x004D7D50`; cell/object-at-cell arms; envelope/event/dispatch chain | exact approach candidate-cell fan remains deferred |
| Command-origin absence after envelope creation | verified negative | envelope layout, Event Execute, Assign_Target_Command, Mission Dispatch | no ordinary-vs-force discriminator exists on the effective Attack path |
| In-range Attack no-nav path | verified | `0x004D5690` CanFireAt + playfield early return | LOS/boundary fixture selection |
| Out-of-range Attack nav path | verified for clear ordinary path | `0x004D5690`; destination setters; PerCell callbacks | exact candidate-cell fan/path cost out of scope |
| Attack RNG order | verified | Mission_Attack decompile | raw RNG advance count intentionally not claimed |
| Fire-after-conversion suppression | verified | Foot/Unit AI liveness guards | normal shots during long approach remain combat-owned |
| Target loss | verified | Mission_Attack no-target arm; EnterIdle `+0x68C` gate | none for promoted Attack |
| Source death/conversion | verified | Object UnInit and AI/callback liveness guards | deferred removal internals out of scope |
| Rust snapshot/hash latch coverage | verified | `game_entity.rs`, `snapshot.rs` tests, `state_hash.rs` | paired command-state fixture still required |
| Rust correction architecture | handoff-ready | current source scan plus binary order | implementation and focused test pending |

## 18. Open Questions — Final Investigation Log

- `[RESOLVED] OQ-01 — Is Unit+0x68C DeployToFire? -> No. It is the pending facing/conversion retry byte set by UnitClass::Deploy; command events do not write it.` Evidence: writer census and `0x007393C0`.
- `[RESOLVED] OQ-02 — Which type supplies DeployFacing? -> The DeploysInto target BuildingTypeClass.` Evidence: `UnitClass::Deploy`; `AMCV_DEPLOY_FACING_RULE_GHIDRA_REPORT.md`.
- `[RESOLVED] OQ-03 — Which active stock carrier can Attack? -> SMIN only.` Evidence: stock AMCV/SMCV/PCV/SMIN INI sections.
- `[RESOLVED] OQ-04 — Do commands execute before the same frame's object AI? -> No, object AI precedes command-queue processing.` Evidence: `Main_Tick @ 0x0055D360`.
- `[RESOLVED] OQ-05 — Does a different-cell Move directly clear the byte? -> No; it installs NavCom, then current Unload state 2 clears the byte after a failed Deploy next visit.` Evidence: Event Execute; `0x0073DD76`.
- `[RESOLVED] OQ-06 — Does Stop queue a general native Stop mission? -> No. Only the Harvest/Return Unit exception queues and commences Guard.` Evidence: Event Execute case 6.
- `[RESOLVED] OQ-07 — Does Stop clear the byte? -> No.` Evidence: case 6 plus writer census.
- `[RESOLVED] OQ-08 — What is Unit vtable +0x484? -> Enter_Idle_Mode, not ForceScatter/OnArrival.` Evidence: Unit vtable bytes and `0x00738970` behavior.
- `[RESOLVED] OQ-09 — Does target loss replace Attack with Guard while the byte is live? -> No; EnterIdle exits without ordinary reassignment.` Evidence: `0x00738970`.
- `[RESOLVED] OQ-10 — Does ordinary Attack carry a destination token? -> No; it carries the clicked target and a null destination.` Evidence: `0x004D74E0`; Event Execute case 4/5.
- `[RESOLVED] OQ-11 — Is Attack cadence RNG before or after approach selection? -> After.` Evidence: `0x004D4DC0` call order.
- `[RESOLVED] OQ-12 — Does a fireable in-range target install NavCom? -> Not for a playfield-resident non-Aircraft stock SMIN.` Evidence: `0x004D5690` early return near `0x004D5A1A`.
- `[RESOLVED] OQ-13 — What retries deploy after facing finishes? -> Drive Process calls PerCellProcess(0).` Evidence: `0x004B0500`, `0x00739EC0`.
- `[RESOLVED] OQ-14 — What retries deploy after a far approach cell/arrival? -> Process_Drive_Track calls PerCellProcess(2), with terminal primary destination cleared before callback.` Evidence: `0x004B0F20`.
- `[RESOLVED] OQ-15 — Can successful near conversion also fire? -> No; source liveness is tested before Unit fire.` Evidence: `0x004DA530`, `0x00736400`.
- `[RESOLVED] OQ-16 — Does far Attack preserve the retry? -> Yes; PerCell has no Mission-Unload NavCom-clear tail.` Evidence: `0x00739EC0` vs `0x0073DD76`.
- `[RESOLVED] OQ-17 — Can far-approaching SMIN shoot before conversion? -> Potentially yes through normal turret/opportunity/combat gates after it enters range; Mission_Attack does not itself fire.` Evidence: stock INI plus AI/fire order.
- `[RESOLVED] OQ-18 — What does Stop do to an active Drive track? -> Clears primary destination and retains the committed track/head so the current segment can complete.` Evidence: `0x004AFE00`, `0x004B0500`.
- `[RESOLVED] OQ-19 — Which same-frame command class wins ordering? -> Immediate opcode 6 Stop runs before staged opcode 4; multiple opcode 4 events are FIFO and later field writes win.` Evidence: `0x0064C380`.
- `[RESOLVED] OQ-20 — Does source destruction need an explicit byte clear? -> No; the dead/consumed Unit exits all later AI/callback paths.` Evidence: `0x005F65F0` and liveness guards.
- `[RESOLVED] OQ-21 — Is current Rust latch persistence serialized and hashed? -> Yes.` Evidence: `game_entity.rs`, snapshot round-trip tests, and state hash inputs in checkpoint f5.
- `[RESOLVED] OQ-22 — Does terminal Drive arrival call PerCell through +0x504 or +0x18C? -> It calls +0x18C(2) first; +0x504 is a later separate callback.` Evidence: fresh `0x004B0F20` decompile.
- `[DEFERRED] OQ-23 — Exact attack approach candidate-cell fan and path-cost parity.` Category: out-of-scope; next step: dedicated `0x004D5690` implementation contract if boundary/LOS fixtures are required.
- `[DEFERRED] OQ-24 — Stop after a far Attack that never committed any Drive head.` Category: blocked/mod edge; next step: fixture plus focused Drive path-failure trace. It does not block clear-path stock parity.
- `[DEFERRED] OQ-25 — Mod-only ROT=0/no FacingClass progress semantics.` Category: mod-only; next step: separate FacingClass zero-rate investigation.
- `[DEFERRED] OQ-26 — Full/malformed/network command-ring behavior.` Category: exceptional networking; next step: EventClass ring-specific slice, not deploy behavior.
- `[DEFERRED] OQ-27 — Exact number of shots during a long SMIN approach.` Category: owned by combat/ROF/turret timing; next step: trace a fixed geometry if a player-visible acceptance needs it.
- `[DEFERRED] OQ-28 — SlaveManager state-2 scheduler interaction with a manually created Unit +0x68C retry.` Category: adjacent owner; existing docs show it waits on Attack+NavCom and later calls Deploy, but direct PerCell/Unit mission evidence already owns the clear-path result. Next step: scheduler trace only if duplicate-attempt timing becomes observable.
- `[RESOLVED] OQ-29 — Which active globals are Ctrl, Alt, and Shift? -> Ctrl is DAT_00A8EC00/04, Alt is DAT_00A8EC08/0C, Shift is DAT_00A8EBF8/FC.` Evidence: both active action classifiers plus `DRIVE_QUEUED_CLICK_EVENT_PLANNING_MODE_OUTCOME_RESWARM_20260528.md`.
- `[RESOLVED] OQ-30 — Does Ctrl object force-fire reach the object Attack branch? -> Yes. Accepted attack actions share the same common switch body after the weapon/action gate.` Evidence: `0x006FFEC0`, `0x004D74E0`, raw table `0x004D7D04`.
- `[RESOLVED] OQ-31 — Do ordinary and force-fire object actions emit different missions? -> No. The common body pushes mission 1, the clicked object, null destination, and null auxiliary target.` Evidence: assembly `0x004D76B1..0x004D76C1`.
- `[RESOLVED] OQ-32 — Does Ctrl empty-cell force-fire emit Attack? -> Yes. Accepted cell actions 5/0x3F emit mission 1.` Evidence: `0x00700600`, `0x004D7D50`.
- `[RESOLVED] OQ-33 — What target kind is retained for force-fire on a cell? -> A live object found at the cell becomes an object token; otherwise the CellClass token remains a cell target.` Evidence: `0x004D80CD..0x004D80FF`.
- `[RESOLVED] OQ-34 — Do force-fire variants carry a command destination? -> No. Both object and cell common tails pass a null destination token.` Evidence: ClickedAction call-site assembly.
- `[RESOLVED] OQ-35 — Does the MegaMission envelope retain Ctrl/origin state? -> No. It stores mission at +0x0C, target at +0x0E/+0x12, and destination at +0x13/+0x17; no modifier/provenance field exists.` Evidence: `0x006FFBE0`, `0x004C6860`.
- `[RESOLVED] OQ-36 — Can Event Execute recover ordinary-vs-force origin? -> No. Shared cases 4/5 consume only the actor, mission, and resolved target/destination tokens.` Evidence: `0x004C6CB0`.
- `[RESOLVED] OQ-37 — Does target assignment remap force-fire mission 1? -> No. Mission byte 1 returns unchanged; only 0x1D has special remapping.` Evidence: `0x004DF0E0`.
- `[RESOLVED] OQ-38 — Which Mission Dispatch case owns the effective mission? -> Mission 1 dispatches through vtable +0x210.` Evidence: `0x005B3060`.
- `[RESOLVED] OQ-39 — What is Unit's mission-1 binding? -> Unit vtable +0x210 points to 0x007447A0, a thunk to FootClass::Mission_Attack.` Evidence: COL/vtable base `0x007F5C70`, slot pointer at `0x007F5E80`, thunk body.
- `[RESOLVED] OQ-40 — Does force-fire skip Attack cadence? -> No. Once mission 1 dispatches, Mission_Attack unconditionally invokes RandomRanged(0,2), independent of target kind and target presence.` Evidence: `0x004D4DC0`.
- `[RESOLVED] OQ-41 — Can a cell target participate in approach selection? -> Yes. The helper has an explicit What_Am_I==0x0B cell arm.` Evidence: `0x004D5690`.
- `[RESOLVED] OQ-42 — Must owner selection require a still-live target? -> No. The no-target Mission_Attack arm still reaches cadence and the deploy retry gates idle replacement.` Evidence: `0x004D4DC0`, `0x00738970`.
- `[RESOLVED] OQ-43 — Does Rust production input actually reach distinct ForceAttack commands for stock SMIN? -> Yes after the builder correction. Object force-fire already emitted Command::ForceAttack. The cell branch previously emitted Command::ForceAttackCell only when `unit_armed && !is_harvester`; because stock SMIN owns a live Miner component, its Ctrl empty-cell click became Move and bypassed this mechanism. The same branch also treated the literal null spelling `Primary=none` as armed, which sent stock CMIN to ForceAttackCell instead of its native-shaped Move fallback. The corrected production branch calls `cell_force_fire_payload`, whose only admission distinction is the verified real-weapon-reference test (`none`, `<none>`, and empty are null), and its stock-shaped regression feeds SMIN's exact payload through `Simulation::apply_command` into the live deploy-retry Attack owner while proving CMIN retains Move. The cursor's force-fire reticle gate now calls the same predicate, so presentation and click delivery cannot disagree on these stock types.` Evidence: `0x00700600`, `0x004D7D50`, `src/app/input/context_order.rs`, `src/app/input/cursor.rs`, `src/sim/world/world_commands.rs`, `src/sim/combat/mod.rs`.
- `[RESOLVED] OQ-44 — Is Rust's forward_deploy_attack_eligible native state? -> No. It is a Rust-only command-origin carrier, explicitly false for force-fire and serialized/hashed despite the native convergence.` Evidence: `game_entity.rs`, `world_commands.rs`, `techno_ai.rs`, `snapshot.rs`, `world_hash.rs` versus the verified envelope/dispatch chain.

## 19. Stale-Doc Corrections

The following existing claims must not be used as implementation authority without correction:

1. `docs/research/UNIT_MISSION_DEPLOY_BUILDING_GHIDRA_REPORT.md` calls Unit `+0x68C` **DeployToFire**, calls Foot `+0x5A4` **IsSlaveMiner**, and calls `+0x484` **ForceScatter**. For this verified path those are wrong: `+0x68C` is pending deploy-facing retry/progress, `+0x5A4` is NavCom, and Unit `+0x484` is Enter_Idle_Mode.
2. `docs/research/TECHNO_VTABLE_0x484_DRIVE_PROCESS_ARRIVAL_GHIDRA_REPORT.md` frames the slot as a post-arrival hook and names `0x00738970` Scatter_Force. Live vtable identity plus the no-target Mission_Attack caller prove Enter_Idle_Mode.
3. `docs/research/FOOTCLASS_MISSION_ATTACK_GHIDRA_REPORT.md` labels slot `+0x3A8` GetFireError and overgeneralizes the NavCom-null path. The slot is CanFireAt; a successful CanFireAt plus playfield membership/non-Aircraft gate returns without destination, while false enters approach selection.
4. `docs/research/units/allied/AMCV.md` says the source type supplies deploy facing. The `DeploysInto` target building type supplies it.
5. Older `MCV_DEPLOY_GHIDRA_REPORT.md`/arrival prose using Unit `+0x484` as cancel-deploy/arrival/scatter terminology must be read through the corrected slot identity above.
6. `docs/research/SLAVE_MANAGER_STATE_MACHINE_GHIDRA_REPORT.md` calls Foot `+0x5A4` TargetObject. It is NavCom. Its state-2 `Mission==Attack && NavCom!=NULL` wait can reinforce the far-approach result, but it is not evidence that `+0x5A4` is TarCom.
7. `docs/research/DRIVELOCOMOTOR_ACCEPTED_CELL_ARRIVAL_VISIBILITY_GHIDRA_REPORT.md` / `DRIVELOCOMOTION_ARRIVAL_QUEUE_NULL_DESTINATION_GHIDRA_REPORT.md` prose that attributes the relevant PerCell callback solely to `+0x504` is incomplete for this slice. Fresh `0x004B0F20` decompile shows `+0x18C(2)` occurs first and is the retry owner.
8. `docs/research/units/AUDIT_INDEX.md` groups SMIN with weaponless `StupidHunt` units. Stock SMIN has `Primary=20mmRapid`; `StupidHunt` is a Hunt fallback fact, not Attack rejection.
9. Older `OPPORTUNITY_FIRE_GHIDRA_REPORT.md` wording that says OpportunityFire does not gate scanning conflicts with the newer consumer report for passive Move/Guard acquisition. That conflict is non-load-bearing here because an explicit Attack event already installs the target; do not use `OpportunityFire=yes` as the cause of explicit Attack admission or firing.
10. This report's earlier revision said force-fire/cell Attack was outside scope. That was a scope boundary, not evidence of different ownership. The verified replacement is: accepted ordinary object Attack, Ctrl object force-fire, and Ctrl cell force-fire converge on mission 1 with target kind retained, null destination, and no command-origin discriminator. Rust comments/tests that cite the old exclusion are stale and must be removed with `forward_deploy_attack_eligible`.

No other existing doc was edited in this research-only slice; these corrections are recorded here for a later bounded audit pass.

## 20. Ghidra Annotation Candidates

No Ghidra mutation was authorized or applied.

- Field candidate: `UnitClass+0x68C` -> `PendingDeployFacingRetry` or `DeployConversionPending` (High confidence).
- Function-label candidate: `0x004D5690` should not remain only `Greatest_Threat_Scan`; its verified Attack use includes CanFireAt evaluation and approach-destination selection. A cautious label such as `FootClass__Select_Attack_Approach_Destination` is Medium confidence because the function also serves other mission modes.
- `0x00738970` is already correctly documented in the live instance as `UnitClass::Enter_Idle_Mode`; do not restore older OnArrival/Scatter labels.

## 21. Implementation Handoff

This table is the research-to-builder handoff captured at the investigation cutoff. Its mismatch column is intentionally historical; Section 22 records what the builder subsequently changed and validated.

| Verified behavior | Binary/INI evidence | Rust mismatch at investigation cutoff | Required effect | Acceptance |
|---|---|---|---|---|
| Move cancels pending deploy only after next state-2 failed retry sees NavCom | `0x004C6CB0`, `0x004D94B0`, `0x0073DD76` | resolved in dirty patch | retain the state-2-equivalent clear at Unit-AI timing | different-cell Move fixture clears before translation and never builds |
| Stop during facing retains current Unload and latch | Event case 6; no byte write; current mission 16 | resolved in dirty patch | retain teardown without a general Stop mission authority | Stop fixture completes turn/deploy or real rejection |
| In-range Attack runs approach then ranged jitter before PerCell deploy | `0x004D4DC0`, `0x004D5690`, `0x004B0500` | resolved for ordinary object Attack in dirty patch | extend the same effective-Attack owner to force-fire variants | RNG state equals jitter + six constructor draws; no source shot |
| Out-of-range Attack installs nav and carries latch through movement | `0x004D5690`, destination setters, `0x00739EC0`, `0x004B0F20` | resolved for ordinary object Attack in dirty patch | extend the same approach/retry owner to force-fire variants | source/no building while moving; conversion at terminal callback/turn |
| Accepted object/cell force-fire converges on the ordinary Attack owner | `0x006FFEC0`, `0x00700600`, `0x004D74E0`, `0x004D7D50`, `0x004C6860`, `0x004C6CB0`, `0x005B3060`, `0x007447A0` | Rust sets `forward_deploy_attack_eligible=false`, selects DeployBuilding, and only clears high-level movement for force-fire | derive owner from live retry plus queued/current Attack; preserve entity-vs-cell target; apply native null-destination teardown before issuing either force-fire variant; remove origin bit from snapshot/hash authority | near/far object and cell force-fire reproduce approach/cadence/PerCell order and deploy at the approached cell |
| Stop during active approach retains only committed head and retries at its cell | `0x004AFE00`, `0x004B0500`, `0x004B0F20` | resolved in dirty patch, including the eager-uncommitted-curve correction | retain head truncation and post-movement retry ownership | Stop-far fixture deploys at head/cell, not old goal |
| Stop broadcasts BREAK to every contact and clamps Drive speed while keeping the head | Event case 6; `0x0065ACE0`; `0x004AFE00` | resolved in dirty patch | retain radio authority and clamped target until head retires | contacts cleared and Drive target `<=0.3` during final segment |
| Placement rejection clears latch and queues native fallback | Deploy clear at `0x00739573` | resolved in dirty patch | preserve rejection's Guard authority | blocked footprint ends latch on Guard |

### Negative Facts / Do Not Do

- Do not clear `+0x68C` at Attack command receipt.
- Do not keep blocking all Attack mission work behind the latch.
- Do not deploy an out-of-range SMIN before its Attack approach decision.
- Do not omit Attack cadence RNG because the source converts before firing.
- Do not claim one raw RNG advance; claim one `RandomRanged(0,2)` call.
- Do not give Stop a general native mission assignment.
- Do not clear the latch merely because Stop cleared target/NavCom.
- Do not claim far-approaching SMIN can never shoot.
- Do not source the facing gate from the mobile unit's `DeployFacing` line.
- Do not fix this by globally moving pursuit ahead of every object's movement; preserve per-object order.
- Do not exclude object or cell force-fire from Attack ownership once mission 1 is accepted.
- Do not require a still-live target to retain Attack ownership while the retry is live.
- Do not serialize or hash an ordinary-vs-force command-origin bit that native discards before mission execution.
- Do not treat clearing only `movement_target` as native null-destination teardown; NavCom, locomotor destination/head state, and reservations are part of the authority.

## 22. Builder Resolution and Focused Production Validation

Builder state after the bounded correction, still uncommitted and awaiting a fresh read-only critic:

1. Removed the synthetic `forward_deploy_attack_eligible` field and every snapshot/hash/default/test writer. `rg -n "forward_deploy_attack_eligible" src` returns no matches. Snapshot schema v116 now persists and hashes only the real `forward_deploy_retry` authority.
2. Added `GameEntity::owns_forward_deploy_attack_retry`, derived solely from a live retry plus effective queued/current mission 1. Object/cell command provenance and continued target liveness are not inputs.
3. Routed ordinary object Attack, Ctrl object ForceAttack, and Ctrl cell ForceAttack through the native-shaped null-destination boundary before attaching their retained entity/cell target. `clear_navigation_preserving_committed_head` clears NavCom, locomotor destination, trailing path, and synthetic reservations while retaining only a processed Drive/Ship head.
4. Kept the legacy/direct combat entry points behavior-compatible through internal helpers; production command callers use the pre-cleared variants so a real committed head is not destroyed a second time.
5. Extended production tests across near/far object and cell force-fire, target loss, same-frame Move replacement in both TurnFirst and eager-uncommitted-curve shapes, committed-head ForceAttack replay state, snapshot/hash replay, cadence RNG, placement revalidation, and conversion location.
6. Strengthened the slice-6 production replay at its tick-5 cell and tick-7 object force-fire tails: both assert the retained target kind, null NavCom, and null Drive destination. All historical hash probes moved together, proving a behavior-bearing null-destination correction rather than a v116-only schema change. Two no-edit runs reproduced `pre-v28=34D37696254132CF`, `pre-v29=3C180855C28365A6`, `pre-v110=DFF6B538E99B819C`, `current=D3D9A2C04F67EC79` exactly.
7. A subsequent critic found that the app producer still excluded every entity with a Miner component from Ctrl empty-cell force-fire. That made the direct simulation ForceAttackCell coverage unreachable for stock SMIN. The builder removed the non-native harvester gate and factored the exact per-unit branch into `cell_force_fire_payload`. Its negative control then exposed that the old `Option::is_some` test also misclassified stock CMIN's literal `Primary=none` as armed; the builder unified this path with the already-verified null-weapon spelling predicate. The stock-shaped regression now starts SMIN with a live facing retry, feeds the production payload through the real world-command consumer, and proves an unarmed CMIN with the same Miner component retains Move.
8. The next critic found the same stale `Option::is_some` admission in the production cursor. CMIN therefore showed a force-fire Attack reticle even though the corrected click emitted Move. The builder exposed the real-weapon predicate to the sibling cursor module, routed `force_fire_cursor_weapon_eligible` through it for both weapon slots, and added an armed-MTNK versus `Primary=none` CMIN cursor-gate regression.
9. The following critic found three remaining locomotor-authority gaps. First, a retained processed head left `movement_target` non-null, so the owner-local Attack approach producer declined to install the new far retarget even though the command had nulled NavCom. The producer now treats null NavCom as missing destination authority for a live deploy-retry Attack; the movement issuer anchors the new route at the still-physical head. This does not suppress the verified `PerCellProcess(2)` callback at a physical contact. Second, Ship command admission eagerly created `drive_track` and `head_to` with no equivalent of Drive's process-owned `track_valid`, so same-frame Move-to-mission-1 retasks preserved an unprocessed naval curve. `ShipLocomotionRuntime::track_valid` is now false at command admission, becomes true only inside Ship `Process_Movement`, resets at track retirement, participates in snapshot/hash authority, and gates null-destination retention. Third, processed Bridge-layer Drive curves owned no ground `occupation_head_to`, so retention discarded their movement target and the movement gate froze the live track. Committed-head discovery now uses `track_valid` plus the live shared-track geometry and takes the head layer from the retained path; ground reservation ownership remains an independent cleanup concern. New production regressions cover object/cell far retarget authority, all three same-frame Ship mission-1 producers, and processed Bridge Drive Stop/Attack retention.
10. The final broad Stop filter exposed one stale pre-existing Ship fixture that called the command-staged `head_to` committed without ever visiting production `Process_Movement`. The builder did not relax the new process-owned authority. The fixture now asserts that command admission leaves `track_valid=false`, advances one real simulation tick, proves Ship `Process_Movement` makes it true, and only then exercises Stop's committed-head arm. No production behavior changed for this test repair.

Latest focused literal results after the cursor correction, against the current 7,910-test enumeration:

- `cargo test -p vera20k --lib force_fire_cursor_weapon_gate_rejects_primary_none -- --nocapture`: `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 7909 filtered out`.
- `cargo test -p vera20k --lib app::input::cursor::tests::`: `test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 7897 filtered out`.
- `cargo test -p vera20k --lib app::input::context_order::tests::`: `test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 7899 filtered out`.
- `cargo test -p vera20k --lib ctrl_empty_cell_producer_routes_armed_smin_into_deploy_retry_attack_owner -- --nocapture`: `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 7909 filtered out`.
- `cargo test -p vera20k --lib forward_deploy_retry_`: `test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 7890 filtered out`.
- `cargo test -p vera20k --lib force_fire`: `test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 7898 filtered out`.

Earlier focused sim results on the same correction, before the two app-only producer/cursor regressions increased the enumeration from 7,908 to 7,910:

- `cargo test -p vera20k --lib sim::slave_miner::tests::`: `test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 7888 filtered out`.
- `cargo test -p vera20k --lib sim::deploy_tests::`: `test result: ok. 51 passed; 0 failed; 0 ignored; 0 measured; 7857 filtered out`.
- `cargo test -p vera20k --lib stop`: `test result: ok. 64 passed; 0 failed; 0 ignored; 0 measured; 7844 filtered out`.
- `cargo test -p vera20k --lib area_guard`: `test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 7900 filtered out`.
- `cargo test -p vera20k --lib replay_hash_stable_through_slice6 -- --nocapture`, twice without edits: `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 7907 filtered out`, with the four hashes above on both runs.
- `git diff --check`: clean (Git emitted only the repository's existing LF-to-CRLF checkout warnings).

Critic-9 correction seal, against the final 7,914-test enumeration unless an earlier enumeration is stated explicitly:

- Three exact new regressions for unprocessed Ship teardown, processed Bridge Drive retention, and committed-head far retarget each passed when the enumeration was 7,913: `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 7912 filtered out`.
- `cargo test -p vera20k --lib forward_deploy_retry_`, before the final movement regression increased the enumeration: `test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 7890 filtered out`.
- `cargo test -p vera20k --lib sim::movement::navcom::tests::`, at the same 7,913 enumeration: `test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 7904 filtered out`.
- `cargo test -p vera20k --lib sim::movement::movement_step::tests::`, at the same 7,913 enumeration: `test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 7906 filtered out`.
- `cargo test -p vera20k --lib gsi_13_06_body_frame_counter_roundtrips_and_changes_hash -- --nocapture`, at the same 7,913 enumeration: `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 7912 filtered out`.
- `cargo test -p vera20k --lib ship_head_becomes_committed_only_after_process_and_then_survives_null_destination -- --nocapture`: `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 7913 filtered out`.
- `cargo test -p vera20k --lib force_fire`: `test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 7902 filtered out`.
- The first final `cargo test -p vera20k --lib stop` correctly rejected the stale Ship fixture: `test result: FAILED. 64 passed; 1 failed; 0 ignored; 0 measured; 7849 filtered out`. After the fixture crossed and asserted the production Ship process boundary, its exact rerun passed with `1 passed; 0 failed; 7913 filtered out`, and the complete Stop filter passed: `test result: ok. 65 passed; 0 failed; 0 ignored; 0 measured; 7849 filtered out`.
- `cargo test -p vera20k --lib sim::slave_miner::tests::`: `test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 7891 filtered out`.
- `cargo test -p vera20k --lib replay_hash_stable_through_slice6 -- --nocapture`, twice without edits: `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 7913 filtered out`, with `pre-v28=34D37696254132CF`, `pre-v29=3C180855C28365A6`, `pre-v110=DFF6B538E99B819C`, and `current=D3D9A2C04F67EC79` identical on both runs.
- `rustfmt --edition 2024 --check` on all correction-touched leaf files: clean.
- `git diff --check`: clean (only LF-to-CRLF checkout warnings).
- `rg -n "primary\\.is_some\\(\\).*secondary\\.is_some|unit_armed && !is_harvester|forward_deploy_attack_eligible" src/app/input`: no matches (exit 1).

The one full `cargo test -p vera20k --lib` run remains deliberately unspent until a fresh critic passes the complete mechanism diff, as required by the repository's PR-ready gate.

## 23. Independent Critic Result

The original fresh read-only critic returned **PASS** for the then-claimed slice, which explicitly excluded force-fire/cell Attack. That verdict remains valid for Move, Stop, ordinary object Attack, Attack approach-before-`RandomRanged`, Drive `PerCellProcess(0/2)`, and committed-head behavior, but it did not prove the excluded variants.

A later fresh read-only implementation critic returned **CHANGES REQUESTED [P1]** after observing that `forward_deploy_attack_eligible=false` prevents object/cell force-fire from selecting the Attack owner. This bounded extension independently confirms the finding from the active action-classifier through Mission_Attack and supersedes any use of the earlier scope exclusion as implementation evidence. Section 22 records the builder correction; the mechanism still cannot receive a final PASS until that complete correction and its focused production output are reviewed by another fresh critic.

The next fresh read-only critic confirmed a second **CHANGES REQUESTED [P1]** production-delivery blocker: `src/app/input/context_order.rs` required `unit_armed && !is_harvester` before emitting Ctrl empty-cell ForceAttackCell. Stock SMIN is armed but owns a Miner component, so real player input routed to Move even though direct simulation tests passed. Section 22 records the builder response. A new critic must review the corrected complete artifact and focused output before this mechanism can pass.

That new critic returned **CHANGES REQUESTED [P2]** for the presentation half of the same decision: `src/app/input/cursor.rs` still treated any present weapon string as armed, so stock CMIN's `Primary=none` displayed a force-fire Attack reticle while the click producer emitted Move. Section 22 records the shared-predicate correction. Another fresh critic must review the complete updated artifact.

That fresh critic rechecked findings 4–8 and returned **CHANGES REQUESTED** for three locomotor-authority blockers: **[P1]** a retained processed head suppressed a new far Attack approach after null-destination retarget; **[P2]** Ship had no process-owned committed-head bit and preserved same-frame eager state; and **[P2]** processed Bridge Drive curves were discarded because commitment was inferred from a Ground-only reservation. Section 22 records the bounded builder response. Focused output and another fresh critic are still required before PASS.

The next fresh read-only critic reviewed the complete frozen artifact at report SHA-256 `4D44BEBE52CE082F4E3761A86267D2043EB9C4ED9A3580BCE4D8D4E3DDDB14FC` and returned **PASS**. It independently rechecked all three locomotor fixes plus prior findings 4–8; confirmed production ordering across Move, Stop, ordinary Attack, object/cell force-fire, near/far approach, target loss, same-frame promotion, PerCell timing, RNG cadence, placement retry, radio BREAK teardown, and snapshot/hash authority; and found no remaining mechanism blocker. The intentionally unspent full `cargo test -p vera20k --lib` remains the post-`origin/main` integration certification gate rather than a critic prerequisite.

## Sources

- Live Ghidra, read-only: `gamemd.exe` in project `testProsjekt`.
- Decompile/disassembly: `Main_Tick @ 0x0055D360`; `FUN_0064C380`; `EventClass::BuildMegaMissionEnvelope @ 0x004C6860`; `EventClass::Execute @ 0x004C6CB0`; `StopCommandClass::Execute @ 0x00730EA0`.
- Decompile/disassembly: `UnitClass::Deploy @ 0x007393C0`; `UnitClass::Mission_Unload @ 0x0073D630` (deploy-building branch); `UnitClass::PerCellProcess @ 0x00739EC0`; `UnitClass::AI @ 0x00736400`; `UnitClass::Enter_Idle_Mode @ 0x00738970`.
- Decompile/disassembly: `FootClass::AI @ 0x004DA530`; `FootClass::Mission_Attack @ 0x004D4DC0`; target/approach helper `0x004D5690`; `FootClass::ClickedAction_Object @ 0x004D74E0`; `FootClass::ClickedAction_Cell @ 0x004D7D50`; `FootClass::Assign_Target_Command @ 0x004DF0E0`; `FootClass::Set_Destination_Internal @ 0x004D94B0`.
- Decompile/disassembly: object action classifier `0x006FFEC0`; cell action classifier `0x00700600`; active MegaMission producer `0x006FFBE0`; Mission-1 Unit thunk `0x007447A0`; Unit vtable `0x007F5C70` and slot pointer `0x007F5E80`.
- Decompile/disassembly: `DriveLocomotionClass::Is_Moving @ 0x004AFB80`; `Is_Moving_Now @ 0x004AFC20`; `Set_Destination @ 0x004AFD40`; `Stop_Moving @ 0x004AFE00`; `Process @ 0x004B0500`; `Process_Drive_Track @ 0x004B0F20`.
- Decompile: `MissionClass::Mission_Dispatch @ 0x005B3060`; `Queue_Mission @ 0x005B35E0`; `Commence @ 0x005B3570`; `ObjectClass::UnInit @ 0x005F65F0`; `RadioClass::Broadcast_Radio_ToAll @ 0x0065ACE0`; `Random::RandomRanged @ 0x0065C7E0`.
- INI: `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini`, cited line ranges above.
- Prior modifier mapping: `docs/research/DRIVE_QUEUED_CLICK_EVENT_PLANNING_MODE_OUTCOME_RESWARM_20260528.md`.
- Current Rust read-only comparison: `src/app/input/context_order.rs`, `src/sim/combat/mod.rs`, `src/sim/world/techno_ai.rs`, `src/sim/world/techno_ai/mission_handlers.rs`, `src/sim/world/world_spawn.rs`, `src/sim/world/world_commands.rs`, `src/sim/world/world_orders.rs`, `src/sim/world/mod.rs`, `src/sim/slave_miner.rs`, and latch snapshot/hash surfaces at checkpoint `f5d052a9759edfc8d24c68ff1f9ec319eb53ba37` plus the uncommitted mechanism patch.
