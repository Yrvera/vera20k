# TechnoClass IC/FS Field +0x190 Consumers - Ghidra Research Report

**Address(es):** `0x0070E2B0`, `0x0041BF40`, `0x004B4D70`, `0x0070E780`, `0x00457C90`, `0x00522600`, `0x004DEB9D`, `0x006F2B40`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** TechnoClass `+0x190` inside the Iron Curtain / Force Shield timer cluster: active writes, active reads/consumers, and Rust-facing semantic need beyond current start/duration/kind invulnerability state.  
**Non-Scope:** Generic Techno save/load persistence, exact native `.SAV` byte-size contracts, full render tint composition, all Techno visual phase parity, and unrelated classes that also have an offset `+0x190`.  
**Confidence:** High for the decompiled IC/FS apply/check/visual remaining consumers; Medium for the bounded negative direct-displacement sweep because field-style accesses have no first-class xrefs in Ghidra.  
**Active in YR:** Yes. Iron Curtain and Force Shield are stock YR superweapons in `rulesmd.ini`; the base Techno function is reached from BuildingClass and Foot/vehicle paths, while InfantryClass has a live killing override.

## Working Notes

Target question: Identify TechnoClass offset `+0x190` reads/writes and the exact Iron Curtain / Force Shield semantic cluster it participates in; decide whether Rust needs a semantic field beyond current invulnerability/timer modeling.
Non-goals: Do not redo Techno save/load persistence; do not classify all unrelated `+0x190` offsets in other classes; do not implement Rust.
Evidence needed to mark COMPLETE: decompile plus assembly for IC/FS apply, active check, CDTimer remaining helper, live caller/override evidence, Rust state scan, and a bounded `+0x190` read/write sweep.
Stop conditions: Stop if proof requires mutating Ghidra, runtime stack observation, full native raw-save byte mirror design, or a whole visual-render contract.

## 1. Overview

`TechnoClass+0x190` is the middle dword of the inline 12-byte timer block at `+0x18C..+0x194` used by Iron Curtain / Force Shield. The active gameplay check and the visual-phase remaining-time checks read only timer start `+0x18C` and duration `+0x194`; the middle dword is written by apply but was not found to have an active semantic consumer in this IC/FS slice.

Current Rust's `InvulnerabilityState { start_frame, duration_frames, kind }` covers the live semantic state for damage rejection and IC-vs-FS kind. A future native raw-byte save/state mirror may still need an opaque preserved dword for `+0x190`, but Rust should not drive gameplay, expiry, or tint selection from it.

## 2. Class Layout / Key Offsets

| Offset | Field in this slice | Verified behavior | Active in YR |
|---:|---|---|---|
| `+0x18C` | IC/FS timer start frame | Written to `g_CurrentFrameCounter` by apply; read by active check and CDTimer remaining helper. | Yes; `0x0070E2B0`, `0x0041BF40`, `0x004B4D70`. |
| `+0x190` | IC/FS timer middle/aux dword | Written by apply from an in-function stack local slot; not read by `IsIronCurtainActive` or `CDTimerClass__Remaining`; no Techno IC/FS semantic direct reader found in the bounded sweep. | Yes as a raw field write/persisted byte; no verified active semantic consumer. |
| `+0x194` | IC/FS duration in frames | Written from `duration`; read by active check and CDTimer remaining helper. | Yes; stock `IronCurtainDuration=750`, `ForceShieldDuration=500`. |
| `+0x198..+0x1A0` | IC visual phase timer | Separate inline timer used by the IC active visual phase machine, not the target `+0x190`. | Yes for active IC/FS visual phase update. |
| `+0x1A4` | IC visual phase/state | Cleared by apply; later advances through phase states `1..10` while IC/FS is active/ending. | Yes; `0x0070E780`. |
| `+0x1C4` | ForceShield kind flag | `1` for ForceShield, `0` for Iron Curtain; read by draw/tint paths. | Yes; `0x0070E2E4`, `0x0070E2F4`, draw reads such as `0x0043D442`. |

## 3. Core Logic

### 3.1 Apply writes `+0x190`, but the value is not a duration or kind

`TechnoClass__IronCurtain @ 0x0070E2B0` writes the timer cluster in this order: `+0x18C = g_CurrentFrameCounter`; `+0x190 = dword from the function's local stack area`; `+0x1A4 = 0`; `+0x194 = duration`; `+0x1C4 = 1` if ForceShield, else `0`.

Assembly evidence: `0x0070E2BD` loads `ESI = ECX + 0x18C`; `0x0070E2C3` writes `[ECX+0x18C]`; `0x0070E2CD` writes `[ESI+0x4]`, i.e. `+0x190`; `0x0070E2D2` clears `[ECX+0x1A4]`; `0x0070E2D8` writes `[ESI+0x8]`, i.e. `+0x194`; `0x0070E2E4`/`0x0070E2F4` write `+0x1C4`.

Tiny detail: after `SUB ESP,0xC; PUSH ESI`, the read at `0x0070E2C9 MOV EAX,[ESP+0x8]` reads a local stack slot; this function does not initialize that slot before storing it to `+0x190`. That supports treating `+0x190` as an opaque/padding-like timer member in this slice, not as a gameplay input.

Active in YR: Yes. `BuildingClass__IronCurtain @ 0x00457C90` calls the base function after clearing a building-specific state; `Foot/vehicle path @ 0x004DEB9D` calls the base function; InfantryClass has a live override at `0x00522600` that applies damage instead of protection.

### 3.2 The active check ignores `+0x190`

`TechnoClass__IsIronCurtainActive @ 0x0041BF40` reads only `EDX = [ECX+0x18C]` and `EAX = [ECX+0x194]`. If start is not `-1`, it computes `elapsed = g_CurrentFrameCounter - start`; active is true when `elapsed < duration`, with expiry exactly at `elapsed == duration`. No instruction in this function reads `+0x190`.

Assembly evidence: `0x0041BF40..0x0041BF5D`; decompile `0x0041BF40`. Active in YR: Yes. Multiple Techno-derived vtables point to `0x0041BF40` at vtable slot `+0x160`, and active damage/render callers invoke this virtual.

### 3.3 CDTimer remaining helper also ignores `+0x190`

`CDTimerClass__Remaining @ 0x004B4D70` reads only `[ECX]` and `[ECX+0x8]` from a 12-byte timer object. It never reads `[ECX+0x4]`, which is the slot corresponding to `Techno+0x190` when called with `ECX = Techno+0x18C`.

Assembly evidence: `0x004B4D70 MOV EDX,[ECX]`; `0x004B4D72 MOV EAX,[ECX+0x8]`; no `[ECX+0x4]` access in `0x004B4D70..0x004B4D8B`. Active in YR: Yes. Ghidra xrefs to `0x004B4D70` include `0x0070E78F` and `0x0070E7D5`, where `TechnoClass__UpdateTemporalVisual` passes `ECX = ESI+0x18C`.

### 3.4 IC/FS visual phase consumes remaining time, not `+0x190`

`TechnoClass__UpdateTemporalVisual @ 0x0070E780` first calls vtable `+0x160` (`IsIronCurtainActive`). If inactive, it sets `+0x1A4 = 0` and returns. If active, it advances a separate phase timer at `+0x198..+0x1A0` and phase state `+0x1A4`.

Two late-phase gates call `CDTimerClass__Remaining` on the main IC/FS timer: `0x0070E789 LEA ECX,[ESI+0x18C]; CALL 0x004B4D70; CMP EAX,0x36` and `0x0070E7CF LEA ECX,[ESI+0x18C]; CALL 0x004B4D70; CMP EAX,0x1E`. Because `0x004B4D70` ignores `[ECX+4]`, these visual gates do not consume `+0x190`.

Active in YR: Yes, conditional on IC/FS active; the path is explicitly gated by `IsIronCurtainActive`.

### 3.5 ForceShield kind is `+0x1C4`, not `+0x190`

Draw/tint code reads `+0x1C4` after checking active state. Example assembly contexts: `0x0043D434 CALL [EAX+0x160]`, then `0x0043D442 CMP [ESI+0x1C4],1`; and `0x0043DCD3 CALL [EAX+0x160]`, then `0x0043DCE1 CMP [EBP+0x1C4],1`.

Active in YR: Yes for building draw/tint paths when IC/FS is active. This is a render-facing consumer of kind, not of `+0x190`.

## 4. INI Keys

| Key | Stock YR value | Binary relevance | Active in YR |
|---|---:|---|---|
| `[CombatDamage] IronCurtainDuration` | `750` | Passed as duration to the IC apply virtual; stored at `+0x194`. | Yes, `rulesmd.ini:872`. |
| `[General] ForceShieldDuration` | `500` | Passed as duration to the same apply virtual; stored at `+0x194`. | Yes, `rulesmd.ini:142`. |
| `[General] ForceShieldRadius` | `4` | Target selection radius, outside this field slice. | Yes, `rulesmd.ini:141`. |
| `[General] ForceShieldBlackoutDuration` | `1000` | House blackout, not `Techno+0x190`. | Yes, `rulesmd.ini:143`. |
| `[General] ForceShieldPlayFadeSoundTime` | `75` | ForceShield fade-sound scheduling, not `Techno+0x190`. | Yes, `rulesmd.ini:144`. |
| `[IronCurtainSpecial]`, `[ForceShieldSpecial]` | stock sections present | Activate the stock superweapons that reach these paths. | Yes, `rulesmd.ini:30861`, `rulesmd.ini:30878`. |

No INI key was found that maps to `Techno+0x190`; it is written internally by the timer/apply routine.

## 5. Integration Points

| Path | Evidence | Active in YR |
|---|---|---|
| Building IC/FS apply | `BuildingClass__IronCurtain @ 0x00457C90` calls `0x0070E2B0`; xref from `0x00457CD3`. | Yes. |
| Foot/vehicle IC/FS apply | `0x004DEB9D` calls `0x0070E2B0` after related warp/fidget handling. | Yes for non-infantry Techno paths. |
| Infantry override | `InfantryClass__IronCurtain @ 0x00522600` does not call base; it applies damage using rules damage. | Yes; Iron Curtain kills infantry. |
| Active damage/render gate | `IsIronCurtainActive @ 0x0041BF40`, vtable slot `+0x160`. | Yes. |
| IC/FS visual phase remaining-time gates | `0x0070E789`, `0x0070E7CF` call `CDTimerClass__Remaining` with `ECX=Techno+0x18C`. | Yes when active. |
| ForceShield-vs-IronCurtain render kind | Draw contexts read `+0x1C4 == 1`. | Yes when active and rendered. |

## 6. Current Rust Implementation Status

- `src/sim/superweapon/invulnerability.rs:26..32` stores `start_frame`, `duration_frames`, and `kind`.
- `src/sim/superweapon/invulnerability.rs:39..44` checks passive active state with `elapsed < duration_frames`.
- `src/sim/superweapon/invulnerability.rs:49..59` reapplies by replacing start/duration/kind.
- `src/sim/game_entity.rs:252..256` stores optional invulnerability on each entity; constructor default is `None` at `src/sim/game_entity.rs:470`.
- `src/sim/world/world_hash.rs:440..448` hashes start, duration, and kind; there is no explicit opaque `+0x190` field.
- `src/sim/superweapon/iron_curtain.rs:31` and `src/sim/superweapon/force_shield.rs:38` currently use `sim.tick as u32`; the separate native-frame contract already owns the required frame-source correction.

Rust status for this slice: current semantic IC/FS state matches the verified consumed fields, except the already-known frame-source issue and absent IC/FS visual phase machine. No gameplay consumer requires a semantic `ic_fs_aux_0x190` field.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Base IC/FS apply writes | verified | `0x0070E2B0`, asm `0x0070E2BD..0x0070E2F4` | Runtime stack value for `+0x190` not observed. |
| `+0x190` active check consumption | verified negative | `0x0041BF40`, asm `0x0041BF40..0x0041BF5D` | none for active damage gate. |
| CDTimer remaining helper consumption | verified negative | `0x004B4D70`, asm `0x004B4D70..0x004B4D8B` | none for helper semantics. |
| IC/FS visual phase use of main timer | verified | `0x0070E780`, asm `0x0070E789`, `0x0070E7CF` | Full draw composition is separate render contract. |
| ForceShield kind flag | verified | `0x0070E2E4`, `0x0070E2F4`, `0x0043D442`, `0x0043DCE1` | Full tint math not claimed here. |
| Live subclass apply paths | verified | `0x00457C90`, `0x004DEB9D`, `0x00522600`, xrefs to `0x0070E2B0` | Aircraft-specific wrapper not separately decompiled in this slot. |
| Bounded direct-displacement `+0x190` sweep | touched-not-exhausted | Local Capstone `.text` sweep over `gamemd.exe` direct mem disp `0x190`; Ghidra spot-checks show unrelated classes/stack/vtables for other hits | Does not prove absence of every possible dataflow alias; use as supporting evidence only. |
| Rust semantic state | verified-by-source-scan | `src/sim/superweapon/invulnerability.rs`, `src/sim/game_entity.rs`, `src/sim/world/world_hash.rs` | Visual phase state and native raw-byte mirror not implemented. |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - Does base apply write +0x190? -> Yes, via ESI=ECX+0x18C then store [ESI+4].` (evidence: `0x0070E2BD`, `0x0070E2CD`; Active in YR: Yes)
- `[RESOLVED] OQ-02 - Is the +0x190 write a duration write? -> No; duration is written to [ESI+8] = +0x194.` (evidence: `0x0070E2D8`; Active in YR: Yes)
- `[RESOLVED] OQ-03 - Is the +0x190 write a ForceShield-kind write? -> No; kind is written to +0x1C4.` (evidence: `0x0070E2E4`, `0x0070E2F4`; Active in YR: Yes)
- `[RESOLVED] OQ-04 - Does IsIronCurtainActive read +0x190? -> No; it reads +0x18C and +0x194 only.` (evidence: `0x0041BF40..0x0041BF5D`; Active in YR: Yes)
- `[RESOLVED] OQ-05 - Does CDTimerClass__Remaining read the middle dword? -> No; it reads timer offsets +0 and +8 only.` (evidence: `0x004B4D70..0x004B4D8B`; Active in YR: Yes)
- `[RESOLVED] OQ-06 - Does any active IC/FS visual path consume remaining time from +0x18C? -> Yes, two phase gates pass ECX=Techno+0x18C to CDTimerClass__Remaining.` (evidence: `0x0070E789`, `0x0070E7CF`; Active in YR: Conditional, only while IC/FS active)
- `[RESOLVED] OQ-07 - Does that visual path consume +0x190 through CDTimerClass__Remaining? -> No, because the helper ignores [ECX+4].` (evidence: `0x004B4D70..0x004B4D8B`; Active in YR: Yes)
- `[RESOLVED] OQ-08 - Which field distinguishes ForceShield from Iron Curtain? -> +0x1C4, not +0x190.` (evidence: `0x0070E2E4`, `0x0043D442`; Active in YR: Yes)
- `[RESOLVED] OQ-09 - Does Rust already model the consumed semantic timer fields? -> Yes: start, duration, kind exist and are hashed.` (evidence: `src/sim/superweapon/invulnerability.rs:26..32`, `src/sim/world/world_hash.rs:440..448`)
- `[RESOLVED] OQ-10 - Is an explicit +0x190 field required for gameplay active/inactive semantics? -> No verified consumer reads it for damage/timer expiry.` (evidence: `0x0041BF40`, `0x004B4D70`; Active in YR: Yes)
- `[RESOLVED] OQ-11 - Is an explicit +0x190 field required for IC-vs-FS kind? -> No; +0x1C4 owns kind.` (evidence: `0x0070E2E4`, `0x0070E2F4`, `0x0043D442`; Active in YR: Yes)
- `[DEFERRED] OQ-12 - What exact runtime value lands in +0x190 after apply?` (category: `needs-runtime-debugger`; reason: assembly shows an in-function local stack-slot read but static analysis cannot observe runtime stack contents; next-step-if-pursued: instrument/apply IC in native debugger and dump Techno+0x18C..+0x194)
- `[DEFERRED] OQ-13 - Does a full native raw-byte save mirror require storing +0x190 in Rust?` (category: `requires-different-system-context`; reason: this report classifies consumers, not Rust native `.SAV` byte-format goals; next-step-if-pursued: native save-byte contract for Techno raw body mirror)
- `[DEFERRED] OQ-14 - Does AircraftClass use a wrapper path with additional writes?` (category: `bounded-cost-too-high`; reason: base/apply consumers are proven and generic subclass save/load wrapper was out of scope; next-step-if-pursued: aircraft-specific IC apply wrapper sweep)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| IC/FS active gameplay state consumes start `+0x18C` and duration `+0x194`, not `+0x190`. | `0x0041BF40..0x0041BF5D` | none for semantic field shape; frame source mismatch owned by timing contract | `src/sim/superweapon/invulnerability.rs`, `src/sim/combat/mod.rs` | Keep passive start/duration expiry; no gameplay branch should inspect an auxiliary `+0x190` surrogate. | `ic_fs_aux_0x190_does_not_affect_damage_blocking`: two otherwise identical targets with different opaque aux value block/expire damage at identical frames. | Do not invent gameplay meaning for `+0x190`. |
| IC/FS visual phase uses remaining time from the main timer through `CDTimerClass__Remaining`, which ignores `+0x190`, and uses separate visual phase fields at `+0x198..+0x1A4`. | `0x0070E789`, `0x0070E7CF`, `0x004B4D70` | missing visual phase state in current Rust render/sim model | future visual/sim surface for IC/FS tint phase, plus render handoff | If IC/FS visual phase is implemented, add the phase timer/state separately; do not reuse `+0x190` as phase state. | `ic_fs_visual_phase_thresholds_use_remaining_duration_not_aux`: at remaining `0x36` and `0x1E`, phase transitions follow start/duration only even if aux differs. | Do not overload `InvulnerabilityState.kind` or `+0x190` to stand in for phase `+0x1A4`. |
| ForceShield-vs-IronCurtain kind is `+0x1C4`; Rust's `InvulnKind` is the correct semantic field for this slice. | `0x0070E2E4`, `0x0070E2F4`, `0x0043D442`, `0x0043DCE1` | none for semantic kind storage; render tint still separate | `src/sim/superweapon/invulnerability.rs`, render tint follow-up | Preserve kind across apply/save/load/hash; render should select FS vs IC color from kind/`+0x1C4`, not aux. | `force_shield_kind_not_aux_selects_tint`: ForceShield and IronCurtain with same start/duration but different kind select different tint rows; aux changes do not. | Do not derive kind from duration, superweapon owner, or `+0x190`. |

## 10. Negative Facts / Do Not Do

- Do not implement `+0x190` as the IC/FS duration. Duration is `+0x194`. Active in YR: Yes; evidence `0x0070E2D8`, `0x0041BF46`.
- Do not implement `+0x190` as the ForceShield kind. Kind is `+0x1C4`. Active in YR: Yes; evidence `0x0070E2E4`, `0x0070E2F4`.
- Do not make damage blocking depend on `+0x190`. Active check ignores it. Active in YR: Yes; evidence `0x0041BF40..0x0041BF5D`.
- Do not make IC/FS visual phase state live in `+0x190`. Visual phase state is `+0x1A4`, with separate timer `+0x198..+0x1A0`. Active in YR: Conditional on IC/FS active; evidence `0x0070E780`.
- Do not claim byte-perfect native raw-save equivalence if Rust never models an opaque `+0x190` byte and the save format is expected to mirror the native Techno raw body. Active in YR: Yes as raw save/load persistence per prior report; this report only proves no semantic consumer.

## 11. Remaining Uncertainty

- The exact runtime value written to `+0x190` by `0x0070E2CD` remains unobserved. Static evidence shows an in-function local stack-slot read; a runtime debugger is needed to determine whether it is consistently zero, prior stack residue, or affected by caller/compiler frame layout.
- A future native `.SAV` byte-for-byte mirror may need to preserve `+0x190` opaquely even though no active semantic consumer was found here.
- AircraftClass-specific apply wrapper behavior was not separately drained. The base consumer conclusion still holds for `TechnoClass__IsIronCurtainActive` and `CDTimerClass__Remaining`.
- Full IC/FS visual tint composition is a separate render contract; this report only identifies the timer and kind fields that feed it.

## Stale Docs / Follow-up Docs

- `docs/research/IRONCURTAIN_FORCESHIELD_GHIDRA_REPORT.md`: keep "`+0x190` is padding/unused" only if it is scoped to active gameplay consumers. Better replacement: "`+0x190` is the middle dword of the inline IC/FS timer block. `TechnoClass::IronCurtain` writes it from a local stack slot, but `IsIronCurtainActive` and `CDTimerClass::Remaining` read only start `+0x18C` and duration `+0x194`; no active semantic IC/FS consumer is verified."
- `docs/contracts/2026-05-28-technoclass-shared-state-implementation-contract.md`: if it says current `InvulnerabilityState` fully covers every native byte, narrow that claim to "covers consumed semantic IC/FS state: start, duration, and kind." Add: "Opaque `Techno+0x190` is not needed for gameplay/timer semantics but remains a raw-byte preservation concern for native save/state mirrors."
- Any doc that labels `+0x1A4` as merely "cleared by IC apply" should add that `+0x1A4` is also the live IC/FS visual phase state advanced by `0x0070E780`.

## Sources

- Ghidra read-only decompile/assembly: `0x0070E2B0`, `0x0041BF40`, `0x004B4D70`, `0x0070E780`, `0x00457C90`, `0x004DEB9D`, `0x00522600`, `0x006F2B40`, draw contexts `0x0043D442`, `0x0043DCE1`.
- Ghidra xrefs: `0x004B4D70` callers include `0x0070E78F`, `0x0070E7D5`; `0x0070E2B0` callers include `0x00457CD3`, `0x004DEB9D`; vtable data refs for `0x0041BF40`.
- Local read-only binary sweep: Capstone direct memory-displacement scan over `<ra2-install>/gamemd.exe` `.text` for `0x18C/0x190/0x194/0x1A4/0x1C4`, followed by Ghidra spot-checks of Techno-relevant hits.
- INI: `ini/rulesmd.ini` `ForceShieldRadius=4`, `ForceShieldDuration=500`, `ForceShieldBlackoutDuration=1000`, `ForceShieldPlayFadeSoundTime=75`, `IronCurtainDuration=750`, `[IronCurtainSpecial]`, `[ForceShieldSpecial]`.
- Prior docs: `docs/research/TECHNOCLASS_SAVE_LOAD_ACCUMULATOR_INVULNERABILITY_FIELDS_RESWARM_20260528.md`, `docs/research/IRONCURTAIN_FORCESHIELD_GHIDRA_REPORT.md`, `docs/research/TECHNOCLASS_SYSTEMS_GHIDRA_REPORT.md`, `docs/contracts/2026-05-28-technoclass-shared-state-implementation-contract.md`.
- Rust source scanned: `src/sim/superweapon/invulnerability.rs`, `src/sim/game_entity.rs`, `src/sim/world/world_hash.rs`, `src/sim/superweapon/iron_curtain.rs`, `src/sim/superweapon/force_shield.rs`, `src/sim/combat/mod.rs`.
