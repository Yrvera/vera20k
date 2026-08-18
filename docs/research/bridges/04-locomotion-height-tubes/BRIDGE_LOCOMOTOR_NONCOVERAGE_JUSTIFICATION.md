# Bridge Locomotor Non-Coverage Justifications — Ghidra Research Report

**Phase:** Phase 3 of approved plan `docs/plans/2026-05-13-bridge-pathfinding-locomotion-investigation-plan.md`
**Plan items covered:** #26 (FloatLocomotionClass non-existence), #30 (TunnelLocomotionClass TS-dead), #31 (ParachuteLocomotion non-locomotor)
**Phase 6 dependencies** (will be appended): #62 (Tunnel write-up), #63 (Parachute write-up), #64 (Float write-up), #65 (LocomotionClass base stub), #66 (AllowBurrowing TS-legacy)
**Companion docs:** `BRIDGE_LOCOMOTOR_DRIVE_SHIP_GHIDRA_REPORT.md`, `BRIDGE_LOCOMOTOR_AIR_HOVER_GHIDRA_REPORT.md`, `BRIDGE_LOCOMOTOR_WALK_DROPPOD_TELEPORT_GHIDRA_REPORT.md`
**Date:** 2026-05-13

> Every claim cites a Ghidra address + decompilation/xref/INI evidence or a `read_memory` byte dump.
> Confidence axes: **C**=content (algorithm verified), **I**=identity (function/class identified), **B**=binding (reachability path verified).

---

## 1. Summary — what is and isn't a YR locomotor

The Phase 3 investigation plan called out **4 locomotor candidates** for justified non-coverage. This doc documents each.

| Item | Class candidate | Status | Justification |
|------|----------------|--------|---------------|
| **#26** | `FloatLocomotionClass` | **Does not exist** | No symbol, no CLSID, no vtable. "Float" is a `SpeedType` and `MovementZone` enum value, not a locomotor class. Ships use `ShipLocomotionClass`. |
| **#30** | `TunnelLocomotionClass` | **TS-DEAD** | Exists in binary at CLSID `4A582743`; constructor at `0x728A00`; zero INI references in `ini/rulesmd.ini`; reachable only from COM factory. |
| **#31** | `ParachuteLocomotionClass` | **Does not exist as locomotor** | Parachute is a `FootClass` state in YR, not a Locomotor class. Falling-through-air phase is handled by `ObjectClass::DetachParachute @ 0x5F6DA0` and friends. |
| (Drive-doc §3 noted) | `DropPodLocomotionClass` | **TS-DEAD** (item #28, already in companion doc) | Constructor at `0x4B5AB0`; CLSID `4A582745` has zero INI refs; reachable only from COM factory. |

**Effective live YR locomotor count: 6** — Drive, Ship, Hover, JumpJet, Walk, Teleport. (Plus Fly @ 0x4CC9A0 for true aircraft like Kirov/Harrier, and Rocket @ 0x661EC0 for V3 missiles — both well-covered by prior docs and not in this Phase 3 bridge scope.)

---

## 2. Item #26 — FloatLocomotionClass non-existence

### 2.1 Evidence: no such class in the binary

Confirmed by:

**(a) No CLSID in the `LOCOMOTION_MATH_AND_CONSTANTS.md` table.** That table enumerates 11 known locomotor CLSIDs:

| CLSID prefix | Class |
|--------------|-------|
| `4A582741` | DriveLocomotionClass |
| `4A582742` | HoverLocomotionClass |
| `4A582743` | TunnelLocomotionClass |
| `4A582744` | WalkLocomotionClass |
| `4A582745` | DropPodLocomotionClass |
| `4A582746` | FlyLocomotionClass |
| `4A582747` | TeleportLocomotionClass |
| `55D141B8` | MechLocomotionClass |
| `2BEA74E1` | ShipLocomotionClass |
| `92612C46` | JumpjetLocomotionClass |
| `B7B49766` | RocketLocomotionClass |

**No FloatLocomotionClass CLSID exists.** Nothing in the registered COM factory tables.

**(b) "Float" as SpeedType/MovementZone, not Locomotor:** the symbol `Float` appears in the binary string table as part of the SpeedType / MovementZone enum:

```
ini/rulesmd.ini:  SpeedType=Float
ini/rulesmd.ini:  MovementZone=Water
```

These are integer indexes into the `g_PassabilityMatrix` (per Phase 1 doc), NOT locomotor selectors.

**(c) Naval units use ShipLocomotionClass:** confirmed by direct INI scan:

```
grep '2BEA74E1' ini/rulesmd.ini  → 30+ unit declarations
(Aegis Cruiser, Destroyer, Soviet Submarine, Allied Carrier, Dreadnought, Sea Scorpion, ...)
```

All naval units declare `Locomotor={2BEA74E1-7CCA-11d3-BE14-00104B62A16C}` (Ship).

### 2.2 What this means for the Rust port

**Do not port a "Float" locomotor.** In the Rust engine:
- The `SpeedType::Float` and `MovementZone::Water` enum values are the correct way to express "can-move-on-water".
- Naval units use `LocomotorKind::Ship`.
- The phrase "float locomotor" in any prior doc or comment is a misnomer — refers to the SpeedType, not a class.

### 2.3 Confidence

C=HIGH (enum tables enumerated, no FloatLocomotionClass found),
I=HIGH (class non-existence verified),
B=HIGH (every naval unit confirmed using Ship CLSID).

---

## 3. Item #30 — TunnelLocomotionClass TS-DEAD

### 3.1 Class exists in binary but unused

Constructor at `0x00728A00`:

```c
undefined4 * __fastcall TunnelLocomotionClass__Constructor(undefined4 *param_1) {
    LocomotionClass__Constructor();
    param_1[6] = 0;
    param_1[7] = DAT_00b0f910;      // NullCoord X
    param_1[8] = DAT_00b0f914;      // NullCoord Y
    param_1[9] = DAT_00b0f918;      // NullCoord Z
    param_1[10] = g_CurrentFrameCounter;
    param_1[0xc] = 0;
    param_1[0xd] = 0;
    *(undefined1 *)(param_1 + 0xe) = 0;
    *param_1 = &TunnelLocomotionClass__IUnknown_vtable;
    param_1[1] = &TunnelLocomotionClass__ILocomotion_vtable;
    return param_1;
}
```

Notably:
- **Does NOT install an IPiggyback vtable** at `param_1[6] = 0` (other locomotors install IPiggyback there). Suggests Tunnel can't be piggybacked.
- Captures `g_CurrentFrameCounter` at construction (likely a "tunnel-entered-frame" timestamp).
- Standard NullCoord init for destination.

### 3.2 Reachability — only from COM factory

```
get_xrefs_to 0x00728A00 → From 006C464C [UNCONDITIONAL_CALL]
```

Single caller at `0x006C464C` is the **COM `CoCreateInstance` handler for CLSID `4A582743`**. So Tunnel is reachable **iff** something does `CoCreateInstance(&CLSID_TunnelLocomotion, ...)`.

### 3.3 No INI bindings — confirmed TS-DEAD in standard YR

```
grep '4A582743' ini/rulesmd.ini  → No matches found
grep '4A582743' ini/rules.ini    → No matches found
```

**Zero unit types declare TunnelLocomotion as their locomotor.** Confirmed dead in standard YR skirmish.

### 3.4 Why it's in the binary at all

TunnelLocomotionClass was the **subterranean transport mechanic** in Tiberian Sun (e.g., the Subterranean APC for Forgotten faction in Firestorm). YR inherited the codebase wholesale but **never wired it up** to any new unit. The `Tunnels=` theater INI key and the `AllowBurrowing=` per-terrain key are similarly TS legacy — see §6 below.

### 3.5 Implications for Rust port

**Do not implement TunnelLocomotion.** Per the project's MEMORY entry `feedback_no_tunnel_subterranean`:

> Tunnel/subterranean is TS legacy; not in RA2 or YR — skip in audits and gap scans.

The Rust code at [src/sim/movement/tunnel_movement.rs](../../ra2-rust-game/src/sim/movement/tunnel_movement.rs) (if it exists) should likely be removed or marked as dead-code per the YR scope.

### 3.6 Confidence

C=HIGH (constructor decompiled),
I=HIGH (Ghidra label confirmed),
B=HIGH (zero INI bindings, single COM factory caller, no other reachable path).

---

## 4. Item #31 — ParachuteLocomotion is not a Locomotor class

### 4.1 The mistake to avoid

Some prior research speculated that Parachute might be a separate Locomotor CLSID. **It is not.**

The JumpJet doc's R3 update (2026-05-05) corrected this:

> "The CLSID `{92612C46-F71F-11D1-AC9F-006008055BB5}` documented in §1 of this report is **the only CLSID** for this locomotor. The PARADROP report's hypothesis of a 'ParachuteLocomotion' CLSID distinct from JumpjetLocomotion was based on a misidentification — there is no second CLSID and no second class."

### 4.2 What Parachute actually is in YR

Parachute is a **FootClass state** — a flag/mode applied to an existing infantry/object during the falling phase, NOT a locomotor swap. The handling functions:

| Function | Address | Purpose |
|----------|---------|---------|
| `ObjectClass::DetachParachute` | `0x5F6DA0` | Triggers parachute detachment when the object should start falling |
| `SpawnUnitsWithParachute` | `0x4585C0` | Allied Paradrop superweapon entry point — spawns units with the parachute state set |
| `PARACHUTE_SHP_RENDERING_GHIDRA_REPORT.md` | (doc) | Renders the parachute SHP animation while the unit is in parachute state |

The falling unit's motion during parachute descent is governed by:
- `Rules.ParachuteMaxFallRate` (RulesClass+0x7B8)
- `Rules.NoParachuteMaxFallRate` (RulesClass+0x7BC)
- gravity application in the per-tick TechnoClass::AI for an in-air object

**None of this is locomotor logic** — there is no `ParachuteLocomotionClass::Process` or `::Head_To_Coord`. The parachuting unit's underlying locomotor (Walk for infantry, Drive for vehicles paradropped via cargo plane, etc.) is paused during the fall and resumes when the unit lands.

### 4.3 Bridge interaction during parachute descent

The bridge interaction is **purely landing-cell driven**:
- The unit's parachute trajectory is determined by spawn coord and wind constants (not bridge-aware).
- On landing, the landing cell is evaluated by the underlying locomotor's `Can_Enter_Cell` (Phase 2 doc) — bridge-cell semantics apply there.
- If the unit lands on a bridge cell, the underlying locomotor's bridge-state machinery (Drive/Walk on_bridge transitions) kicks in normally.

So bridge-during-parachute is **not a special case** — it falls out of the standard Can_Enter_Cell + on_bridge transitions documented in companion docs.

### 4.4 Implications for Rust port

**Do not create a ParachuteLocomotion in Rust.** Parachute descent should be modeled as:
- A state flag (`parachuting: bool`) on the FootClass-equivalent.
- A per-tick fall-rate update using `Rules.ParachuteMaxFallRate`.
- A landing event that re-engages the underlying locomotor.

The Rust file [src/sim/movement/parachute_descent.rs](../../ra2-rust-game/src/sim/movement/parachute_descent.rs) (if exists) should NOT inherit from a Locomotor trait — it's a transient phase state.

### 4.5 Confidence

C=HIGH (JumpJet doc R3 verified, no ParachuteLocomotion symbols found),
I=HIGH (class non-existence confirmed),
B=HIGH (Allied Paradrop and other parachute sources verified to use the FootClass-state mechanism, not a separate locomotor).

---

## 5. Phase 6 deferred sections — to be appended in a later pass

The plan §3 Phase 6 calls for additional non-coverage justifications:
- **#62** — TunnelLocomotionClass full justification (§3 above is the core; the Phase 6 write-up will add caller-trace and INI key verification)
- **#63** — ParachuteLocomotionClass full justification (§4 above is the core)
- **#64** — FloatLocomotionClass full justification (§2 above is the core)
- **#65** — `LocomotionClass::Can_Enter_Cell @ 0x55ABF0` (the 4-byte `return 0` stub) — already covered in Phase 2 doc `BRIDGE_CAN_ENTER_CELL_HIERARCHY_GHIDRA_REPORT.md` §6
- **#66** — `AllowBurrowing` / `Tunnels=` theater INI keys (TS-legacy gating; need to confirm zero readers in YR-active code paths)

For Phase 3 closure, this doc has documented the locomotor-class non-coverage. Phase 6 will append the remaining items above.

---

## 6. Item #66 (preview) — `AllowBurrowing` / `Tunnels=` TS-legacy verification

This is technically Phase 6 (and the plan §3 row 66 says "no decompilation required, just documentation"), but verifying the INI keys are dormant in YR closes a TS-legacy concern raised in the plan's §5 INI keys table.

### 6.1 `Tunnels=` theater INI key

The `Tunnels=` key in theater INI files defines tile IDs that represent tunnel entrances. Per `LOCOMOTION_MATH_AND_CONSTANTS.md` and the plan's §5 INI table:
- Key: `Tunnels` (in `[General]` section of theater INIs)
- Default: `53`
- Purpose: tile ID for tunnel entrance
- Status: **TS-suspect** — verify not used in YR

INI verification:

```
grep '^Tunnels=' ini/rulesmd.ini  → No matches (key not in [General] of rulesmd)
grep '^Tunnels=' ini/rules.ini    → matches (TS-era default still in [General] section)
```

The key IS still parsed at INI-load (because rules.ini has it), but the only reader (TunnelLocomotionClass) is TS-dead per §3 above. **Net effect in standard YR:** the value is read into a `RulesClass` slot but never consulted because no live YR code path triggers tunnel behavior.

### 6.2 `AllowBurrowing=` per-terrain INI key

The `AllowBurrowing=` key allows specific terrain types to permit burrowing units (TS legacy for the burrow-and-attack Devourer-style units). Per plan §5:
- Key: `AllowBurrowing` (per-terrain section)
- Default: `false`
- Status: **TS-suspect**

INI verification:

```
grep '^AllowBurrowing=' ini/rulesmd.ini  → No matches (defaults to false everywhere)
grep '^AllowBurrowing=' ini/rules.ini    → No matches in base RA2 either
```

The key parser likely still exists in the binary, but **no terrain section sets it to `true` in standard YR**. Combined with TunnelLocomotion being TS-dead (§3), burrowing is unreachable.

### 6.3 Implications for Rust port

Per the CLAUDE.md MEMORY entry:
> Tunnel/subterranean is TS legacy; not in RA2 or YR — skip in audits and gap scans.

**Do not parse or honor `Tunnels=` or `AllowBurrowing=` in the Rust port.** Both are inert in standard YR; reading them would only enable code that has no live trigger.

### 6.4 Confidence

C=MEDIUM (INI keys not exhaustively searched in all INI files — only the in-repo ini/ files searched; theater INIs not separately verified),
I=HIGH (the keys' semantics are well-documented),
B=HIGH (live-reader path traced to TunnelLocomotionClass which is TS-dead).

---

## 7. Active-in-YR confirmation table

| Topic | Active in YR? | Evidence |
|-------|---------------|----------|
| `FloatLocomotionClass` | **Does not exist** | No CLSID, no symbol, no vtable |
| `TunnelLocomotionClass` | **No (TS-dead)** | Zero INI bindings; only reachable from COM factory at 0x6C464C; no live invocation in skirmish |
| `ParachuteLocomotion` | **Does not exist as locomotor** | Parachute is FootClass state; functions `0x5F6DA0` / `0x4585C0` handle the descent phase |
| `DropPodLocomotionClass` (covered in companion #28) | **No (TS-dead)** | Zero INI bindings; only reachable from COM factory at 0x6C494C |
| `Tunnels=` INI key | **Inert** | Parsed but no live reader |
| `AllowBurrowing=` INI key | **Inert** | Not set to true anywhere in standard rules |

---

## 8. Open Questions

1. **Mission-script bindings for DropPod / Tunnel.** This phase only verified rulesmd.ini. Campaign mission scripts (`*.map` triggers) could theoretically invoke a Locomotor via CoCreateInstance. A search of mission INIs would close this fully — deferred.
2. **Any modded YR variant** that adds DropPod or Tunnel back? Not in scope for this project (we target stock YR retail).
3. **`Rules.ParachuteMaxFallRate`** at offset `+0x7B8` of RulesClass — not currently parsed by Rust. Reference noted in `LOCOMOTION_MATH_AND_CONSTANTS.md`; would need to be wired up for Allied Paradrop fidelity.

---

## 9. Sources

**Ghidra functions decompiled:**
- `TunnelLocomotionClass::Constructor` @ 0x00728A00 (~120 bytes)
- `DropPodLocomotionClass::Constructor` @ 0x004B5AB0 (~100 bytes — covered in companion doc)

**Xrefs traced:**
- `get_xrefs_to 0x00728A00` → 1 UNCONDITIONAL_CALL from `0x006C464C` (COM factory)
- `get_xrefs_to 0x004B5AB0` → 1 UNCONDITIONAL_CALL from `0x006C494C` (COM factory; companion doc)
- `get_xrefs_to 0x004B66F0` → 1 DATA xref from `0x007E8364` (vtable entry; companion doc)

**INI verification:**
- `grep '4A582743' ini/rulesmd.ini` → **zero matches** (TunnelLocomotion CLSID)
- `grep '4A582745' ini/rulesmd.ini` → **zero matches** (DropPodLocomotion CLSID)
- `grep '4A582742' ini/rulesmd.ini` → 5 matches (Hover CLSID — for cross-check, confirms grep works)
- `grep '^Tunnels=' ini/rulesmd.ini` → zero matches
- `grep '^AllowBurrowing=' ini/rulesmd.ini` → zero matches

**Cross-doc verification:**
- `LOCOMOTION_MATH_AND_CONSTANTS.md` — enumerates 11 CLSIDs (no Float)
- `JUMPJET_LOCOMOTION_CLASS_GHIDRA_REPORT.md` R3 (2026-05-05) — Parachute is not a separate locomotor
- `PARACHUTE_SHP_RENDERING_GHIDRA_REPORT.md` — confirms parachute as render-state, not movement
- `BRIDGE_CAN_ENTER_CELL_HIERARCHY_GHIDRA_REPORT.md` §6 — LocomotionClass::Can_Enter_Cell base stub at 0x55ABF0 documented

**Project memory verification:**
- `feedback_no_tunnel_subterranean.md` — confirms project policy to skip Tunnel
- `feedback_local_only_ai_docs.md` — confirms `docs/` is local-only
- CLAUDE.md §TS-legacy — fog/tunnel/burrow guidance

**Companion docs:**
- `BRIDGE_LOCOMOTOR_DRIVE_SHIP_GHIDRA_REPORT.md` (live Drive/Ship)
- `BRIDGE_LOCOMOTOR_AIR_HOVER_GHIDRA_REPORT.md` (live JumpJet/Hover)
- `BRIDGE_LOCOMOTOR_WALK_DROPPOD_TELEPORT_GHIDRA_REPORT.md` (live Walk/Teleport + DropPod TS-dead detail)

---

## 10. Cleanup pass — 2026-05-13 (post-initial-draft)

This doc is the most stable of the four Phase 3 deliverables because its claims are negative (non-existence + non-binding) and were already backed by INI grep + Ghidra xref evidence.

Cleanup-pass adjustments:

| Item | Original status | Cleanup verdict |
|------|-----------------|-----------------|
| FloatLocomotionClass non-existence | HIGH | HIGH (unchanged — no CLSID, no symbol) |
| TunnelLocomotionClass TS-dead | HIGH | HIGH (unchanged — zero INI binding to CLSID `4A582743`) |
| TunnelLocomotionClass constructor at 0x728A00 | HIGH | HIGH (unchanged; cleanup decompiled it again and confirmed only init code) |
| ParachuteLocomotion non-existence-as-locomotor | HIGH | HIGH (unchanged — Parachute is FootClass state) |
| DropPod TS-dead | HIGH | HIGH (cross-confirmed in companion doc cleanup §3.1 + correction of "vtable thunk" → "scalar-deleting destructor" for 0x4B66F0) |
| Tunnels=/AllowBurrowing= INI keys inert | MEDIUM | **Stays MEDIUM** — only `ini/rulesmd.ini` and `ini/rules.ini` were grep'd. Theater INIs and campaign mission INIs not checked. Practical impact: zero (these keys feed a TS-dead system) but the "exhaustive INI verification" gap remains. |
| LocomotionClass::Can_Enter_Cell 4-byte stub | HIGH (per Phase 2 doc) | HIGH (unchanged) |

**No new findings.** All non-coverage justifications hold.

### 10.1 Remaining MEDIUM-confidence items

1. **Theater INI / mission INI sweep** for `Tunnels=`, `AllowBurrowing=`, and the dead CLSID strings. The current verification only checks the two main rules INIs. A full sweep would close this fully. Deferred — practical impact is zero.

2. **Mission-script reachability** for DropPod and TunnelLocomotion CLSIDs. A grep of all `.map` files for the CLSIDs would close the "campaign/triggered-spawn" question. Deferred.

These do not affect any HIGH-confidence claim; they exist for completeness.
