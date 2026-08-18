# UniqueID -2 Producers / Consumers - Reswarm Report

**Address(es):** `0x004D7D50`, `0x00421EA0`, `0x005F4EC0`, `0x005F4D30`, `0x004228E0`, `0x0064DAB0`, `0x0064DEA0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** producers, preservation, and consumers of `AbstractClass/ObjectClass.UniqueID == -2` relevant to ObjectClass Reveal/Conceal active registration gates.  
**Non-Scope:** re-proving ObjectClass Reveal ordering, CanEnter return codes, or full `g_GameMode` taxonomy except where this sentinel branch needs mode context.  
**Confidence:** High for the producer/consumer chain and Reveal/Conceal gates; Medium for full save/load persistence because no savegame runtime was traced.  
**Active in YR:** Conditional. The sentinel producer is active in standard YR executable paths for non-`g_GameMode` 0/5 sessions; offline campaign (`0`) and offline Skirmish (`5`) bypass both the sentinel producer and the Reveal/Conceal sentinel skip.

## Working Notes Required By Parent

**Target question:** Who creates, assigns, preserves, or consumes `Abstract/Object UniqueID == -2`, especially in relation to `ObjectClass::Reveal`/`Conceal` active registration gates?  
**Non-goals:** Do not re-prove Reveal ordering, CanEnter return codes, or `g_GameMode` meanings except as context for this sentinel.  
**Evidence needed to mark COMPLETE:** Binary decompile plus disassembly/xref evidence for the producer, the assignment mechanism, preservation, consumers, YR activity, Rust surface scan, and proposed tests.  
**Stop conditions:** Stop after resolving the `-2` producer/consumer chain for object UniqueIDs, confirming mode liveness, and separating non-UniqueID `-2` constants into negative facts or uncertainty.

## 1. Overview

The object `UniqueID == -2` sentinel is not a normal constructor default. The verified active producer temporarily rewinds the scenario UniqueID counter to `-3`; the next `AbstractClass::AssignUniqueID` increments it to `-2` for a special `AnimClass` created from a player cell-click feedback path.

In non-`g_GameMode` 0/5 modes, `ObjectClass::Reveal` and `ObjectClass::Conceal` call the secondary vtable GetID slot and skip `FUN_0055BAA0`/`FUN_0055BAE0` when the object ID is `-2`. Offline campaign and offline Skirmish bypass that sentinel check and proceed to register/unregister normally when other gates pass.

## 2. Key Fields / Offsets

| Field / address | Meaning | Evidence | Active in YR |
|---|---|---|---|
| `AbstractClass+0x10` | signed 32-bit `UniqueID`; read by secondary GetID | `AbstractClass__IRTTITypeInfo_GetID @ 0x00410220`; `MOV EAX,[ECX+0x0C]` when `ECX=this+4` | Yes |
| `ScenarioClass+0x214` | UniqueID counter consumed by `AssignUniqueID` / `GetNextID` | `0x0068BCB0`: load `[ECX+0x214]`, `INC`, store, return | Yes |
| `ObjectClass+0x98` | LogicClass active-vector membership, not limbo/storage | prior helper report; registration/removal uses `0x0055BAA0` / `0x0055BAE0` | Yes |
| `g_GameMode @ 0x00A8B238` | Mode gate for `-2` skip | Reveal `0x005F501B..0x005F5038`; Conceal `0x005F4DB0..0x005F4DCD`; parent context fixes 0=campaign/SP, 5=offline Skirmish | Yes |

## 3. Producer / Assignment Chain

| Finding | Evidence | Confidence | Active in YR |
|---|---|---:|---|
| The only object-sentinel producer found in this slice is the cell-click feedback path in `FootClass::ClickedAction_Cell`. For action cases `1` / `0x3E`, if selection voice/feedback is enabled and `g_GameMode` is neither `0` nor `5`, it saves `Scenario+0x214`, writes `-3`, and constructs an `AnimClass`. | Decompile `0x004D7D50`; disassembly `0x004D7F28..0x004D7F57`: load scenario pointer, save `[ECX+0x214]` in `EBP`, `MOV [ECX+0x214],0xFFFFFFFD`, write anim type field `+0x340 = 0xFFFFEC78`, allocate/call `AnimClass::Constructor`. | High | Conditional: non-0/non-5 modes only; bypassed in campaign/SP and offline Skirmish. |
| `AnimClass::Constructor` assigns the actual `UniqueID == -2`: it calls `ObjectClass::Constructor`, sets Anim vtables, then calls `AbstractClass::AssignUniqueID(this+4)`. With the counter temporarily at `-3`, `GetNextID` increments to `-2` and stores it at `Anim+0x10`. | `0x0042201C..0x0042203D` vtable setup then `CALL 0x00410230`; `0x00410246..0x0041024F` calls `0x0068BCB0` and stores `EAX` at `[EDX+0x0C]`; `0x0068BCB0..0x0068BCBD` increments `[Scenario+0x214]`. | High | Conditional: only while the caller has set the counter to `-3`; constructor itself is widely active. |
| The producer restores `Scenario+0x214` immediately after construction in the same non-0/non-5 branch, then removes the new anim from the ordinary `g_AnimClass_Array` and appends it to a separate vector at `DAT_00A83E04` / count `DAT_00A83E10`. | Disassembly `0x004D7FA6..0x004D8062`: mode recheck, `MOV [EAX+0x214],EBP`, find/remove from `0x00A8E9A8` array, append to `0x00A83E04`. | High | Conditional: non-0/non-5 modes only. |
| No direct static write of `0xFFFFFFFE` into `Object+0x10` was found. The active object sentinel is produced indirectly by counter rewind to `-3` plus normal `AssignUniqueID`. | Byte-pattern scans found no `C7 ?? 10 00 00 00 FE FF FF FF` object-offset write; generic `C7 ?? ?? FE FF FF FF` hits were prerequisite parser, non-object helper, and `FUN_007209D0`, not object `+0x10`. | Medium | Yes as a negative static finding for this binary slice. |

## 4. Consumers

| Consumer | Behavior | Evidence | Active in YR |
|---|---|---|---|
| `ObjectClass::Reveal @ 0x005F4EC0` | After type `+0x234` and terrain anim gates, modes `0` and `5` jump directly to registration. Other modes call secondary GetID and skip `FUN_0055BAA0` if `UniqueID == -2`. | Decompile and disassembly `0x005F501B..0x005F5040`: `TEST EAX,EAX`, `CMP EAX,5`, `CALL [ECX+0x10]`, `CMP EAX,-2`, `JZ 0x005F5045`, else `CALL 0x0055BAA0`. | Conditional: skip only in non-0/non-5 modes; registration bypasses this sentinel in campaign/SP and offline Skirmish. |
| `ObjectClass::Conceal @ 0x005F4D30` | Same mode/ID shape as Reveal, but skips `FUN_0055BAE0` unregister when ID is `-2` in non-0/non-5 modes. | Decompile and disassembly `0x005F4DB0..0x005F4DD3`: mode checks, GetID call, `CMP EAX,-2`, jump over remover. | Conditional: non-0/non-5 modes only. |
| `AnimClass::~AnimClass @ 0x004228E0` | Uses direct `GetID(this+4)` after clearing type/owner fields; if ID is `-2`, removes from the special `DAT_00A83E04` vector, otherwise removes from ordinary `g_AnimClass_Array`. | Decompile and disassembly `0x00422A4F..0x00422AD9`: `CALL 0x00410220`, `CMP EAX,-2`, special-vector find/remove vs ordinary anim array. | Yes for Anim destruction; the `-2` arm is Conditional on the special producer. |
| Sync/checksum and desync-log paths | Skip anim RTTI `4` objects whose GetID returns `-2` when hashing/logging display and logic layers, so these temporary anims do not perturb sync diagnostics. | `FUN_0064DAB0` decompile: map/display and logic layer loops check `WhatAmI()==4 && GetID()==-2`; assembly around `0x0064DD0C..0x0064DD19`. `FUN_0064DEA0` contains the same skip in sync-file output. | Conditional: active when sync/hash/desync diagnostics run; sentinel arm requires special anim. |

## 5. Preservation

| Finding | Evidence | Confidence | Active in YR |
|---|---|---:|---|
| `UniqueID == -2` is preserved in-memory for the anim after construction; the producer does not rewrite the anim's `+0x10` when restoring the scenario counter. | Producer restore writes `[Scenario+0x214]` at `0x004D7FCB`; no nearby write to `anim+0x10`; destructor later reads the anim's GetID at `0x00422A5F`. | High | Conditional on the special anim path. |
| Base object CRC/save-style accumulator includes `AbstractClass+0x10`, so a `-2` object ID is not invisible to generic AbstractClass value folding unless a higher-level consumer skips it. | `AbstractClass__ComputeCRC @ 0x00410410`: `MOV EAX,[ESI+0x10]`, `CALL 0x004A1D50`; Object save/debug routine calls it at `0x005F6258..0x005F6259`. | High | Yes. |
| Save/load persistence of a live special `-2` anim was not runtime-traced. `ObjectClass::Load` calls `AbstractClass::Load`, which reads stream data and registers swizzle entries, but this report does not prove whether these special feedback anims are serialized or rebuilt. | `ObjectClass::Load @ 0x005F5E80`; `AbstractClass::Load @ 0x00410380`; no savegame runtime trace. | Medium | Unchecked/Conditional; see Remaining Uncertainty. |

## 6. Current Rust Implementation Status

Rust has stable positive `u64` IDs and a `live_object_order` surrogate, but no signed native `UniqueID` sentinel model and no separate special `AnimClass` vector for move-click feedback anims.

| Rust surface | Current status | Evidence | Active in YR |
|---|---|---|---|
| `src/sim/world/mod.rs` | `next_stable_entity_id: u64` starts at `1`; `allocate_stable_id` saturating-adds; no signed `-2` sentinel. | lines around `next_stable_entity_id`, `allocate_stable_id`. | Rust delta |
| `src/sim/world/mod.rs` | `register_live_object` checks `Vec::contains` and appends; no mode + native UniqueID sentinel skip. | lines around `register_live_object` / `unregister_live_object`. | Rust delta |
| `src/sim/components.rs` / app world effects | Anim-like runtime effects exist, but no scanned field marks the native `UniqueID == -2` special feedback anim family. | static `rg` for `AnimClassSpawnDescriptor`, `WorldEffect`, `stable_id`. | Rust delta |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Producer `FootClass::ClickedAction_Cell` | verified | `0x004D7F28..0x004D8062` | none |
| Assignment via `AnimClass::Constructor` / `AssignUniqueID` | verified | `0x0042203D`; `0x00410230`; `0x0068BCB0` | none |
| Scenario counter restore | verified | `0x004D7FBC..0x004D7FCB` | none |
| Special vector transfer | verified | `0x004D7FC6..0x004D8062`; destructor `0x00422A64..0x00422AB8` | semantic name of vector deferred |
| Reveal registration skip | verified | `0x005F501B..0x005F5040` | none |
| Conceal unregistration skip | verified | `0x005F4DB0..0x005F4DD3` | none |
| Anim destructor consumer | verified | `0x00422A4F..0x00422AD9` | none |
| Sync/hash debug consumers | touched-not-exhausted | `0x0064DAB0`; `0x0064DEA0` | exact runtime trigger of every sync diagnostic path |
| Save/load persistence | deferred | `0x00410380`, `0x005F5E80` touched | runtime savegame trace |
| Non-UniqueID `-2` constants | touched-not-exhausted | shroud edge functions, prerequisite parser hits | not relevant to object UniqueID sentinel |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Is there a live producer of Object UniqueID -2? -> Yes, `FootClass::ClickedAction_Cell` in non-0/non-5 modes temporarily sets `Scenario+0x214=-3`, then constructs an Anim.` (evidence: `0x004D7F28..0x004D7F93`)
- `[RESOLVED] OQ-02 - Does the constructor assign -2 by ordinary AssignUniqueID? -> Yes, `AnimClass::Constructor` calls `0x00410230`, which increments the temporarily set counter and stores the result.` (evidence: `0x0042203D`; `0x00410246..0x0041024F`; `0x0068BCB0`)
- `[RESOLVED] OQ-03 - Is the scenario counter restored? -> Yes, the producer writes saved `EBP` back to `Scenario+0x214` before moving the anim into the special vector.` (evidence: `0x004D7FBC..0x004D7FCB`)
- `[RESOLVED] OQ-04 - Do Reveal/Conceal consume the sentinel? -> Yes, in non-0/non-5 modes both call GetID and skip LogicClass register/unregister when ID is -2.` (evidence: `0x005F5029..0x005F5040`; `0x005F4DBE..0x005F4DD3`)
- `[RESOLVED] OQ-05 - Do modes 0/5 skip the sentinel check? -> Yes, both Reveal and Conceal jump to registration/removal before GetID when `g_GameMode` is 0 or 5.` (evidence: same ranges)
- `[RESOLVED] OQ-06 - Does the destructor preserve special handling? -> Yes, `AnimClass::~AnimClass` reads GetID and removes -2 anims from the special vector rather than ordinary anim array.` (evidence: `0x00422A4F..0x00422AD9`)
- `[RESOLVED] OQ-07 - Is there a direct `Object+0x10=-2` static writer? -> None found in this slice; producer is indirect through counter rewind.` (evidence: byte-pattern scans; `0x004D7F42` is `Scenario+0x214=-3`, not object field)
- `[RESOLVED] OQ-08 - Is this TS legacy only? -> No. The path is in active Foot click handling, Anim construction/destruction, Object Reveal/Conceal, and sync diagnostics in gamemd.exe.` (evidence: xrefs to `AnimClass::Constructor`; live vtable calls)
- `[DEFERRED] OQ-09 - Are special -2 anims serialized in savegames?` (category: `requires-different-system-context`; reason: save/load runtime ownership is outside this sentinel producer slice; next-step-if-pursued: save during a live special feedback anim in non-0/non-5 mode and trace stream ownership.)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Non-0/non-5 move-click feedback anims can receive native `UniqueID == -2` by temporarily setting the scenario ID counter to `-3` before `AnimClass::Constructor`. | `0x004D7F28..0x004D7F93`; `0x0042203D`; `0x0068BCB0` | Missing native signed UniqueID/special anim lifecycle | `src/sim/world/mod.rs`; world-effect / AnimClass-like runtime surfaces | Model the special feedback anim as a distinct native-ID/special-vector case if MP/sync parity implements these anims. | In a non-0/non-5 fixture, issuing a move-click feedback effect creates an anim-equivalent with native ID `-2`, while the next ordinary object ID sequence is restored. | `move_feedback_anim_uses_unique_id_minus_two_without_advancing_global_counter` |
| Reveal and Conceal skip LogicClass register/unregister for `UniqueID == -2` only outside modes 0/5. | `0x005F501B..0x005F5040`; `0x005F4DB0..0x005F4DD3` | `register_live_object` has no native sentinel/mode gate | `src/sim/world/mod.rs::register_live_object`, future reveal/conceal API | Gate active-list membership by native signed ID and mode, not just stable `u64`. | Same anim in non-0/non-5 does not enter `live_object_order`; same code path in mode 5 bypasses the skip if an object were otherwise eligible. | `unique_id_minus_two_skips_live_registration_only_in_network_modes` |
| Special `-2` anims are removed from a separate vector in `AnimClass::~AnimClass`; normal anims use `g_AnimClass_Array`. | `0x00422A4F..0x00422AD9`; producer transfer `0x004D7FC6..0x004D8062` | No special vector found | Anim/world-effect lifecycle | Keep special feedback anim membership separate from normal anim objects if implementing parity-visible MP feedback/sync. | Destroying a special feedback anim removes it from the special list, not the normal anim list, and leaves active object order untouched. | `unique_id_minus_two_anim_despawn_removes_special_vector_entry` |

## Negative Facts / Do Not Do

- Do not model `UniqueID == -2` as an `ObjectClass+0x98` membership state. Active in YR: Yes; `+0x98` is separate and the sentinel gates whether `+0x98` helper calls happen.
- Do not assign `-2` by hardcoding the object field in the verified producer; the binary produces it through `Scenario+0x214=-3` plus normal `AssignUniqueID`.
- Do not apply the Reveal/Conceal skip in modes `0` or `5`; the binary bypasses the sentinel check there.
- Do not treat every `-2` comparison in gamemd.exe as this sentinel; shroud edge calculators and parser/helper code use unrelated `-2` values.
- Do not let the special MP feedback anim affect normal active-object order, normal anim-array membership, or sync CRC/log inputs unless a later runtime trace proves a different owner path.

## Remaining Uncertainty

- Save/load persistence of an already-live special `UniqueID == -2` anim remains unchecked. Likely low gameplay priority because the producer appears to be a transient feedback anim, but parity status is UNCHECKED until traced.
- Exact semantic name of `DAT_00A83E04` / `DAT_00A83E10` remains unnamed; behavior as the special `-2` anim vector is verified.

## Stale Docs / Replacement Wording

- `docs/research/timing/logic-vs-render-loop.md`: replace any wording that says "`g_GameMode == 5` is replay" with "`g_GameMode == 5` is offline Skirmish in the active gamemd.exe mode checks audited by the 2026-05-28 UniqueID -2 reswarm; replay gating in this area is `DAT_00A8D5F8 & 2`, not `g_GameMode == 5`."
- `docs/research/timing/multiplayer-frame-step.md`: replace "`g_GameMode == 0` (skirmish)" with "`g_GameMode == 0` is campaign/single-player in the audited active gamemd.exe mode checks; offline Skirmish is `g_GameMode == 5`."
- `docs/research/ADDRESS_MAP.md`: replace `0x00A8B238` row text "`GameMode (0=SP,1=Skirm,2=LAN,3=WOL,4=TCP)`" with "`GameMode; audited active checks include 0=campaign/single-player and 5=offline Skirmish. Do not label 5 as replay; replay is separately gated by `DAT_00A8D5F8 & 2` in the audited timing/mode reports.`"

## Sources

- Ghidra decompile/disassembly: `0x004D7D50`, `0x00421EA0`, `0x00410230`, `0x0068BCB0`, `0x005F4EC0`, `0x005F4D30`, `0x004228E0`, `0x0064DAB0`, `0x0064DEA0`.
- Existing research: `docs/research/LOGIC_OBJECT_REGISTRATION_HELPER_FUN_0055BAA0_GHIDRA_REPORT.md`, `docs/research/ABSTRACTCLASS_GHIDRA_REPORT.md`, `docs/research/ACTIVE_OBJECT_ORDER_SOURCE_LOAD_REVEAL_SPAWN_GHIDRA_REPORT.md`.
- Rust scan: `src/sim/world/mod.rs`, `src/sim/components.rs`.
