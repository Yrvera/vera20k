# MissionClass Raw Bytes `+0xB8` / `+0xCC` Full Census — Ghidra Research Report

**Target question:** What are every active-YR writer, reader, persistence/checksum/serialization path, lifecycle meaning, and ordering rule for the raw MissionClass fields at `+0xB8` and `+0xCC`?
**Non-goals:** The full mission dispatcher handler table and individual mission-handler semantics, except where a field access is required to prove a writer or reader.
**Evidence needed to mark COMPLETE:** Receiver-proven decompile plus disassembly/caller evidence for every positive access; an auditable executable-wide instruction/byte search and owner/save/checksum boundary census for negative claims; all open questions resolved or explicitly deferred.
**Stop conditions:** Stop only after the field-access, constructor/reset, save/load, checksum/network/copy, and downstream-consumer searches add no new candidates, two load-bearing findings are cold rechecked, and the final open-question log contains no `[OPEN]` entries.

## 1. Verdict

**Status: COMPLETE for the bounded active-YR MissionClass/member surface.**

`MissionClass+0xB8` is an active one-byte **queued-Guard movement-rejection bypass latch**. Normal construction and successful mission-transition verbs clear it. Two active mechanisms set it after queueing `Guard`: Unit refinery-unload/deploy completion and Jumpjet `Move_To` completion. The only gameplay reader found is `UnitClass::ReadyToCommence`: zero participates in one conjunctive moving-unit rejection, while nonzero bypasses that rejection and continues to the downstream tracker/factory checks. It is not safely named `IsCommenced`, `MissionJustStarted`, or `MissionComplete`.

`MissionClass+0xCC` is a four-byte **opaque timer-adjacent scratch word**. Assignment, commencement, and dispatch write an indeterminate stack word; Foot radio message `0x12` writes the target coordinate's Y word when its payload is non-null and an indeterminate stack word when null. No active gameplay reader was found. It nevertheless remains byte-observable because native raw-object save/load preserves it.

Both fields are present in raw savegame object bytes and restored unchanged. Neither is added to gamemd's Mission checksum/CRC fold. No Mission member serializes either into a command/network packet, and no Mission copy constructor or assignment surface exists. The full global network protocol and arbitrary generic `memcpy` caller universe are outside this bounded member census; no evidence from the reached serialization/caller surfaces makes either field a network payload.

### Player-visible importance

- Omitting the `+0xB8=1` producer can leave a qualifying moving vehicle rejected as not-ready, delaying or preventing queued `Guard` commencement after refinery unload/deploy or Jumpjet arrival.
- Reordering Jumpjet's set/readiness sequence can change whether the latch is cleared in the same call.
- Omitting `+0xCC` does not currently change a gameplay branch, but it breaks native raw-save byte fidelity and native-save round trips.

## 2. Scope and evidence method

The investigated binary is the active Yuri's Revenge `gamemd.exe`, image base `0x00400000`. Ghidra was used read-only.

Evidence ranking in this report:

1. instruction-level body plus proven receiver identity;
2. concrete callers/vtable slot identity and raw vtable/RTTI bytes;
3. whole-program instruction census and member-function census;
4. existing project reports only as navigation or active-stock corroboration.

The executable-wide direct-field searches were:

- `search_instructions(program="gamemd.exe", operand_pattern="0xb8]", limit=1000)`: 1,151,260 instructions scanned, 451 textual matches, not truncated;
- filtering that result to `byte ptr` left 14 candidates, all classified below;
- `search_instructions(program="gamemd.exe", operand_pattern="0xcc]", limit=1000)`: 1,151,260 instructions scanned, 351 textual matches, not truncated;
- because the compiler addresses the timer cluster through an adjusted base, `operand_pattern="0xc8]"` and `mnemonic="ADD", operand_pattern="0xc8"` were also censused; Mission receiver candidates were checked instruction-by-instruction.

Direct absolute-offset searches alone are insufficient for `+0xCC`: Mission code commonly forms `this+0xC8` and then writes `[adjusted+4]`.

## 3. Field census summary

| Field | Native width | Normal constructor | Explicit live values | Gameplay readers | Raw save/load | Mission CRC |
|---|---:|---|---|---|---|---|
| `Mission+0xB8` | 1 byte | cleared to `0` | `0`, `1` | Unit `ReadyToCommence` only | preserved | omitted |
| `Mission+0xCC` | 4 bytes | not initialized | target Y or indeterminate stack word | none found | preserved | omitted |

These conclusions describe active executable behavior, not preferred C++ field names. Raw load can restore any byte/word bit pattern even though ordinary active writers only write `0` or `1` to `+0xB8`.

## 4. `MissionClass+0xB8` complete access census

### 4.1 Normal construction

`MissionClass` normal construction at `0x005B2DA0` clears the byte at `0x005B2DBF` (`MOV byte ptr [ESI+0xB8],AL`, with `AL=0`). The same constructor initializes the surrounding mission selectors and timer members but does **not** initialize `+0xCC`. This was checked in disassembly, not inferred from a decompiler declaration.

### 4.2 Mission-verb clears and their guards

Every explicit verb clear is conditional on that verb actually taking its native mutation path:

| Verb | Clear instruction(s) | Exact qualification |
|---|---|---|
| Assign `0x005B2FD0` | `0x005B2FF9` | The `current==0x1C && new==Guard(5)` guard is a whole-function no-op; that path preserves `+0xB8`. All actual Assign transitions clear it. |
| Commence `0x005B3570` | `0x005B35BF` | Clears only when queued mission is not `-1`; no queued mission preserves it. |
| Queue `0x005B35E0` | `0x005B361A` | Clears only inside the non-sentinel, non-redundant queue mutation. A guard-blocked or redundant/no-op Queue can preserve it even though an allowed `commence_now` tail may still consult readiness. |
| Override `0x005B3650` | `0x005B3681`, `0x005B3698` | Only successful override branches clear it. Blocked paths preserve it. |
| Restore `0x005B36B0` | `0x005B36CB` | Clears only when a suspended mission exists and is restored. |

These bodies were checked with `decompile_function` and `disassemble_bytes` around each listed store. A Rust helper must therefore not perform an unconditional pre-guard clear.

### 4.3 Nonzero writer 1: Unit refinery-unload/deploy completion

`UnitClass::Mission_Deploy_Building` at `0x0073D630`, substate/case 4, contains:

1. function entry `0x0073D638` clears `EBX`, so `EBX=0`;
2. `0x0073DCBC` pushes `EBX` (`commence_now=false`);
3. `0x0073DCBD` pushes mission `5` (`Guard`);
4. `0x0073DCC1` calls Mission vtable slot `+0x1E8` (`Queue`);
5. `0x0073DCC7` writes `1` to `[this+0xB8]`.

The order is therefore **Queue(Guard, false), then set latch**. The store is not a generic “mission completed” flag: it belongs to this concrete deploy/refinery-unload transition. Active-stock reachability is established by the normal harvester/refinery unload path documented in the corresponding deploy and dock research.

### 4.4 Nonzero writer 2: Jumpjet `ILocomotion::Move_To`

The second direct nonzero store, `0x0054B496`, initially lies in a Ghidra undefined-function gap. Receiver identity was reconstructed from raw executable evidence:

- Jumpjet `ILocomotion` vtable is `0x007ECD68`;
- `vtable-4` points to complete-object-locator `0x00804C88`;
- its type descriptor at `0x00829648` contains `.?AVJumpjetLocomotionClass@@`;
- the locator gives adjusted-object offset `4`;
- vtable slot `+0x44` points to `0x0054B1C0`, while the next slot points to `0x0054B4D0`, bounding the body;
- at entry, the adjusted locomotor receiver's `[this+8]` is the linked Techno owner used for Mission vtable calls.

The completion branch is:

1. Jumpjet internal state `[loco+0x4C] == 4` gates the branch (`0x0054B455..0x0054B45B`);
2. it changes locomotor state and clears its cached/owner movement fields;
3. owner effective mission, vtable `+0x184`, must equal `0x10` (`0x0054B474..0x0054B482`);
4. it calls owner `Queue(Guard, true)` through `+0x1E8` (`0x0054B484..0x0054B48D`);
5. it writes owner `+0xB8=1` (`0x0054B496`);
6. it calls owner `ReadyToCommence` through `+0x200` (`0x0054B4A2`);
7. if ready, it calls owner `Commence` through `+0x1EC` (`0x0054B4B1`).

This has an easily missed double-gate: `Queue(..., true)` may perform its own readiness/Commence attempt while the prior B8 value is still in force. After that returns, Jumpjet sets B8 to `1` and performs a **second** Ready/Commence attempt; the nonzero byte now bypasses Unit Ready's conjunctive moving-unit rejection, although downstream checks can still reject. A successful second Commence clears `+0xB8`; otherwise it remains set. Jumpjet is active YR, including stock units using locomotor GUID `{92612C46-F71F-11D1-AC9F-006008055BB5}`; it is not dormant TS-only code.

### 4.5 Sole gameplay reader: Unit `ReadyToCommence`

The sole Mission-derived direct gameplay read is `0x0074431F` inside `UnitClass::ReadyToCommence` at `0x00744270` (a stale local symbol called it `ShouldIdle`; vtable slot `+0x200` and callers prove the role).

A cold reconciliation of `decompile_function(address="0x00744270")` and `disassemble_bytes(start="0x007442E0", end="0x007443B0")` establishes the exact branch direction. The function's first rejection can be written structurally as:

```text
if queued != Enter(7)
   && locomotor_slot_0x80_predicate != 0
   && height_slot_0x1C8 >= 0
   && effective_mission != Guard(5)
   && (effective_mission != Attack(1) || NavCom != 0)
   && B8 == 0
{
    return false;
}
```

The decisive instructions are `MOV AL,[ESI+0xB8]` at `0x0074431F`, `TEST AL,AL` at `0x00744325`, and `JZ 0x00744383` at `0x00744327`; `0x00744383` executes `XOR AL,AL` and returns false. When `+0xB8` is nonzero, execution falls through to `0x00744329` and continues through tracker/factory/pad checks that can still return either false or true. Earlier failures of any conjunct also reach that downstream section. Queued `Enter` bypasses the entire locomotor/`+0xB8` rejection.

Therefore `+0xB8` is a branch-local **movement-rejection bypass**, not a readiness veto and not a universal readiness result. Nonzero does not itself guarantee ready; it merely prevents this one conjunctive rejection. Building, Infantry, and Aircraft overrides were checked in the already-complete vtable override census and do not read this byte.

### 4.6 Rejected direct `+0xB8` candidates

The filtered executable-wide search contained exactly 14 `byte ptr` matches. Ten are the Mission accesses above. The other four are unrelated receiver/stack accesses: OwnerDraw UI (`0x0061572A`), serial-settings UI (`0x00695C1B`), and stack-frame offsets at `0x006AAD66` and `0x007A0782`. Decompiling the serial-settings owner (`0x00695BC0`) shows Win32 controls/modem strings, not a Mission receiver.

No additional active writer or reader survived receiver classification.

## 5. `MissionClass+0xCC` complete access census

### 5.1 Constructor and representation

The normal Mission constructor does not write `+0xCC`. The word is not a timer duration: the timer gate consumes `+0xC8` and `+0xD0`. Treat `+0xCC` as an opaque raw word unless a specific producer context interprets the bits.

### 5.2 Assign and Commence writers

Both verbs form a base at `this+0xC8` and write an uninitialized local to `[base+4]`:

- Assign: adjusted base at `0x005B301B`, stack local loaded at `0x005B3023`, store at `0x005B3027`;
- Commence: adjusted base at `0x005B3594`, stack local loaded at `0x005B35A4`, store at `0x005B35A8`.

This is executable behavior, not a meaningful initialized value. The exact bit pattern depends on prior stack contents. It must not be replaced with a claimed constant in a native-byte save compatibility layer without executable-derived fixture evidence.

### 5.3 Dispatch writers

`MissionClass::Dispatch` at `0x005B3060` forms `this+0xC8` in `ESI` at `0x005B3086`. Every handled return block stores:

- current frame to `+0xC8`;
- an uninitialized stack local to `+0xCC`;
- handler return delay to `+0xD0`.

The 32-entry jump table shares a default block for the absent/out-of-range entry, so every actual Dispatch return path performs the same timer-cluster update. Searching this bounded body for `[...+4]` yielded 31 concrete shared/leaf store blocks, ending at `0x005B34DA`; no Dispatch consumer reads `+0xCC`. The due gate reads `+0xC8` and `+0xD0` only.

This report does not classify the individual dispatcher handlers; only their common write epilogue is in scope.

### 5.4 Foot radio `0x12` writer

`FootClass::Receive_Radio` at `0x004D8FB0`, case `0x12`, is the only writer that supplies a contextually meaningful word:

- for a non-null payload target, vtable `+0x48` fills a coordinate local; the target Y word is at stack `+0x20`;
- after possible queue/commence work and `Set_Destination(*payload,1)`, it stores current frame to `+0xC8` (`0x004D9203`), target Y to `+0xCC` (`0x004D91F6` / `0x004D920A`), and zero to `+0xD0` (`0x004D920D`);
- for `*payload==NULL`, coordinate filling is skipped, but the same stack slot is still stored, so the word is indeterminate stack residue.

This message is active in normal YR refinery docking. `BuildingClass::Receive_Radio` at `0x0043C2D0` contains four send sites pushing `0x12`: `0x0043CA43`, `0x0043CAB4`, `0x0043CBAA`, and `0x0043CC86`.

### 5.5 Negative-reader proof and rejected candidates

No active Mission gameplay reader of `+0xCC` survived the combined direct and adjusted-base census. Direct textual `+0xCC` candidates belong to other layouts. For example, the candidate under `0x005AE4C0` belongs to MapSelect UI: its owning body logs `MapSelect`, creates a font, and queries the radar timer.

Adjusted-base candidates were classified as follows:

- true Mission receivers: Dispatch and Commence from `LEA ...+0xC8`; Assign and Foot radio from `ADD ...+0xC8`;
- false receiver: Building Unlimbo adds `0xC8` to a pointer stored at Building `+0x21C`, then operates on that embedded object;
- false receiver: `0x00482900` adds `0xC8` to a return value used as an allocation parameter;
- remaining candidates are stack or non-Mission class layouts.

The strongest honest conclusion is **no gameplay reader in active gamemd**, not “the byte is irrelevant”: persistence still observes it.

## 6. Lifecycle and ordering model

### `+0xB8`

```text
normal construction -> 0
successful Assign / Queue mutation / Commence / Override / Restore -> 0
Unit deploy state 4: Queue(Guard,false) -> 1
Jumpjet completion: Queue(Guard,true) -> 1 -> Ready -> maybe Commence -> maybe 0
Unit Ready moving-rejection conjunct: B8==0 can complete the false-return condition
Unit Ready with B8!=0: bypass that rejection, then continue tracker/factory checks
raw load: preserve saved byte exactly
```

The latch belongs to Mission authority, while its two set sites belong to concrete Unit/locomotor mechanisms. It allows the post-Guard-queue transition to get past one moving-unit rejection; it does not force a true readiness result. The ordering is observable within the same call and must remain serial in Rust.

### `+0xCC`

```text
normal construction -> untouched / indeterminate
Assign or Commence -> indeterminate stack word
Dispatch return -> indeterminate stack word
Foot radio 0x12(non-null) -> target coordinate Y bits
Foot radio 0x12(null) -> indeterminate stack word
gameplay -> never read by identified active code
raw load -> preserve saved word exactly
```

## 7. Save, load, checksum, network, and copy boundaries

### 7.1 Raw save includes both fields

`AbstractClass::Save` at `0x00410320` writes a saved pointer, calls the concrete object's vtable `+0x30` size method, and writes that entire object body (`0x0041034F..0x0041035E`). Concrete active Techno sizes all extend beyond `+0xCC`:

| Concrete type | Primary vtable | Size method | Raw size |
|---|---:|---:|---:|
| Aircraft | `0x007E22A4` | `0x0041C170` | `0x6D8` |
| Infantry | `0x007EB058` | `0x005232F0` | `0x6F0` |
| Building | `0x007E3EBC` | `0x00459E70` | `0x720` |
| Unit | `0x007F5C70` | `0x00746DD0` | `0x8E8` |

Consequently both raw offsets are emitted in the object block.

### 7.2 Raw load preserves both fields

`AbstractClass::Load` at `0x00410380` registers pointer swizzling, gets the concrete raw size, and reads the entire object body into the instance (`0x004103A5..0x004103D0`). The post-read reconstruction chain does not call the normal Mission constructor:

```text
Unit load raw body 0x7444FE
  -> Foot load-context constructor 0x4D3540
  -> Techno load-context constructor 0x6F4300
  -> Radio no-init constructor 0x65A7E0
  -> Object vtables-only constructor 0x5F3B50
  -> Abstract vtables-only constructor 0x4101C0
```

At `0x006F430B`, the call target is specifically the Radio no-init entry, not normal Radio construction. The chain restores vtables/selected higher-level load state and never touches Mission `+0xB8` or `+0xCC`. Both raw values therefore survive a native save/load round trip.

### 7.3 Mission checksum/CRC omits both fields

The local Ghidra symbol `MissionClass__Save` at `0x005B3970` is misleading: its body is the Mission checksum fold. It first calls the Object checksum and then passes selected four-byte values to `0x004A1D50`; decompiling `0x004A1D50` shows `CRCEngine::AddData(&value,4)` behavior.

The fold includes current/queued/suspended/substate, a computed remaining duration from `+0xC8/+0xD0`, and another Mission word. It explicitly omits `+0xB8`, `+0xCC`, `+0xC4`, and raw `+0xC8`. Caller tracing reaches Techno and Foot/Building checksum paths. This is not savegame serialization.

Rust's `Simulation::state_hash` is a project integrity/desync hash, not automatically the gamemd CRC surface. It may hash a represented `+0xB8` latch for Rust lockstep integrity, but a future gamemd-compatible CRC implementation must omit both fields.

### 7.4 Network surface

The complete Mission member-function census contains no explicit command/packet serializer, and no identified `+0xB8`/`+0xCC` access feeds a network writer. The reached native synchronization surface is the checksum above, which omits both. Their live values arise as deterministic local side effects of mission/radio/locomotor execution.

Bounded limitation: this did not reclassify every global network protocol function in gamemd. It proves the Mission member/vtable and reached checksum surfaces, and found no direct field serializer; it does not certify that an unrelated generic whole-object byte dumper could never be invoked by code outside the reached graph.

### 7.5 Copy/clone surface

The Mission member census contains Constructor, Destructor, Assign, Queue, Commence, Override, Restore, Dispatch, query/timer helpers, load notification, checksum, and deleting destructor; it contains no Mission copy constructor or copy assignment routine. No field-by-field B8/CC copy survived the whole-program access census. The only proven block transfer is Abstract raw save/load.

Bounded limitation: arbitrary generic memory-copy callers were not exhaustively semantically classified. There is no positive evidence of an active Mission clone/copy mechanism.

## 8. Current Rust disparity

Current source inspection found:

- `src/sim/mission/mod.rs`: `MissionCom` represents current/queued/suspended/substate/timer/tick counter, but no `+0xB8` latch or `+0xCC` compatibility word;
- `src/sim/mission/verb.rs`: transition verbs do not apply the native conditional B8 clears, and `ReadySnapshot` / `ready_to_commence` reduce Unit readiness to `!is_driving`;
- `src/sim/miner/miner_dock_sequence.rs`: the refinery-departing/Deploy-state-4 projection queues/resumes Harvest behavior but cannot reproduce the native `Queue(Guard,false) -> B8=1` Mission side effect;
- `src/sim/movement/air_movement.rs`: Jumpjet arrival/landing state changes do not reproduce the native `Queue(Guard,true) -> B8=1 -> second Ready/Commence` sequence;
- `src/sim/radio/receive.rs`: represented radio receive choreography does not expose the active Foot message-`0x12` Mission timer-cluster write;
- `src/sim/world/world_hash.rs`: all represented MissionCom fields are folded into the Rust state hash; this must not be mistaken for the native Mission CRC membership proved above.

This is **DRIFT/UNIMPLEMENTED**, not a parity-neutral structural difference. `+0xB8` changes active mission timing. `+0xCC` changes native raw-save bytes.

## 9. Implementation handoff

### Shape A — model the active latch in Mission authority

Add a native-width `u8` such as `queued_guard_movement_rejection_bypass` to `MissionCom`, rather than a `bool`; native raw load can restore any byte even though active writers use `0/1`. Make `src/sim/mission/verb.rs` own the conditional clears on **successful native mutation paths only**. Extend the Unit readiness snapshot enough to reproduce the full conjunction: only `B8==0` completes that moving-unit false-return condition; `B8!=0` continues to the tracker/factory checks and does not itself return true.

Primary entry points:

- `src/sim/mission/mod.rs` — storage and serde state;
- `src/sim/mission/verb.rs` — verb clears, Unit readiness predicate inputs/order;
- `src/sim/world/world_hash.rs` — decide Rust integrity-hash inclusion explicitly; keep it out of any gamemd CRC implementation.

### Shape B — preserve producer ordering

Wire the two producers without collapsing their call sequences:

- refinery/Deploy completion: `Queue(Guard,false)` then set byte `1`;
- Jumpjet `Move_To` completion: `Queue(Guard,true)`, then set `1`, then a second Ready call, then conditional Commence.

Primary current projections are `src/sim/miner/miner_dock_sequence.rs` and `src/sim/movement/air_movement.rs`, but the final ownership should remain Mission verbs plus scheduler/locomotor orchestration rather than ad-hoc field writes.

### Shape C — isolate `+0xCC` as compatibility state

Do not give `+0xCC` gameplay-timer semantics. If/when native raw-save compatibility is implemented, preserve a `u32` bit pattern such as `legacy_timer_scratch_cc`, load/save it verbatim, and update it at the proven producers. Use a captured gamemd fixture for indeterminate-stack outcomes; do not synthesize a fixed value. It is reasonable to keep this out of ordinary gameplay decisions while still retaining it for native save bytes.

### Acceptance scenarios

1. `mission_verbs_clear_b8_only_on_native_success_paths`
2. `unit_deploy_state4_queues_guard_then_sets_b8`
3. `jumpjet_move_to_sets_b8_before_second_ready_movement_bypass`
4. `unit_ready_b8_zero_completes_moving_rejection_nonzero_bypasses_it`
5. `native_raw_save_round_trips_b8_and_cc`
6. `gamemd_mission_crc_omits_b8_and_cc`
7. `foot_radio_0x12_nonnull_records_target_y_bits_in_cc`
8. `foot_radio_0x12_null_uses_executable_captured_residue_fixture`

Parity certification still requires gamemd-derived executable traces/save fixtures; Rust-only unit tests are regression checks.

## 10. Negative facts / do not do

- Do not name `+0xB8` `IsCommenced`, `MissionJustStarted`, or `MissionComplete`.
- Do not clear `+0xB8` before verb guards or on failed/no-op Commence/Restore.
- Do not treat nonzero B8 as a readiness veto or reduce Unit readiness to either `B8==0` or `B8!=0`; it bypasses one rejection and then downstream checks still decide.
- Do not collapse Jumpjet completion to one Queue/Ready attempt.
- Do not call `+0xCC` a timer duration, counter-high word, or dead padding.
- Do not initialize `+0xCC` to zero and claim native raw-save byte parity.
- Do not use Rust `MaybeUninit` to imitate stack garbage in live simulation. Preserve captured raw bits only where native-byte compatibility requires them.
- Do not include either field in a future gamemd-compatible Mission CRC merely because Rust's broader state hash includes represented state.
- Do not port the C++ object/raw-stream architecture; preserve the verified ordering and bytes through Rust-native ownership.

## 11. Coverage ledger

| Surface | Evidence | Result |
|---|---|---|
| Mission normal constructor | disassembly `0x5B2DA0` | B8 cleared; CC untouched |
| Assign | decompile + disassembly `0x5B2FD0` | conditional B8 clear; CC indeterminate write |
| Queue | decompile + disassembly `0x5B35E0` | conditional B8 clear |
| Commence | decompile + disassembly `0x5B3570` | conditional B8 clear; CC indeterminate write |
| Override | decompile + disassembly `0x5B3650` | successful-branch B8 clears |
| Restore | decompile + disassembly `0x5B36B0` | successful-branch B8 clear |
| Dispatch | adjusted-base/store census `0x5B3060` | CC written on return; never consumed by due gate |
| Unit Deploy state 4 | disassembly `0x73D630` | Queue Guard then B8=1 |
| Jumpjet Move_To | RTTI/vtable/raw-body proof `0x54B1C0` | Queue, B8=1, second Ready/Commence |
| Unit Ready | vtable identity + disassembly `0x744270` | sole B8 gameplay reader |
| Other Ready overrides | prior complete override census + field search | no B8/CC readers |
| Foot radio `0x12` | switch body + four Building senders | CC target-Y/indeterminate writer |
| Direct B8 whole-program search | all 14 byte candidates classified | no remaining candidate |
| Direct/adjusted CC searches | direct CC, C8 LEA, C8 ADD candidates classified | no gameplay reader remaining |
| Abstract Save/Load | decompile + disassembly + size vtables | both raw fields persist |
| Load reconstruction chain | cold-read call chain | no Mission normal constructor; values survive |
| Mission checksum and callers | decompile + helper/caller trace | both omitted |
| Mission member/copy census | function and caller enumeration | no copy or packet member |
| Rust mission/producer/hash surfaces | direct source reads | both fields absent; active B8 drift |

## 12. Resolved-question log

| # | Question | Resolution |
|---:|---|---|
| 1 | Does normal construction initialize B8? | `[RESOLVED]` Yes, to zero. |
| 2 | Does normal construction initialize CC? | `[RESOLVED]` No. |
| 3 | Which explicit B8 values exist? | `[RESOLVED]` Active writers use zero and one; raw load may restore any byte. |
| 4 | Are verb clears unconditional? | `[RESOLVED]` No, only native mutation/success paths clear. |
| 5 | Is Unit Deploy a B8 writer? | `[RESOLVED]` Yes, after Queue(Guard,false) in state 4. |
| 6 | Who owns the undefined `0x54B496` store? | `[RESOLVED]` Jumpjet ILocomotion `Move_To`, writing its linked Techno/Mission owner. |
| 7 | Is Jumpjet active YR or TS-only? | `[RESOLVED]` Active stock YR locomotion. |
| 8 | What is Jumpjet's exact order? | `[RESOLVED]` Queue(Guard,true), set B8, second Ready, conditional Commence. |
| 9 | Who reads B8? | `[RESOLVED]` Unit `ReadyToCommence`; no other active gameplay reader survived census. |
| 10 | What is B8's exact Ready branch direction? | `[RESOLVED]` `B8==0` completes one moving-unit false-return conjunction; nonzero bypasses that rejection and continues downstream. Queued Enter bypasses the entire branch. |
| 11 | Which code writes CC? | `[RESOLVED]` Assign, Commence, Dispatch epilogues, Foot radio `0x12`, and raw load. |
| 12 | Does Dispatch read CC? | `[RESOLVED]` No; the due gate reads C8/D0. |
| 13 | What does non-null radio `0x12` write? | `[RESOLVED]` Target coordinate Y bits. |
| 14 | What does null radio `0x12` write? | `[RESOLVED]` The unfilled/indeterminate stack slot. |
| 15 | Is there an active gameplay CC reader? | `[RESOLVED]` None found in the complete direct/adjusted receiver census. |
| 16 | Does native raw save include both fields? | `[RESOLVED]` Yes, within every concrete Techno raw object block. |
| 17 | Does load construction overwrite either? | `[RESOLVED]` No; the no-init/vtables-only chain skips Mission normal construction. |
| 18 | Does Mission CRC include either? | `[RESOLVED]` No. |
| 19 | Is either directly packet-serialized by Mission? | `[RESOLVED]` No such Mission member/access path exists; global protocol audit remains outside the bounded claim. |
| 20 | Is there a Mission copy/clone path? | `[RESOLVED]` No member or field-wise path found; only raw stream block transfer is proven. |
| 21 | Does current Rust model B8? | `[RESOLVED]` No. |
| 22 | Does current Rust preserve CC raw-save bits? | `[RESOLVED]` No native raw-save compatibility field exists. |

## 13. Stale-document replacement wording

The following older labels should be treated as superseded by this report. This investigation intentionally changes no other file.

| Existing document | Replace stale claim with |
|---|---|
| `docs/research/MISSIONCLASS_STATE_MACHINE.md` | “`Mission+0xB8` is a queued-Guard movement-rejection bypass latch set by Unit Deploy state 4 and Jumpjet Move_To completion and read by Unit ReadyToCommence. `B8==0` participates in one moving-unit false-return conjunction; nonzero bypasses that rejection and continues downstream. `Mission+0xCC` is opaque raw scratch: written by Assign/Commence/Dispatch and Foot radio `0x12`, not read by active gameplay, but preserved by raw save/load.” |
| `docs/research/FOOTCLASS_FIELD_0xAC_PROCESS_ARRIVAL_CHECK_GHIDRA_REPORT.md` | “Do not label Mission `+0xB8` MissionJustStarted; it is the queued-Guard movement-rejection bypass latch proven by the full census.” |
| `docs/research/MISSION_DEPLOY_BUILDING_0x73D630_STATE_MACHINE_GHIDRA_REPORT.md` | “The state-4 write is `Mission+0xB8=1`, enabling Unit ReadyToCommence to bypass one moving-unit rejection after Queue(Guard,false); it is not a generic MissionComplete flag.” |
| `docs/research/READYTOCOMMENCE_VTABLE_0X200_SUBCLASS_OVERRIDES_GHIDRA_REPORT.md` | “Unit ReadyToCommence includes `B8==0` in one conjunctive moving-unit false-return condition. Nonzero B8 bypasses that rejection and proceeds to tracker/factory checks; it is not a readiness veto or an IsCommenced predicate.” |
| `docs/research/MISSIONCLASS_VERB_API_GUARDS_OVERRIDE_RESTORE_SEMANTICS_GHIDRA_REPORT.md` | “`Mission+0xCC` has no active gameplay reader, but it is raw-save/load observable and therefore cannot be omitted from native save byte parity. Verb success paths also conditionally clear `Mission+0xB8`.” |
| `docs/research/core-services-map/mission-radio.md` | “B8 is the queued-Guard movement-rejection bypass latch: zero can complete Unit Ready's moving rejection, while nonzero continues downstream. CC is opaque scratch with a Foot radio `0x12` target-Y producer and raw save/load observability; neither field belongs to Mission CRC.” |
| `docs/research/FOOTCLASS_RECEIVE_RADIO_FULL_SWITCH_GHIDRA_REPORT.md` | “Case `0x12` writes target Y to Mission `+0xCC` for non-null payload and indeterminate stack residue for null payload; it is not a counter-high word.” |
| `docs/research/FOOTCLASS_RADIO_MOVE_FIELDS_0XB4_0XCC_GHIDRA_REPORT.md` | “The null-payload branch skips coordinate fill but still stores the same stack slot, so `+0xCC` receives indeterminate stack residue.” |

## 14. Remaining uncertainty and bounded exclusions

No access-census question remains unresolved. Two properties cannot be assigned a single static value by decompilation:

- the exact `+0xCC` bits produced by an uninitialized stack slot depend on the concrete execution stack history;
- a full independent audit of every global network protocol routine and every generic memory-copy caller was not performed.

These do not weaken the active Mission mechanism findings. Exact CC residue fixtures require a gamemd execution capture. The network/copy negative claim is deliberately limited to the complete Mission member/access/checksum/caller surface documented above.

## 15. Sources

Primary source: read-only live Ghidra analysis of active `gamemd.exe`, including executable-wide `search_instructions`, `decompile_function`, `disassemble_bytes`, function/caller enumeration, vtable bytes, RTTI complete-object-locator bytes, and concrete class size methods at the addresses cited inline.

Rust source inspected read-only:

- `src/sim/mission/mod.rs`
- `src/sim/mission/verb.rs`
- `src/sim/miner/miner_dock_sequence.rs`
- `src/sim/movement/air_movement.rs`
- `src/sim/radio/receive.rs`
- `src/sim/world/world_hash.rs`

Existing research used for navigation/reachability context was rechecked against the cited live bodies before carrying load-bearing claims into this report.
