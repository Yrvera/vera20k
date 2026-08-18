# Temporal SQDG Remove-Listener Lifecycle - Ghidra Research Report

**Address(es):** `Temporal/WarpAttach visual AI @ 0x006297F0`, `AdvanceAnimFrame @ 0x00629720`, shared pointer-expired listener `0x0062A260`, `WarpAttachClass::Detach @ 0x0062A4A0`, `Detach_From_All_Lists @ 0x007258D0`, `AnimClass::SetOwnerObject @ 0x00424B50`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** temporal `SQDG` visual creation, storage at the temporal/warp-attach visual object `+0x44`, target attachment, anim-remove-listener dispatch, callback body, target death interaction, SQDG anim expiry interaction, and cleanup/removal conditions.
**Non-Scope:** TeleportLocomotion free `WarpOut` rows, full TemporalClass WarpHP erasure math, exact `SQDG` SHP frame/palette rendering, and the non-temporal parasite damage branch except where the shared listener body must be distinguished.
**Confidence:** High for row construction, listener dispatch, vtable identity, `+0x44`/`+0x54` writes, and Rust-facing deltas. Medium for some semantic labels in the shared `0x0062A260` target-expiry branch because Ghidra has no function boundary there and this report uses disassembly plus vtable proof.
**Active in YR:** Conditional. Active for Chrono Legionnaire-style temporal attacks that enter `WarpAttachClass::UpdateAttack @ 0x00629FD0` and dispatch to the temporal branch; stock YR has `[CLEG] Primary=NeutronRifle`, `[NeutronRifle] Warhead=ChronoBeam`, and `[ChronoBeam] Temporal=yes`.

## Working Notes Gate

Target question: What exactly creates, owns, attaches, observes, clears, and destroys the temporal `SQDG` visual anim, including `Temporal/WarpAttach+0x44`, `SetOwnerObject`, `g_AnimClass_RemoveListeners`, listener vtable `+0x28`, target death, anim expiry, and cleanup?
Non-goals: Do not investigate TeleportLocomotion free `WarpOut` rows, Chronosphere rows, complete Temporal WarpHP damage math, or final SHP rendering frames/palettes.
Evidence needed to mark COMPLETE: decompile plus disassembly for `0x006297F0`, `0x00629720`, `0x0062A4A0`, and `0x007258D0`; vtable/data proof for listener `+0x28`; disassembly for `0x0062A260`; INI stock liveness; Rust surface scan.
Stop conditions: stop after the `SQDG` visual object's lifecycle and Rust handoff are proven; defer exact framebuffer composition and broad pointer-expiry rosters.

## 1. Overview

The temporal `SQDG` visual is not a free sparkle row and not a TeleportLocomotion `WarpOut`. The temporal/warp-attach visual AI creates an `AnimClass(type=SQDG, coords=target, delay=0, loop=1, drawFlags=0x600, zAdjust=0, reverse=0)`, stores the returned anim pointer at the visual object `+0x44`, appends the visual object to the anim-remove-listener vector, and later attaches the anim to the target with `AnimClass::SetOwnerObject(target)`.

The remove-listener callback is the shared warp-attach/parasite vtable, not the HP-countdown `TemporalClass` primary vtable. The active listener vtable starts at `0x007EF890`; its `+0x28` slot is `0x0062A260`. When the `SQDG` anim itself expires or is destroyed, that callback clears `visual+0x44` and sets `visual+0x54 = 1`; cleanup blocks then remove the visual object from `g_AnimClass_RemoveListeners` by find and left-compaction.

## 2. Class Layout / Key Offsets

| Offset / global | Owner | Meaning in this slice | Active in YR | Evidence |
|---|---|---|---|---|
| `+0x24` | temporal/warp-attach visual object | owner/attacker techno pointer | Yes, conditional | `0x006297F0`, `0x0062A260` |
| `+0x28` | temporal/warp-attach visual object | target techno pointer | Yes, conditional | `0x006297F0`, `0x0062A260` |
| `+0x44` | temporal/warp-attach visual object | persistent `SQDG` `AnimClass*` for this visual path | Yes, conditional | store `0x0062991C`, clear `0x0062A489`, destroy sites `0x00629C96`, `0x00629E05`, `0x0062A7F0` |
| `+0x48` | temporal/warp-attach visual object | visual state `0..4`; state 0 creates `SQDG` | Yes, conditional | switch `0x006298B0..0x006298BC` |
| `+0x4C` | temporal/warp-attach visual object | major frame/cycle index written into anim frame `+0xAC` | Yes, conditional | `AdvanceAnimFrame @ 0x00629768..0x006297B6` |
| `+0x50` | temporal/warp-attach visual object | sub-frame counter before major frame advance | Yes, conditional | `AdvanceAnimFrame @ 0x00629768..0x00629780` |
| `+0x54` | temporal/warp-attach visual object | pending remove-listener cleanup flag set when `+0x44` anim expires | Yes, conditional | set `0x0062A490`, tested at `0x00629CA1`, `0x00629E0E`, `0x0062A7FB` |
| `Anim+0xCC` | `SQDG` AnimClass | attached owner pointer; set to target through `SetOwnerObject` | Yes, conditional | `0x00629DAA..0x00629DC4`, `AnimClass::SetOwnerObject @ 0x00424B50` |
| `g_AnimClass_RemoveListeners` | global vector | buffer `0x00B0F5BC`, capacity `0x00B0F5C0`, count `0x00B0F5C8`, grow `0x00B0F5CC` | Yes, conditional | append `0x0062991F..0x0062996F`, dispatch `0x00725A16..0x00725A47` |
| vtable `0x007EF890` | shared warp-attach visual object | `+0x28` slot is listener body `0x0062A260`, `+0x5C` is `UpdateAttack @ 0x00629FD0` | Yes, conditional | retail PE vtable read; disasm refs to `0x007EF890`; `0x00629FD0` dispatch |

Important naming correction: the HP-countdown `TemporalClass` vtable at `0x007F5180` has primary slot `+0x28 -> 0x00410480` (no-op). The `SQDG` remove-listener in this slice uses the shared temporal/warp-attach visual object's vtable `0x007EF890 + 0x28 -> 0x0062A260`.

## 3. Core Logic

### 3.1 Creation and listener registration

Active in YR: Yes, conditional on the temporal visual branch. `WarpAttachClass::UpdateAttack @ 0x00629FD0` calls `Temporal/WarpAttach visual AI @ 0x006297F0` only when owner type fields `+0xCCE` and `+0xD97` are true. Stock Chrono Legionnaire temporal fire reaches this branch through `Temporal=yes` weapon data in `rulesmd.ini`.

State 0 at `0x006298C3..0x00629986`:

1. Writes target `+0x328 = 0`.
2. Looks up hardcoded PE string `SQDG` at `0x0083665C` through `AnimTypeClass::FindByIndex`.
3. Allocates `0x1C8` bytes for `AnimClass`.
4. Calls `AnimClass::Constructor(type=SQDG, coords=target +0x9C/+0xA0/+0xA4, delay=0, loop=1, drawFlags=0x600, zAdjust=0, reverse=0)`.
5. On constructor success, writes returned anim pointer to `visual+0x44`.
6. Appends the visual object pointer to `g_AnimClass_RemoveListeners` if capacity/grow succeeds.
7. Sets visual state `+0x48 = 1`, `+0x50 = 0`, `+0x4C = 0`, then calls `AdvanceAnimFrame`.

If lookup/allocation/constructor/vector-grow fails, the code still advances to state 1 at `0x00629972`; the missing `SQDG` pointer is tolerated by later null checks.

### 3.2 Frame steering and attachment

Active in YR: Yes, conditional on a live `SQDG` pointer. `AdvanceAnimFrame @ 0x00629720` increments `+0x50`, advances `+0x4C` when the state-specific frame cadence expires, and if `+0x44 != 0` writes native anim timing/frame fields:

- `Anim+0xB4 = g_CurrentFrameCounter`
- `Anim+0xC0 = 0x80`
- `Anim+0xB8 = local timer value`
- `Anim+0xBC = 0x80`
- `Anim+0xAC = visual+0x4C + (((RateTimer >> 12) + 1) >> 1 & 7) * 10 + state_base`

The tail of `0x006297F0` checks `visual+0x44`, owner `+0x24`, and target `+0x28`. If the anim's current owner at `Anim+0xCC` is not the target, it calls `AnimClass::SetOwnerObject(target)`, copies an owner-derived value into `Anim+0xD4`, and if the target has a cell/surface object, copies cell height `+0x10A` into `Anim+0xFC`. Evidence: decompile `0x006297F0`, tail `0x00629DAA..0x00629DE5` in prior attached-owner report.

### 3.3 Anim removal dispatch into the listener

Active in YR: Yes when the `SQDG` anim is removed. `Detach_From_All_Lists @ 0x007258D0` reads the expiring object's RTTI through vtable `+0x2C`; for RTTI `4` it iterates `g_AnimClass_RemoveListeners` forward and calls each listener's vtable `+0x28(target, removal_flag)`. Assembly `0x00725A16..0x00725A47` shows:

```text
0x00725A1F  read g_AnimClass_RemoveListeners_Count
+0x00725A2E read g_AnimClass_RemoveListeners buffer
+0x00725A35 ecx = listener[i]
+0x00725A38 edx = [ecx]
+0x00725A3A call [edx+0x28]
```

Retail vtable bytes prove the listener slot for this visual object: vtable start `0x007EF890`, slot `+0x28` at `0x007EF8B8` contains `0x0062A260`; slot `+0x5C` contains `0x00629FD0` (`WarpAttachClass::UpdateAttack`). Ghidra has no function boundary at `0x0062A260`, so this report uses read-only disassembly for the callback body.

### 3.4 Listener callback behavior at `0x0062A260`

Active in YR: Yes, conditional on a registered temporal/warp-attach visual object. Disassembly `0x0062A260..0x0062A497` verifies three relevant equality branches:

- If the expired pointer equals `visual+0x24` owner, write `visual+0x24 = 0` and return (`0x0062A26C..0x0062A280`).
- If the expired pointer equals `visual+0x28` target, enter the target-expiry branch (`0x0062A283..0x0062A481`). This branch can clear the target pointer directly when global byte `0x00A8ED5C` is false, or can run the shared detach/placement logic and may call owner virtual `+0xF8` on failure. It does not itself clear `visual+0x44` unless the expired pointer is also the anim pointer.
- If the expired pointer equals `visual+0x44` `SQDG` anim, write `visual+0x44 = 0`, then `visual+0x54 = 1`, and return (`0x0062A484..0x0062A494`).

This means target death and `SQDG` anim expiry are distinct notifications. Target expiry clears or detaches the target relationship; `SQDG` expiry clears the stored anim pointer and marks the listener-vector entry for later removal.

### 3.5 Cleanup and destroy paths for `SQDG`

Active in YR: Yes, conditional on temporal visual cleanup. There are three verified cleanup families:

1. State-4 accepted kill/cleanup path in `0x006297F0`: if `visual+0x44 != 0`, call anim vtable `+0xF8`, then write `visual+0x44 = 0`; if `visual+0x54 != 0`, find the visual object in `g_AnimClass_RemoveListeners`, decrement count, left-compact, then write `+0x54 = 0`. Evidence: `0x00629C90..0x00629CF5`.
2. State-4 target-null/reset cleanup path and adjacent fallback cleanup repeat the same remove-listener pattern at `0x00629E00..0x00629E5E`.
3. `WarpAttachClass::Detach @ 0x0062A4A0` success cleanup destroys `visual+0x44` via vtable `+0xF8`, clears it, then tests `+0x54` and removes the listener entry by find/left-compaction. Evidence: decompile `0x0062A4A0`, disassembly `0x0062A7E0..0x0062A862`.

The `+0x54` flag is not set by the initial vector append at `0x0062995A..0x0062996F`. It is set by the listener callback when the observed `SQDG` anim removal is dispatched (`0x0062A489..0x0062A490`). In normal explicit cleanup, calling `Anim+0xF8` can re-enter anim removal notification, set `+0x54`, and allow the cleanup block to remove the listener vector entry immediately afterward.

## 4. INI / Data Keys

| Key / data | Stock YR value | Effect in this slice | Active in YR | Evidence |
|---|---|---|---|---|
| `[CLEG] Primary` | `NeutronRifle` | stock owner weapon chain for Chrono Legionnaire | Yes | `ini/rulesmd.ini:4125..4129` |
| `[NeutronRifle] Warhead` | `ChronoBeam` | points to temporal warhead | Yes | `ini/rulesmd.ini:23758` section; prior temporal reports |
| `[ChronoBeam] Temporal` | `yes` | makes temporal branch active | Yes | `ini/rulesmd.ini:27286..27291` |
| `SQDG` hardcoded string | `0x0083665C` | temporal persistent anim lookup | Yes, conditional | `0x006298CC..0x00629913`; `ini/rulesmd.ini:2236`, `ini/artmd.ini:15786` |
| `SQDG_*` directional anim entries | listed in rules/art | adjacent squid/temporal art entries; not directly selected by this `SQDG` row | Conditional | `ini/rulesmd.ini:2227..2236`, `ini/artmd.ini:15786..15840` |

## 5. Integration Points

| Integration | Finding | Active in YR | Evidence |
|---|---|---|---|
| Temporal visual tick | `WarpAttachClass::UpdateAttack` dispatches to `0x006297F0` only for temporal owner type flags | Conditional; standard CLEG yes | `0x00629FD0` |
| Anim creation | state 0 creates `SQDG` with constructor row fields and stores returned pointer | Conditional | `0x006298C3..0x0062991C` |
| Listener registration | vector append stores the visual object pointer in `0x00B0F5BC` and increments count `0x00B0F5C8` | Conditional | `0x0062991F..0x0062996F` |
| Target attachment | tail calls `AnimClass::SetOwnerObject(target)` when `Anim+0xCC != target` | Conditional | `0x006297F0` tail; `0x00424B50` |
| Anim expiry | `Detach_From_All_Lists` RTTI 4 branch calls listener vtable `+0x28`; callback clears `+0x44`, sets `+0x54` | Conditional | `0x00725A16..0x00725A47`, `0x0062A484..0x0062A490` |
| Target death | callback branch for expired `visual+0x28` handles target expiry separately from anim expiry | Conditional | `0x0062A283..0x0062A481` |
| Cleanup | explicit cleanup destroys `+0x44` via anim `+0xF8` and removes listener entry when `+0x54` is set | Conditional | `0x00629C90..0x00629CF5`, `0x00629E00..0x00629E5E`, `0x0062A7E0..0x0062A862` |

## 6. Current Rust Implementation Status

Rust has no native temporal `SQDG` visual object or remove-listener surface in the scanned code:

- `src/sim/components.rs` has `AnimClassSpawnDescriptor` and `WorldEffect`, but those are row/effect records, not native `AnimClass` identities with owner attachment and listener callbacks.
- `src/sim/movement/teleport_movement.rs` models free teleport rows; that is a negative boundary and must not be reused for temporal `SQDG`.
- `src/rules/warhead_type.rs` parses `Temporal=yes`, but no temporal weapon runtime with `SQDG` object/listener lifecycle was found.
- `src/app_instances/units.rs` and current render bridges can draw effects/units, but no generic attached anim pool with `SetOwnerObject`, `Anim+0xCC`, remove-listener vector, or target-expiry callback was found.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Temporal visual state 0 `SQDG` creation | verified | decompile `0x006297F0`, disasm `0x006298B0..0x00629990` | exact SHP frame/palette render out-of-scope |
| Listener vector append/capacity branch | verified | disasm `0x0062991F..0x0062996F` | allocator failure runtime sample not captured |
| Tail `SetOwnerObject(target)` | verified | decompile `0x006297F0`, prior attached-owner disasm | none for attachment mechanism |
| Anim RTTI 4 remove-listener dispatch | verified | decompile `0x007258D0`, disasm `0x00725A16..0x00725A47` | mutation safety during iteration not re-audited |
| Listener vtable identity | verified | retail PE vtable `0x007EF890 + 0x28 -> 0x0062A260`, `+0x5C -> 0x00629FD0`; refs to `0x007EF890` | Ghidra has no named function boundary at `0x0062A260` |
| Listener callback `expired == +0x44` | verified | disasm `0x0062A484..0x0062A490` | none |
| Listener callback target-expiry branch | touched-not-exhausted | disasm `0x0062A283..0x0062A481` | exact labels for global byte `0x00A8ED5C` and all placement sub-branches |
| Explicit cleanup `+0xF8`/listener removal | verified | decompile `0x006297F0`, `0x0062A4A0`; disasm `0x00629C90..0x00629CF5`, `0x00629E00..0x00629E5E`, `0x0062A7E0..0x0062A862` | fail-path runtime sample deferred |
| Rust scan | verified | `rg Temporal|SQDG|WorldEffect|AnimClassSpawnDescriptor src` | no code changes made |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Which object owns the `SQDG` pointer? -> The temporal/warp-attach visual object stores the `AnimClass*` at `+0x44`.` (evidence: `0x0062991C`; Active in YR: Conditional)
- `[RESOLVED] OQ-02 - What row creates `SQDG`? -> `AnimClass(type=SQDG, target coords, delay=0, loop=1, drawFlags=0x600, zAdjust=0, reverse=0)`.` (evidence: `0x006298CC..0x00629913`; Active in YR: Conditional)
- `[RESOLVED] OQ-03 - Is `SQDG` owner-attached? -> Yes, the AI tail calls `SetOwnerObject(target)` when `Anim+0xCC` is not already the target.` (evidence: `0x006297F0` tail; `0x00424B50`; Active in YR: Conditional)
- `[RESOLVED] OQ-04 - Which vector receives the remove listener? -> `0x00B0F5BC` buffer / `0x00B0F5C8` count, the AnimClass remove-listener vector.` (evidence: `0x0062991F..0x0062996F`, `0x00725A16..0x00725A47`; Active in YR: Conditional)
- `[RESOLVED] OQ-05 - What vtable body does listener `+0x28` use? -> Shared warp-attach vtable `0x007EF890 + 0x28` points to `0x0062A260`.` (evidence: retail PE vtable read plus disasm; Active in YR: Conditional)
- `[RESOLVED] OQ-06 - What happens when the `SQDG` anim expires? -> Callback clears `visual+0x44`, sets `visual+0x54=1`; later cleanup removes the listener entry.` (evidence: `0x0062A484..0x0062A490`, cleanup ranges; Active in YR: Conditional)
- `[RESOLVED] OQ-07 - Is target death the same as SQDG expiry? -> No; expired target enters the `+0x28` branch and does not use the `expired == +0x44` clear path unless the anim itself is the expired object.` (evidence: `0x0062A283..0x0062A490`; Active in YR: Conditional)
- `[RESOLVED] OQ-08 - Does registration itself set `+0x54`? -> No; the append stores `esi` and increments count, but `+0x54` is set by pointer-expiry of `+0x44`.` (evidence: `0x0062995A..0x0062996F` vs `0x0062A490`; Active in YR: Conditional)
- `[RESOLVED] OQ-09 - Does cleanup use swap-remove? -> No, it decrements count and left-compacts subsequent entries.` (evidence: `0x00629CC4..0x00629CEF`, `0x00629E31..0x00629E5C`; Active in YR: Conditional)
- `[RESOLVED] OQ-10 - Is this TeleportLocomotion `WarpOut`? -> No; TeleportLocomotion rows are free `[General] WarpOut` rows and are outside this listener path.` (evidence: `TELEPORTLOCOMOTION_GENERIC_VISUAL_ROW_CENSUS_GHIDRA_REPORT.md`; Active in YR: Yes for that negative boundary)
- `[DEFERRED] OQ-11 - Exact `SQDG` rendered frames/palette/composition.` (category: out-of-scope; reason: this slice is lifecycle/listener, not SHP draw path; next-step-if-pursued: `WARPOUT/SQDG_SHP_Draw_Frame_Palette_Rate` style render audit)
- `[DEFERRED] OQ-12 - Exact semantic label and defaults for global byte `0x00A8ED5C` in the target-expiry branch.` (category: requires-different-system-context; reason: target branch is decoded enough for SQDG separation but not fully named; next-step-if-pursued: shared `WarpAttachClass::PointerExpired` branch audit)
- `[DEFERRED] OQ-13 - Runtime mutation safety if listener callback edits `g_AnimClass_RemoveListeners` during `Detach_From_All_Lists` iteration.` (category: needs-runtime-debugger; reason: static code proves count rereads in cleanup, not every concurrent mutation shape; next-step-if-pursued: debugger watch on vector count during anim removal)

## 9. Visual Composition Ledger

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---|---|---|---|---|---|---|---|
| 1 | `0x00629913` constructor | visual state 0, `SQDG` lookup/alloc succeeds | `SQDG` | target coords at creation | normal `AnimClass` draw path, not audited | Yes, conditional | persistent attached temporal grab visual |
| 2 | `0x00629720` frame steering | `visual+0x44 != 0` | `SQDG`, frame via `Anim+0xAC` formula | owner-attached target coords after `SetOwnerObject` | not audited | Yes, conditional | temporal visual animation |
| 3 | `AnimClass::SetOwnerObject @ 0x00424B50` | `Anim+0xCC != target` | `SQDG` | stored as owner-relative offset | attached layer semantics from AnimClass | Yes, conditional | target-following visual |

Asset role matrix:

| Asset | Loaded | Drawn | Visible in target | Content/preview | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|---|---|---|---|---|---|---|---|---|
| `SQDG` | Yes if art present | Yes in temporal state 0 | Conditional | No | No | Yes | Persistent during temporal attack | No | `0x0083665C`, `0x00629913`, `ini/artmd.ini:15786` |
| `SQDG_N..SQDG_NW` | Yes if art present | Not by this hardcoded `SQDG` constructor row | No in this row | No | No | Adjacent directional set | No | Not used by this row | `ini/rulesmd.ini:2227..2234`, `ini/artmd.ini:15791..15840` |
| `[General] WarpOut` | Yes | No in this listener path | No in this slice | No | No | Teleport overlay elsewhere | Yes elsewhere | Inactive here | teleport row census negative boundary |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Temporal visual state 0 creates a real `SQDG` `AnimClass`, stores it at visual object `+0x44`, appends the visual object to anim-remove listeners, and attaches it to the target. | `0x006298C3..0x0062996F`, `0x00629DAA..0x00629DE5`, `0x00424B50` | Missing: no temporal visual object, owner-attached anim identity, or remove-listener vector found. | future temporal weapon runtime; generic `AnimClass` pool/listener support; `src/sim/components.rs` | Add a native anim identity that can be stored by the temporal visual state, owner-attached to target, and observed for removal. | Chrono Legionnaire attacks a warpable target: one `SQDG` anim exists, follows target through owner-relative coords, and has a registered removal listener. Proposed test: `temporal_sqdg_created_attached_and_registered_as_anim_remove_listener`. | Do not emit `SQDG` as fixed `WorldEffect` or TeleportLocomotion `WarpOut`. |
| `SQDG` anim expiry does not just disappear; `Detach_From_All_Lists` RTTI 4 dispatches listener `+0x28`, callback clears `+0x44` and sets `+0x54`, and later cleanup left-compacts the listener vector. | `0x00725A16..0x00725A47`, vtable `0x007EF890+0x28 -> 0x0062A260`, `0x0062A484..0x0062A490`, cleanup ranges | Missing: Rust effects expire by retention/tick without dependent listener callback. | generic anim lifetime and temporal runtime cleanup | On anim expiry, notify registered visual object before dropping anim identity; clear stored handle and remove listener by stable order/left-compaction. | Force `SQDG` anim removal before target death: temporal visual `+0x44` equivalent is cleared, listener entry removed on cleanup, and no stale handle remains. Proposed test: `temporal_sqdg_anim_expiry_clears_stored_handle_and_listener_entry`. | Do not remove listener at creation time or by swap-remove; do not leave stale visual object refs after anim expiry. |
| Target death and `SQDG` anim expiry are distinct pointer-expiry cases; target expiry branches on `expired == +0x28`, while anim expiry branches on `expired == +0x44`. | `0x0062A283..0x0062A490`; `AnimClass::Detach` owner-expiry path from attached-owner report | Missing: Rust despawn has no pre-conceal pointer-expiry dispatch for attached anim owner and temporal visual target. | `Simulation::despawn_entity` or future UnInit/listener stage; temporal target refs; attached anim owner refs | Dispatch target expiry before final removal, and separately dispatch anim expiry when the `SQDG` object is destroyed. | Destroy a target under CLEG temporal attack: temporal visual target ref clears through target-expiry branch, attached `SQDG` receives owner-expiry detach, and later anim cleanup can still clear listener state. Proposed test: `temporal_target_death_dispatches_target_and_attached_anim_expiry_separately`. | Do not model target death as simply deleting the `SQDG` row; native has separate callbacks and state writes. |

## 11. Negative Facts / Do Not Do

- Do not wire `g_AnimClass_RemoveListeners` callback to the HP-countdown `TemporalClass` primary vtable at `0x007F5180`; its `+0x28` slot is inherited no-op `0x00410480` in this binary.
- Do not treat `visual+0x54` as "registered from creation"; creation appends to the vector but `+0x54` is set when the observed `SQDG` anim expires.
- Do not treat target death and `SQDG` anim expiry as the same event; the listener body has separate equality branches for `+0x28` and `+0x44`.
- Do not use TeleportLocomotion `[General] WarpOut`, `WarpIn`, `WarpAway`, or `ChronoSparkle1` for this visual.
- Do not implement final behavior as a free fixed-position `WorldEffect`; `SQDG` is a stored, owner-attached anim with remove-listener callbacks.

## 12. Remaining Uncertainty

- Exact `SQDG` SHP frame/palette/render composition is out of scope.
- The full semantic name/default of global byte `0x00A8ED5C` in the shared target-expiry branch remains unresolved.
- Runtime mutation safety if the remove-listener vector is edited during `Detach_From_All_Lists` iteration needs debugger instrumentation.
- The fail path in shared `WarpAttachClass::Detach` that destroys the owner does not have a live retail scenario sample here; the success cleanup path for `SQDG` is verified.

## 13. Stale Docs / Replacement Wording

- `ANIMCLASS_ATTACHEDOWNER_DETACH_LIFECYCLE_GHIDRA_REPORT.md`: replace "Temporal `SQDG` is stored at `Temporal+0x44`, registered in `g_AnimClass_RemoveListeners`" with: "The temporal/warp-attach visual object stores `SQDG` at `+0x44` and appends itself to the AnimClass remove-listener vector. The callback uses the shared warp-attach vtable `0x007EF890 + 0x28 -> 0x0062A260`; the HP-countdown `TemporalClass` primary vtable `0x007F5180 + 0x28` is inherited no-op and is not the `SQDG` listener body."
- `ANIMCLASS_WARP_CHRONO_RUNTIME_SPAWNS_GHIDRA_REPORT.md`: add after the `SQDG` row: "When the `SQDG` anim expires, `Detach_From_All_Lists` RTTI 4 dispatches to the registered visual object's vtable `+0x28`; the shared listener clears `visual+0x44`, sets `visual+0x54=1`, and later temporal cleanup removes the listener entry by find/left-compaction."
- `TEMPORAL_WARP_PIPELINE_GHIDRA_REPORT.md` and `TEMPORAL_WEAPON_SYSTEM_GHIDRA_REPORT.md`: disambiguate `+0x44`. The HP-countdown `TemporalClass+0x44` is chain-next in the erasure system; the `SQDG` pointer belongs to the shared temporal/warp-attach visual object used by `0x006297F0`.
- `traces/NON_HARVESTER_SELF_TELEPORT_WARPOUT_ROWS_TRACE_20260528.md`: keep TeleportLocomotion rows as a negative boundary and add: "Temporal `SQDG` is not a teleport row; it needs owner attachment plus anim-remove-listener lifecycle."

## Sources

- Ghidra read-only decompile: `0x006297F0`, `0x00629720`, `0x00629FD0`, `0x0062A4A0`, `0x007258D0`, `0x00424B50`, `0x00410480`.
- Ghidra/read-only disassembly success and local retail-byte disassembly: `0x006298B0..0x00629990`, `0x0062A260..0x0062A498`, `0x0062A7E0..0x0062A862`, `0x00629C90..0x00629D05`, `0x00629E00..0x00629E70`, `0x00725A16..0x00725A47`.
- Retail PE vtable read: `0x007EF890 + 0x28 = 0x0062A260`, `0x007EF890 + 0x5C = 0x00629FD0`; `0x007F5180 + 0x28 = 0x00410480`.
- INI checked: `ini/rulesmd.ini`, `ini/artmd.ini`, `ini/rules.ini`, `ini/art.ini`.
- Prior docs referenced: `ANIMCLASS_ATTACHEDOWNER_DETACH_LIFECYCLE_GHIDRA_REPORT.md`, `ANIMCLASS_WARP_CHRONO_RUNTIME_SPAWNS_GHIDRA_REPORT.md`, `DETACH_FROM_ALL_LISTS_LISTENER_EFFECTS_RESWARM_20260528.md`, `TEMPORAL_WARP_PIPELINE_GHIDRA_REPORT.md`, `TEMPORAL_WEAPON_SYSTEM_GHIDRA_REPORT.md`, `TELEPORTLOCOMOTION_GENERIC_VISUAL_ROW_CENSUS_GHIDRA_REPORT.md`.
- Rust scanned: `src/sim/components.rs`, `src/sim/movement/teleport_movement.rs`, `src/rules/warhead_type.rs`, `src/app_instances/units.rs`.
