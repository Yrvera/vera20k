# Phase 3 Team Base-Defense Suspension Expiry — Ghidra Research Report

**Address(es):** `0x006E9140` (Team update/expiry owner), `0x006EC250` (base-defense suspension writer)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** the active-YR Team state written by the House base-defense suspension transaction: eligibility, arm/rearm writes, exact frame timer gate and expiry boundary, same-call state after expiry, constructor defaults, raw save/load coverage, CRC coverage, and the current Rust delta
**Non-Scope:** the remainder of `TeamClass::AI`, complete recruitment and TaskForce admission, ScriptType opcode semantics beyond the timer gate, the rest of the House base-defense responder, Railgun/LaserDraw/Sonic Wave/destroyable-cliff behavior, and all Tiberian Sun legacy
**Confidence:** High for the claimed slice
**Active in YR:** Yes. The writer is reached from the normal YR damage receivers through the active House base-defense responder, and retail `rulesmd.ini` supplies `SuspendPriority=1` and `SuspendDelay=2`.

## 1. Overview

An eligible nonhuman House base-defense response walks live Teams in creation order and suspends every Team owned by that House whose signed TeamType priority is below `[General] SuspendPriority`. Suspension removes every current member, sets three Team bytes, and arms a frame timer from `[General] SuspendDelay`; `TeamClass::AI` checks this timer before every other Team branch and returns immediately while time remains.

Expiry is not a decrement-to-zero transaction. It is a signed, wrapping comparison against the global binary frame counter. At exact equality the same Team update clears only the active byte `+0x83`, leaves the raw timer pair and the two response latches intact, and immediately continues into the ordinary `+0x7D` recruitment/status path.

## 2. Class Layout / Key Offsets

The Team owner identity is independently bound by the RTTI walk: `vtable 0x007F4730 - 4` contains COL pointer `0x0080BE58`; `COL+0x0C` contains TypeDescriptor pointer `0x00842D68`; the TypeDescriptor name bytes are `.?AVTeamClass@@`. The `vtable+0x5C` entry at `0x007F478C` contains `0x006E9140` (verified via `read_memory 0x007F472C`, `read_memory 0x0080BE58`, `read_memory 0x00842D68`, and `read_memory 0x007F4730`; body verified via `decompile_function`/`disassemble_function 0x006E9140`).

| Offset | Width / signedness | Verified role in this slice | Evidence |
|---|---:|---|---|
| `Team+0x24` | pointer | TeamType pointer; the writer reads signed priority at `TeamType+0xB4` | `disassemble_function 0x006EC250`, `0x006EC287-0x006EC294` |
| `Team+0x2C` | pointer/token | owning House identity compared with the responder-supplied House | `decompile_function 0x006EC250`, `0x006EC27C-0x006EC285` |
| `Team+0x54` | pointer | first member in the linked member list; writer repeatedly removes this first member until null | `decompile_function`/`disassemble_function 0x006EC250`, `0x006EC296-0x006EC2AE` |
| `Team+0x64` | signed 32-bit frame | response suspension start frame | writer `0x006EC2D7-0x006EC2E0`; consumer `0x006E9153-0x006E9168`; constructor `decompile_function 0x006E8A90` |
| `Team+0x68` | 32-bit indeterminate payload | middle dword written from a non-dominating stack slot; not read by the expiry owner and omitted from the Team CRC sequence | writer `0x006EC2D3-0x006EC2E2`; CRC bytes `read_memory 0x006EC5A0` decoded through first `RET 4` at `0x006EC720` |
| `Team+0x6C` | signed 32-bit duration | low dword of the chopped `SuspendDelay*900.0` conversion | writer `0x006EC2C2-0x006EC2E5`; consumer `0x006E9156-0x006E916E` |
| `Team+0x7D` | byte/bool | response/recruitment status latch; set by suspension, tested immediately after expiry | writer `0x006EC2B3`; Team update `0x006E917B-0x006E918B` |
| `Team+0x7E` | byte/bool | second response/recruitment status latch; set by suspension, not cleared by the timer block | writer `0x006EC2B0`; follow-on helper `0x006EA467` |
| `Team+0x83` | byte/bool | response suspension active gate | writer `0x006EC2DA`; first Team update read `0x006E9149`; sole expiry clear in the mapped block `0x006E9174` |

The `+0x68` conclusion is deliberately role-scoped. It is not called “zero” or a second duration word: the constructor does not initialize it, the writer stores `[ESP+0x1C]` without a dominating write in `0x006EC250`, Team expiry does not read it, and Team CRC skips it. Raw save/load nevertheless carries it because the native Team block is serialized wholesale.

## 3. Core Logic

### 3.1 Eligibility and arm/rearm transaction

`0x006EC250` is called exactly once from the mapped base-defense responder at `0x007081A9`; `get_function_callers 0x006EC250` and `get_xrefs_to 0x006EC250` both identify only `0x00708080`. The caller loads `Rules+0x14E0` into `ECX` and the attacked House identity into `EDX` before the call (`disassemble_function 0x00708080`, `0x00708198-0x007081A9`).

For every live Team array entry in ascending array index:

1. Null entries are skipped.
2. `Team+0x2C` must equal the supplied House identity.
3. The signed TeamType priority at `(Team+0x24)+0xB4` must be strictly less than signed `SuspendPriority`. The assembly uses `CMP` followed by signed `JGE`; equality is excluded.
4. The writer repeatedly passes the current `Team+0x54` first member to `TeamClass::Remove_Member( member, -1, 0 )` and reloads `+0x54` after every call. Removal order is therefore linked-list order, not entity ID or type order.
5. It writes `+0x7E=1`, then `+0x7D=1`.
6. It loads `[General] SuspendDelay`, multiplies it by the image literal `900.0` at `0x007E27F8`, and calls `Math__ftol @ 0x007C5F00`. The helper uses x87 control word `0x0E7F`: round toward zero with 53-bit precision. The 64-bit result is returned in `EDX:EAX`, but this writer stores only `EAX` at `+0x6C`.
7. It writes `+0x83=1`, then `+0x64=g_CurrentFrameCounter`, then `+0x68=[ESP+0x1C]`, then `+0x6C=EAX`, in that exact order.

There is no `+0x83` admission guard. Calling the writer again on an already suspended eligible Team drains any members acquired since the prior call and restarts the full delay from the new current frame. This is rearm/overwrite, not “keep the earlier deadline” (verified via the complete `0x006EC250` body).

### 3.2 Exact update and expiry state machine

The following is behavior pseudocode, not a C reconstruction:

```text
if active_83:
    duration = signed_i32(Team+0x6C)
    start = signed_i32(Team+0x64)

    if start == -1:
        remaining = duration
        if remaining != 0:
            return remaining_as_u32
    else:
        elapsed = wrapping_i32(current_frame - start)
        if elapsed < duration:             # signed strict comparison
            remaining = wrapping_i32(duration - elapsed)
            if remaining != 0:
                return remaining_as_u32

    active_83 = false

if latch_7d:
    if recruitment_status_helper() == 0:
        return 0

continue the same Team update
```

Tiny but load-bearing details from `0x006E9149-0x006E918B`:

- `+0x83` is the first gameplay-state read after the function prologue. Suspension therefore precedes Team completion, member, mission, target, or ScriptType handling.
- The elapsed subtraction is 32-bit wrapping arithmetic. `CMP ECX,EAX` followed by signed `JGE` makes the continuation predicate signed `elapsed < duration`.
- Equality expires: `elapsed == duration` branches to the clear at `0x006E9174`.
- No `+1` is added. A Team armed at frame `N` with duration `D>0` returns early at frames `N` through `N+D-1` and becomes eligible to continue on its update at frame `N+D`.
- The timer is not decremented or rewritten while active. Remaining time is derived on every call.
- `start == -1` bypasses subtraction. A nonzero duration then returns forever; a zero duration clears `+0x83` immediately.
- With a normal non-sentinel start, zero or negative duration expires on the first Team update because a normal nonnegative elapsed is not signed-less-than such a duration.
- Expiry writes only `+0x83=0`. It does not clear `+0x7D`, `+0x7E`, `+0x64`, `+0x68`, or `+0x6C`.
- The `+0x7D` helper runs in the same call after expiry. There is no one-frame idle gap.
- A nonzero remaining value exits through the shared return tail at `0x006E95AB`; none of the later Team state is touched on that call.

### 3.3 Same-call latch behavior after expiry

The direct follow-on helper at `0x006EA3E0` is not the timer owner, but its first-order effects are required to avoid inventing an expiry clear:

- When its `Team+0x48` count is positive, it evaluates TeamType required-member totals, then clears `+0x7E` at `0x006EA467` and `+0x7D` at `0x006EA46A` before returning `1`.
- When `Team+0x48 <= 0`, it takes `0x006EA483`; if `+0x78==0`, it reaches the common return without executing those two clears. An empty suspended Team can therefore retain both latches after timer expiry.
- Consequently, Rust must not fold `+0x7D/+0x7E` clearing into the timer-expiry operation. Their later lifecycle belongs to the recruitment/status mechanism.

The full recruitment mechanism is intentionally non-scope; only the branches directly reached from timer expiry were read.

### 3.4 Constructor, persistence, and deterministic checksum

Constructor `0x006E8A90` establishes:

- `+0x64 = g_CurrentFrameCounter`
- `+0x6C = 0`
- `+0x7D = 1`
- `+0x7E = 0`
- `+0x83 = 0`
- no constructor write to `+0x68`

The inactive `+0x83` makes the initial start/duration pair non-suspending. The surprising initial `+0x7D=1` belongs to normal Team recruitment/status initialization and is not evidence that a new Team starts suspended.

Persistence is raw-object persistence:

- Team vtable `+0x14` contains `0x006EC450` (Load), `+0x18` contains `0x006EC540` (Save), and `+0x30` contains `0x006F0430`.
- Raw bytes at `0x006F0430` are `B8 A0 00 00 00 C3`, returning Team object size `0xA0`.
- `AbstractClass::Save @ 0x00410320` writes the receiver token and then exactly the virtual size bytes. `AbstractClass::Load @ 0x00410380` reads that block; `TeamClass::Load @ 0x006EC450` restores Team vtables and queues pointer swizzles but does not normalize the timer.
- Therefore `+0x64/+0x68/+0x6C/+0x7D/+0x7E/+0x83` survive native save/load as raw fields, including the indeterminate `+0x68` payload.

The Team CRC/checksum callback is the vtable `+0x34` entry `0x006EC5A0`. Ghidra lacks a function boundary there, so the body was verified from `read_memory 0x006EC5A0` and decoded read-only through its first `RET 4` at `0x006EC720`:

- It feeds a single normalized remaining-time dword for the `+0x64/+0x6C` pair.
- If start is `-1`, the fed value is duration.
- Otherwise it performs the same wrapping subtraction and signed `elapsed < duration` test, feeding `duration-elapsed` while active and `0` at/after expiry.
- It does not feed raw `+0x64` or `+0x6C` as two independent values.
- It skips `+0x68`.
- It feeds bytes `+0x7D`, `+0x7E`, and `+0x83` individually (`0x006EC6C3`, `0x006EC6CE`, `0x006EC700`).

This makes remaining-time normalization and latch coverage deterministic lockstep behavior, not save-file trivia.

## 4. INI Keys

YR loads standalone `rulesmd.ini`; `rules.ini` is included below only as the RA2 base comparison, not as a YR fallback.

| Section / key | Type | Retail YR value | RA2 value | Binary read/use | Exact effect |
|---|---:|---:|---:|---|---|
| `[General] SuspendPriority` | signed integer | `1` | `1` | `RulesClass::ReadGeneral @ 0x00670BD9` stores at `Rules+0x14E0`; writer compares at `0x006EC28E-0x006EC294` | TeamType priority must be strictly less; equality is not suspended |
| `[General] SuspendDelay` | double minutes | `2` | `2` | `RulesClass::ReadGeneral` stores at `Rules+0x14E8`; writer reads at `0x006EC2C2` | multiplied by exactly `900.0`, x87 chopped to signed 64-bit, low 32 bits stored as frames |

INI evidence: `ini/rulesmd.ini:65-66` and `ini/rules.ini:61-62`. Binary key mapping: `decompile_function 0x00670BD9`, lines around the string xrefs at `0x0083BED0` (`SuspendDelay`) and `0x0083BEE0` (`SuspendPriority`).

## 5. Integration Points

### Arm path

- `get_function_callers 0x00708080` identifies the active damage receivers `BuildingClass::ReceiveDamage @ 0x00442230`, `FootClass::ReceiveDamage @ 0x004D7330`, and `TechnoClass::ReceiveDamage @ 0x00701900`.
- `0x00708080` calls the suspension writer at `0x007081A9` before the response candidate scans.
- The writer runs even if the subsequent responder budget is nonpositive; the later scan exit does not undo Team suspension. This ordering is also captured in the parent report `PHASE3_HOUSE_BUILDING_ATTACK_RESPONDER_00708080_GHIDRA_REPORT.md`.

### Update path and active-YR proof

- `LogicClass::PerTickUpdate @ 0x0055AFB0` copies `g_TeamClass_Array @ 0x008B40EC` in ascending index order (`0x0055B4EA-0x0055B53A` in the current decompile) and calls virtual slot `+0x5C` on each copied Team at `0x0055B55C`.
- The COL/vtable walk above binds that slot to Team owner `0x006E9140`.
- This Team loop runs before the later DiskLaser and main object-AI loops in the ordinary active scheduler. No TS-only flag guards the copied Team loop.
- The standard YR INI supplies both enabling values, and the arm path begins at ordinary YR damage receivers. Active in YR is therefore **Yes**, conditional only on the responder's ordinary House/attacker gates and the Team owner/priority tests.

## 6. Current Rust Implementation Status

Scanned surfaces:

- `src/sim/team_script_vm.rs:87-113` stores `response_latch_7d`, `response_latch_7e`, `response_latch_83`, start, and duration, and derives serde persistence.
- `src/sim/team_script_vm.rs:313-346` correctly visits Rust Teams in stable `BTreeMap` order, uses signed priority `<`, drains members in stored order, sets all three bytes, and stores start/duration. Rearm overwrite is already represented.
- `src/sim/team_script_vm.rs:380-474` does **not** consult any response suspension field. A suspended Team can continue its ScriptType state immediately, which is a direct behavior mismatch.
- The current tick skips completed/refused/inactive-owner Teams before any Team logic. Native evaluates `+0x83` first. Placing expiry beneath the current Rust skip would preserve a mismatch for retained Team state.
- Rust decrements `delay_remaining_frames` as a separate ScriptType wait. That field is not the response timer and must not be substituted for the `+0x64/+0x6C` absolute-frame mechanism.
- `src/sim/team_script_vm.rs:479-501` hashes raw response start and raw response duration independently. Native hashes one normalized remaining-time dword, skips `+0x68`, and then hashes the three latch bytes. The current hash is mismatched even when execution eventually gains a gate.
- `src/sim/team_script_vm.rs:528-565` initializes the response fields as `false,false,false,0,0`; native constructor defaults are `true,false,false,current_frame,0` with `+0x83=false`. The active suspension result is unaffected because the writer overwrites them, but initial serialized/hash state and the ordinary recruitment latch are not native.
- `src/sim/combat/base_defense_response.rs:120-135` already owns an exact x87-chop `minutes*900` helper and the suspension writer calls it. The implementation is suitable for `SuspendDelay`, although its provenance prose currently names only the sibling `BaseDefenseDelay` callsite.
- `src/sim/world/mod.rs:1128-1166` arms suspension with `session.binary_frame as i32`, matching the native frame domain.
- `src/sim/world/mod.rs:6584-6593` calls Team updates in the correct broad scheduler rung but passes monotonic `execute_tick`; `TeamScriptVm::tick_effects` ignores that value. Exact expiry needs the pre-increment wrapping `binary_frame`, not the monotonic command ordinal.
- Serde round-trip already preserves the five response fields. Save/load retention matches the required state effect, but a midpoint restore test must prove that subsequent remaining-time calculation uses the restored raw start/duration against the restored binary frame.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Base-defense writer identity/caller | verified | `get_function_callers`/`get_xrefs_to 0x006EC250`; caller `0x007081A9` | none for claimed slice |
| Writer Team eligibility | verified | `0x006EC270-0x006EC294` | none |
| Ordered member removal | verified | `0x006EC296-0x006EC2AE` | none |
| Arm write order | verified | `0x006EC2B0-0x006EC2E5` | none |
| SuspendDelay conversion | verified | `0x006EC2C2-0x006EC2E5`, `Math__ftol 0x007C5F00`, literal `0x007E27F8=900.0` | none |
| Repeated suspension/rearm | verified | complete writer has no active-timer guard | none |
| Team owner/vtable binding | verified | COL `0x0080BE58`, TypeDescriptor `0x00842D68`, vtable `0x007F4730`, slot `+0x5C` | none |
| Active scheduler path/order | verified | `LogicClass::PerTickUpdate 0x0055AFB0`, copied Team loop/call | none |
| Active timer continuation | verified | `0x006E9149-0x006E916E` | none |
| Equality/zero/negative/sentinel boundaries | verified | assembly branches `0x006E9159-0x006E9174` | none |
| Expiry clear and same-call continuation | verified | `0x006E9174-0x006E918B` | none |
| Direct `+0x7D` follow-on latch effects | verified | `0x006EA3E0`, especially `0x006EA467-0x006EA46A` and `0x006EA483` path | broader recruitment semantics are non-scope |
| Constructor defaults | verified | `decompile_function 0x006E8A90` | none for claimed fields |
| Raw save/load coverage | verified | vtable slots, `0x00410320`, `0x00410380`, `0x006EC450`, `0x006EC540`, size stub `0x006F0430` | native save-file byte compatibility is not a VERA goal here |
| Team CRC timer normalization | verified | vtable `+0x34 -> 0x006EC5A0`; raw body through `0x006EC720` | Ghidra function-boundary creation requires separate authorization |
| Retail INI key mapping | verified | `RulesClass::ReadGeneral 0x00670BD9`, `rulesmd.ini` | none |
| Rust arm transaction | verified current-state scan | `team_script_vm.rs::suspend_teams_for_base_defense` | execution/CRC corrections listed in handoff |
| Rust expiry execution gate | verified missing | `team_script_vm.rs::tick_effects` | implement and test |
| Rust deterministic hash | verified mismatched | `team_script_vm.rs::hash_state` versus native CRC body | normalize to remaining time |
| Full Team recruitment/TaskForce lifecycle | deferred | direct expiry-adjacent branches inspected only | separate Team mechanism investigation if the Phase 3 row requires it |

The zero-add pass re-read the complete timer prologue, writer, constructor, direct follow-on helper, scheduler callsite, persistence callbacks, and CRC byte sequence and added no new material question. Cold spot-checks independently re-read the signed `JGE` expiry boundary at `0x006E9168` and the writer's `+0x83/+0x64/+0x68/+0x6C` order at `0x006EC2DA-0x006EC2E5`.

## 8. Open Questions — Final State of the Investigation Log

- `[RESOLVED] OQ-01 — What function owns the active expiry check? → Team vtable slot +0x5C binds to 0x006E9140 through the TeamClass COL, and the copied Team scheduler invokes that slot.` (evidence: `0x007F472C`, `0x0080BE58`, `0x00842D68`, `0x007F478C`, `0x0055B55C`)
- `[RESOLVED] OQ-02 — Is the suspension writer on a normal YR path? → Yes; ordinary Building, Foot, and Techno damage receivers call the active responder, whose sole call to the writer is 0x007081A9.` (evidence: callers of `0x00708080`; xref to `0x006EC250`)
- `[RESOLVED] OQ-03 — What exact Teams are admitted? → Non-null Teams with matching owner and signed TeamType priority strictly below signed SuspendPriority.` (evidence: `0x006EC270-0x006EC294`)
- `[RESOLVED] OQ-04 — In what order are members removed? → Current first-member pointer order, reloaded after every Remove_Member call until null.` (evidence: `0x006EC296-0x006EC2AE`)
- `[RESOLVED] OQ-05 — What fields does the writer arm? → +0x7E=1, +0x7D=1, +0x83=1, +0x64=current frame, +0x68=non-dominating stack payload, +0x6C=low chopped duration.` (evidence: `0x006EC2B0-0x006EC2E5`)
- `[RESOLVED] OQ-06 — Where do the INI values come from? → RulesClass::ReadGeneral reads SuspendPriority into +0x14E0 and SuspendDelay into +0x14E8; YR retail values are 1 and 2.` (evidence: `0x00670BD9`; `ini/rulesmd.ini:65-66`)
- `[RESOLVED] OQ-07 — What is the duration conversion? → SuspendDelay double times exact 900.0, x87 53-bit/chop ftol, low signed 32 bits stored.` (evidence: `0x006EC2C2-0x006EC2E5`; `0x007C5F00`; `0x007E27F8`)
- `[RESOLVED] OQ-08 — Is an already suspended Team ignored or extended? → Neither; it is fully rearmed from the new frame and any newly attached members are removed.` (evidence: complete `0x006EC250` body has no +0x83 gate)
- `[RESOLVED] OQ-09 — What happens when start is -1? → Nonzero duration returns forever without clearing; zero duration clears immediately.` (evidence: `0x006E9159-0x006E9174`)
- `[RESOLVED] OQ-10 — Is expiry at <, <=, or one tick later? → Continue only for signed elapsed < duration; equality expires in that call with no +1.` (evidence: signed `JGE 0x006E9174` at `0x006E9168`)
- `[RESOLVED] OQ-11 — Does frame arithmetic clamp or wrap? → The 32-bit SUB wraps; its result is interpreted by a signed compare.` (evidence: `0x006E915E-0x006E9168`)
- `[RESOLVED] OQ-12 — Which fields clear at expiry? → Only +0x83; timer fields and +0x7D/+0x7E remain untouched by the timer block.` (evidence: `0x006E9174-0x006E918B`)
- `[RESOLVED] OQ-13 — Does normal Team work resume in the same update? → Yes; execution falls directly into the +0x7D helper and then later Team branches if that helper permits.` (evidence: `0x006E9174-0x006E9191`)
- `[RESOLVED] OQ-14 — May +0x7D/+0x7E remain true after expiry? → Yes; the empty-count/+0x78==0 path in 0x006EA3E0 returns without its positive-count clears.` (evidence: `0x006EA483-0x006EA46D` control flow)
- `[RESOLVED] OQ-15 — What do zero and negative durations do? → With a normal writer start, both expire on the first Team update under the signed comparison.` (evidence: `0x006E915E-0x006E9174`)
- `[RESOLVED] OQ-16 — Are these fields persisted? → Yes; Team Save/Load uses the raw 0xA0-byte object block, then repairs vtables/swizzles without timer normalization.` (evidence: vtable slots; `0x00410320`, `0x00410380`, `0x006EC450`, `0x006EC540`, `0x006F0430`)
- `[RESOLVED] OQ-17 — What does native CRC observe? → One normalized remaining-time dword, bytes +0x7D/+0x7E/+0x83, not raw start/duration and not +0x68.` (evidence: bytes `0x006EC5A0-0x006EC720`)
- `[RESOLVED] OQ-18 — What is +0x68? → It is not required to compute or expire suspension in the mapped owner and is excluded from CRC; its exact broader class meaning is intentionally not promoted from indeterminate writer data.` (evidence: writer, constructor, Team AI, CRC sequence)
- `[RESOLVED] OQ-19 — Does Rust currently block Team execution? → No; tick_effects never reads the response fields.` (evidence: `src/sim/team_script_vm.rs:380-474`)
- `[RESOLVED] OQ-20 — Does Rust hash the native timer state? → No; it hashes raw start and duration independently instead of native normalized remaining time.` (evidence: `src/sim/team_script_vm.rs:479-501`; native CRC sequence)
- `[RESOLVED] OQ-21 — Which Rust frame domain is available? → Arm uses wrapping binary_frame, while Team tick currently receives the monotonic execute_tick and ignores it; expiry needs the pre-increment binary frame.` (evidence: `src/sim/world/mod.rs:1140`, `6588-6593`)
- `[RESOLVED] OQ-22 — Is this TS-only legacy? → No; normal YR damage receivers, the unguarded Team scheduler, YR Rules reading, and stock rulesmd values activate it.` (evidence: receiver callers, `0x0055AFB0`, `0x00670BD9`, `rulesmd.ini`)
- `[DEFERRED] OQ-23 — What is the complete post-expiry recruitment lifecycle?` (category: `out-of-scope`; reason: the claimed slice needs only the direct latch-clear/non-clear branches, while full recruitment spans TeamType/TaskForce admission; next-step-if-pursued: run a separate exhaustive Team recruitment investigation from `0x006EA3E0` and `TeamClass::Recruit_Or_Add`)
- `[DEFERRED] OQ-24 — Should VERA produce byte-compatible native save files including +0x68 junk?` (category: `requires-different-system-context`; reason: VERA uses its own snapshot format and the Phase 3 requirement is gameplay/deterministic state; next-step-if-pursued: define a native-save compatibility goal before modeling indeterminate padding)

Adversarial questions covered by the log include exact equality (`OQ-10`), rearm during an active delay (`OQ-08`), sentinel start (`OQ-09`), zero/negative durations (`OQ-15`), retained latches after expiry (`OQ-14`), frame wrap (`OQ-11`), midpoint save/load (`OQ-16`), and equal-remaining/different-start CRC states (`OQ-17`). The two deferrals are outside the claimed timer slice and do not downgrade it.

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `+0x83` timer is the first Team update gate | `0x006E9149-0x006E916E` | missing | `src/sim/team_script_vm.rs::tick_effects`; world Team tick call | evaluate every retained Team's response timer before the current completed/refusal/owner gates; return/continue while remaining is nonzero | arm at frame `N`, prove no Script action/effect at `N..N+D-1` even when the later Rust gates would otherwise differ | do not put the gate beneath the current early skip |
| Exact signed/wrapping expiry at equality | `0x006E9153-0x006E9174` | missing | Team response timer helper/state | use wrapping signed subtraction, strict signed `<`, sentinel `-1`, and clear only active byte | boundary table for `N`, `N+D-1`, `N+D`, wrap across `i32::MAX`, start `-1`, duration `0`, duration `<0` | do not use saturating math, `<=`, decrement-only state, or a monotonic command ordinal |
| Expiry continues in the same update | `0x006E9174-0x006E9191` | missing | `tick_effects` control flow | after clearing `+0x83`, continue through represented Team work in that same pass | a supported action/effect becomes observable exactly at `N+D`, not `N+D+1` | do not `continue` merely because the timer was active at entry |
| Timer expiry clears only `+0x83` | timer block plus `0x006EA3E0` | missing | `TeamScriptState` mutation | preserve raw start/duration and `+0x7D/+0x7E`; leave their lifecycle to recruitment work | empty Team expiry retains both latches while `+0x83` becomes false | do not clear all three response bytes together |
| Repeated writer calls restart the full delay | complete `0x006EC250` | arm path already matches | `suspend_teams_for_base_defense` regression tests | retain unconditional overwrite and ordered drain | arm at `N`, rearm at `N+k`, prove expiry at `N+k+D` and removal of a member added between calls | do not retain the earlier deadline or take max/min |
| CRC hashes normalized remaining time and latch bytes | `0x006EC643-0x006EC709` raw decode | mismatched | `TeamScriptVm::hash_state` and world hash call if current frame is unavailable | hash one native-shaped remaining-time value plus `+0x7D/+0x7E/+0x83`; do not hash raw start separately | two states with different `(start,duration)` but identical native remaining/latches hash identically; one-frame remaining difference hashes differently | do not hash `+0x68`; do not call raw start “equivalent” |
| Save/load preserves an active timer | Team raw Save/Load chain | fields already serde-persisted; continuation untested | Team VM serde/snapshot tests | restore raw response state and use restored binary frame for remaining calculation | save halfway, restore, prove the same expiry frame and same pre-/post-expiry hash transitions | do not reset the start to load time |
| Native constructor defaults are `7D=1,7E=0,83=0,start=current,duration=0` | `0x006E8A90` | initial fields differ | `insert_team` / creation context and hash tests | either model these defaults with the correct frame authority or explicitly keep the broader recruitment mismatch open | newly constructed Team is not suspended, native-shaped CRC timer contribution is zero, and latch bytes reflect constructor state | do not infer `7D=1` means active suspension; `83` is the suspension gate |
| Existing minute conversion is suitable for SuspendDelay | writer + `Math__ftol` | mechanism matches | `base_defense_response::response_delay_frames` provenance/tests | retain x87 53-bit chop and low-dword behavior; cite the SuspendDelay callsite too | stock `2.0 -> 1800`, fractional/negative/out-of-range cases match the helper's native golden set | do not replace with ordinary Rust float cast without the established x87 contract |

Classification: execution gate, exact equality, same-call continuation, and CRC normalization are **milestone-blocking/compounding** because they change ordinary AI Team behavior and deterministic state after every qualifying base attack. Constructor-default correction is a **broader Team exactification dependency**: it affects initial latch/hash state but not the writer's armed result. Native save-byte treatment of `+0x68` is an **out-of-scope compatibility residual**, not a gameplay implementation requirement.

### Stale Docs / Follow-up Docs

- Parent report `PHASE3_HOUSE_BUILDING_ATTACK_RESPONDER_00708080_GHIDRA_REPORT.md` says the Team AI timer consumer/expiry is verified but does not state the exact transition. Replacement wording: “`TeamClass::AI @ 0x006E9140` first checks `+0x83`; `+0x64` is start and `+0x6C` duration. Signed wrapping elapsed strictly less than duration returns early; equality clears only `+0x83` and continues into the `+0x7D` helper in the same call. `+0x68` is not part of expiry and is skipped by Team CRC.”
- Any Rust-facing note that describes the current five serialized fields as a completed suspension mechanism is stale. They currently record arm state only; `tick_effects` has no consumer and `hash_state` does not match native CRC normalization.

## 11. Ghidra Annotation Candidates

No Ghidra metadata was synchronized; this investigation was read-only.

| Address/source | Current metadata | Proposed metadata | Kind | Live proof | Status |
|---|---|---|---|---|---|
| `0x006EC250` | `FUN_006EC250` | plate comment: “Active House base-defense Team suspension writer: matching owner, signed TeamType priority below Rules SuspendPriority; ordered member drain; set 7E/7D/83; arm 64/6C from chopped SuspendDelay*900.” | comment | unique responder caller plus complete body | deferred — sync not authorized |
| `0x006EA3E0` | `FUN_006EA3E0` | plate comment limited to proved role: “Team AI +7D recruitment/status recheck; positive-count path clears +7E/+7D; empty/+78==0 path can retain them.” | comment | sole Team AI caller and complete direct branch body | deferred — sync not authorized |
| `0x006EC5A0` | no function boundary | no structural change; record as Team vtable `+0x34` CRC callback pending explicit function-boundary authorization | structural candidate withheld | vtable slot and raw body through `RET 4` | deferred — structural authorization required |

## Sources

- Live active `gamemd.exe` Ghidra reads/decompiles/disassembly: `0x00410320`, `0x00410380`, `0x0055AFB0`, `0x00670BD9`, `0x006E8A90`, `0x006E9140`, `0x006EA3E0`, `0x006E8160`, `0x006EC250`, `0x006EC450`, `0x006EC540`, raw `0x006EC5A0-0x006EC720`, `0x006F0430`, `0x00708080`, `0x007C5F00`
- Live memory/vtable/RTTI: `0x007E27F8`, `0x007F472C`, `0x007F4730`, `0x0080BE58`, `0x00842D68`
- Existing feature-worktree report: `docs/research/PHASE3_HOUSE_BUILDING_ATTACK_RESPONDER_00708080_GHIDRA_REPORT.md`
- Retail data: `ini/rulesmd.ini`, `ini/rules.ini`
- Rust implementation: `src/sim/team_script_vm.rs`, `src/sim/combat/base_defense_response.rs`, `src/sim/world/mod.rs`


