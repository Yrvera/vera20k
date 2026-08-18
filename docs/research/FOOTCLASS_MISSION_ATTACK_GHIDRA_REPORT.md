# FootClass::Mission_Attack — Ghidra Research Report

**Date:** 2026-04-23
**Primary address:** `0x004D4DC0`
**VTable slot:** FootClass vtable (`0x007E8C94`) + `0x210` (slot index 132)
**Mission enum:** 1 (`Attack`)
**Confidence:** HIGH for the handler body and flow; MEDIUM for downstream callees (approach driver decompiled only in summary)
**Active in YR:** **Yes** — core handler for every mobile unit in Attack mission. No TS-only gating.

---

## 1. Overview

**Mission_Attack does not fire weapons.** Like `Mission_Move`, it is a per-dispatch monitor/positioner, not an action handler. Firing happens in `InfantryClass::AI` / `UnitClass::AI` each frame — the Mission_Attack handler's job is three things:

1. **Anchor** the unit (if `DefaultToGuardArea`) to stay near its position between engagements.
2. **Finalize auto-acquisition** if the `HasFoundAutoTarget` flag is pending.
3. **Drive approach / scatter** via `Greatest_Threat_Scan` (a misnomer — it's the approach/scatter driver for units with a committed target).

If there's no TarCom, the handler calls `OnArrival(0, 1)` to transition out (typically to Guard). The weapon cooldown, range check, LOS, and actual `Fire_At` call all happen elsewhere — in the per-frame AI, driven by the unit's weapon state machine, not by this dispatch.

This is the crucial modelling observation: **the mission handler and the firing code are decoupled**. Mission_Attack sets up positioning; per-frame AI fires when ready. A Rust port that puts firing inside its mission handler will couple these in a way gamemd.exe doesn't, and will produce observable differences in firing cadence (because the per-frame loop runs every frame; the mission handler runs every 14–16 frames).

---

## 2. The Body (Disassembly Summary)

Full body of `FootClass::Mission_Attack @ 0x004D4DC0`:

```c
int FootClass::Mission_Attack(this) {
    // ─── STEP 1: DefaultToGuardArea ground-unit re-anchor ───────────────
    TypeClass = this->GetType();                         // vtable +0x84
    if (TypeClass->DefaultToGuardArea /* +0x390 */ != 0 &&
        this->GetHeight() == 0              /* ObjectClass::GetHeight, vtable +0x1C8 */) {
        // Ground unit with DefaultToGuardArea=yes: re-set destination to a
        // passable cell near its current position. (Not a "home base" — the
        // search centre is the unit's CURRENT coord.)
        FUN_00703590(&out_cell_coord, this);             // find nearby passable cell
        cell = MapClass::Get_CellClass(&out_cell_coord);
        this->Set_Destination(cell, 1);                  // vtable +0x480
    }

    // ─── STEP 2: Finalize HasFoundAutoTarget acquisition ────────────────
    if (this->HasFoundAutoTarget /* byte +0x68E */ != 0) {
        center = { this->Location_X, this->Location_Y, this->Location_Z };
        // vtable +0x3C4 = FootClass::Greatest_Threat — wraps TechnoClass::Greatest_Threat
        // args: (bitflag mask, center_coord, ?), returns target AbstractClass*
        target = this->Greatest_Threat(1, &center, 0);
        if (target != NULL) {
            this->Set_ArchiveTarget(target);             // vtable +0x3C8
            this->HasFoundAutoTarget = 0;                // one-shot flag
        }
    }

    // ─── STEP 3: Approach driver vs. arrival transition ────────────────
    if (this->TarCom /* +0x2B4 */ == NULL) {
        // No target — transition out (typically to Guard via OnArrival)
        this->OnArrival(0, 1);                           // vtable +0x484
    } else {
        // Have target — drive approach / scatter / re-aim via the big
        // 737-line helper at 0x004D5690
        this->Greatest_Threat_Scan(0);                   // vtable +0x53C
    }

    // ─── STEP 4: Return timer ──────────────────────────────────────────
    MissionClass::GetMissionTimerEntry();                 // this->CurrentMission × 32 + 0xA8E3A8
    base = Math::ftol(TopOfFPU × 900.0);                 // [Attack] Rate=.016 × 900 = 14.4 → 14
    jitter = RandomClass::RandomRanged(0, 2);            // 0, 1, or 2
    full_timer = base + jitter;                           // 14–16 frames

    // ─── STEP 5: Halve timer when close, for melee/short-range units ──
    if (this->TarCom != NULL) {
        // Condition A: InfantryClass (What_Am_I == 0xF) with TypeClass+0x695 flag set
        //   (likely C4/melee/Yuri-mind-control-type close-combat marker)
        is_melee_infantry = (this->What_Am_I() == 0xF &&
                             TypeClass->+0x695 != 0);
        // Condition B: primary weapon has range < 513 leptons (< 2 cells)
        weapon = this->GetWeapon(0);                      // vtable +0x3F8 — returns &TypeClass->Weapons[0]
        has_short_weapon = (weapon != NULL &&
                            weapon->WeaponType != NULL &&
                            weapon->WeaponType->Range /* +0xB4 */ < 0x201);

        if (is_melee_infantry || has_short_weapon) {
            target_coord = this->TarCom->Get_Coord();    // target vtable +0x48
            my_coord = this->Get_Coord();                // self vtable +0x48
            dist = Sqrt_Approx(dx² + dy²);               // 2D only
            dist_int = Math::ftol(dist);

            // Halve the dispatch interval when the unit is close enough
            // to need per-approach re-check but far enough to not be
            // already engaged. 0x301 = 769 leptons ≈ 3 cells.
            if (dist_int < 0x301 &&
                dist_int >= _DAT_007E9228 /* small positive minimum */) {
                return full_timer / 2;                   // ~7 frames (double check rate)
            }
        }
    }

    return full_timer;                                    // 14–16 frames (standard)
}
```

### 2.1 Assembly confirmation

The call sequence in assembly (abbreviated):
- `CALL [EAX + 0x84]` — GetType
- `CMP [type + 0x390], 0` — DefaultToGuardArea check
- `CALL [EAX + 0x1C8]` — ObjectClass::GetHeight
- `CALL FUN_00703590` — find-nearby-passable-cell helper
- `CALL MapClass::Get_CellClass`
- `CALL [EAX + 0x480]` — Set_Destination
- `CMP [this + 0x68E], 0` — HasFoundAutoTarget byte flag
- `CALL [EAX + 0x3C4]` — Greatest_Threat (the auto-acquire wrapper)
- `CALL [EAX + 0x3C8]` — Set_ArchiveTarget
- `CMP [this + 0x2B4], 0` — TarCom null check (field index 0xAD in int* indexing)
- `CALL [EAX + 0x484]` — OnArrival (null-target branch)
- `CALL [EAX + 0x53C]` — Greatest_Threat_Scan (the approach driver for has-target branch)
- `CALL GetMissionTimerEntry`
- `FLD/FMUL/FTOL` — base = ftol(Rate × 900)
- `CALL RandomRanged(0, 2)`
- `SHR base+jitter, 1` in halve-branch; `RET base+jitter` otherwise

Note the halving test uses `0x201` (not `0x200`) — the exact threshold is **< 513 leptons**, not "≤ 2 cells". This matters for parity: 2 cells = 512 leptons, which is at the boundary.

### 2.2 What this handler does NOT do

- **Does not call `Fire_At`.** No weapon discharge, no Fire_At_Target, no damage application.
- **Does not read weapon cooldowns.** The `ROF`/`ReloadRate` fields aren't touched.
- **Does not check LOS or armor-vs-warhead effectiveness.** Those happen in `Fire_At_Target` / `Can_Fire` on the per-frame AI path.
- **Does not set TarCom.** The target pointer at `+0x2B4` must already be set before Mission_Attack gets called. Setting happens via:
   - Player command (`Command::Assign_Target_Command @ 0x4DF0E0` sets TarCom to the clicked target)
   - Retaliation (ReceiveDamage sets TarCom to the attacker if `Retaliate=yes`)
   - AI acquisition (Mission_Hunt's `Greatest_Threat` call)
   - Step 2 of Mission_Attack itself, but only to **finalize** a prior `HasFoundAutoTarget` decision.
- **Does not set MissionState.** Unlike `AircraftClass::Mission_Attack`, this handler is stateless across dispatches. `+0xBC (MissionState)` is never written.

---

## 3. The Approach Driver — `FootClass::Greatest_Threat_Scan @ 0x004D5690`

Called when TarCom is non-null. Despite its name, this is **not** a target-scan function in the Mission_Attack context — it's the **scatter-and-approach driver** that decides whether to move, scatter, or stand still to get into firing position.

Behavior summary (full decomp is 737 lines, not reproduced here):

1. **Early returns:**
   - If `WarpedOutOf == TarCom` (target is the same thing we're warping out of) → return 0.
   - If `TarCom == NULL` → return 0. (Shouldn't happen given caller checks, but belt-and-suspenders.)

2. **Weapon range acquisition:**
   - Get the "current weapon" via some range helper (`vtable+0x168` returns a range-scaled value, capped at `0x201`).
   - Query `GetFireError` (`vtable+0x3A8`) — determines if firing would succeed right now.

3. **Player-player edge cases:**
   - For AreaGuard (mission 11) + player-controlled, or Hunt (15) + non-player, bypass the standard allow-fire logic.
   - If fire is impossible *and* this isn't an Attack mission context, clear ArchiveTarget + call `Set_Destination(NULL, 1)` and return 0.

4. **Piggyback locomotor check:**
   - Queries `IID_00818858` (internal — likely `IID_IDrive` or similar) on the locomotor to identify whether to run Drive-specific approach logic vs Walk/Fly.
   - Compares the loco CLSID against `DAT_007E9AC0` (likely `CLSID_WalkLocomotion`).

5. **NavCom null branch** (unit is currently stopped, hasn't moved toward target yet):
   - **If NavQueue.Count2 > 0**: pop the first queued destination and `Set_Destination(queued, 0)`. Shift-left the queue. Return the queued cell pointer.
   - **Else if target is a CellClass**: check spawn-map feasibility; if passable, Set_ArchiveTarget and proceed.
   - **Else if target is a Building**: compute approach radius as foundation-dimensioned padding (`(width + height) × 0x40`).
   - Compute a **bearing angle** from self to target: `atan2(target_y − self_y, self_x − target_x)`, floored to int, converted to 8-bit facing direction `(angle >> 7 + 1) >> 1 & 0xFF`.
   - Use `g_DirectionOffsets` + bearing to step around the target in 8 compass directions, looking for a passable in-range cell. For each candidate cell:
     - Check `TechnoClass::InRange(cell, target, weapon)` — is the weapon effective from here?
     - Check `CellClass::CheckCellPassability` — can the unit stand there?
     - Check infantry-specific `CellClass::PlaceInfantryInCell` for sub-cell positioning.
     - If pass: set this as the approach destination and return its cell pointer.
   - If no in-range passable cell found, spiral outward using the table at `&DAT_008224DC..0x822540` (a fixed approach-offsets table, 16-byte stride).
   - If after all spiraling, still no cell: give up (`TypeClass+0xD27 != 0` or infantry special case `param_1[0x1B0]+0xEC6`), just call `Set_Destination(TarCom, 1)` (the raw target) and return.

6. **NavCom non-null branch** (unit is already moving toward some destination):
   - If unit is crashing and TarCom is within range: compute 3D distance to TarCom's *current* coord, compare to `RulesClass+0xDF8 × foundation_scale`. If actual distance > expected, clear NavCom (target moved).
   - If NavCom is still valid or `vtable+0x54` (some readiness check) passes: return 0 (keep doing what we're doing).
   - Else fall through to the scan branch above.

7. **Scatter-infantry special case:**
   - For InfantryClass with `TypeClass+0x695` (melee/Yuri/similar flag), when within `_DAT_007E9240` distance of target, call `CellClass::PlaceInfantryInCell` and call the locomotor's `Set_Coord_Direct` (`vtable+0x78`) to snap the infantry's sub-cell position. This is the "Tanya shuffles into position" behavior.

**The approach driver's effect on NavCom/TarCom:**
- May call `Set_Destination(cell, 0)` or `Set_Destination(target, 1)` to re-aim toward an approach cell.
- May call `Set_ArchiveTarget(cell)` to commit an approach target.
- May pop from NavQueue (same mechanism as OnArrival).
- May clear NavCom (`+0x168 = 0; +0x169 = 0`) when the target has clearly moved out of tracking range (crashing branch only, with `param_2 == 0` soft mode).

Returns an int — typically the approach cell pointer, or 0 if nothing to do. Mission_Attack **discards the return value** — it only calls this helper for its side effects on NavCom/TarCom/locomotor.

---

## 4. Per-Class Overrides

### 4.1 UnitClass — inherits

UnitClass vtable slot +0x210 continues to point at FootClass::Mission_Attack. No override. Vehicles use the base handler unchanged.

Bonus finding: `UnitClass::Mission_Repair @ 0x7447A0` (vtable slot +0x25C, mission enum 24) is a **one-line thunk** that calls `FootClass::Mission_Attack`. This is because "repair" in original RA2 means "target a friendly vehicle with your repair wrench" — the approach logic (scatter into range, check LOS, drive to the target) is identical to attack. Only the per-frame `Fire_At_Target` / repair-application code differs.

### 4.2 InfantryClass — override at `0x0051F3E0` (labeled `FUN_0051F3E0`)

Three branches before falling through:

```c
int InfantryClass::Mission_Attack(this) {
    // ── Branch 1: Engineer / spy entering a garrisonable/civilian building ──
    if (this->InfantryTypeClass->+0xEC2 ||
        TechnoClass::HasWeaponAbility(14 /* Infiltrate */)) {
        if (this->TarCom != NULL && this->TarCom->What_Am_I() == 6 /* Building */) {
            bldg_type = this->TarCom->+0x520;  // BuildingTypeClass
            if (bldg_type->+0x1577 /* CanBeOccupied / Enterable */ &&
                !bldg_type->+0x1701 /* BlockedByConditionX */) {
                this->Set_Destination(this->TarCom, 1);    // aim at building
                this->Set_Mission(17 /* Sabotage */, 0);    // switch to SABOTAGE mission
                return 1;
            }
        }
    }

    // ── Branch 2: AI-only — engineer targeting friendly repairable or capturable ──
    if (!HouseClass::IsPlayerControl() && this->TarCom != NULL &&
        this->TarCom->What_Am_I() == 6) {
        infantry_type = this->InfantryTypeClass;
        if (infantry_type->+0xEBE /* Engineer */) {
            this->Set_Destination(this->TarCom, 1);
            this->Commence_Mission(8 /* Capture */);
            return 1;
        }
        if (infantry_type->+0xEB4 /* CanRepair */ ||
            infantry_type->+0xEB5 /* Medic-analog or DockForHeal */) {
            if (BuildingClass::CanDock(this)) {
                this->Set_Destination(this->TarCom, 1);
                this->Commence_Mission(8 /* Capture = dock */);
                return 1;
            }
        }
    }

    // ── Branch 3: Player infantry in panic/prone/deploy sequences 0x1B–0x1E ──
    if (HouseClass::IsPlayerControl() &&
        (this->Sequence == 0x1B || 0x1C || 0x1D || 0x1E)) {
        this->vtable[0x428]();                     // abort-in-sequence call
        return ftol(Rate × 900) + Random(0, 2);    // standard timer, no Mission_Attack body
    }

    // ── Fallthrough ──
    return FootClass::Mission_Attack(this);
}
```

The three branches are: spy/engineer entering a building, AI engineer repairing/capturing, and player-infantry panic-state bailout. All three are mission-transition paths — the infantry handler intercepts Attack-mission dispatches to redirect into Sabotage, Capture, or idle timer, before the base handler ever runs. Otherwise it delegates to FootClass's handler unchanged.

### 4.3 AircraftClass — full override at `0x00417FE0`

An **11-state machine** driven by `+0xBC (MissionState)`:

| State | Purpose |
|-------|---------|
| 0 | Init: if no TarCom → state 10 (park); else state 3 (approach) |
| 1 | Find approach cell (`AircraftClass::Find_Approach_Cell`), Set_Destination |
| 3 | Approach target, check range/altitude/alignment, fire-ready → state 4 |
| 4 | **Fire**: calls `vtable+0x3CC (Fire_At)` with TarCom. Aircraft FIRES in the mission handler (ground units don't). Decrements `+0x2FC` (ammo/pass counter). Scatters nearby objects, advances to state 5/6/10 based on `vtable+0x3C0` fire-result. |
| 5 | Post-fire: pull-up, check if more passes available, loop to state 4 or advance. |
| 6 | Extra pass positioning (state 7/8 staircase for multi-pass). |
| 7, 8 | Subsequent passes (mirror state 6). |
| 9 | Final pass teardown, then state 3 (return to approach). |
| 10 | Return to base / park: clear state, Set_Destination to parked cell, Set_Mission(enter-transport). |

Two critical differences from the ground handler:

1. **Firing is inline.** `Fire_At` (vtable +0x3CC) is called from state 4 every time the aircraft has a valid fire-ready state. Ground units don't do this in Mission_Attack — they do it in `AI()` each frame.
2. **Ammo accounting is inline.** Byte `+0x2FC` (the ammo/pass counter) is decremented per firing pass. When it hits 0 AND the state machine reaches state 10, the aircraft RTBs to reload.

This makes sense: aircraft have a "run" semantic — fly in, drop ordnance, fly out — which can't decouple into per-frame bookkeeping. The state machine IS the attack run.

Aircraft also use the same 14–16 frame timer when they *aren't* firing (state transitions, idle), and a fast timer during firing (state 2 returns `0x2D = 45` frames between reload-pass-attempts when `+0xBF != 0`, etc.).

---

## 5. Caller Chain

```
TechnoClass::AI_Update (0x6F9E50) — once per tick per unit
  ↓
MissionClass::Mission_Dispatch (0x5B3060)
  ↓  case 1 (Attack): CALL vtable[0x210]
  ├─ FootClass::Mission_Attack (0x4D4DC0)          ← for UnitClass (inherit) and as fallthrough for InfantryClass
  ├─ InfantryClass::Mission_Attack (0x51F3E0)      ← infantry override (then calls FootClass if no redirect)
  └─ AircraftClass::Mission_Attack (0x417FE0)      ← aircraft 11-state machine
```

Direct callers of `FootClass::Mission_Attack` found in Ghidra xrefs:
- `FUN_0051F3E0` (InfantryClass::Mission_Attack) — fallthrough delegation
- `UnitClass::Mission_Repair_Thunk (0x7447A0)` — Mission_Repair (enum 24) reuses Attack handler verbatim

The per-frame firing chain, which runs **in parallel** with the mission dispatcher:

```
<main loop>
  ↓
InfantryClass::AI (0x51BAB0) / UnitClass::AI (0x7360C0)
  ↓  every frame
<weapon cooldown check, range check, LOS check, facing check>
  ↓  if ready
InfantryClass::Fire_At_Target (0x5206B0) / UnitClass::Fire_At_Target (0x736DF0)
  ↓
TechnoClass::Fire_At (0x6FDD50) — applies damage, spawns bullets, fires sound
```

Mission_Attack and this AI-firing chain **do not call each other**. They share state (TarCom, NavCom, Facing) but operate on different clocks: mission dispatch at 14–16 frames, per-frame AI every tick.

---

## 6. Struct Offsets Used

All on `FootClass*` unless marked.

| Offset | Size | Field | Read by Mission_Attack? |
|--------|------|-------|-------------------------|
| `+0x9C..+0xA4` (param_1[0x27..0x29] in int* indexing) | 12 | Location_X/Y/Z (int ea) | Yes (step 2 center, step 5 distance) |
| `+0x2B4` (idx 0xAD) | 4 | **TarCom** (AbstractClass*) | Yes (step 3 null-check, step 5 distance) |
| `+0x68E` | 1 | **HasFoundAutoTarget** (bool) | Yes (step 2 gate; cleared on acquisition) |
| `+0x688` | 1 | Auto-scan in-progress flag | Written by Greatest_Threat wrapper at +0x3C4 |

On TypeClass (`TechnoTypeClass*`, dereferenced via `GetType()` at vtable +0x84):

| Offset | Size | Field | Read by Mission_Attack? |
|--------|------|-------|-------------------------|
| `+0x390` | 1 | **DefaultToGuardArea** (bool) | Yes (step 1 gate) |
| `+0x695` | 1 | Close-combat flag (TS-era "C4"-family? unlabeled) | Yes (step 5 condition A) |
| `+0x898` | — | Weapons array base (stride 0x1C per WeaponStruct) | Yes (via vtable +0x3F8) |

On WeaponTypeClass (fetched via `GetWeapon(0)` indirection):

| Offset | Size | Field | Read by Mission_Attack? |
|--------|------|-------|-------------------------|
| `+0xB4` | 4 | **Range** (leptons) | Yes (step 5 condition B, compared against `0x201`) |

### 6.1 Mission timer entry (reminder from prior reports)

`DAT_00A8E3A8` is the mission timer table, 32 entries × 32 bytes. Entry for mission 1 (Attack) starts at `0xA8E3C8`:
- `+0x00` (int): mission index = 1
- `+0x04..+0x09` (bytes): `NoThreat`, `Zombie`, `Recruitable`, `Paralyzed`, `Retaliate`, `Scatter` — all read from `[Attack]` section at load time
- `+0x10` (double): `Rate` — `0.016` for `[Attack]`
- `+0x18` (double): `AARate` — `0.016` for `[Attack]`

Both Rate and AARate are set for Attack (unlike Move which only has Rate). This reflects that Mission_Attack's timer can be replaced by AARate when the unit is in anti-air mode, though the selection logic isn't in the Mission_Attack body itself (it's in how the table entry is indexed via `CurrentMission`). The body always reads the same entry's `+0x10` (Rate) — not `+0x18` (AARate). To pick AARate, some other layer would have to switch `CurrentMission` to a different enum value that shares the AARate path; the code here doesn't do that.

---

## 7. INI Keys Affecting Mission_Attack

| Key | Section | Default | Effect |
|-----|---------|---------|--------|
| `Rate` | `[Attack]` | `.016` | Timer base. `ftol(.016 × 900) = 14 frames` |
| `AARate` | `[Attack]` | `.016` | Not read by Mission_Attack directly (see §6.1) |
| `Scatter` | `[Attack]` | `no` | Stored at timer entry +0x09. Read by other mission handlers; Mission_Attack doesn't check it. |
| `DefaultToGuardArea` | unit section | `no` | Step 1 gate. Ground units with `yes` re-anchor to a nearby passable cell each dispatch when not already heading somewhere. |
| `Range` | `[WeaponName]` | per-weapon | Step 5 condition B: `< 513 leptons` triggers the halve-timer fast-check path. |
| `ROF` / `ReloadRate` | various | per-unit | **Not read by Mission_Attack.** These drive the per-frame firing AI, not the mission handler. |
| `Retaliate` | `[Attack]` timer | (bool on table entry) | Not read by Mission_Attack; used by the damage handler to decide whether to auto-target the attacker. |

No Mission_Attack-specific INI key controls approach radius, scatter distance, or re-acquire cadence — all are hard-coded (`0x301` halve threshold, `0x201` weapon-range threshold, etc.).

---

## 8. Active in YR

**Yes, unconditionally** for the core flow. All three branches are reachable under normal YR play:

- **DefaultToGuardArea re-anchor**: active for any ground unit with `DefaultToGuardArea=yes` in its INI section. Used by a handful of units (e.g., patrol infantry, some turret-like ground defenses).
- **HasFoundAutoTarget finalization**: set by `Mission_Hunt` and retaliation paths; Mission_Attack is where the flag is cleared after a one-shot acquisition.
- **Approach driver (Greatest_Threat_Scan)**: the main per-tick behavior for any unit with a committed TarCom.

No `SpecialFlags` gating, no TS-only fields. The only TS-legacy concern is the `+0x695` TypeClass flag (Step 5 condition A) which is set on melee/C4-carrying infantry; these units ARE active in YR (Tanya, Yuri, SEAL). The flag itself is a TS holdover name but behaves live.

The 11-state `AircraftClass::Mission_Attack` has several aircraft-specific branches (RulesClass+0x17E1 for the Attack→state-2 vs Attack→state-4 fork; `g_RulesClass_Instance + 0x678` for ammo pass scaling) that are standard YR rules values — not TS-gated.

---

## 9. Firing (the part Mission_Attack doesn't do)

For completeness and to underscore the handler's scope: firing for ground units in YR happens as follows (not the focus of this report, but essential context):

```
Every frame per unit with TarCom:
  InfantryClass::AI / UnitClass::AI
    ↓
  Check cooldown: +0x??? (ROF-derived reload timer counts down each frame)
  Check burst:    +0x??? (BurstDelay between shots within a burst)
  Check range:    TechnoClass::InRange(self, TarCom, weapon)
  Check LOS:      implicit in range check for most weapons
  Check facing:   turret-vs-target angle vs TurnTolerance
    ↓  all pass
  InfantryClass::Fire_At_Target / UnitClass::Fire_At_Target
    ↓
  TechnoClass::Fire_At (0x6FDD50)
    ↓
  Spawn BulletClass (or instant-damage for laser-type weapons)
  Play WeaponFireSound, WeaponFireAnim
  Apply damage (instant or deferred via bullet travel)
  Decrement Ammo (ammo-limited units)
  Reset cooldown
```

See `TECHNOCLASS_COMBAT_WEAPON_SYSTEMS_REPORT.md` and `FIRE_AT_ANALYSIS.md` for the firing-chain details.

The important contract: **TarCom must already be set and valid** for firing to happen. Mission_Attack guarantees this by (a) calling `OnArrival` when TarCom goes null (transitioning out of Attack before firing can misfire), and (b) letting the approach driver re-aim the unit each dispatch so the per-frame firing code finds the target in range.

---

## 10. Current Rust Implementation Status

From the parallel Rust scan ([src/sim/combat/](../../ra2-rust-game/src/sim/combat/)):

| Aspect | Rust status | Gap to close for parity |
|--------|-------------|-------------------------|
| Mission enum for ground units | Absent (attack is implicit via `AttackTarget` component) | Needs full mission state machine before parity is feasible |
| Per-dispatch cadence | Fires every frame ([mod.rs:859](../../ra2-rust-game/src/sim/combat/mod.rs#L859)) | Rust approach-check runs 14×–16× faster than gamemd |
| Separate approach vs fire | Inlined ([mod.rs:1090-1210](../../ra2-rust-game/src/sim/combat/mod.rs#L1090)) | Needs split: positioning on dispatch timer, firing on per-frame |
| `HasFoundAutoTarget` flag | Not represented | Not critical until Mission_Hunt is implemented |
| `DefaultToGuardArea` re-anchor | Not implemented | Needed for authentic idle-wander of patrol units |
| Halve-timer on close/melee | Not applicable (single-cadence loop) | Would collapse into whatever timer discretization the port adopts |
| Greatest_Threat_Scan approach driver | Partial via [combat_targeting.rs::acquire_best_target_for_entity](../../ra2-rust-game/src/sim/combat/combat_targeting.rs#L67) | Rust re-acquires the target, not the approach cell — different semantics |
| `OnArrival(0,1)` on no-target | Simple: `attack_target = None` | Close, but no idle-mode transition/guard dispatch |
| Aircraft 11-state attack run | Absent; [AircraftMission::Attack](../../ra2-rust-game/src/sim/aircraft/mod.rs#L33) is a placeholder | Huge gap — aircraft attack runs are their own sub-project |

The main Rust divergence is the **dispatch cadence coupling**: by firing every frame and re-acquiring targets every frame, Rust's behavior doesn't match gamemd's discretized approach / smoothed timer-jittered cadence. A unit that just lost its target in gamemd takes 14–16 frames to run OnArrival and settle; in Rust, the same unit settles instantly next frame. Observable in tight engagements: gamemd units have a "settle pause" after losing their target, then smoothly transition to Guard. Rust units will look twitchier.

---

## 11. Open Questions

1. **TypeClass+0x695** (Step 5 condition A): unnamed flag that gates the halve-timer path for InfantryClass. Strong candidates: `C4=yes` (demo-charge melee), `Suicide=yes`, or `Tanya-family` marker. Would require checking which INI keys read into that byte offset during InfantryTypeClass::Read_INI. Not traced here.
2. **TypeClass+0xD27, +0xEC2, +0xEB4, +0xEB5, +0xEBE**: various unit-type flags consumed by the infantry override (engineer-type, medic-type, canRepair). Exact INI key mapping not verified; inferred by context.
3. **`DAT_007E9228`, `DAT_007E9230`, `DAT_007E9238`, `DAT_007E9240`, `DAT_007E9248`**: double constants used as thresholds in Mission_Attack (halving floor) and `Greatest_Threat_Scan` (scatter/infantry thresholds). Values not read. Likely tunable magic numbers for approach bubble sizes (1 cell, 2 cells, 3 cells, etc.) in lepton units.
4. **`vtable+0x168`** on FootClass (read in Greatest_Threat_Scan for the initial range calc). Not identified. Probably `GetWeaponRange` or `GetEffectiveRange`.
5. **`Greatest_Threat_Scan` (737 lines) full decomp.** Only sketched in §3 for side-effect accounting. A dedicated investigation would be needed for parity on scatter behavior (the 8-direction spiral, the fixed offset table at `0x822540`, the spawn-map interaction at `FUN_00487C10`).
6. **The exact semantics of `+0x688` vs `+0x68E`.** Both are "is scanning for auto-target" flags. The wrapper at `0x4D9920` sets `+0x688` under some conditions, reads it, and clears it if no target. Mission_Attack reads `+0x68E`. Are these independent flags or the same byte seen through different offsets? Needs a struct-layout re-verification.
7. **The `DAT_008224DC → 0x822540` approach-offset table** (`Greatest_Threat_Scan`). 0x64 / 0x10 stride = 10 entries if 16-byte stride. Probably a spiral pattern for scatter-into-range. Not decoded.

---

## 12. Sources

**Ghidra decompilation this investigation:**
- `0x004D4DC0` — `FootClass::Mission_Attack` (full, ~85 lines)
- `0x0051F3E0` — `InfantryClass::Mission_Attack` (FUN_, unlabeled but clearly is)
- `0x00417FE0` — `AircraftClass::Mission_Attack` (11-state machine, ~500 lines)
- `0x007447A0` — `UnitClass::Mission_Repair_Thunk` (1 line)
- `0x004D5690` — `FootClass::Greatest_Threat_Scan` (summary only; 737 lines)
- `0x004D9920` — `FootClass::Greatest_Threat` wrapper
- `0x0070E140` — `TechnoClass::GetWeapon`
- `0x007177C0` — `GetNormalWeapon` (inline address computation)
- `0x00703590` — Find-nearby-passable-cell helper (DefaultToGuardArea path)
- `0x005F5F40` — `ObjectClass::GetHeight` (confirmed via `TECHNOCLASS_VTABLE_COMPLETE.md:162`)

**Caller chain verified:**
- `InfantryClass::Fire_At_Target @ 0x5206B0` ← `InfantryClass::AI @ 0x51BAB0` (confirms firing is on per-frame path)
- `UnitClass::Fire_At_Target @ 0x736DF0` ← `UnitClass::AI @ 0x7360C0` (same)

**Vtable slots resolved via raw memory reads:**
- FootClass vtable base `0x007E8C94`
- `+0x1C8` → `0x7E8E5C` = `40 5F 5F 00` → `ObjectClass::GetHeight @ 0x5F5F40`
- `+0x3C4` → `0x7E9058` = `20 99 4D 00` → `FUN_004D9920` (Greatest_Threat wrapper)
- `+0x3F8` → `0x7E908C` = `40 E1 70 00` → `TechnoClass::GetWeapon @ 0x70E140`
- `+0x53C` → `0x7E91D0` = `90 56 4D 00` → `FootClass::Greatest_Threat_Scan @ 0x4D5690`

**Docs referenced:**
- `FOOTCLASS_MISSION_HANDLERS_GHIDRA_REPORT.md` — dispatch-table summary (handler pseudo-behavior listed but not verified at asm level)
- `FOOTCLASS_VTABLE_COMPLETE.md` — vtable slot identities
- `TECHNOCLASS_VTABLE_COMPLETE.md` — vtable +0x1C8 resolution
- `TECHNOCLASS_COMBAT_WEAPON_SYSTEMS_REPORT.md` — weapon selection semantics
- `TARGET_ACQUISITION_GHIDRA_REPORT.md` — Greatest_Threat_Scan role description
- `FIRE_AT_ANALYSIS.md` — per-frame firing pipeline
- `FOOTCLASS_MISSION_MOVE_GHIDRA_REPORT.md` — prior report in this series (timer formula, Mission_Dispatch)
- `NAVCOM_LIFECYCLE_GHIDRA_REPORT.md` — prior report (Set_Destination, Set_ArchiveTarget)

**INI files checked:**
- `ini/rulesmd.ini` — `[Attack]` section (`Rate=.016`, `AARate=.016`, `Scatter=no`)

**Global memory cited:**
- `0x007E27F8 = 900.0` — minute-to-frame conversion
- `0x00A8E3A8` — mission timer table base (Attack at +32)
- `0x00A8B230` — `g_RulesClass_Instance`
- FootClass vtable at `0x007E8C94`
- `0x007E9AC0` — `CLSID_WalkLocomotion` (used by Greatest_Threat_Scan's loco-type check)
- `0x00818858` — `IID_IPiggyback` interface GUID
- `0x00822540` — end of approach-offset table (started at `0x008224DC`)
