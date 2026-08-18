# Ground Phase-1 Locomotor Population and Precedence - Ghidra Research Report

**Address(es):** `0x004DA530`, `0x007360C0`, `0x0051BAB0`, `0x0075AC80`, `0x00514310`, `0x0069FC10`, `0x007192F0`, `0x004B0C40`, `0x007359F0`, `0x0051B350`, `0x0073E5E0`
**Investigation Mode:** coverage-map with exhaustive-slice subaudits for host precedence, stock locomotor population, forced-track callers, and Unit/Infantry tube leaves
**Claimed Scope:** the complete explicitly bound stock ground-Foot population relevant to the planned authority migration, with the two unbound VehicleTypes registrations dispositioned separately; each active locomotor's per-object entry and idle/exception precedence; forced Drive callers; miner same-pass authority; current Rust routing at one frozen clean commit
**Non-Scope:** full locomotor numeric algorithms already owned by focused reports; malformed explicit Tube-data semantics; first-post-load scheduling of mixed Tube/forced/Teleport state; exact lifecycle/effect ownership inside every movement helper; pathfinding parity; executable retail runtime capture; production implementation
**Confidence:** High for the bounded population and precedence contract; Medium where explicitly marked for leaf-internal semantic names inherited from prior reports
**Active in YR:** Yes, with conditional/mod-map branches identified separately

## 1. Overview

The native owner is not a global "move every ground unit" phase. A reached
`FootClass` object runs its mission work and its currently active locomotor's
`ILocomotion::Process` in that object's live-vector slot. Active Unit/Infantry
tube leaves can preempt that ordinary host, and commands such as `Force_Track`
initialize locomotor state synchronously inside the calling object or mission;
they do not create another movement scheduler.

Checkpoint C's bounded native population/precedence map is now closed. It also
finds a load-bearing planning correction: stock ground Teleport users are part
of the population that must preserve this per-object ordering. The current
design's exclusion of Teleport from the atomic ground dispatcher is therefore
stale. This report does **not** authorize a production flip. Checkpoint D still
owns lifecycle/effect placement and Checkpoint E still owns an executable native
oracle.

### 1.1 Verdict at a glance

| Question | Verdict |
|---|---|
| Ordinary Drive, Walk, Hover, Ship one-object entry | verified |
| Ground Teleport one-object entry | verified; missing from the current approved population list |
| Idle/no-`MovementTarget` invocation | verified natively; current Rust skips or globally defers several kinds |
| Unit/Infantry active-tube precedence | verified |
| Forced Drive callers and selector reachability | verified for all direct callers; no stock caller proved for 64/65 |
| Miner mission-to-locomotor same-pass order | verified; current Rust snapshot pipeline is DRIFT |
| Rust `sync_formation_speeds` native equivalence | refuted; DRIFT |
| Exact lifecycle/effect ownership | deferred to Checkpoint D |
| Atomic production readiness | **NO-GO** |

### 1.2 Evidence boundary

All binary checks in this report targeted the active `gamemd.exe` program whose
read-only `get_current_program_info(program="gamemd.exe")` result identified the
retail executable at
`C:/Users/enok/Documents/Command and Conquer Red Alert II/gamemd.exe`, x86
little-endian 32-bit, image base `0x00400000`.

Current-Rust conclusions use only clean commit
`cacc073f017a9d5f1d0c537aa9cc2b63975884f3`. The shared working tree was already
under companion implementation when inspected at
`2026-07-21T02:07:46.8017736+02:00`; its moving contents were excluded from every
durable parity conclusion. Section 6 records the clean-commit blob identities.

## 2. Class Layout / Key Offsets

Offsets below name their receiver frame. Local Ghidra labels were treated only
as navigation hints; the load-bearing identities come from bodies, vtable bytes,
RTTI bytes, argument flow, and active callsites.

| Receiver | Offset | Width | Verified role in this report | Evidence |
|---|---:|---:|---|---|
| Foot object | `+0x90` | byte | active-state guard read immediately after ordinary locomotor `Process` | `disassemble_bytes(0x004DA840..0x004DA8A0)`; compare at `0x004DA87A` |
| Foot object | `+0x674` | pointer | active ILocomotion pointer used for vtable `+0x40` | `0x004DA85C..0x004DA877` |
| Foot object | `+0x684` | signed byte | active Tube index; negative means ordinary leaf | Unit `0x007363A4..0x007363AC`; Infantry `0x0051BAB8..0x0051BAC0` |
| Foot object | `+0x685` | byte | Tube path cursor | Unit tube `0x007359F0`; Infantry tube `0x0051B350` |
| Unit object | `+0x2E4` | pointer | reciprocal bunker/linked-building relation required by selector-71 helpers | `0x004593A0`; `0x004595C0`; reciprocal-link reports |
| Unit object | `+0x6C8` | pointer | native convoy follower link | `disassemble_bytes(0x004B121D..0x004B125D)` |
| TechnoType object | `+0x34C` | 16 bytes | locomotor CLSID; constructor default is Teleport and ReadINI preserves that default when `Locomotor=` is absent/invalid | `0x00710C21..0x00710C4B`; `TechnoTypeClass::ReadINI 0x007123ED..0x00712437`; Unit constructor `0x007354CE..0x00735518` |
| TechnoType object | `+0xCD4` | byte | `Teleporter=` predicate used by teleport/Drive destination selection | `UnitClass::Mission_Harvest @ 0x0073E5E0`; `TechnoClass::Set_Destination` slice in cited Teleport reports |
| active locomotor | vtable `+0x40` | function pointer | per-object `Process` entry called by Foot | `0x004DA877` plus class vtable reads |
| active locomotor | vtable `+0x70` | function pointer | interface slot whose Drive implementation is `Force_Track(coord, selector)`; Teleport's implementation is a no-op | seven Drive-dispatched direct callsites in Section 3.8; Teleport vtable/body in Section 3.6 |
| Tube object | `+0x1C0` | dword | explicit path length consumed by direction-8 producers/leaves | Walk/Drive/Hover/Ship producer audits; bridge Tube reports |

### 2.1 Locomotor identity anchors

| Kind | ILocomotion vtable | COL / TypeDescriptor proof | `+0x40` Process |
|---|---:|---|---:|
| Drive | focused Checkpoint-B report | `DRIVE_RAWTRACK_METADATA_INITIALIZER_RECONCILIATION_GHIDRA_REPORT.md` | `0x004B0500` |
| Walk | `0x007F69F8` | `[vtable-4] -> 0x0080D240`; `COL+0x0C -> 0x00847BF0`, `.?AVWalkLocomotionClass@@` | `0x0075AC80` |
| Hover | `0x007EACFC` | `[vtable-4] -> 0x00803228`; TypeDescriptor `0x008254B8` | `0x00514310` |
| Ship | `0x007F2D8C` | `[vtable-4] -> 0x008093A0`; TypeDescriptor `0x0083F880` | `0x0069FC10` |
| Teleport | `0x007F5000` | `[vtable-4] -> 0x0080C178`; `COL+0x0C -> 0x00844538`, `.?AVTeleportLocomotionClass@@` | `0x007192F0` |

The Walk/Hover/Ship proof legs were independently rebuilt inline rather than
accepted from prose. `read_memory(0x007F69F4,76)`,
`read_memory(0x007EACF8,76)`, and `read_memory(0x007F2D88,76)` show each
`[vtable-4]` COL pointer and each `vtable+0x40` slot. The resulting slots are
Walk `0x0075AC80`, Hover `0x00514310`, and Ship `0x0069FC10`.
`read_memory(0x0080D240/0x00803228/0x008093A0,24)` gives the corresponding
TypeDescriptor pointers `0x00847BF0/0x008254B8/0x0083F880`;
`inspect_memory_content(...,96)` decodes
`.?AVWalkLocomotionClass@@`, `.?AVHoverLocomotionClass@@`, and
`.?AVShipLocomotionClass@@`. `get_function_by_address` then bounds the three
slot bodies at `0x0075AC80..0x0075ACA0`, `0x00514310..0x00514C20`, and
`0x0069FC10..0x006A0192`.

Teleport identity was cold-spotted with
`read_memory(0x007F4FF0,128)`: vtable address `0x007F5000` has slot `+0x40 ->
0x007192F0`. `read_memory(0x0080C178,24)` and
`inspect_memory_content(0x00844538,96)` supply the COL and TypeDescriptor chain.
The Teleport CLSID bytes at `0x007E9A90` decode as
`{4A582747-9839-11D1-B709-00A024DDAFD1}`.

## 3. Core Logic

### 3.1 Shared native one-object spine

For an ordinary eligible Foot object, the load-bearing order is:

1. Its Unit/Infantry leaf checks any category-specific early branch.
2. `FootClass::AI @ 0x004DA530` calls `TechnoClass::AI_Update`.
3. Techno runs Object AI, the stored Mission Dispatch timer/health gates, and
   the concrete mission handler if due.
4. Foot runs substantial pre-locomotor work and five immediate gates.
5. `0x004DA877` calls the active ILocomotion vtable `+0x40` exactly once.
6. `0x004DA87A` immediately reads owner `+0x90`; zero exits Foot.
7. If still active, later Foot work runs. Near the end, Foot queries
   `IID_IPiggyback`, tests `Is_Ok_To_End`, and may restore the underlying
   locomotor before its final transport/team tail.

The five immediate gates are owner `+0x674 != 0`, owner `+0x3CD == 0`, owner
`+0x8D == 0`, owner `+0x2A8 == 0` or Type `+0x692 != 0`, and owner `+0x81 == 0`.
A failure skips `Process` and joins later Foot work at `0x004DAA01`; it is not a
return by itself. The direct `disassemble_bytes(0x004DA840..0x004DA8A0)` read
reconfirmed the last two gates, the `+0x40` call, and the immediate active check.
Checkpoint A is the authority for the earlier Techno/Mission segmentation.

This host does not test for a Rust `MovementTarget`. An idle locomotor still
receives `Process` whenever the five gates pass. What the class does after entry
is class-specific.

The derived-class return guards are not uniform. Infantry reads owner `+0x90`
immediately after its normal Foot call at `0x0051BCA4`. Unit has no immediate
post-Foot active guard: its intervening tail begins at `0x00736480`, and its next
`+0x90` read is delayed until `0x007365BB`.

### 3.2 Ordinary Drive contract

| Stage | Verified behavior | Evidence |
|---|---|---|
| Entry | Foot calls active Drive vtable `+0x40 -> 0x004B0500` | host contract; Drive vtable bytes |
| Idle/no destination | Process is still invoked; Drive chooses its internal no-track/no-movement path | `DriveLocomotionClass::Process @ 0x004B0500`; scheduling report |
| Existing normal/forced track | Process consumes the installed track before a fresh ordinary selection | `Process @ 0x004B0500`; `Process_Drive_Track @ 0x004B0F20` |
| Fresh ordinary movement | `Process_Movement @ 0x004B2630` owns selection/initialization and may process the new track in the same invocation | Checkpoint-B reports |
| Tube producer | direction 8 initializes owner Tube state at `0x004B1380`; Unit/Infantry tube leaf is not entered until the next object turn | producer/consumer disassembly and caller order |
| Completion | Drive-specific track/arrival work occurs inside the locomotor path; the exact lifecycle effects remain Checkpoint D scope | Drive movement reports |
| Post-Process | Foot immediately reads owner `+0x90` | `0x004DA87A` |

Fresh normal, accepted-chain, forced, and tube state are different initializers.
The RawTrack report's exact cursor/metadata contracts remain binding; this report
does not collapse them into one generic start.

### 3.3 Walk invocation order

| Stage | Unit with Walk | Infantry with Walk |
|---|---|---|
| Leaf entry | `UnitClass::AI @ 0x007360C0` | `InfantryClass::AI @ 0x0051BAB0` |
| Active Tube test | after possible Unit class-special Process/countdown | first body branch, before normal Infantry work |
| Normal host | calls Foot, then active Walk `+0x40 -> 0x0075AC80` | calls Foot, then active Walk `+0x40 -> 0x0075AC80` |
| Idle behavior | Walk Process still calls ProcessMovement; the null-destination branch clears owner `+0x68A` and can set speed fraction zero | same |
| Envelope | set Walk byte `+0x35=1`; call `ProcessMovement @ 0x0075AEC0`; clear `+0x35`; call ILocomotion `+0x10` and discard the return | same |
| Ordinary completion | `< 0x11` lepton threshold, path shift/reset, per-cell/subcell work, stop/arrival chain | same mechanism with category virtual bindings |
| Tube producer | direction 8 writes owner `+0x684/+0x685` at `0x0075B3FC` and seeds its Z/path state | same |
| Post-Process | common Foot `+0x90` check | common Foot check, followed by Infantry's later post-Foot active check |

`disassemble_bytes(0x0075AC80..0x0075ACAF)` confirms the wrapper envelope.
The Walk exhaustive subaudit established the destination-null and arrival details;
the exact sub-cell/path algorithm is not re-certified here.

### 3.4 Hover invocation order

| Stage | Verified behavior | Evidence |
|---|---|---|
| Entry | Unit -> Foot -> Hover `Process @ 0x00514310` | Unit/Foot host plus Hover vtable bytes |
| Moving prefix | `Is_Moving`, movement setup, a second `Is_Moving`, then `SpeedUpdate @ 0x00515ED0` | `disassemble_bytes(0x00514310..0x0051435F)` |
| Mid-Process active guard | owner `+0x90` is read immediately after SpeedUpdate on that moving branch | same range |
| Tube producer | direction 8 writes owner Tube state at `0x0051515B` | Hover Process/producer audit |
| XY/cell completion | class-owned inside Hover Process | `decompile_function(0x00514310)` and focused Hover report |
| Tail wake | `Is_Moving_Now`; conditional ten-frame wake work | `0x00514A10..0x00514AE4` |
| Vertical | direct call `0x00513D20` at `0x00514ACB`, after XY/cell/wake work and reachable while idle | same range |
| Return/post | final `Is_Moving` result is ignored by Foot; Foot then performs its immediate owner `+0x90` check | Hover tail plus `0x004DA877..0x004DA880` |

The native vertical controller is therefore not a global after-all-movers pass.
It belongs to each Hover Process and still runs on an eligible parked Hover.

### 3.5 Ship invocation order

| Stage | Verified behavior | Evidence |
|---|---|---|
| Entry | Unit -> Foot -> Ship `Process @ 0x0069FC10` | Unit/Foot host plus Ship vtable bytes |
| Prefix | slope sample/three-frame state updates precede track or translation, including idle cases | `disassemble_bytes(0x0069FC10..0x0069FD0F)` |
| Existing track | `Process_Drive_Track(0)`; if completion and gates allow, `Process_Movement`; then `Process_Drive_Track(1)` | `0x0069FC10..0x0069FD0F` |
| No track | `Process_Movement @ 0x006A1C80`; then `Process_Drive_Track(0)` if a track was installed | `0x006A0130..0x006A0194` |
| Tube producer | direction 8 writes owner Tube state at `0x006A0A48` and cursor at `0x006A0A51` | Ship track audit |
| Tail | `Is_Moving_Now`, eight-frame wake branch, idle speed-zero work, final `Is_Moving` | Ship focused report and live spot-check |
| Post | Foot immediate owner `+0x90` check | common host |

Current Ship is not interchangeable with Drive or generic straight-line ground
translation. Its slope prefix runs before its track decision and its wake/idle
tail remains within the same object invocation.

### 3.6 Teleport invocation order and the population correction

`TeleportLocomotionClass__StateMachineTick @ 0x007192F0` is the actual per-tick
ILocomotion `+0x40` entry. `0x00718B70` is a distinct synchronous destination
validator called from Teleport `Head_To_Coord`; despite an old local label and
older prose, it is **not** the per-tick Process. This was rechecked with
`get_function_by_address(0x007192F0)`, `decompile_function(0x007192F0)`, and the
vtable/RTTI reads in Section 2.1.

| Stage | Verified behavior | Evidence |
|---|---|---|
| Entry | Unit or Infantry -> Foot -> active Teleport `+0x40 -> 0x007192F0` | Foot host; Teleport vtable bytes |
| Idle phase 0 | Process is called; if Teleport `Is_Moving` is false and no special phase/pending state applies, it returns without relocation | StateMachineTick entry and phase-0 branch |
| Armed self-teleport | phase 0 reads `Is_Moving`; a non-null destination different from owner coordinates performs the class-owned targeting, animation, timer, occupancy/position, sound, crate, and phase work in this object call | `decompile_function(0x007192F0)` |
| Other phases | phases 1-7 advance their timers/position/validation/state in later reached object calls | same body `0x007192F0..0x00719BED` |
| Drive piggyback | `Set_Destination` or the land war-factory path can place a Drive locomotor above a Teleport primary; active Drive then receives ordinary Process | destination reports; factory selector-66 census |
| Force slot | Teleport vtable `+0x70 -> 0x0055AC10` is a no-op; Teleport itself cannot execute Drive ForceTrack | Teleport vtable bytes and trivial slot body |
| Restore | late Foot tail queries `IID_IPiggyback`, calls `Is_Ok_To_End`, releases/clears active Foot `+0x674`, calls `End_Piggyback(&Foot+0x674)`, then releases the interface reference | `decompile_function(0x004DA530)`, `0x004DAE5F..0x004DAEFD` |
| Post-Process | Foot's immediate `+0x90` guard runs before the late restore | common host |

This ordering applies to ground Infantry Teleport types (`CLEG`, `CCOMAND`,
`CIVAN`) as well as ground Vehicle Teleport types (`CMON`, `CMIN`, `SMON`). A
Phase-1 production migration that moves their missions/other ground peers into
the live object slot while leaving Teleport relocation and piggyback restore in
a later global pass changes later-object visibility. Teleport therefore needs a
prepared per-object handler in the atomic population.

The previously unresolved CMIN arming question is closed by a load-bearing label
correction. `0x0065AD30` is `RadioClass::Contact_With_Whom(index)`, not a
Foot/NavCom getter. Its body returns
`*(*(this+0xE4) + index*4)`, the radio-contact array; it does not read Foot
`+0x5A4`. `TechnoClass::Set_Destination` calls it with index zero at
`0x0074240F`. The old-object side of the Teleporter predicate is therefore the
current radio contact, not the previous NavCom destination.

The exact stock accepted-dock chain is:

1. In Harvest state 2, a Teleporter within the inclusive
   `ChronoHarvTooFarDistance * 0x100` threshold sends radio `HELLO (0x02)` at
   `0x0073EE59`. Stock `ChronoHarvTooFarDistance=50`.
2. `RadioClass::Transmit_Radio_Impl @ 0x0065A970` calls the refinery receiver
   synchronously and, only on reply `1`, stores that refinery in the miner's
   contact array at `0x0065AA54..0x0065AA5A`. Harvest writes substate 3 at
   `0x0073EE68`.
3. The later `Mission_Enter` dispatch obtains contact zero and sends
   `CAN_DOCK (0x0E)` at `0x004D92B9`.
4. The stock `DockUnload` refinery builds accepted cell `NW+(3,1)` and sends
   `0x12` synchronously at `0x0043CAB8`.
5. `FootClass::Receive_Radio` case `0x12` calls Unit vtable `+0x480`
   `Set_Destination(accepted_cell,1)` at `0x004D91EB`.
6. At `0x0074240F`, Set_Destination still sees the refinery radio contact. If
   that contact is BuildingClass, its Type `+0x16B3 DockUnload` is set, the new
   target is a CellClass, and `FindFirstUnit @ 0x0047EBA0` finds no occupying
   Unit, the flag becomes zero at `0x007424F6`. An already-active Teleport reaches
   the normal active-locomotor `Head_To_Coord` path in this call. An outer Drive
   reaches that restoration only if its IPiggyback interface first returns true
   from vtable `+0x1C Is_Piggybacking` at `0x00742534` (tested at `0x00742537`)
   and then true from vtable `+0x14 Is_Ok_To_End` at `0x00742554` (tested at
   `0x00742557`). Only then does release/clear/`End_Piggyback` run at
   `0x00742561..0x00742587`. The restored Teleport receives `Head_To_Coord`, and
   its same-turn eligible Process can consume the armed state.

Failed immediate restoration is a distinct ordering branch. If either the
`Is_Piggybacking` or `Is_Ok_To_End` gate is false, Set_Destination
calls current active-locomotor slot `+0x48` at `0x0074259B..0x007425A0`, calls
the owner mission setter `+0x1F0(-1)` at `0x007425A3..0x007425AA`, queues/enters
Mission 7 with argument 0 at `0x007425B0..0x007425B9`, then sets owner bytes
`+0x6AC=1` and `+0x1F8=1` at `0x007425BF/0x007425C6`. The common call into
`FootClass::Set_Destination_Internal` at `0x0074314F..0x00743161` commits Foot
NavCom `+0x5A4` at `0x004D950E..0x004D9516`, consumes/clears `+0x6AC` at
`0x004D9607..0x004D9618`, and therefore skips the ordinary target-coordinate and
active-locomotor `Head_To_Coord` path at `0x004D961A..0x004D965D`. After
`CAN_DOCK` returns, the same `Mission_Enter` runs its Site-C sequence at
`0x004D93FC..0x004D941D`: it saves NavCom `+0x5A4`, clears `+0x5A0/+0x5A4`, and
synchronously reissues `Set_Destination(saved,1)`. A still-non-endable Drive
repeats the pending/`+0x6AC` branch, so the current outer Drive receives this
turn's Process. Foot's late restore at `0x004DAE5F..0x004DAEC3` only restores the
saved locomotor; it does not call `Head_To_Coord`, and it is reached only if the
immediate post-Process owner `+0x90` check at `0x004DA87A` passes. A subsequent
Mission_Enter/Set_Destination reissue is what arms the restored Teleport.

This also resolves the close/far distinction without inventing a distance-only
warp trigger. At `<= 50` stock cells, an accepted HELLO can arm a warp directly
from the miner's current position to the accepted cell. At `> 50` cells, or when
HELLO is refused, no accepted refinery contact is installed; Harvest targets a
nearby passable `QueueingCell` staging cell and Set_Destination defaults to an
outer Drive. After Drive brings the miner within the threshold, a later accepted
HELLO can run the chain above, usually as a short final approach. A Unit-occupied
accepted cell also fails the flag-zero predicate and retains/creates Drive;
`FindFirstUnit` does not make infantry- or building-only occupancy equivalent.

No locomotor vtable `+0x70 Force_Track` call occurs anywhere in this accepted
CMIN chain. Its Drive/Teleport choice is destination and piggyback authority,
separate from the forced-track initializer census in Section 3.8.

This wrapper-selection chain is specific to `Teleporter=yes` CMIN before its
unloading-class swap. `CMON` binds the Teleport locomotor and is CMIN's
`UnloadingClass`, but has no `Teleporter=yes` key; do not generalize the
`0x007423CD` Teleporter block to an independently instantiated CMON merely from
its locomotor CLSID.

No Teleport direction-8 Tube producer was found. A Teleport-primary object can
still acquire Tube state while an outer Drive is active. On its next Unit or
Infantry turn, nonnegative owner `+0x684` selects the class Tube leaf before
ordinary Foot can dispatch either Drive or restored Teleport.

### 3.7 Active low-bridge Tube precedence

Active low-bridge TubeClass movement is not TS Tunnel locomotion.

| Surface | Exact precedence |
|---|---|
| Infantry | signed owner `+0x684` is the first body gate at `0x0051BAB8`; nonnegative calls `0x0051B350`, then virtual `+0x4A0(0)`, then returns from Infantry without normal Foot/locomotor work |
| Unit | after an earlier possible Unit class-special Process and lifecycle countdown, signed owner `+0x684` is tested at `0x007363A4`; nonnegative calls `0x007359F0`, then virtual `+0x4A0(0)`, then returns from Unit without normal Foot/locomotor work |
| Producer timing | Drive `0x004B1380`, Walk `0x0075B3FC`, Hover `0x0051515B`, and Ship `0x006A0A48` can write direction-8 Tube state during their ordinary Process. Because their Unit/Infantry tube gate has already passed, the leaf consumes that state on the next reached object turn. |
| Completion | Unit clears active Tube at `0x00735FF8`; Infantry clears it at `0x0051BA8D`. The leaf still returns from the class AI, so ordinary/forced movement does not resume in that same object turn. |

Unit uses `ftol(Type+0x678 * 1.5)` as its per-turn tube budget; Infantry uses raw
Type `+0x678`. Both can increment the cursor at most once per object turn and
spend residual budget toward the following segment. Their legal/blocked exits
are category-specific. Neither tube caller suffix has an explicit owner `+0x90`
read, but lifecycle effects inside every invoked virtual are not claimed inert;
that is a Checkpoint-D question.

The stock-data distinction is important:

- Tube predicates and automatic low-bridge shells are stock-active.
- A verified retail corpus scan found zero `[Tubes]` sections in 385 map payloads.
- Automatic shells have `path_len=0` and an unused `-1` path buffer.
- Direction-8 multi-step Unit/Infantry traversal is therefore conditional on
  explicit/mod-map data or another valid nonzero Tube record, not a routine stock
  map-authored path.

This does not make the mechanism dormant. It means stock normal maps exercise
the auto-shell/predicate surface while the multi-step leaf remains a supported
conditional path.

### 3.8 Forced Drive caller census

A complete direct-xref plus 97-site indirect `CALL [reg+0x70]` review found
seven true `ILocomotion::Force_Track` callsites representing six semantic paths.

| Callsite | Selector | Native caller/condition | Synchronous order and stock status |
|---:|---:|---|---|
| `0x0044DFA1` | 66 | land WeaponsFactory mission slot 26, output already has Drive | direct Force66 -> set speed fraction `0.5` -> building state 3; stock-active |
| `0x0044E160` | 66 | same mission/state after a Tunnel/Teleport primary is wrapped by a newly created outer Drive | create/query/piggyback Drive -> Force66 -> speed `0.5` -> state 3; only Teleport is stock-bound, while Tunnel is mod-only |
| `0x004591AF` | 67-70 | tank-bunker entry after facing stops rotating | choose selector from live current-facing quantization -> exact building coords -> Force -> speed `1.0`; stock-active conditional |
| `0x0045943B` | 71 | `BuildingClass::UndockUnit`, reciprocal Unit/Building `+0x2E4` relation | Power_On -> offset coords -> Force71 -> speed `1.0` -> clear both links -> break; stock-active conditional bunker teardown |
| `0x00459760` | 71 | `BuildingClass::ReleaseDockedHarvester`, same reciprocal relation | clear unit link -> Power_On -> Force71 -> speed `1.0` -> passable destination/Mission Move -> clear building link; stock-active for bunker release, not ordinary refinery unload |
| `0x004DF92D` | -1 | Unit relocation helper reached by TriggerAction execute action `0x80` | explicit coord plus sentinel reset; conditional engine path, stock-map corpus reach not established |
| `0x007101B3` | -1 | `TechnoClass::PerformDeploy`, `IsLocomotor` replacement leaf | NullCoord plus sentinel reset before locomotor replacement; stock-active conditional Magnetron-style mechanism |

Direct byte spot-checks include
`disassemble_bytes(0x0044DEC0..0x0044DFB0)`, which compares CLSIDs
`0x007E9A50` (Tunnel), `0x007E9A90` (Teleport), then `0x007E9A30`
(Drive), and `disassemble_bytes(0x0044DFAD..0x0044E165)`, which constructs
the outer Drive/piggyback before the second Force call. Raw
`read_memory(0x007E9A30/40/50/90,16)` reads prevent the older shifted-GUID
Hover interpretation. Other callsite checks include
`disassemble_bytes(0x0044DF85..0x0044DFB0)` and
`disassemble_bytes(0x0044E145..0x0044E170)`, which show `PUSH 0x42` and vtable
`+0x70`; `0x0045912D..0x004591AF`, which derives 67-70 before the call;
`0x0045942C..0x0045943B` and `0x00459751..0x00459760`, which push 71; and
`0x007101A4..0x007101B3`, which pushes `-1` with NullCoord.

No active/stock caller for selector 64 or 65 was proven. Selectors 66-71 are not
a generic refinery-exit family:

- 66 is the normal land war-factory drive-out row.
- 67-70 are the four bunker-entry rows selected from live facing.
- 71 is reciprocal-link bunker release/undock.
- healthy stock zero-link `HARV/CMIN -> GAREFN/NAREFN` unload does not Force71.
- `-1` is cancel/reset behavior, not a special TurnTrack row.

The normal zero-link exclusion is a direct body result, not an inference from
the caller census. `UnitClass::Mission_Deploy_Building @ 0x0073D630` sends a
nonzero Unit `+0x2E4` relation to `ReleaseDockedHarvester` at `0x0073D66D`, but a
zero relation jumps into the stock deployment FSM. Its state-4 exit at
`0x0073E17F..0x0073E283` clears Unit `+0x6D1`, queues Harvest/break/commence work,
and contains no locomotor `+0x70` call or destination write. The reciprocal-link
writer at `0x00459301/0x0045930F` is itself bunker-gated through
`0x0044B797..0x0044B7A3`.

Force initializes state synchronously. On the forced unit's later reached Foot
turn, active Drive Process consumes that installed state before fresh ordinary
movement. Active Tube state still preempts the ordinary Drive Process at the
Unit leaf, so a coexisting forced track waits for a later object turn.

### 3.9 Miner authority and same-pass visibility

`MissionClass::Mission_Dispatch @ 0x005B3060` calls concrete mission slots and
stores their returned delay before returning to the same object's Foot work.
Live disassembly reconfirmed Enter at `0x005B3110` (vtable `+0x240`), Deploy at
`0x005B3198` (`+0x230`), and Harvest at `0x005B31BA` (`+0x224`), followed by
timer writes based on the current frame.

The one-live-miner contract is therefore:

1. The miner reaches its Unit/Foot/Techno slot in live-vector order.
2. If its stored mission timer is due and health/active gates pass, its concrete
   Harvest, Enter, or Deploy handler runs now.
3. Destination, radio, dock, cargo/credit, mission, and Scenario RNG mutations
   made by that handler are visible immediately.
4. If ordinary Foot gates pass, the miner's now-active locomotor Process runs in
   the same object turn against that post-mission state.
5. The next live object observes all completed same-object effects, subject to
   the exact lifecycle boundaries still owned by Checkpoint D.

For two miners contending for one refinery, releasing miner A does not globally
promote miner B. B can advance in the same native frame only if B occurs later in
the live vector and B's own Mission_Enter retry is due when its slot is reached.
This is an ordered per-object dependency, not a snapshot-all/process-all batch.

Stock family corrections:

- `HARV` and `HORV` are Drive harvest/dock types.
- `CMIN` and `CMON` are Teleport harvest/dock types; `CMIN` is the normal Allied
  chrono miner/free unit from `GAREFN`, while `CMON` is its stock
  `UnloadingClass` and has `TechLevel=-1`.
- `SMON` is also Teleport plus `Harvester=yes`, but is explicitly named useless
  and has `TechLevel=-1`; no normal stock activation was found.
- `SMIN` is Drive, has `ResourceGatherer=yes`, and `DeploysInto=YAREFN`; it does
  not have `Harvester=yes` and must not be classified as the same native
  Harvest/Enter/DockUnload family.

For CMIN specifically, radio contact and NavCom are different authorities.
`RadioClass+0xE4` holds the contact array populated by accepted `HELLO`; Foot
`+0x5A4` is the committed movement destination written by the later
Set_Destination path. The Teleporter predicate reads contact zero. Older reports
that call `0x0065AD30` a NavCom getter therefore invert the arming provenance.

### 3.10 Native convoy versus Rust formation sync

The native Drive track slice at `0x004B121D` calls owner `WhatAmI`, requires
category 1, reads owner `+0x6C8`, and walks that linked follower chain. For each
follower it calls virtual `+0x544` with the leader's current speed-fraction
qword from owner `+0x578/+0x57C`. The mutation belongs to the leader's current
Drive Process, before later objects run.

`TeamClass` target/convoy helpers are separate. A live
`decompile_function(0x006EC3A0)` read walks members through Team `+0x54` and
member `+0x5D8`, while `decompile_function(0x006E9050)` manages Team target/nav
state. Neither is a global "all command-group members take the minimum speed"
pass.

Therefore Rust `sync_formation_speeds` is **DRIFT**, not an alternative native
representation with proved equivalence. Native is an explicit linked Drive-only
propagation of the leader's current fraction in that leader's Process. Rust uses
selection-command `group_id`, scans all entities after translation, computes a
group minimum, and permanently lowers each `MovementTarget.speed` for later
ticks. The future dispatcher must remove this global mutation from parity
authority; it must not simply relocate it into every object call.

## 4. INI Keys and Stock Population

### 4.1 Exact effective `rulesmd.ini` population

The count is by numeric registration row, not by buildability. A PowerShell
read-only section/key parse of `ini/rulesmd.ini` (SHA-256
`3D341EF8A13A4B5AB24AF2EEF48AC94931AC2BB87D950FE3330A07E2D25672EF`)
classified each registered row by its explicit `Locomotor=` value.

| Type list | Registration count | Explicit locomotor split |
|---|---:|---|
| `InfantryTypes` | 65 | Walk 60; Teleport 3; JumpJet 2 |
| `VehicleTypes` | 80 | Drive 52; Ship 13; Hover 4; Teleport 3; JumpJet 6; `DeathDummy` no Locomotor key; `YDUM` has no section |

The non-Walk Infantry rows are exact:

- Teleport: `CLEG`, `CCOMAND`, `CIVAN`.
- JumpJet: `JUMPJET`, `LUNR`.
- The other 60 registered Infantry rows bind Walk.

Reachability within the three Infantry Teleport rows differs: `CLEG` is an
ordinary buildable type, while `CCOMAND` and `CIVAN` are stolen-tech conditional.

The Vehicle groups are:

- **Drive (52):** `AMCV`, `HARV`, `APOC`, `HTNK`, `CAR`, `BUS`, `WINI`,
  `PICK`, `MTNK`, `HORV`, `TRUCKA`, `TRUCKB`, `V3`, `DRON`, `HTK`, `SMCV`,
  `TNKD`, `HOWI`, `TTNK`, `LTNK`, `SREF`, `XCOMET`, `MGTK`, `FV`, `DTRUCK`,
  `PROPA`, `CONA`, `COP`, `EUROC`, `LIMO`, `STANG`, `SUVB`, `SUVW`, `TAXI`,
  `PTRUCK`, `PCV`, `SMIN`, `YCAB`, `YTNK`, `BFRT`, `TELE`, `CAOS`, `DDBX`,
  `BCAB`, `JEEP`, `MIND`, `UTNK`, `DOLY`, `CBLC`, `FTRK`, `AMBU`, `CIVP`.
- **Ship (13):** `CARRIER`, `DEST`, `SUB`, `AEGIS`, `DRED`, `SQD`, `DLPH`,
  `HYD`, `VLAD`, `CRUISE`, `TUG`, `CDEST`, `BSUB`.
- **Hover (4):** `SAPC`, `LCRF`, `YHVR`, `ROBO`.
- **Teleport (3):** `CMON`, `CMIN`, `SMON`.
- **JumpJet (6):** `ZEP`, `SHAD`, `HIND`, `SCHP`, `DISK`, `SCHD`.
- **No explicit binding (2 registrations):** `DeathDummy` has a section but no
  Locomotor key; `YDUM` has no section. Both allocated UnitType objects retain
  the TechnoType constructor's Teleport CLSID default. No shipped-stock instance
  producer was found, so they are not members of the realized stock Phase-1 Foot
  population; custom-map/mod instantiation remains conditional and would create
  Teleport-backed Units.

This supersedes any statement that InfantryClass means Walk, that every infantry
uses Walk, or that the stock list is "about 64" infantry.

Within the Vehicle Teleport trio, `CMIN` is the ordinary active chrono miner,
`CMON` is used as its unload-display class, and `SMON` is a registered
`TechLevel=-1` useless type with no normal stock activation found. Registration,
class binding, buildability, and normal runtime reachability are distinct facts.

The two unbound registrations are now fully dispositioned. `RulesClass` reads
every VehicleTypes row through `RulesClass::ReadVehicleTypes @ 0x00672360` and
`UnitTypeClass::FindOrAllocate @ 0x007480D0`, so both metadata objects exist.
`DeathDummy` is the explicit dummy workaround at `rulesmd.ini:1111` and has only
`[DeathDummy] Primary=DefaultDeathWeapon` at `:9653..9654`; `YDUM` is registered
at `:1147` but has no section. `UnitTypeClass::Constructor @ 0x007470D0` reaches
`TechnoTypeClass::Constructor @ 0x00710AF0`, whose
`disassemble_bytes(0x00710C10..0x00710C60)` read shows the 16 bytes at
`0x007E9A90` copied to Type `+0x34C`. ReadINI copies that current value as the
default into `ReadCLSID @ 0x00527920` at `0x007123ED..0x00712437`; an absent key
therefore preserves Teleport. `UnitClass::Constructor` consumes Type `+0x34C` at
`0x007354CE..0x00735518`.

A case-insensitive, delimiter-valid stock-data scan covered all 16 retail MIX
archives plus 54 loose maps and found no object or trigger producer for either
ID. This proves zero shipped-stock instance producer in the audited corpus, not
structural impossibility. If a custom map/mod instantiates one, it is a
conditional ground Teleport Unit and belongs in the same active-locomotor
dispatcher.

### 4.2 Relevant keys

| Key | Native type | Fresh default / missing-key behavior | Stock examples | Effect in this report |
|---|---|---|---|---|
| `Locomotor=` | 16-byte CLSID/GUID | Teleport `{4A582747-9839-11D1-B709-00A024DDAFD1}`; missing, empty, or invalid input preserves the current CLSID | GUID values above | selects the class instantiated into Foot `+0x674` |
| `Teleporter=yes` | boolean byte | `false`; missing preserves current | `CLEG`, `CCOMAND`, `CIVAN`, `CMIN` | participates in Unit destination/piggyback decisions; not present on every registered Teleport type |
| `Harvester=yes` | boolean byte | `false`; missing preserves current | `HARV`, `HORV`, `CMIN`, `CMON`, `SMON` | activates harvest/dock family logic; does not by itself select a locomotor |
| `Dock=NAREFN,GAREFN` | comma-delimited vector of `BuildingTypeClass*` references | empty vector; missing or empty input preserves the existing vector | harvest families above | supplies refinery candidates |
| `DeploysInto=YAREFN` | nullable `BuildingTypeClass*` reference | null; missing or empty input preserves the current pointer | `SMIN` | deploy-building family, not proof of `Harvester=yes` |
| `TechLevel=-1` | signed 32-bit integer | `255` (`0xFF`), not `-1`; missing preserves current | explicit `-1` on `HORV`, `CMON`, `SMON` | registered but not normally buildable through the standard tech tree |
| `WeaponsFactory=yes` | boolean byte | `false`; missing preserves current | `GAWEAP`, `NAWEAP`, `YAWEAP` | enables the land WeaponsFactory classification used by selector 66 |
| `Factory=UnitType` | 32-bit object-category/RTTI enum | `0` (`<none>`); missing preserves current and an invalid explicit token maps to `0` | `GAWEAP`, `NAWEAP`, `YAWEAP` | identifies the produced category used by the factory mission |
| `ExitCoord=` | three signed 32-bit lepton coordinates | `(0,0,0)`; missing preserves the current triple | `GAWEAP`, `NAWEAP`, `YAWEAP` | supplies the land-factory output coordinate used before selector 66 |
| `Bunker=yes` | boolean byte | `false`; missing preserves current | tank bunker | makes selectors 67-71 conditionally stock-active |
| `ChronoHarvTooFarDistance=50` | signed 32-bit integer, cells | `50`; missing preserves current | `[General]` | inclusive CMIN threshold for attempting refinery HELLO before fallback staging |

These are active-binary constructor/parser defaults, not defaults inferred from
INI comments. The `Locomotor`, `Teleporter`, `Dock`, `DeploysInto`, and
`TechLevel` reads are at `0x007123ED..0x00712437`,
`0x00713FE2..0x00713FF6`, `0x00713180..0x00713239`,
`0x00713264..0x00713297`, and `0x00714570..0x00714584`; their constructor
values are at `0x00710C21..0x00710C4B`, `0x00711484`,
`0x00710D39..0x00710D57`, `0x00710D61`, and `0x00711082`.
`Harvester` is read at `0x0074769F..0x007476B9` after zero initialization at
`0x0074710F`. The four BuildingType keys are read at
`0x00460A72..0x00460A8C`, `0x0046051A..0x00460545`,
`0x00460F9C..0x00460FDF`, and `0x0046093A..0x00460954`, with constructor
defaults at `0x0045E139`, `0x0045DEB6`, `0x0045DECE..0x0045DEDA`, and
`0x0045E0CC`. `ChronoHarvTooFarDistance` is initialized at `0x00666846` and
read at `0x0066FFF6..0x0067001B`.

### 4.3 Stock-active, conditional, and dormant distinctions

| Mechanism/class | Classification | Basis |
|---|---|---|
| Drive, Walk, Hover, Ship, Teleport | stock-active | explicit bindings in registered stock type lists |
| Tube predicates/auto shells | stock-active | binary auto-creation and bridge predicates |
| explicit nonzero `[Tubes]` multi-step traversal | conditional/mod-map | 0 of 385 retail map payloads had `[Tubes]` |
| selector 66 | stock-active | land WeaponsFactory mission |
| selectors 67-71 | conditionally stock-active | tank-bunker lifecycle |
| selectors 64-65 | compiled data, no stock caller proved | exhaustive direct caller census |
| Tunnel, DropPod | compiled/runtime-recognized, no stock instantiation | zero INI GUID matches |
| Mech | compiled/runtime-recognized, no stock instantiation | only commented GUID mentions; no effective binding |
| DeathDummy, YDUM | registered metadata; zero shipped-stock instance producers found | default Teleport CLSID survives missing key; 16 MIX plus 54 loose-map producer scan |

Do not describe the dormant trio as "factory references only." The accurate
statement is that the binary recognizes/constructs the classes and mods can bind
their CLSIDs, but stock effective INI data does not instantiate them.

## 5. Integration Points and Exact-Once Population

### 5.1 Native category precedence matrix

| Population member | First relevant movement branch in its own slot | Ordinary Process owner | Same-turn postcondition |
|---|---|---|---|
| Unit/Drive | Unit special/tube gates | Foot -> Drive | Foot immediate active check; later Foot/Unit tail |
| Infantry/Walk | Infantry Tube gate first | Foot -> Walk | Foot active check; Infantry later active check |
| Unit/Walk | Unit special/tube gates | Foot -> Walk | Foot active check; later Unit tail |
| Unit/Hover | Unit special/tube gates | Foot -> Hover | Hover XY/wake/vertical complete before Foot returns |
| Unit/Ship | Unit special/tube gates | Foot -> Ship | slope/track/wake complete before Foot returns |
| Unit or Infantry/Teleport | corresponding class Tube/special gates | Foot -> Teleport StateMachineTick | phase/relocation effects precede later live objects; piggyback restore is later in the same Foot call |
| miner | Unit/Techno Mission Harvest/Enter/Deploy first | then active Drive or Teleport | mission mutations feed locomotor in same object turn |
| active Unit Tube | Unit Tube leaf | ordinary Process skipped | class AI returns; resume no earlier than next object turn |
| active Infantry Tube | Infantry Tube leaf | ordinary Process skipped | class AI returns; resume no earlier than next object turn |
| forced Drive | Force command initializes state at caller point | later active Drive Process consumes it | forced track precedes fresh ordinary selection |

### 5.2 Required future atomic dispatcher population

The future exact-once dispatcher must prepare, and at activation route, at least:

1. Unit/Drive ordinary and forced states.
2. Unit/Walk and Infantry/Walk ordinary states.
3. Unit/Hover including its idle vertical tail.
4. Unit/Ship including its idle slope/wake tail.
5. ground Unit/Infantry Teleport including phase state and late same-Foot
   piggyback restore.
6. one-live-miner Mission Harvest/Enter/Deploy before its active locomotor.
7. Unit/Infantry active Tube leaves, which preempt ordinary Process.
8. class-specific arrival/completion and all lifecycle/effect owners once
   Checkpoint D proves them.

JumpJet/Fly and projectile locomotors remain outside this ground report. The
Teleport addition is not an invitation to fold every special locomotor into the
same handler; it follows from stock ground Unit/Infantry users and their native
Foot-slot visibility.

### 5.3 Design/contract consequence

The following current planning claims are stale after this census:

- design line 219 classifies Teleport with later special locomotors outside the
  ground slice;
- design line 604 forbids folding Teleport into the ground slice without an
  evidence-backed extension;
- design/contract population lists name Drive/Walk/Hover/Ship/miner/tube/forced
  but omit ground Teleport types.

This report is the evidence-backed extension. Before any production plan is
executed, the design and implementation contract must be reconciled to include
ground Teleport and removal/extraction of its later global authority. The atomic
architecture decision itself remains valid; its enumerated population was
incomplete.

## 6. Current Rust Implementation Status

### 6.1 Frozen comparison identity

All line references below are from clean commit
`cacc073f017a9d5f1d0c537aa9cc2b63975884f3`, not the concurrently edited
working tree.

| File | Clean Git blob |
|---|---|
| `src/sim/world/mod.rs` | `9ec2bdb1ae39cec10cdf9cd1155848dbfb8235d8` |
| `src/sim/world/techno_ai.rs` | `ec9ba915aa830e813322b21ea2616b5f5f977915` |
| `src/sim/world/world_spawn.rs` | `13408a4c8bfbe2cd5f57f17bd67a774288e6d7ed` |
| `src/map/actions.rs` | `fb70b98c724f5fb1b81a7948a41d5cccab882a49` |
| `src/sim/trigger_runtime.rs` | `d56f92d74a5aeb0a381885701289cac0dd65de63` |
| `src/rules/locomotor_type.rs` | `875a4caf6ef40415cfdcf0d7c2b0611550202edd` |
| `src/rules/object_type.rs` | `54a845daa77908b623e21eda25a80c7dbad0e010` |
| `src/rules/ruleset.rs` | `0d58c2fd5ef4672ccc09d2e644997f0e7a62b6e0` |
| `src/sim/movement/movement_tick.rs` | `38c891f6ed9b8bd805e1c0fd51fd11a56ddc876e` |
| `src/sim/movement/movement_step.rs` | `c91dafb9d0a486e52491d0d13fec87cbf256c249` |
| `src/sim/movement/locomotor.rs` | `2179c3dd2a37f8c54e09b1cd89c1cdf3a47ccbbf` |
| `src/sim/movement/drive_locomotion.rs` | `032306d3572efa747441bc770971c758e2fb38e7` |
| `src/sim/movement/tube_movement.rs` | `60f6f31a0a315469f472700f3bf15ccb0c2b6f1f` |
| `src/sim/movement/teleport_movement.rs` | `6c05622bbb559237ec82975ad87c7e7ebeadfe67` |
| `src/sim/movement/movement_commands.rs` | `db000da47d4735809e76d280b5b9a036ed81e8f1` |
| `src/sim/movement/mod.rs` | `cf0f78bd09d5ecbbac2ca94eaec89d4bf2b366d8` |
| `src/sim/movement/drive_track.rs` | `0003b1dc71ab609ef4563f52c474aa3eb3086ffc` |
| `src/sim/miner/miner_system.rs` | `529941eef74abfd248c07860b9c933dc966563b1` |
| `src/sim/miner/miner_dock_sequence.rs` | `2ef7d30ae496c8625f755f22012bf756a2204301` |
| `src/rules/warhead_type.rs` | `8591cd9012c728ac6962340088fc18a8f048758e` |
| `src/sim/docking/bunker_install.rs` | `f7d035476d14cb9e1559457256a76004e663f466` |
| `src/sim/docking/bunker_link.rs` | `2d654d15fcb3ac763fb2403b122fe5463ef9c48b` |
| `src/sim/production/production_queue.rs` | `417c86202ad0e64ba0fc06eb1536b44d6fb3d309` |
| `src/sim/production/production_spawn.rs` | `f73db9a1fb58c6ef588148e4a0c5c23cf65faf25` |
| `src/sim/production/production_economy.rs` | `9a2d0084c5ae77fd88a7c3a83d0ac784d455e826` |
| `src/sim/production/war_factory_exit.rs` | `90eeb61aef04b7775612b7d4eed616c09f521d97` |
| `src/sim/world/world_commands.rs` | `fc73f76243be3b53b1c0757ca824865d3beb153e` |

### 6.2 Clean-HEAD routing

The settled Rust order is a set of population-wide phases:

1. `world/mod.rs:2225..2232` runs whole-population `object_ai_stage`.
2. `world/mod.rs:2234..2262` runs whole-population Phase-1
   `tick_movement_with_grids`.
3. Inside that function, `movement_tick.rs:894..906` first performs Drive
   entity-NavCom reaim, `:908..910` advances low-bridge tubes, `:911` advances
   forced tracks, and `:913..934` performs the initial target-based mover scan
   plus the inert Drive marker. Pending Drive arrivals run at `:965..973`; the
   ordinary target-based list is rebuilt at `:974..988` and processed at
   `:990..1748`.
4. `movement_tick.rs:1750` globally calls `sync_formation_speeds` after mover
   translation, `:1752..1779` applies deferred crush removal, `:1781` performs
   global target finalization, and `:1782` performs global locomotor-phase
   updates.
5. Hover vertical state is a separate all-Hover tail at `:1784..1849`, including
   parked Hover entities.
6. Before Phase 2, `world/mod.rs:2263..2270` runs gate runtimes,
   `:2271..2278` performs a population-wide war-factory exit-contact break, and
   `:2280..2284` performs wall crush.
7. `world/mod.rs:2303..2348` runs population-wide Teleport and other special
   locomotors after the complete ground pass; `:2357` later runs population-wide
   piggyback restoration, `:2359..2370` global rocking, and `:2379..2425` a
   global water-mover wake scan.
8. Phase-7 production/miner work is reached at `world/mod.rs:2804..2848`;
   `production_queue.rs:415` calls resource economy, which runs miner authority.
9. `miner_system.rs:98..176` snapshots all eligible non-dying, non-Slave
   entities with a Miner component, and `:181` processes all those snapshots.
   `:192..215` writes miner/debug state back, while `:217..247` writes derived
   voxel-animation and harvest-overlay visuals in later loops. Shared
   `Simulation` side effects can still be visible between miner iterations; the
   miner-component and derived-visual writebacks are what remain batched.
10. `world/mod.rs:2951..2956` performs another tail mission projection.

The internal iteration order is also mixed. Drive entity-NavCom reaim
(`drive_locomotion.rs:61..70`), low-bridge Tube service
(`tube_movement.rs:224..268`), pending-arrival service
(`movement_tick.rs:452..453`; no-grid path `navcom.rs:110..125`), later
piggyback restoration (`movement/mod.rs:217..240`), the war-factory contact break
(`war_factory_exit.rs:34..70`), and the water-wake scan
(`world/mod.rs:2389..2409`) iterate stable-ID order. Forced, ordinary, Hover, and
Teleport movement instead receive supplied live-order snapshots. Neither the
population-wide phase split nor this mixture reproduces one native live-vector
object slot.

That mechanism is **DRIFT** even where an ID list follows live order. Native
mission, locomotor, phase effects, and restore happen before the next object;
Rust separates their authority across population phases. This does not imply
that every Rust miner side effect is deferred: the snapshot processor can mutate
shared `Simulation` state between miner iterations, while miner-component writes
remain batched.

The current host bracket is narrower than its name suggests. At
`techno_ai.rs:278..355`, only Unit enters the Unit host bracket; Infantry gets no
corresponding Techno/Foot mission host. Non-miner Units commit only a
`derived_mission` projection and miners return `MinerDeferred`
(`techno_ai.rs:525..558`). The Drive surface at `:580..695` and
`drive_locomotion.rs:17..31` is an inert/read-only marker, not locomotor
authority. Consequently all 60 stock Walk Infantry and all three stock Teleport
Infantry are outside the intended host bracket, while Units still do not execute
the actual native mission handlers there.

### 6.3 Category deltas

| Surface | Current clean Rust | Parity verdict |
|---|---|---|
| idle Drive/Walk/Ship/ground Teleport | generic mover collection skips no-`MovementTarget` entities | DRIFT: native Process still enters |
| Hover | shared mover loop with Hover-specific steering/throttle plus global idle/moving vertical tail | DRIFT: native vertical is inside each Hover Process after its XY/wake work |
| Walk/Ship/ground Teleport with target | generic straight-line path (only Drive and Hover get their dedicated branches) | DRIFT: no production Walk/Ship Process and Teleport can receive generic Phase-1 service |
| Teleport special state | separate `tick_teleport_movement` after all ground; only entities with `teleport_state`; global later restore | DRIFT: the global authority is out of native Foot-slot order; Rust source permits combined state, but this report does not use that fact to assert a shipped-stock native mixed-state fixture |
| miner | eligible non-dying, non-Slave Miner-component entities run late snapshot-all/process-all/state-and-visual-writeback-all | DRIFT |
| tubes | global category-neutral pre-pass using `low_bridge_tube_state`; one whole path cell per pass; completion can fall through to forced/ordinary work | DRIFT |
| forced track | global pre-mover pass | DRIFT versus caller-time setup plus own later Drive Process |
| formation | global post-translation group minimum | DRIFT; no equivalent native owner |
| arrival/finalization/crush | deferred/global collections | ownership unresolved; Checkpoint D blocker |
| GI/Conscript speed | all three clean-HEAD spawn construction routes multiply `GI/CONS/E1/E2` movement speed by six | DRIFT: explicit high-frequency testing override |
| missing/no-section VehicleType locomotor default | `DeathDummy` is constructed as Drive; no `ObjectType` metadata is created for `YDUM` | DRIFT: native creates both UnitType metadata rows and preserves Teleport; no shipped producer lowers priority but does not change the verdict for custom/mod placement |

The production Tube authority is `GameEntity::low_bridge_tube_state`.
Production passively reads `DriveLocomotionRuntime::active_tube` through
`drive_requires_native_step @ drive_locomotion.rs:33..35`, called at
`movement_step.rs:887..890`. However, its initializer, advancer, and finalizer
(`begin_drive_tube_traversal`, `tick_unit_tube_payload`, and
`finish_unit_tube_movement`) have no production caller in this clean snapshot;
only tests invoke them. Normal runtime flow therefore never initializes or
advances this payload, although serde-deserialized or otherwise pre-seeded state
can still reach the passive read. The global `low_bridge_tube_state` pass remains
normal movement authority. That pass lacks the native Unit/Infantry budget split
and permits a tube completion to fall through into another movement family in
the same bulk tick.

The ordinary selector at `movement_tick.rs:916..930,975..985` is based on target
presence, tube/forced exclusion, and non-Air/non-Underground layer, not an exact
active-locomotor/class-shell matrix. Drive gets special speed/raw-track work at
`:1171..1214`, Hover gets steering/throttle at `:1109..1144,1215..1277`, and
Walk, Ship, and ground Teleport otherwise reach the generic path at
`:1278..1356` plus `movement_step.rs:734..1006`.

The six-times infantry override is present in every clean-HEAD spawn route:
`world_spawn.rs:182..197`, `:348..357`, and `:485..493`. It applies to the IDs
`GI`, `CONS`, `E1`, and `E2`. This is a common player-visible speed disparity,
not a scheduling-only concern.

Clean Rust also diverges before dispatch for the two native default-Teleport
registrations. `object_type.rs:1063..1067` sends an absent `Locomotor=` key to
`LocomotorKind::default_for_category`; `locomotor_type.rs:106..113` maps Vehicle
to Drive, so `DeathDummy` becomes Drive rather than Teleport. At
`ruleset.rs:1844..1876`, a registry ID without an INI section is logged but no
`ObjectType` is allocated, so `YDUM` has no Rust metadata object. Native creates
both UnitType rows and preserves Teleport. The zero shipped-stock producer result
lowers implementation priority, not the **DRIFT** verdict for custom/mod
placement.

### 6.4 Forced-track Rust corrections

- `production_spawn.rs:130..226`, `production_queue.rs:500..575`, and
  `war_factory_exit.rs:20..75` spawn/route factory products without the native
  selector-66/bib drive-out sequence or a temporary outer Drive for a Teleport
  primary.
- `docking/bunker_install.rs:336..341` chooses 67-70 from cell delta via
  `facing_from_delta`; native waits for and quantizes the unit's live FacingClass
  current value, then targets exact building coordinates.
- `docking/bunker_link.rs::{release_normal,release_sell_destroy}` omits native
  Power_On/Force71/speed/link-clear ordering.
- `miner_dock_sequence.rs:624..697` applies Force71 from a refinery `was_on_pad`
  reservation. That is a false native gate: stock zero-link refinery unloading
  does not establish the reciprocal bunker relation.
- Rust forced initialization zeros residual and always passes `use_short=false`;
  native Force preserves Drive residual and its short-selector byte while
  resetting the forced selector/cursor fields.

`tick_forced_drive_tracks @ movement_tick.rs:60..125` accepts any entity from the
supplied live order that has `forced_drive_track`, has no active low-bridge Tube,
and whose effective layer is not Air/Underground. It has no category or active-
locomotor-kind gate, and records processed IDs so ordinary movement stays skipped
even when the forced state clears. Selector 67-70 production exists
in `bunker_install.rs:293..348`; selector 71 exists only in the linked/on-pad
refinery interruption at `miner_dock_sequence.rs:594..701`. Normal refinery
completion explicitly does not force at `:1358..1418`. Normal/sell bunker exits
in `bunker_link.rs:98..154` omit the native selector-71 sequence.
Native `0x007101B3` belongs to TechnoClass::PerformDeploy's `IsLocomotor`
warhead-driven locomotor-replacement leaf, not MCV or Slave Miner type conversion.
Clean Rust parses/stores `WarheadType::is_locomotor` at
`src/rules/warhead_type.rs:84,182`, but `git grep is_locomotor -- src` at the
frozen commit finds exactly those two rows and no simulation consumer. Rust thus
has no gameplay locomotor-replacement leaf and no corresponding
`+0x70(-1, NullCoord)` reset. `Command::DeployMcv`, Slave Miner conversion, and
generic Stop are unrelated and are not treated as counterparts here.

The other proved selector-`-1` surface is also absent at runtime. Clean
`src/map/actions.rs:63..96` parses arbitrary numeric TriggerAction kinds, but
`src/sim/trigger_runtime.rs:25..36,222..306` implements only a small named subset
and drops kind `128` (`0x80`) through `_ => {}`. The native action-`0x80`
relocation helper and its explicit-coordinate plus `-1` reset at `0x004DF92D`
therefore have no Rust execution counterpart.

The current destination bridge is also broader than the proved native contexts:
`movement_commands.rs:122..219` activates Drive from a general
`destination_has_building` decision, whereas selector 66 is the specific land
factory mission and CMIN's proved inbound gate reads an accepted refinery radio
contact, DockUnload, the new CellClass, and Unit occupancy. The exact CMIN
contract is now closed; the Rust boolean does not represent it.

Its only fresh production caller found in the clean snapshot is the miner path at
`miner_system.rs:1049..1097`, which passes
`destination_has_building=false`; it therefore does not establish the intended
fresh Drive piggyback. Outbound CMIN comments acknowledge Drive authority but use
generic movement at `miner_system.rs:419..428,1471..1491`. Player-issued
harvesters likewise use generic ground movement, while non-harvester Teleport
types enter the later Teleport state (`world_commands.rs:158..187`).

### 6.5 Why Rust regression tests cannot certify this checkpoint

Existing Rust tests are useful regression ratchets, but they compare Rust to
Rust fixtures. They do not certify native ordering, especially where tests encode
the stale refinery-to-Force71 interpretation or a global phase. The eventual
acceptance surface must include executable native observations from Checkpoint E.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|--------------------------|--------|----------|--------------|
| active `gamemd.exe` identity | verified | `get_current_program_info(program="gamemd.exe")` | none |
| common Foot locomotor host | verified | `0x004DA530`; direct `0x004DA840..0x004DA8A0`; Checkpoint-A host report | full lifecycle effects belong to D |
| ordinary Drive entry/idle/track precedence | verified | `0x004B0500`; Checkpoint-B reports | exact effect ownership belongs to D |
| Walk wrapper and idle entry | verified | `0x0075AC80`; `0x0075AEC0` subaudit | full pathfinding/sub-cell parity outside scope |
| Hover Process/vertical ownership | verified | `0x00514310`; `0x00513D20`; `0x00514A10..0x00514AE4` | full numeric algorithm not reaudited here |
| Ship slope/track/idle ownership | verified | `0x0069FC10`; `0x006A05F0`; `0x006A1C80` | full numeric algorithm not reaudited here |
| Teleport class identity, Process/Is_Moving/Force slots | verified | vtable `0x007F5000`; RTTI `0x0080C178/0x00844538`; `0x007192F0`; `0x00718080`; `0x0055AC10` | none for entry identity |
| Teleport destination validator distinction | verified | function bodies `0x00718B70..0x007192BD` and `0x007192F0..0x00719BED` | none for entry identity |
| CMIN accepted-contact warp-arm chain | verified | `0x0073EE59`; `0x0065AA54..0x0065AA5A`; `0x004D92B9`; `0x0043CAB8`; `0x004D91EB`; `0x0074240F..0x007425C6`; IPiggyback gates `0x00742534/0x00742554` | runtime visual oracle remains Checkpoint E |
| late Foot piggyback restore placement | verified | `0x004DAE5F..0x004DAEFD`; Foot decompile | exact effects of every interface failure outside scope |
| Unit active Tube leaf | verified | `0x007363A4`; `0x007359F0`; clear `0x00735FF8` | lifecycle inside virtual callees deferred to D |
| Infantry active Tube leaf | verified | `0x0051BAB8`; `0x0051B350`; clear `0x0051BA8D` | lifecycle inside virtual callees deferred to D |
| all four direction-8 Tube producers | verified | Drive `0x004B1380`; Walk `0x0075B3FC`; Hover `0x0051515B`; Ship `0x006A0A48` | full malformed-tube behavior not reaudited |
| retail explicit `[Tubes]` prevalence | verified | bridge follow-up report: 385 payloads, zero sections | another retail build only if target changes |
| forced selector-66 callers | verified | `0x0044DFA1`, `0x0044E160`; factory mission table/caller body | lifecycle effects after exit belong to D |
| forced selector 67-70 caller | verified | `0x0045912D..0x004591AF` | none for selector/precedence |
| forced selector-71 callers | verified | `0x0045943B`, `0x00459760`; reciprocal writer `0x00459301/0x0045930F`; zero-link FSM `0x0073D630,0x0073E17F..0x0073E283` | none for stock role classification |
| forced selector -1 callers | verified | `0x004DF92D`, `0x007101B3` | stock-map use of trigger action `0x80` not established |
| selectors 64/65 stock callers | verified | complete direct/indirect caller census found none | external/mod invocation cannot be globally excluded |
| Harvest/Enter/Deploy dispatch order | verified | `0x005B3110`, `0x005B3198`, `0x005B31BA`; miner reports | leaf lifecycle effects belong to D |
| two-miner same-frame contention order | verified | live-order/refinery reports | runtime oracle still required for certification |
| native convoy link behavior | verified | `0x004B121D..0x004B125D`; Unit `+0x6C8` lifecycle report | none for formation-equivalence verdict |
| Team convoy/target helpers | verified | `0x006EC3A0`; `0x006E9050` | unrelated Team AI details outside scope |
| effective Infantry/Vehicle CLSID population | verified | read-only `rulesmd.ini` numeric-list/section parse | none for explicit bindings |
| DeathDummy/YDUM default and stock disposition | verified | `0x00672360`; `0x007480D0`; `0x00710C21..0x00710C4B`; `0x007123ED..0x00712437`; 16 MIX plus 54 loose-map scan | custom/mod runtime placement remains conditional |
| clean Rust DeathDummy/YDUM construction | verified | clean `object_type.rs:1063..1067`; `locomotor_type.rs:106..113`; `ruleset.rs:1844..1876` | implement native default/allocation semantics before conditional placement parity |
| dormant Tunnel/Mech/DropPod stock binding | verified | zero effective GUID bindings; Mech comment-only matches | mod/runtime behavior outside scope |
| clean-HEAD Rust order | verified | commit `cacc073f...`; blobs and lines in Section 6 | moving worktree intentionally excluded |
| exact movement lifecycle/effect ownership | deferred | Checkpoint-D scope | execute `GROUND_MOVEMENT_LIFECYCLE_EFFECT_OWNERSHIP` investigation |
| executable native oracle | deferred | Checkpoint-E scope | capture required native fixtures |
| malformed explicit Tube data | deferred | bounded valid producer/leaf audit did not exhaust invalid length/index/divisor states | run the OQ-27 malformed-Tube exhaustive slice if mod-data parity is pursued |
| first-post-load mixed Tube/forced/Teleport scheduling | deferred | static persistence evidence does not prove the first reached post-load branch | capture the OQ-28 native saves and first two object turns |

## 8. Open Questions — Final State of the Investigation Log

- `[RESOLVED] OQ-01 — Does every InfantryClass object use Walk? → No. The 65 registered InfantryTypes split into 60 Walk, 3 Teleport, and 2 JumpJet.` (evidence: `ini/rulesmd.ini:[InfantryTypes] plus each section's Locomotor=`)
- `[RESOLVED] OQ-02 — What is the exact VehicleTypes locomotor population? → 80 registrations: 52 Drive, 13 Ship, 4 Hover, 3 Teleport, 6 JumpJet, DeathDummy without a Locomotor key, and YDUM without a section.` (evidence: `ini/rulesmd.ini:[VehicleTypes] plus registered sections`)
- `[RESOLVED] OQ-03 — Is 0x00718B70 Teleport's per-tick Process? → No. It is the synchronous Head_To_Coord validator; vtable +0x40 points to StateMachineTick at 0x007192F0.` (evidence: `vtable 0x007F5000; 0x00718B70..0x007192BD; 0x007192F0..0x00719BED`)
- `[RESOLVED] OQ-04 — Are idle locomotors called only when a movement target exists? → No. Foot invokes active +0x40 whenever its five gates pass; the class decides what idle means.` (evidence: `0x004DA840..0x004DA880`)
- `[RESOLVED] OQ-05 — Does Hover vertical work belong to a later global tail? → No. Hover Process directly calls 0x00513D20 after its XY/cell/wake work and can reach it while idle.` (evidence: `0x00514A10..0x00514AE4`)
- `[RESOLVED] OQ-06 — Does Ship skip work while idle? → No. Its slope prefix and class tail are entered before/around the track decision even without translation.` (evidence: `0x0069FC10..0x006A0194`)
- `[RESOLVED] OQ-07 — Can a direction-8 producer immediately run the Unit/Infantry tube leaf in the same object turn? → No. The leaf gate was already passed; the new state is consumed on the next reached object turn.` (evidence: `Unit 0x007363A4; Infantry 0x0051BAB8; Drive producer 0x004B1380; Walk 0x0075B3FC; Hover 0x0051515B; Ship 0x006A0A48`)
- `[RESOLVED] OQ-08 — If an active tube leaf completes, may ordinary or forced movement resume in that same object turn? → No. Both category callers run their suffix and return from the leaf AI.` (evidence: `0x007363A4 onward; 0x0051BAB8 onward`)
- `[RESOLVED] OQ-09 — Are explicit multi-step Tube paths routine stock-map data? → No. The retail scan found zero [Tubes] sections in 385 payloads; auto shells are path_len=0.` (evidence: `BRIDGE_REMAINING_GAPS_FOLLOWUP_GHIDRA_REPORT.md lines 35-44, 193-206`)
- `[RESOLVED] OQ-10 — Is active low-bridge Tube movement TS Tunnel locomotion? → No. They are distinct mechanisms and activation populations.` (evidence: `Unit Tube 0x007359F0; Infantry Tube 0x0051B350; docs/research/bridges/00-system-models/BRIDGE_REMAINING_GAPS_FOLLOWUP_GHIDRA_REPORT.md; ini/rulesmd.ini Locomotor GUID bindings`)
- `[RESOLVED] OQ-11 — Which stock path owns selector 66? → The land WeaponsFactory output mission; already-Drive output uses the first callsite, while Tunnel/Teleport primaries get an outer Drive before the second callsite. Only Teleport is stock-bound.` (evidence: `0x0044DEDD..0x0044E160; CLSID bytes 0x007E9A30/0x007E9A50/0x007E9A90; mission slot-26 bytes`)
- `[RESOLVED] OQ-12 — How are bunker selectors 67-70 chosen? → From the unit's live current-facing quantization after rotation finishes, not cell delta.` (evidence: `0x00459122..0x004591AF`)
- `[RESOLVED] OQ-13 — Is selector 71 the normal refinery/miner unload exit? → No. Both helpers require the reciprocal Unit/Building +0x2E4 relation used by bunker lifecycle; healthy stock zero-link refinery unload does not Force71.` (evidence: `0x004593A0; 0x004595C0; 0x00459301/0x0045930F; 0x0044B797..0x0044B7A3; 0x0073D630; 0x0073E17F..0x0073E283`)
- `[RESOLVED] OQ-14 — Are selectors 64 and 65 stock-reachable? → No stock caller was proven by the exhaustive direct and indirect +0x70 census.` (evidence: `true callsites 0x0044DFA1, 0x0044E160, 0x004591AF, 0x0045943B, 0x00459760, 0x004DF92D, 0x007101B3 from the 97-site indirect-call review`)
- `[RESOLVED] OQ-15 — What does selector -1 mean at the proved callers? → It clears/resets a forced destination for relocation or locomotor replacement; it is not a special TurnTrack row.` (evidence: `0x004DF92D; 0x007101A4..0x007101B3`)
- `[RESOLVED] OQ-16 — Can a miner mission mutation feed movement in the same native object turn? → Yes. Mission Dispatch runs the concrete handler before Foot reaches active locomotor Process.` (evidence: `0x005B3060; 0x005B3110; 0x005B3198; 0x005B31BA; 0x004DA877`)
- `[RESOLVED] OQ-17 — Does releasing miner A globally promote miner B? → No. B advances only through B's own later due Mission_Enter retry, potentially in the same frame if its live-vector slot follows A.` (evidence: `TWO_MINER_ONE_REFINERY_ZERO_LINK_HANDOFF_FRAME_ORDER_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-18 — Is SMIN part of the Harvester=yes miner family? → No. It is Drive and DeploysInto=YAREFN, but has no Harvester=yes key.` (evidence: `ini/rulesmd.ini:[SMIN]`)
- `[RESOLVED] OQ-19 — Is Rust sync_formation_speeds equivalent to native convoy/team logic? → No. Native propagates a leader fraction through explicit Unit +0x6C8 links inside that Drive Process; Rust globally applies a command-group minimum after translation.` (evidence: `0x004B121D..0x004B125D; clean movement_tick.rs:1750,1861; components.rs:248`)
- `[RESOLVED] OQ-20 — Can ground Teleport remain in a later global special pass after the atomic ground migration? → No. Stock Unit/Infantry Teleport Process and piggyback restore are ordered inside their Foot slot, so leaving them global changes later-object visibility.` (evidence: `0x004DA530; 0x007192F0; stock Teleport roster; clean world/mod.rs:2303..2357`)
- `[RESOLVED] OQ-21 — Are Tunnel, Mech, and DropPod stock-instantiated because their classes compile? → No. They are runtime-recognized/mod-bindable, but effective stock INI has no binding; Mech appears only in comments.` (evidence: `ini/rules.ini; ini/rulesmd.ini GUID search`)
- `[RESOLVED] OQ-22 — Which call arms the normal CMIN inbound self-teleport, and is the old object NavCom? → The accepted refinery's synchronous radio-0x12 call to Set_Destination is the arming call. The old object at 0x0074240F is Contact_With_Whom(0), populated by accepted HELLO, not Foot+0x5A4 NavCom. Immediate restoration from Drive additionally requires both Is_Piggybacking and Is_Ok_To_End.` (evidence: `0x0065AD30; 0x0073EE59; 0x0065AA54..0x0065AA5A; 0x004D92B9; 0x0043CAB8; 0x004D91EB; 0x0074240F..0x007425C6; 0x00742534; 0x00742554`)
- `[RESOLVED] OQ-23 — What exact locomotor defaults and stock disposition result for DeathDummy and missing-section YDUM? → Native creates both UnitType metadata objects and preserves the Teleport CLSID constructor default, but a scan of all 16 retail MIX archives plus 54 loose maps found no shipped-stock instance producer. They are excluded from the realized stock Foot population, while custom/mod instantiation remains conditional. Clean Rust drifts by defaulting DeathDummy to Drive and allocating no YDUM ObjectType.` (evidence: `rulesmd.ini:1111,1147,9653..9654; 0x00672360; 0x007480D0; 0x00710C21..0x00710C4B; 0x007123ED..0x00712437; 0x007354CE..0x00735518; retail producer scan; clean object_type.rs:1063..1067; locomotor_type.rs:106..113; ruleset.rs:1844..1876`)
- `[DEFERRED] OQ-24 — Can a tube leaf virtual callee kill, conceal, or unregister the owner before the leaf suffix returns?` (category: `requires-different-system-context`; reason: Checkpoint D owns lifecycle visibility across all movement effect callees; next-step-if-pursued: trace each Unit/Infantry tube exit/blocked virtual through UnInit, occupancy, and live-vector mutation.)
- `[DEFERRED] OQ-25 — Which arrival, crush, scatter, sound, gate, wall, factory-contact, and cache mutations must be visible before the next object?` (category: `requires-different-system-context`; reason: these are the explicit Checkpoint-D targets rather than population/entry questions; next-step-if-pursued: execute the approved lifecycle/effect ownership phase using the exact category entries in this report.)
- `[DEFERRED] OQ-26 — Does a retail runtime trace reproduce every static same-turn order and state byte in this report?` (category: `needs-runtime-debugger`; reason: no executable native oracle exists yet; next-step-if-pursued: instrument the named Checkpoint-E fixtures at Mission Dispatch, class Process entry/exit, tube/forced leaves, and live-vector boundaries.)
- `[DEFERRED] OQ-27 — What happens for malformed explicit Tube data with a zero divisor or invalid path cursor in every producer?` (category: `bounded-cost-too-high`; reason: the bounded subaudits proved the live producer/leaf precedence but did not exhaust all malformed mod-map data across four classes; next-step-if-pursued: construct a dedicated malformed-Tube exhaustive slice for zero path length, invalid index, cursor overflow, and blocked exits.)
- `[DEFERRED] OQ-28 — Can save/load resume a mixed active Tube, forced Drive, or Teleport piggyback state with a different first reached branch?` (category: `needs-runtime-debugger`; reason: persistence fields are documented separately but exact first-post-load scheduling needs executable state capture; next-step-if-pursued: capture one native save per mixed state and compare the first two live-object turns after load.)

The deferred items do not leave a silent population or first-entry guess. They
belong to the explicitly separate lifecycle, malformed-mod-data,
persistence/save-load, or executable-oracle scopes.

## 9. Zero-Add Pass and Cold Checks

After all late CMIN, default-locomotor/INI-default, `active_tube`,
`IsLocomotor`, TriggerAction-128, and clean-Rust handoff corrections were
integrated, a final zero-add
pass re-read the complete OQ log, the primary Foot/Unit/Infantry and locomotor
entries, the immediate and deferred CMIN restoration branches, forced/Tube
precedence, the stock roster/default conclusions, and the frozen clean-Rust
comparison. It added zero new in-scope population or precedence questions. The
five deferred entries remained exactly lifecycle (`OQ-24` and `OQ-25`), the
executable runtime oracle (`OQ-26`), malformed Tube data (`OQ-27`), and
persistence/save-load (`OQ-28`).

Cold spot checks performed independently of earlier prose:

1. Teleport RTTI/vtable/Process identity was rebuilt from raw bytes and the two
   adjacent function boundaries.
2. Selector-66, 67-70, 71, and `-1` arguments were re-read from assembly rather
   than local labels.
3. Foot's active locomotor call, immediate active guard, and late piggyback
   restoration were re-read in one body.
4. The effective `rulesmd.ini` registration counts were regenerated from numeric
   list rows and section `Locomotor=` keys.
5. Clean-Rust conclusions were regenerated with `git show` from commit
   `cacc073f...`, never from the moving working tree.

Closure uses three independent read-only exact-snapshot cold reviews: native,
INI, and Tube evidence; clean-Rust and implementation-handoff evidence; and
schema, citations, and internal consistency. Their SHA-bound verdicts are
reported in the task handoff rather than written back into this file, so the
reviewed bytes remain frozen.

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|-------------------|----------|--------------------|-----------------------|--------------------------------|---------------------|------------------|
| one live Foot slot owns mission then current locomotor Process | `0x005B3060`; `0x004DA530`; `0x004DA877` | object AI and movement are separate global phases; only Unit enters the current bracket, Infantry does not, and Unit uses `derived_mission` rather than native mission handlers | `src/sim/world/techno_ai.rs`; movement dispatcher | prepare the complete Unit/Infantry one-object category dispatch without activating production early | two movers in live order: first mission sets movement and first Process completes before second mission begins; Walk/Teleport Infantry also enter their native host | no vehicle-only production flip, Unit-only bracket, or handled-ID bridge |
| ground Teleport is part of atomic population | Teleport vtable `0x007F5000`; Process `0x007192F0`; stock type lists | Teleport and restore run globally after ground | `teleport_movement.rs`; `movement/mod.rs`; `world/mod.rs` | provide per-object Teleport phase and late-Foot restore seams; retire their global authority at the atomic flip | CLEG and CMIN fixtures prove phase/occupancy/restore before the next live object | do not leave Teleport in Phase 2 after moving peer ground objects per-object |
| idle class Process still enters | Foot host plus Walk/Hover/Ship/Teleport bodies | the ordinary target-based collector skips no-`MovementTarget` Drive/Walk/Ship/Teleport; parked Hover receives only the detached global vertical tail, not a full Hover class Process | category one-object helpers | call the class handler based on native gates, not target existence | parked Walk, Hover, Ship, Teleport each produce the exact native no-op/tail trace | no synthetic `MovementTarget` merely to force entry, and do not treat the detached Hover tail as full Process parity |
| Hover vertical belongs inside Hover Process | `0x00514ACB -> 0x00513D20` | global all-Hover tail | `movement/hover.rs`; `movement_tick.rs` | move vertical state into prepared one-Hover invocation at verified order | parked Hover vertical changes before the next live object | do not preserve global post-finalization vertical authority |
| Ship owns slope/track/wake order | `0x0069FC10`; `0x006A05F0`; `0x006A1C80` | generic ground translation plus a later global stable-ID water-wake scan | Ship one-object surface needed | preserve slope prefix, track mode order, idle tail, wake ownership, and class completion | idle and active Ship traces match ordered event/state tuples | do not alias Ship to Drive/generic straight-line motion or retain global wake authority |
| active Tube leaf preempts ordinary Process for the entire object turn | Unit/Infantry gates and return suffixes; native budget at Section 3.7 | global category-neutral Tube advances one whole path cell and can finish then fall through | `tube_movement.rs`; future Unit/Infantry dispatcher | route one active leaf, preserve Unit `ftol(Type speed * 1.5)` versus Infantry raw-Type-speed budgets, residual carry, and at most one cursor increment per object turn, then stop that turn even on completion | paired Unit/Infantry fixtures start on the same path with the same nonzero residual and prove different native budgets, carried residual, and no more than one cursor increment; a completion with forced/ordinary state coexisting advances neither until the next turn | do not use category-neutral one-cell stepping or resume another movement family in the same bulk tick |
| forced initializer happens at caller point; Drive consumes later | seven callsites; Force report | global forced prepass; mismatched callers; initializer zeros residual and production callers hard-code `use_short=false`; no `IsLocomotor` gameplay consumer/replacement leaf; TriggerAction 128 parsed but discarded | `drive_track.rs`; production/docking; `map/actions.rs`; `trigger_runtime.rs`; warhead-impact locomotor replacement | separate command initialization from later one-Drive Process; preserve Drive residual and the short-selector byte while resetting only the forced selector/cursor fields; add both proved selector-`-1` owners before activation | a preloaded residual and each short-selector-byte state survive Force while selector/cursor reset; earlier factory Force66 affects a later unit slot in live order; a Magnetron-style `IsLocomotor` fixture performs replacement plus `-1/NullCoord` reset at impact; action 128 relocates to its explicit coordinate and applies its caller-time `-1` reset | do not make Force a scheduler, zero preserved Drive state, map PerformDeploy to MCV/Slave Miner conversion, or silently discard action 128 |
| selector 66 land-factory exit | `0x0044DFA1`; `0x0044E160` | missing | `production_queue.rs`; factory mission seam | implement target/order and temporary Drive piggyback before eventual activation | Drive output uses the direct call; stock Teleport output gets an outer Drive, uses 66 and speed `0.5`, then restores its primary when allowed | do not treat Hover as the compared CLSID or reduce exit to optional rally movement |
| selectors 67-70 use live facing | `0x0045912D..0x004591AF` | Rust uses cell delta | `docking/bunker_install.rs` | wait for rotation and map live facing to native selector | four facing fixtures choose 67/68/69/70 exactly | do not use building-cell delta as selector authority |
| selector 71 requires reciprocal bunker link | `0x004593A0`; `0x004595C0` | refinery reservation false-positive and bunker releases omit Force | `docking/bunker_link.rs`; `miner_dock_sequence.rs` | gate 71 on the proven relation and preserve operation order | bunker release uses 71; healthy zero-link CMIN/HARV unload never does | remove stale refinery-pad parity assertion; do not generalize link semantics |
| miner runs one snapshot/state in its own mission slot | Harvest/Enter/Deploy dispatch plus two-miner report | eligible non-dying, non-Slave Miner-component entities run snapshot-all/process-all/miner-state-and-derived-visual-writeback-all late | `src/sim/miner/`; mission host | extract a one-live-miner handler and retire batch authority only at atomic activation | A then B contention fixture proves B sees A only when B's later due slot runs | no batch writeback or separate miner RNG/radio epoch |
| accepted CMIN radio contact arms Teleport | HELLO/contact write `0x0073EE59/0x0065AA54`; CAN_DOCK/0x12 `0x004D92B9/0x0043CAB8`; Set_Destination `0x004D91EB/0x0074240F`; immediate gates `0x00742534/0x00742554`; same-Mission reissue `0x004D93FC..0x004D941D` | current miner destination bridge passes `destination_has_building=false` and does not reproduce contact-driven restoration | miner radio/dock plus locomotor authority | preserve contact-zero as the old-object predicate, exact Unit-occupancy gate, ordered Is_Piggybacking then Is_Ok_To_End requirements, deferred restoration, and the same-Mission/subsequent reissue order | `<=50` accepted empty cell with both IPiggyback gates true arms Teleport; separately fail each gate and prove Drive processes this turn; `>50` stages by Drive then retries; an infantry-only accepted cell still arms Teleport while a Unit-occupied accepted cell stays Drive; late-restored Teleport is armed only by a later reissue | do not substitute NavCom, generic `destination_has_building`, distance alone, any-occupant blocking, or Is_Ok_To_End alone for the proved predicates |
| native convoy is explicit Drive link propagation | `0x004B121D..0x004B125D` | global `group_id` minimum | `movement_tick.rs`; `components.rs`; commands | remove `sync_formation_speeds` from parity authority; model convoy only from verified links if needed | mixed-speed command group without native links receives no global cap | do not rename the current group-min pass as convoy parity |
| stock GI/Conscript speed has no six-times testing override | stock type speed plus native GetCurrentSpeed contract; clean spawn audit | all map/runtime construction branches multiply `GI/CONS/E1/E2` movement speed by six | `src/sim/world/world_spawn.rs` | remove the temporary override in the eventual verified movement implementation and source speed from the exact type/GetCurrentSpeed path | map-spawn and both runtime-spawn GI/E1 and CONS/E2 fixtures use unmultiplied stock speed before locomotor math | do not preserve the override as a balancing constant or hide it inside the dispatcher |
| missing/no-section VehicleType rows retain native Teleport default | constructor/read/load evidence in Sections 2 and 4; zero shipped producer scan | DeathDummy defaults to Drive, YDUM receives no ObjectType metadata, absent `Speed=` becomes zero, and all three spawn routes omit `LocomotorState` unless speed is positive | `src/rules/object_type.rs`; `src/rules/locomotor_type.rs`; `src/rules/ruleset.rs`; `src/sim/world/world_spawn.rs` | allocate registered rows before optional section overlay, preserve the native Teleport constructor default when `Locomotor=` is absent, and retain an idle Teleport locomotor for a placed zero-speed VehicleType | DeathDummy metadata resolves Teleport; YDUM metadata exists and resolves Teleport; custom placement of either zero-speed row still owns an idle Teleport locomotor and enters the conditional ground-Teleport dispatcher | do not use zero shipped producers or zero speed to call the mechanism parity-correct, omit locomotor ownership, or declare the type structurally unreachable |
| owner-specific completion/effects remain unresolved | category bodies; Checkpoint-D plan | generic finalizer/deferred lists | movement/lifecycle/cache surfaces | do not activate until D assigns every effect and next-object visibility | crush, arrival, scatter, gate/factory, wall and occupancy fixtures | no claim that clean final positions prove mechanism parity |

### Stale Docs / Follow-up Docs

- `docs/plans/2026-07-20-per-object-ground-movement-drive-process-design.md`:
  replace the exclusion of Teleport from the atomic ground population with:
  "Ground Unit/Infantry Teleport users are prepared and activated with the
  atomic dispatcher because their Process and piggyback restore are Foot-slot
  work. Air/projectile special locomotors remain separate."
- `docs/contracts/2026-07-20-ground-drive-process-track-stepping-implementation-contract.md`:
  add ground Teleport to the population/effect blocker and acceptance matrix.
- `WALK_LOCOMOTION_CLASS_GHIDRA_REPORT.md`: replace "every/~64 infantry" with
  the exact `60 Walk / 3 Teleport / 2 JumpJet`; delete any statement that active
  tube omission is parity-correct.
- `LOW_BRIDGE_TUBEMOVEMENT_FINAL_Z_INTERPOLATION_GHIDRA_REPORT.md`: state that
  `0x0051BF00` is mid-body; Infantry AI entry is `0x0051BAB0`.
- `TS_DORMANT_LOCOMOTORS_GHIDRA_REPORT.md`: scope dormant language to no stock
  instantiation; the classes remain runtime-recognized/mod-bindable.
- Teleport/chrono miner docs: use `0x007192F0` as per-tick Process and
  `0x00718B70` only as Head_To_Coord's synchronous validator. Replace every
  statement that `0x0065AD30` reads old NavCom with: "It is
  `RadioClass::Contact_With_Whom(0)` and reads the accepted HELLO contact from
  `RadioClass+0xE4`; the synchronous accepted-cell radio-0x12 call is the proved
  CMIN arming Set_Destination call."
- `DRIVE_TRACK_TABLES_DEEP_DECODE.md`: remove "ForceTrack has zero callers /
  selectors 64-71 dead."
- `RALLY_POINTS_AND_UNIT_SPAWNING.md`: replace generic creation wording with the
  selector-66 land-factory body, including temporary Drive piggyback for
  Tunnel/Teleport; only Teleport is stock-bound. Any Hover wording is a shifted
  CLSID error (`0x007E9A40` is Hover, while the compared `0x007E9A50` is Tunnel).
- `BRIDGE_LOCOMOTOR_WALK_DROPPOD_TELEPORT_GHIDRA_REPORT.md`: do not call
  `0x00718B70` the per-tick Teleport Process and do not generalize Infantry to
  Walk.
- Any locomotor GUID table mapping `0x007E9A40` to Walk or `0x007E9A50` to
  Hover is shifted and stale. They are Hover and Tunnel respectively.
- Generic wording that "Teleport wraps Drive" must name the context. In the
  selector-66 factory path, Drive is outer and stores Teleport underneath.
- `MISSION_REPAIR_AND_PRODUCE_GHIDRA_REPORT.md`: `+0x58` is Power_On;
  `Force_Track` is `+0x70`.
- refinery/miner Force71 docs/tests: constrain selector 71 to the reciprocal
  bunker link and remove normal zero-link refinery claims.

## 11. Negative Facts / Do Not Implement

- Do not equate InfantryClass with Walk.
- Do not omit ground Teleport from the future atomic population.
- Do not call `0x00718B70` Teleport's per-tick Process.
- Do not call `0x0065AD30` a NavCom/destination getter or use Foot `+0x5A4` as
  the old-object input to the Teleporter predicate.
- Do not model CMIN return warp as a distance-only branch. Distance controls
  whether HELLO is attempted now or after Drive staging; accepted refinery
  contact plus the new Cell/occupancy gates control Teleport selection.
- Do not model immediate Drive-to-Teleport restoration with `Is_Ok_To_End` alone;
  `Is_Piggybacking` must pass first. The distinct late Foot restore tests only
  `Is_Ok_To_End` at `0x004DAEA8` and does not arm Teleport by itself.
- Do not treat a missing Rust `MovementTarget` as a native Process gate.
- Do not run Hover vertical, Teleport phase, piggyback restore, or formation
  mutation as a later population-wide service without positive native proof.
- Do not resume ordinary/forced movement in the same object turn after a Tube
  leaf returns.
- Do not conflate TubeClass low-bridge movement with TS Tunnel.
- Do not infer that compiled locomotor classes are stock-instantiated.
- Do not call DeathDummy or YDUM structurally non-Foot or impossible to place.
  They are unproduced in the shipped-stock corpus, but custom placement would
  inherit Teleport.
- Do not substitute the Rust Vehicle-to-Drive category default for native missing
  `Locomotor=` semantics or omit a registered type merely because its section is
  absent; native preserves Teleport metadata for both rows.
- Do not use selector 71 for healthy stock refinery unloading.
- Do not call Teleport vtable `+0x70` a working Drive ForceTrack method; it is a
  no-op slot.
- Do not select bunker rows 67-70 from cell delta.
- Do not apply fresh-normal cursor rules to forced or accepted-chain starts.
- Do not call Rust regression fixtures a native oracle.
- Do not activate any one category in production before ground Teleport and all
  Checkpoint-D effects are prepared for the same atomic flip.

## 12. Sources

### 12.1 Direct read-only Ghidra evidence

All calls used `program="gamemd.exe"`.

- `get_current_program_info` for target identity.
- `get_function_by_address`, `decompile_function`, and/or
  `disassemble_bytes`: `0x004DA530`, `0x007360C0`, `0x0051BAB0`,
  `0x0075AC80`, `0x0075AEC0`, `0x00514310`, `0x00513D20`, `0x00515ED0`,
  `0x0069FC10`, `0x006A05F0`, `0x006A1C80`, `0x007192F0`, `0x00718B70`,
  `0x007359F0`, `0x0051B350`, `0x005B3060`, `0x0073E5E0`, `0x004D9290`,
  `0x004D8FB0`, `0x0043C2D0`, `0x0065A970`, `0x0065AD30`, `0x00741970`,
  `0x0073D630`, `0x006EC3A0`, `0x006E9050`, `0x0044DCB9`, `0x00458E50`,
  `0x004593A0`, `0x004595C0`, `0x00710000`, `0x00672360`, `0x007480D0`,
  `0x007470D0`, `0x00710AF0`, `0x00712170`, `0x007353C0`, `0x0073D630`.
- exact assembly windows: `0x004DA840..0x004DA8A0`,
  `0x004B121D..0x004B125D`, `0x0044DF85..0x0044DFB0`,
  `0x0044DEC0..0x0044DFB0`, `0x0044DFAD..0x0044E165`,
  `0x0044E145..0x0044E170`, `0x00459120..0x004591B5`,
  `0x00459420..0x0045945F`, `0x00459740..0x0045977F`,
  `0x00710190..0x007101CF`, `0x004DAE5F..0x004DAEFD`,
  `0x0073EDF0..0x0073EE7F`, `0x0065AA20..0x0065AA6F`,
  `0x0043CA10..0x0043CACF`, `0x004D9180..0x004D921F`,
  `0x004D93FC..0x004D941D`, `0x004D950E..0x004D965D`,
  `0x00742400..0x007425C6`, `0x00710C10..0x00710C60`,
  `0x007123D0..0x0071244F`, `0x007354B0..0x0073552F`,
   `0x0073E17F..0x0073E283`, `0x0044B797..0x0044B7A3`.
- INI/default verification windows:
  `0x00710C21..0x00710C4B`, `0x007123ED..0x00712437`,
  `0x00713FE2..0x00713FF6`, `0x00711484`,
  `0x0074769F..0x007476B9`, `0x0074710F`,
  `0x00713180..0x00713239`, `0x00710D39..0x00710D57`,
  `0x00713264..0x00713297`, `0x00710D61`,
  `0x00714570..0x00714584`, `0x00711082`,
  `0x00460A72..0x00460A8C`, `0x0045E139`,
  `0x0046051A..0x00460545`, `0x0045DEB6`,
  `0x00460F9C..0x00460FDF`, `0x0045DECE..0x0045DEDA`,
  `0x0046093A..0x00460954`, `0x0045E0CC`,
  `0x0066FFF6..0x0067001B`, and `0x00666846`.
- `read_memory`: Walk/Hover/Ship vtable-plus-COL windows
  `0x007F69F4/0x007EACF8/0x007F2D88`, their COLs
  `0x0080D240/0x00803228/0x008093A0`, Teleport vtable/COL `0x007F4FF0`,
  CLSID `0x007E9A90`, COL `0x0080C178`; Drive/Hover/Tunnel/Teleport CLSIDs at
  `0x007E9A30/0x007E9A40/0x007E9A50/0x007E9A90`;
  `inspect_memory_content` at `0x00847BF0/0x008254B8/0x0083F880/0x00844538`
  for the four TypeDescriptors.
- forced-caller census: direct xrefs plus all 97 candidate indirect
  `CALL [reg+0x70]` sites, narrowed to the seven true callsites in Section 3.8.

### 12.2 Research documents reconciled

- `TECHNO_MISSION_MOVE_FOOT_LOCOMOTOR_HOST_CONTRACT_GHIDRA_REPORT.md`
- `OBJECT_PASS_DRIVE_INVOCATION_SCHEDULING_GHIDRA_REPORT.md`
- `FOOTCLASS_GET_CURRENT_SPEED_EXACT_GHIDRA_REPORT.md`
- `DRIVE_RAWTRACK_METADATA_INITIALIZER_RECONCILIATION_GHIDRA_REPORT.md`
- `WALK_LOCOMOTION_CLASS_GHIDRA_REPORT.md`
- `E2_STATIC_WALL_WALK_RETRACE_20260720.md`
- `HOVER_LOCOMOTION_CLASS_GHIDRA_REPORT.md`
- `SHIP_LOCOMOTION_CLASS_GHIDRA_REPORT.md`
- `docs/research/miner/CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md`
- `CHRONO_MINER_WARP_TRIGGER_GHIDRA_REPORT.md`
- `docs/research/miner/CHRONO_MINER_SET_DESTINATION_GATE_GHIDRA_REPORT.md`
- `docs/research/miner/CMIN_CLOSE_FAR_RETURN_SPLIT_CHRONOHARVTOOFARDISTANCE_GHIDRA_REPORT.md`
- `CHRONO_MINER_FORCE_TRACK_0X47_EXIT_NAVCOM_STEP_GHIDRA_REPORT.md`
- `BUNKER_SERVICEDEPOT_0X2E4_RECIPROCAL_LINK_TEARDOWN_GHIDRA_REPORT.md`
- `TWO_MINER_ONE_REFINERY_ZERO_LINK_HANDOFF_FRAME_ORDER_GHIDRA_REPORT.md`
- `RADIO_LINK_REFINERY_DOCK_STATE_MACHINE_DOC_CONFLICT_AUDIT_GHIDRA_REPORT.md`
- `UNIT_0X6C8_CONVOY_LINK_LIFECYCLE_RESWARM_20260528.md`
- low-bridge reports under `docs/research/bridges/04-locomotion-height-tubes/`
- `docs/research/bridges/00-system-models/BRIDGE_REMAINING_GAPS_FOLLOWUP_GHIDRA_REPORT.md`

### 12.3 INI, retail data, plan, and Rust

- `ini/rulesmd.ini`, with `rules.ini` checked for base/fallback and comment-only
  dormant GUID references.
- verified retail map scan recorded by
  `BRIDGE_REMAINING_GAPS_FOLLOWUP_GHIDRA_REPORT.md`.
- read-only, case-insensitive delimiter scan of all 16 configured retail MIX
  archives plus 54 loose maps for `DeathDummy` and `YDUM` instance/trigger
  producers; zero valid producers found.
- `docs/plans/2026-07-20-ground-movement-atomic-flip-readiness-investigation-plan.md`.
- `docs/plans/2026-07-20-per-object-ground-movement-drive-process-design.md`.
- `docs/contracts/2026-07-20-ground-drive-process-track-stepping-implementation-contract.md`.
- clean Rust commit and blobs listed in Section 6.1.

## 13. Final Checkpoint Statement

Checkpoint C is **research-closed for the bounded population and first-entry
precedence contract**. The closure includes a required correction to its original
population list: stock ground Teleport Unit/Infantry types must be prepared with
the atomic per-object migration.

Production remains **blocked**. The design and contract must first absorb the
Teleport correction; Checkpoint D must assign every gameplay-bearing lifecycle,
occupancy, arrival, crush/scatter, sound, gate/factory/wall, and cache mutation;
and Checkpoint E must provide executable native fixtures. No Rust behavior,
Cargo state, Ghidra state, staging, or commit was changed by this investigation.
