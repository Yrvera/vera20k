> **RAW LANE OUTPUT — NOT THE AUTHORITY.** This is the unedited the reconciliation pass from the
> 2026-07-30 slot-32 investigation, kept for its per-call citations. It was written before
> the adversarial verify pass, and **some claims in it were subsequently refuted** — the
> verifiers found, among other things, a wrong vtable-offset constant, several claims with
> no citation, at least one omitted term in a decoded predicate, and an under-count of the
> gate's call sites.
>
> **Read `ILOCOMOTION_IS_MOVING_NOW_SLOT32_AND_MISSION_GATE_GHIDRA_REPORT.md` for the
> settled findings.** Treat anything here that the consolidated report does not repeat as
> UNCHECKED, and re-verify from the binary before relying on it.

---

## 1. The settled slot-32 (`Is_Moving_Now`) contract table

Frame convention (VERIFIED by both lanes and both verifiers, three independent routes — ctor stores, RTTI complete-object-locator `offset = 4`, and slot 16 doing `LEA EDI,[ESI-0x4]` before calling `Process`): the ILocomotion vtable pointer sits at **object+0x04**, so an ILocomotion method's `this` is object+4 and **every offset below is interface-relative** (object offset = listed + 4). `owner` = `*(void**)(iface+0x8)`.

| Family | ILocomotion base | slot 32 | Exact predicate (interface-relative) | Confidence |
|---|---|---|---|---|
| Drive | 0x007e7eb0 | 0x004afc20 | `TimerRemaining(owner+0x388) \|\| ( slot4() && !(i32 this[+0x3c/+0x40/+0x44] == NullCoord) && (i32)owner->vt[0x538]() > 0 )` — signed `JLE` | VERIFIED (term set, order, signedness) |
| Ship | 0x007f2d8c | 0x0069f330 | byte-identical shape to Drive, same offsets, own NullCoord globals 0x00b077f8/fc/800 | VERIFIED |
| Walk | 0x007f69f8 | 0x0075ab40 | `slot4() && ((double)owner[+0x578] > 0.0) && !(this[+0x24/+0x28/+0x2c] == NullCoord)` — third term is the **head-to** (COORD B), not the destination | VERIFIED |
| Hover | 0x007eacfc | 0x00514c80 | `slot4() && ((double)this[+0x44] != 0.0)` — `TEST AH,0x40`, so **`!= 0.0`**, negatives count as moving; the double is on the **locomotor** | VERIFIED |
| Fly | 0x007e89f4 | 0x004ccac0 | `((double)this[+0x44] != 0.0)` — **no slot-4 call, no owner deref** | VERIFIED |
| Jumpjet | 0x007ecd68 | 0x0054d0d0 | `s = (i32)this[+0x4c]; s != 0 && s != 2` — fields fully disjoint from slot 4 (`byte this[+0x48]`) | VERIFIED (predicate); **enum value meanings UNDECODED** |
| Teleport | 0x007f5000 | **0x004b6610** (base forwarding thunk) | `slot4()` re-dispatched → `byte this[+0x30] == 1` (**equality with 1**, not `!= 0`) | VERIFIED |
| Rocket | 0x007f0b1c | 0x00661f90 | `s = (i32)this[+0x3c]; 3 <= s && s <= 5` (signed `JL`/`JG`) — object+0x40, written by slot 16's `switch`; 3=boost, 4=cruise, 5=terminal | VERIFIED |
| Mech (dormant) | 0x007edb6c | 0x005b19e0 | Drive-shaped `TimerRemaining(owner+0x388) \|\| (slot4() && …)` on different offsets (coord at this+0x20) | VERIFIED body; dormant in stock YR (CLSID commented out everywhere) |
| DropPod (dormant) | 0x007e8278 | **0x004b6610** (inherits base) | `slot4()` → base slot 4 = `XOR AL,AL` = **always false** | VERIFIED |
| LocomotionClass base | 0x007eadf4 | 0x004b6610 | `slot4()`; base slot 4 (0x0055acd0) = always false | VERIFIED |
| Tunnel | *none* | — | Tunnel's only COL has `offset = 0`; it has **no ILocomotion subobject and no slot 32** | VERIFIED |
| Parachute | *does not exist* | — | No `ParachuteLocomotionClass` in gamemd.exe (12 locomotor classes total) | VERIFIED |

Exhaustiveness: `get_xrefs_to 0x004b6610` returns exactly 3 data refs (0x007e82f8 / 0x007eae74 / 0x007f5080 = base+0x80 of DropPod / base / Teleport). So **Teleport is the only live-YR family that inherits the base answer**, provably, not by hedge.

**Disputes / open items in this table:**
- **Drive/Ship term 1.** The *content* is `TimerRemaining(owner+0x388)` — VERIFIED verbatim, helper 0x004c9480, `__thiscall`, ECX = owner+0x388. That `owner+0x388` **is the turn timer is UNCHECKED.** This is the single most load-bearing unverified identity in the whole set (see §5 item 3).
- `owner->vt[0x538]` (slot 334): body and slot index VERIFIED; the name `Apparent_Speed` is a **pre-existing Ghidra label, not evidence**. Return is a signed int.
- `owner+0x578` (Walk double) and `owner+0x2e8` (Fly float, slot 4 only) identities UNCHECKED.
- Ship slot 4's "Z component dead" detail is INFERRED from the decompile only; Drive's was verified in raw asm.
- Lane-1's `== 0` paraphrase of every null-coord test is a paraphrase: the code compares against per-family NullCoord globals. All five read as 12 zero bytes, so it is behaviourally exact.
- Slot counts: 50 for Drive/Walk/base, 38 for Fly. Not load-bearing (all interesting slots < 38), but the inherited "40 slots" anchor is wrong.

---

## 2. Per-family verdict on `ready_producer.rs`

**Drive / Ship (lines 107–154) — RIGHT SLOT, one UNCHECKED input that fails in the stall direction.**
The doc comment `turning_active || (slot_moving && head_to_nonnull && owner_speed > 0)` is slot 32, term-for-term, in short-circuit order, with the correct signed `> 0`. The Rust `slot_moving` correctly reproduces native slot 4 including the **X/Y-only, Z-ignored** owner-coord compare (VERIFIED in raw asm at 0x004afbdd/0x004afbe0 — Z is loaded to a stack slot and never `CMP`'d). `head_to`/`destination` field naming is faithful: Drive slot 5 `Destination` copies +0x30 and slot 6 `Head_To` copies +0x3c, and slot 32 tests the +0x3c triple. Verdict **MATCHES** on structure; input 1 (`turning_active` ← `body_facing.is_rotating`) is **WRONG-INPUTS-RISK / UNCHECKED** — corrected comment must say *"first term is a timer countdown on the owner at a field whose identity is UNCHECKED; we read the facing-rotation timer"*, not assert `turning_active`.

**Teleport (162) — MATCHES, and it is the only family where slot 4 is the right thing to read.**
`state == 1` reproduces `byte == 1` equality exactly. Missing from the comment: native slot 32 here is the **base forwarding thunk to slot 4**, uniquely among live families — worth stating so nobody "fixes" it toward a phase test. Mapping of native value 1 → `TeleportPhase::Relocate` is UNCHECKED (no writer enumeration of Teleport's +0x30 in either lane).

**Jumpjet (177) — RIGHT SLOT, WRONG-INPUTS (invented enum).**
The predicate `state != 0 && state != 2` is right. The *values* are fabricated: nobody decoded Jumpjet's state machine, and the native field (iface+0x4c) is disjoint from slot 4's byte (iface+0x48), so there is no cross-check. `AirMovePhase::Descending => 4` makes a **landing** jumpjet report moving. Lane 4 showed how to close this: Rocket's phase semantics were recovered by decompiling its slot-16 `switch`. The same route exists for Jumpjet and was not taken. Until it is, the honest arm is `Descending => 2` (not-moving) and a comment saying the enum is UNDECODED.

**Walk (208) — RIGHT SLOT, one factually false comment and one input that fails in the stall direction.**
Right slot: yes, all three conjuncts in order, and the note that the third input is the head-to (not the final destination) is **CONFIRMED**. Two defects:
- Line 214's comment *"Native's speed fraction for a walker is only ever 1.0 or 0.0; the Walk locomotor never writes it"* is **flatly wrong**. `WalkLocomotionClass__ProcessMovement` calls the owner's `vt[0x544]` (`SetSpeedFraction`, 0x004d3710, clamped to [0,1]) at **9 sites**, with `PUSH 0x3ff00000` (1.0) at only 2 of them. Delete the claim.
- `destination_nonnull ← path.get(next_index).is_some()` is **WRONG-INPUTS**. Native's COORD B is the *sub-cell* point produced by `FindSubCellDest` → `CellClass__PlaceInfantryInCell`; it is null when no free infantry sub-cell exists in the next cell, and on the tick an order is issued (byte set synchronously in `Head_To_Coord`, COORD B only filled inside `Process`). VERA's path lookup is `Some` in both cases ⇒ VERA answers moving where native answers not-moving.
- The `Blocked`-phase exclusion should be **relabelled**: it is no longer VERA-internal/UNCHECKED. Native reaches the same answer for a blocked walker, through a different mechanism (COORD B stays null ⇒ third conjunct fails), not a phase enum. Same outcome, different mechanism — say exactly that.

**Hover (233) — RIGHT SLOT, over-inclusive input (admitted), one wrong justification.**
`slot_moving && speed != 0.0` is correct, and `native_double_ordered_not_zero` correctly implements `TEST AH,0x40` including negatives-count-as-moving and NaN→false. Native slot 4 for Hover is a two-triple coord test (both null ⇒ false); `movement_target.is_some() || nav_com.is_some()` is a loose analogue — UNCHECKED but harmless in isolation. The comment *"The speed term is the strict one"* is wrong: `!= 0.0` is **weaker** than `> 0.0`, it accepts negative speeds. The forgiveness argument it supports is therefore unsupported.

**Fly / Rocket / Parachute / dormant `_ => None` (73–86) — the comment is wrong in three places and undercounts the arm.**
- ✅ "Aircraft readiness … never reads the locomotor at all" — **TRUE.** `AircraftClass` slot 0x200 = 0x0041b5e0 makes no locomotor call (full predicate: `mission != 6 && mission != 0x15 && (byte[+0x6d2] == 0 || mission == 0x1e)` → `return byte[+0x6d4] != 0`).
- ✅ "Rocket-locomotor objects are not Unit- or Infantry-category" — **TRUE.** V3ROCKET (11389), DMISL (11429), CMISL (11472) are all in `[AircraftTypes]` (roster lines 1163/1165/1171).
- ❌ "this arm is unreachable state" — **FALSE for the slot, true only for the gate.** gamemd reads slot 32 on every Foot object every tick from `FootClass__AI` (4 sites, receiver = locomotor iface at techno+0x674), and `AircraftClass__Is_Weapon_Ready` (0x0041b980) is literally `!Is_Moving_Now`.
- ❌ "these families do own a readiness-slot override rather than inheriting the base one" — **FALSE.** Fly, Rocket and Mech override; **DropPod inherits the base thunk**; **Tunnel and Parachute have no native slot at all** (Tunnel's COL offset is 0; `ParachuteLocomotionClass` does not exist in the binary).
- ❌ "Fly, Rocket, Parachute and the **two** dormant TS kinds" — the arm actually catches **six** kinds: Fly, Rocket, Parachute, Tunnel, DropPod, **Mech**. Mech is omitted, and Mech is the one with a real Drive-shaped slot-32 override at 0x005b19e0.

The same three false claims are duplicated verbatim in `authority.rs:195–211` above `DEGRADED_NOT_MOVING` and must be fixed in both places.

---

## 3. Gate ordering verdict

**Derive on demand inside the gate. The current precompute-once-per-tick design is DRIFT, and the doc comment defending it is false.**

Citations: `MissionClass__Queue_Mission` (0x005b35e0) calls the readiness virtual and only then `Commence`: `cVar2 = (**(code**)(*param_1+0x200))(); if (cVar2 != '\0') (**(code**)(*param_1+0x1ec))();`. Every gate invocation is a fresh virtual call on the object's own vtable — `disassemble_bytes 0x0051bbf0`: `MOV EAX,[ESI]; MOV ECX,ESI; CALL [EAX+0x200]` — and the gate body itself reads the locomotor live: `MOV EAX,[ESI+0x674]; PUSH EAX; MOV ECX,[EAX]; CALL [ECX+0x80]` at 0x00521ba7. **No cached per-frame moving byte exists anywhere on this path.**

Two things make this more than a stylistic preference:
1. `InfantryClass__AI` calls the gate at **0x0051bc1c, then `FootClass::AI` at 0x0051bc9f (whose `Process` call is at 0x004da877), then the gate again at 0x0051bed1`. The same object's readiness is legitimately evaluated on both sides of its own locomotion in one tick. Lane 3 proved the two answers can differ (pre-`Process` sample at 0x004da692 reads false on a walk order's first tick, post-`Process` samples read true).
2. The gate is consulted from far more than the AI loops — the exhaustive `CALL [reg+0x200]` search returns **28 hits, not truncated**, including `FootClass__Receive_Radio` ×2, `UnitClass__Receive_Radio`, `UnitClass__PerCellProcess`, `TechnoClass__Unlimbo`, `AircraftClass__Set_Destination`, `UnitClass__Mission_Deploy_Building` ×5, `TeleportLocomotionClass__Process`, `BuildingClass__Update` ×2. These fire mid-tick, in response to events that themselves change locomotor state. A once-per-tick cache answers all of them with stale state by construction.

So `ready_producer.rs:48–52` — *"the value it sees is the state as of the last completed movement, which is the same thing the native live virtual call observes at that point in the tick"* — is **false**. It is true for exactly one of the ~24 native gate sites (the one immediately preceding that object's locomotion) and false for the rest. Lane 2's own summary ("evaluated live, twice per tick") is also wrong in the other direction; the verifier caught it. The correct statement is "evaluated live at every call, from ~24 sites, and the answer may change between two calls in the same tick."

Evidence sufficiency, stated plainly: it is **VERIFIED** that native derives readiness live and that no cache exists. It is **UNCHECKED** whether any VERA gate site today sits on the wrong side of a same-tick locomotor state change — no trace was run. The highest-risk shape is a same-tick stop followed by a mid-tick `Queue_Mission(commence)`: dock/unlink/unload/deploy handoffs, where a stale "moving" defers the mission. Recommend converting `ready_state_for` into a pure read called from the gate (it already takes `&GameEntity` and has no side effects), keeping the cached field only if the snapshot/hash consumers need it.

---

## 4. Fly / Rocket

**No readiness mapping is warranted for stock YR play. VERIFIED, from two independent directions:** the `AircraftClass` slot-0x200 override makes no locomotor call, and the three live Rocket-locomotor objects are `[AircraftTypes]` (also `Selectable=no`, so no player order reaches them). Building `AirMovePhase`-shaped or `F64_BITS`-shaped readiness inputs for them now would be speculative work — and lane 4's proposed `F64_BITS_*` table for Fly was **refuted** anyway: `FST double [ESI+0x40]` stores the raw un-quantised quotient `dist/typeclass[+0x2f8]`, ramp D assigns `current = target` verbatim when the gap is ≤0.1, and `Horizontal_Step` writes 0.5 and 0.75 — the value set is not closed, so a lookup table would be unsound.

The current comment's *conclusion* is right and its *reasons* are wrong. Suggested replacement for lines 73–86:

```rust
// Fly, Rocket, Parachute, Tunnel, DropPod and Mech have no producer, and none
// is needed for THIS gate: the native Aircraft readiness override makes no
// locomotor call at all, and the three stock Rocket-locomotor objects
// (V3ROCKET, DMISL, CMISL) are AircraftTypes, so they never reach the Unit or
// Infantry branch either.
//
// What is NOT true is that the native slot is idle for them. gamemd reads
// Is_Moving_Now on every Foot object every tick -- shroud reveal, the movement
// counter, and the looping move sound -- and aircraft weapon-readiness is
// literally !Is_Moving_Now. VERA has none of those consumers yet; when they
// land, Fly needs a ramped 0..1 speed fraction (not a quantised table: the
// native value is an un-quantised distance quotient) and Rocket needs a
// launch-phase enum with `3..=5` meaning moving. Neither is a variant of a
// mapping below.
//
// The dormant kinds are not uniform: Mech overrides the slot, DropPod inherits
// the always-false base, and Tunnel and Parachute have no native locomotor
// class or slot at all -- Parachute is VERA-internal.
```

---

## 5. Prioritised change list

**Tier 1 — can answer "moving" where native says "not moving" (stalls units):**

1. **Walk `destination_nonnull` uses the path, native uses the sub-cell head-to.** `path.get(next_index)` is `Some` in exactly the two cases where native's COORD B is null: no free infantry sub-cell in the next cell, and the order-issue tick before `Process` runs. Consequence: infantry defer missions while native commences them — a squad member that can't claim a sub-cell holds its order. Frequency: fires whenever two or more infantry converge on one cell, which is normal in any squad move; the specific "constantly" framing from lane 3 was flagged by its verifier as uncited, so treat the *mechanism* as verified and the *rate* as unmeasured.
2. **Jumpjet's state enum is invented; `Descending => 4` reports moving on landing.** Native's field is undecoded and disjoint from slot 4, so there is no evidence for any of the five assignments. Consequence: a landing Rocketeer or Siege Chopper defers its mission for the whole descent. Frequency: every jumpjet landing — several per minute once Rocketeers or Siege Choppers are on the field. Interim fix is one line (`Descending => 2`); the real fix is decompiling the Jumpjet slot-16 writer the way lane 4 did Rocket's.
3. **Drive/Ship `turning_active` rests on an unverified field identity.** It is the only term that can return true alone, so a wrong reading is precisely the stall direction, and it is attached to the highest-frequency event in the game (every vehicle turn on every move order). Nothing is known to be broken — but nothing verifies it either, and the comment asserts it as fact. Either close the identity of `owner+0x388` or downgrade the comment to UNCHECKED; do not leave a confident label on an unproven load-bearing term.
4. **Gate ordering: move derivation into the gate.** A cached end-of-movement value is stale for every mid-tick gate call (radio receive, per-cell process, unlimbo, deploy, set-destination). Consequence: a unit that stopped earlier in the same tick still reads "moving" and defers the mission it was just handed — the dock/unload/deploy handoff family. Frequency: every harvester dock cycle and every transport unload, if the mid-tick path exists in VERA; whether it does today is UNCHECKED.

**Tier 2 — false or misleading provenance that will justify the next wrong change:**

5. `ready_producer.rs:213–214` — *"the Walk locomotor never writes it"* and the {0.0, 1.0} range claim: both refuted (9 `SetSpeedFraction` sites, only 2 pushing 1.0). Consequence of leaving it: the next session reasons from a false premise about a term that gates every infantry mission.
6. `ready_producer.rs:73–86` — three false clauses plus a miscount (six kinds, not five; Mech omitted; DropPod inherits; Tunnel/Parachute have no native slot).
7. `authority.rs:195–211` — the same false claims duplicated above `DEGRADED_NOT_MOVING`.
8. `ready_producer.rs:48–52` — the "same thing the native live virtual call observes" claim is false; fix it whether or not item 4 lands.
9. Walk's blocked-phase exclusion: upgrade from "VERA-internal, gamemd equivalent UNCHECKED" to "native agrees via the head-to lifetime; the mechanism differs."
10. Hover's *"the speed term is the strict one"* — it is `!= 0.0`, which is weaker than `> 0.0` and admits negatives. Cosmetic in effect, but it is the sentence that licenses the over-inclusive `slot_moving`.
11. Teleport: record that slot 32 is the base forwarding thunk to slot 4, the only live family where that is true.
12. Add a one-line warning next to `F64_BITS_*` that the closed-value-set trick is a property of Hover's request only and does **not** generalise to Fly (whose native value is an un-quantised quotient).

**No player-visible consequence, worth a line each:** the Rust already carries two terms the lanes reported as missing — `readiness.rs:182` (`tracker_byte_18/19`) is the Unit gate's `FUN_004a51d0` (`byte[self+0x368]==0 && byte[self+0x369]==0`) precondition, and `readiness.rs:247` (`current != MISSION_AIRCRAFT_ACTION_EXCEPTION`) is the aircraft gate's `mission == 0x1e` disjunct. Both in the right branch order. Neither field-offset correspondence is verified, but nothing is missing.

---

## 6. What remains UNCHECKED, and the instrument that closes it

| Open item | Instrument |
|---|---|
| **Every mapping in `ready_producer.rs`** — all eight are traced correspondences, zero are machine-checked | `emulate_function` on each slot-32 body against a synthesized locomotor block, compared to `LocomotorReadyState::is_moving_now` over a fixture grid. Feasible: all bodies return in AL, so the registers-only limit is not a blocker. Walk/Hover/Fly/Rocket/Jumpjet/Teleport are self-contained; Drive/Ship need `TimerRemaining` and `owner->vt[0x538]` stubbed. Rocket needs a Ghidra function created at 0x00661f90 first (none is defined). **This is the only thing that would turn any of these from UNCHECKED to VERIFIED.** |
| Identity of `owner+0x388` (Drive/Ship term 1) | `search_instructions` for writers of `+0x388`, decompile the setter; or `debugger_watch_memory` on a turning tank |
| Identity and name of `owner->vt[0x538]` (slot 334) | Resolve slot 334 in a concrete UnitClass/InfantryClass vtable and decompile the override — the `Apparent_Speed` label is unverified |
| Identity of `vtbl[0x184]` (compared to 1 / 5 / 0xf, called **three separate times** in the Infantry gate — not cached) | Decompile the Unit and Infantry overrides at base+0x184. VERA already treats it as effective-mission with 5=GUARD, 0xf=HUNT, 1=ATTACK+target; that correspondence is UNCHECKED, and the three-call detail means a Rust port evaluating it once may diverge if it has side effects |
| Jumpjet phase enum values and its writer | Decompile Jumpjet slot 16 (same route as Rocket's 0x006622c0) |
| Teleport `+0x30` writers and the meaning of value 1 | `search_instructions` on the Teleport locomotor body set |
| Ship slot 4's Z-dead component | Read Ship slot 4 in raw asm (only Drive's was) |
| Whether Walk's `owner+0x578` double and Drive/Ship's `vt[0x538]` int can disagree | Decompile the `vt[0x538]` override and compare with `SetSpeedFraction`'s writes |
| Whether any VERA gate site is on the wrong side of a same-tick locomotor change | A tick-phase trace of VERA's own gate callers — Rust-side instrumentation, not Ghidra |
| Fly `typeclass[+0x2f8]`, `object+0x50/+0x51` as IsAscending/IsDescending (plate-comment provenance, not derived) | Only needed if the Fly consumers land; ReadINI field-map cross-reference |

Corrections that should be written back to Ghidra before they die with these sessions (none was, both lanes were read-only): `UnitClass__ShouldIdle` @0x00744270 is the UnitClass readiness-to-commence override; `FUN_00521b60` is the Infantry one; 0x004b5b00 is a **destructor**, not the DropPod constructor; `WalkLocomotionClass__Mark_All_Occupation_Bits` @0x0075ae30 marks no occupation bits and is a hidden fourth writer of the Walk moving byte; `FlyLocomotionClass__Move_To`/`Stop_Moving`/`Layer` (0x004cf610 / 0x004cf830 / 0x004ccb40) are a draw-matrix provider, a 20-frame sine bob, and `Process` respectively; the Walk constructor's "7-state machine" plate comment describes a struct that does not exist.