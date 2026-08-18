# GGI (Guardian GI) — Ghidra Research Report

**Address(es):** parse `InfantryTypeClass__ReadINI @ 0x005240a0`; runtime
state machine `InfantryClass__Do_Action @ 0x0051d6f0` (vtable+0x558),
`InfantryClass__DoType_Sequencer @ 0x00520ae0`, `InfantryClass__AI @
0x0051bab0`, fire driver `InfantryClass__Fire_At_Target @ 0x005206b0`,
weapon select `InfantryClass__SelectWeapon @ 0x005218e0` (vtable+0x2E4 of
InfantryClass), eligibility `InfantryClass override of GetFireError @
0x0051c8b0` (vtable+0x3C0) tail-calling `TechnoClass__GetFireError @
0x006fc0b0`, crush gate `TechnoClass__CanCrushCheck @ 0x005f6cd0`, damage
math `ApplyWarheadDamage @ 0x00489180`.

**Confidence:** **HIGH** for parse path, deploy state machine, fire-frame
gate, weapon-select dispatch, crush-gate path, BFRT/IFV routing, AA-gate
location, **ftol rounding mode** (confirmed truncation toward zero — §8.3),
**missile homing flight curve** (§8.1), **TechnoClass::Fire_At order of
operations** (§8.2). **MEDIUM** for `ProneDamage` application site —
investigation showed it is **NOT** in `TechnoClass::Fire_At`; deferred to
`BulletClass::Detonate` (§8.2 disproves the §7.1 hypothesis).
`IsSelectableCombatant` consumer parsed-but-reader-not-located.

**Active in YR:** **Yes** for every behavior documented here. All code
paths run in a stock YR skirmish. `IsDeploying @ 0x0070fec0` is vehicle-only
and not reached by GGI; `Fire_At_Override @ 0x0051df70` is the tail of
`Fire_At @ 0x0051df60`, not a separate entry point.

**Scope note:** This dossier covers only the **GGI-specific deltas** from
the basic GI (E1). Shared infantry infrastructure (panic/fear, sub-cell,
mind control, render, voice routing, generic AI loop) is documented in
`GI_GHIDRA_REPORT.md` and is not repeated here unless GGI diverges.

---

## 1. Overview

GGI (`[GGI]` in `rulesmd.ini`, line 3863) is the Allied secondary infantry.
Cost 400, Strength 100, walking range 4 with M60 (anti-infantry), deploys
to fire MissileLauncher (range 8, AA+AG, `AAHeatSeeker2` projectile, single
missile per shot at ROF 40). Elite variants double primary damage
(`M60E=25`) and accelerate the missile (`MissileLauncherE`: damage 50, ROF
20, speed 40). Deployed GGI is **uncrushable** because `DeployedCrushable=no`
flips the runtime byte at `InfantryClass+0x2A4` to 1 at end-of-Deploy
sequence; the `TechnoClass__CanCrushCheck` predicate returns false when
that byte is non-zero. Deploy is purely player-initiated (no auto-deploy
on air target acquisition). GGI cannot enter civilian garrison
(`Occupier=no`).

**No GGI-specific code branch exists in the binary.** Every observable
difference between GGI and E1 is value-driven: different INI values land
in the same shared parser slots, and the runtime state machine consults
those values at fixed offsets.

---

## 2. Class Layout / Key Offsets

The basic InfantryTypeClass and TechnoTypeClass layouts are already in the
E1 doc; this section captures **only the offsets GGI exercises that the E1
doc either skipped, mislabeled, or needs reinforcement on**.

### 2.1 InfantryTypeClass — GGI-relevant fields

| Offset | Type | Field | GGI value | Default | Read by `InfantryTypeClass__ReadINI @ 0x005240a0` |
|--------|------|-------|-----------|---------|----------------------------------------------------|
| `+0xE40` | int | `FireUp` (anim frame anchor) | art `FireUp=2` | 0 | yes |
| `+0xE44` | int | `FireProne` (anim frame) | (art-section default) | 0 | yes |
| `+0xE48` | int | `SecondaryFire` (anim frame) | (art-section default) | 0 | yes (string `0x825680`, ReadInt @ `0x00524705`) |
| `+0xE4C` | int | `SecondaryProne` (anim frame) | — | 0 | yes |
| `+0xE3C` | ptr  | Sequence table base (32 × 0x24 byte entries, indexed by Doing) | resolved at load | — | populated by sequence loader |
| `+0xEA4` | int  | DeploySound voc-index | `GuardianGIDeploy` (id) | -1 | yes (after `Cyborg=` block) |
| `+0xEA8` | int  | UndeploySound voc-index | `GIUndeploy` (id) | -1 | yes |
| `+0xEB4` | bool | `Occupier` | **0 (no)** | 0 | yes — **GGI delta vs E1=yes** |
| `+0xEC8` | bool | `Deployer` | **1 (yes)** | 0 | yes (string `0x825928`, ReadBool @ `0x0052460d → MOV [ESI+0xEC8],AL @ 0x00524620`) |
| `+0xEC9` | bool | `DeployedCrushable` | **0 (no)** | **1 (TRUE)** ← non-obvious | yes (string `0x825914`, ReadBool @ `0x00524627 → MOV [ESI+0xEC9],AL @ 0x00524643`) |
| `+0xDFC` | int  | `Pip` color slot | `white` | — | yes |

**Critical detail on `+0xEC9`:** the InfantryTypeClass constructor at
`0x005236a0` initializes `+0xEC9 = 1`. GGI's explicit `DeployedCrushable=no`
in the INI overrides this default — without that explicit line, GGI would
become **crushable** when deployed. E1 omits the key entirely and inherits
the default `=1` (technically meaning E1 would also be deployed-crushable
unless DeployedCrushable were defaulted differently — verify E1 in-game
behavior matches gamemd's apparent contradiction; this is a TINY DETAIL
worth flagging in any port).

**Resolution of the offset conflict in scoping (Phase 1):** `+0xEAC` is
`Cyborg`, `+0xEAD` is `NotHuman`, `+0xEC8` is `Deployer`, `+0xEC9` is
`DeployedCrushable`. Verified via direct assembly trace of each `PUSH
0x82XXXX` (key string) preceding `CALL ReadBool` and the subsequent `MOV
byte ptr [ESI+offset], AL`.

### 2.2 TechnoTypeClass — GGI-relevant fields

Parser is `TechnoTypeClass__ReadINI @ 0x00712170`. `param_1` type is `int *`
→ `param_1[N]` reads byte offset `N×4`. Single-byte writes use
`*(undefined1 *)((int)param_1 + 0xN)` → direct byte offset.

| Offset | Type | Field | GGI value | Default | Notes |
|--------|------|-------|-----------|---------|-------|
| `+0x3C8` | double (8 bytes) | `DeployTime` | (unset → 0.0) | 0.0 | seconds; multiplied by 30 (FPS) to ticks; **NOT used for infantry deploy duration** — Sequencer drives that |
| `+0x3D0` | int | `FireAngle` | (unset → 8) | **8** (≈11° BAM) | "soft fire cone" — fire-while-turret-within-N-BAM-of-bearing |
| `+0x688` | int | `IFVMode` | **16** | **0** (not -1!) | passenger slot index into IFV turret-frame table and weapon-list |
| `+0x6A8` | int | `DeployFireWeapon` | (unset → 1) | **1** | when ≥0, indexes the `Weapon%d` array IF `WeaponCount>0`; for GGI (WeaponCount=0), value 1 means **Secondary slot** |
| `+0x6AC` | bool | `DeployFire` | derived (yes via `Deployer`) | 0 | enables weapon-select override during Doing 0x1B–0x1E |
| `+0x6BC` | anim-ptr | `DeployingAnim` | (unset) | 0 | |
| `+0x8B8/+0x8BC/+0x8C0` | int×3 | `SecondaryFireFLH` (F/L/H leptons) | `80,0,90` | `0,0,0` | gated by WeaponCount==0 — for GGI, IS read |
| `+0xAB4/+0xAB8/+0xABC` | int×3 | `EliteSecondaryFireFLH` | (defaults to SecondaryFireFLH) | 0,0,0 | same gate |
| `+0xCD5` | bool | `IsGattling` (derived) | 0 | 0 | not GGI |
| `+0xD50` | int | `OpenTransportWeapon` | **1** | -1 | sentinel -1 = no override |
| `+0xDBC` | bool | `IsSelectableCombatant` | 1 | 0 | reader not located in this pass |

**`PrimaryFireFLH=80,0,105`** lives at the symmetric primary-FLH offset
(typically `+0x8AC/+0x8B0/+0x8B4` — same triplet pattern, slightly earlier
in the struct than Secondary). Not exhaustively verified in this pass but
expected by the parser's structure.

### 2.3 InfantryClass (runtime instance) — deploy-relevant fields

`param_1` type is `int *` in the AI/state-machine functions →
`param_1[N]` indexes by 4.

| Byte offset | `param_1[N]` form | Field | Notes |
|-------------|---------------------|-------|-------|
| `+0x82`     | (byte) | `IsInOpenTransport` | set by `TechnoClass__SetInOpenTransport @ 0x00710470` (called from PerCellProcess), cleared on transport death |
| `+0xF8`     | `[0x3E]` | Current sequence sub-frame index (0..Length-1) | **the value the FireUp gate compares against** |
| `+0xFC`     | `[0x3F]` | Sound-enable byte for the current state |
| `+0x100`    | `[0x40]` | `anim_start_frame_counter` (= `g_CurrentFrameCounter` at state entry) |
| `+0x104`    | `[0x41]` | coord snapshot at state entry |
| `+0x108`    | `[0x42]` | frame-count remaining (decremented) |
| `+0x10C`    | `[0x43]` | frame-count total (constant) |
| `+0x16D`    | (byte) | `FireFlag` ("in firing sequence", set when starting Fire-Up, cleared on exit) |
| `+0x1BB`    | (byte) | `GarrisonFlag` (firing from inside a building) |
| `+0x2A4`    | `[0xA9]` | **`Deployed_Uncrushable_Lock`** — runtime flag; 1 when GGI is deployed AND `InfType+0xEC9==0` |
| `+0x2EC`    | `[0xBB]` | `LastFireFrame` (= `g_CurrentFrameCounter` at fire) |
| `+0x2F8`    | `[0xBE]` | ROF rearm-delay counter |
| `+0x68D`    | (byte) | `RefreshDeployedSeq` flag — when set, AI re-emits `Set_Sequence(Deployed)` |
| `+0x6C0`    | `[0x1B0]` | TypeData pointer (InfantryTypeClass*) |
| `+0x6C4`    | `[0x1B1]` | **`Doing` enum** (the current state) |
| `+0x6DB`    | (byte) | "fire-pending" flag — cleared on Doing 7 and 0x1B |

---

## 3. Core Logic

### 3.1 Deploy state machine

**Sequence groups (Doing values) consumed by GGI:**

| Doing | Hex | Meaning | Art sequence (GGI) |
|-------|-----|---------|--------------------|
| 0x1B | Deploy           | playing Deploy SHP frames | `Deploy=300,15,0` (15 frames) |
| 0x1C | Deployed         | static deployed idle frame | `Deployed=315,1,1` |
| 0x1D | DeployedFire     | firing while deployed | `DeployedFire=323,6,6` (6 frames) |
| 0x1E | DeployedIdle     | extended deployed idle | (sequence-engine selected) |
| 0x1F | Undeploy         | playing Undeploy frames | `Undeploy=180,2,2` |

**Entry — `InfantryClass__Do_Action(this, doing, force_change, randomize)`
@ `0x0051d6f0`, vtable+0x558:**

Order of writes when entering Deploy (0x1B) or Undeploy (0x1F):
1. **Sound trigger** — `VocClass__PlayAt(InfType+0x56C)` for Deploy, or
   `+0x570` for Undeploy. **Plays BEFORE the Doing field is written.**
   Skipped if voc-index == -1.
2. `this[+0x6C4] = doing` (Doing field).
3. `this[+0x100] = g_CurrentFrameCounter` (anim-start tick).
4. `this[+0x104] = this[+0xA0]` (coord snapshot).
5. `this[+0x108] = this[+0x10C] = frame_count` (from the sequence-table
   entry; **not** the hardcoded sound-flag table byte).
6. `this[+0xF8]` set to 0 or `Random__RandomRanged(0, count-1)` if
   `randomize == 1`.
7. **DeployedCrushable side effect:** when entering Doing 0x1B (Deploy),
   if `InfType+0xEC9 == 0` (DeployedCrushable=no), Sequencer at case 0x1B
   sets `this[+0x2A4] = 1` and calls `FUN_0070f770` (likely a vision /
   minimap refresh — not verified in detail). When entering Doing 0x1F
   (Undeploy) at completion, the byte is cleared back to 0.
8. (`this[+0x6DB] = 0`) — fire-pending flag cleared on Doing 0x1B entry.
9. If `this[+0x1B] == 0`, `vtable+0x500()` is called (locomotor stop).

**Early-return gates in `Do_Action`** (refuse to change state, return 0):
- `doing == -1`
- `sequence_table[doing].frame_count == 0` (this Doing not defined for
  this InfType)
- Parachute lock: current Doing == 0x21 AND `this+0x8D != 0`
- Crawl gate: `doing == 5` AND `InfType+0xEBD == 0`
- Replacement gate: `doing == current_Doing` OR (current Doing not -1 AND
  not `force_change` AND interruptibility-table[current*4] == 0)
- Water/swim remap: certain walk Doings remap to swim Doings when the
  current cell is type 6/2 AND `InfType+0x5B4 == 3`. Not applicable to GGI.

**Per-tick state advance — `InfantryClass__DoType_Sequencer @ 0x00520ae0`,
called from `InfantryClass__AI`:**

- Reads `Doing = this[+0x6C4]`; reads `current_frame = this[+0xF8]`.
- Looks up sequence entry at `InfType[+0xE3C] + Doing*0x24`. Entry layout:
  - `+0x00` StartFrame (int)
  - `+0x04` Length (int) — **the gate value**
  - `+0x08` Rate (int) — **NOT consumed here**; consumed by the drawer
  - `+0x0C` FacingDirCode (W/SW/S/SE/E/NE/N/NW → 0..7)
  - `+0x10` SoundCount (0..2)
  - `+0x14/+0x18` Sound[0].frame / Sound[0].vocID
  - `+0x1C/+0x20` Sound[1].frame / Sound[1].vocID
- End-of-sequence test: `current_frame < Length` → still playing,
  fall through to per-tick sound pass. When `current_frame == Length`,
  the switch body runs and transitions are issued.
  **Off-by-one: this is `<`, not `<=`.** A Length=N sequence plays
  frames 0,1,…,N-1, and on the tick where frame == N the transition fires.
- Per-tick sound pass: `current_frame % max(Length, 1) == sound_entry.frame`.
  The `max(Length, 1)` clamp prevents modulo-by-zero on Length-1 entries
  like Deployed.
- Case 0x1B (end of Deploy sequence): calls `vtable+0x558(0x1C,
  force=1, rand=0)` → enters Deployed. Also writes `entity+0x2A4 = 1` if
  `InfType+0xEC9 == 0`.
- Case 0x1F (end of Undeploy sequence): calls `vtable+0x558(0,
  force=1, rand=0)` → enters Ready. Clears `entity+0x2A4 = 0` if
  `InfType+0xEC9 == 0`.
- Case 0x14/0x15/0x24/Default ("play once, mark anim-finished"): when
  end-of-sequence reached, calls `vtable+0xF8()` (animation-finished).
  Drives `DeployedFire → Deployed` loop indirectly via the AI loop's
  `+0x68D` re-arm logic.

**Frame-counter increment** (`this[+0xF8]++`) is **NOT in DoType_Sequencer**.
It lives in the locomotor/animation advance — probably `FootClass_AI` —
called BEFORE Fire_At_Target in the AI tick. Order matters:
1. AI tick begins.
2. Locomotor / frame advance → `this[+0xF8]` increments.
3. `Fire_At_Target` runs — its FireUp gate uses the **post-incremented**
   frame value.
4. `DoType_Sequencer` runs — end-of-sequence test uses the same value.

### 3.2 Weapon selection — `InfantryClass__SelectWeapon @ 0x005218e0`
(vtable+0x2E4)

```
SelectWeapon(this, target):
    type = this[+0x6C0]               // InfantryTypeClass*
    if type[+0x6AC] == 0:              // DeployFire flag
        return TechnoClass__SelectWeaponAgainst(this, target)

    doing = this[+0x6C4]               // current Doing
    if doing ∈ {0x1B, 0x1C, 0x1D, 0x1E}:
        return type[+0x6A8]            // DeployFireWeapon (GGI: 1 → Secondary)

    if this[+0x82] != 0:               // in open-topped transport
        if type[+0xD50] != -1:          // OpenTransportWeapon set
            return type[+0xD50]

    return 0                            // Primary
```

**Branching contract for GGI:**

| GGI state | Returned weapon index | Resolves to |
|-----------|------------------------|-------------|
| Walking (Doing 0,2,3,4,5,7…) | 0 | M60 |
| Walking, inside BFRT (`+0x82=1`) | `OpenTransportWeapon=1` | MissileLauncher |
| Walking, inside IFV (`Gunner=yes`) | (IFV's path takes over — see §3.7) | CRMissileLauncher (IFV.Weapon17) |
| Deploying / Deployed / DeployedFire / DeployedIdle | `DeployFireWeapon=1` (default since GGI omits the key) | MissileLauncher |
| Undeploying (0x1F) | **0 (Primary)** | M60 — note: 0x1F is **not** in the deploy-range; during Undeploy GGI is back to primary mid-animation |

The two-pass nature is critical: `Fire_At_Target` calls SelectWeapon
**twice** — once to pick the firing-sequence to enter (Primary/Secondary
animations are different), once again right before the bullet spawn to
feed `Fire_At`. **Any port that caches the weapon index between calls is
a parity hazard** if state changes mid-tick.

### 3.3 Fire path — `InfantryClass__Fire_At_Target @ 0x005206b0`

Called once per tick from `InfantryClass__AI @ 0x0051bab0`, non-virtual.

```
Fire_At_Target(this):
    target = this->Target
    if target == NULL:
        this->FireFlag (+0x16D) = 0; return

    weapon_idx = vtable[+0x2E4](this)              // SelectWeapon — first call

    if this->FireFlag == 0:                         // not yet in firing seq
        status = vtable[+0x3C0](this, target, weapon_idx)   // GetFireError
        if status == 0:                              // CAN_FIRE
            // Pick firing sequence
            if type[+0xD94] /* NoFiringSequence */ == 0:
                doing = this->Doing
                if doing ∈ {0x1B..0x1E}:
                    vtable[+0x558](this, 0x1D, 0)   // → DeployedFire
                elif Prone && doing ∈ {0x28, 0x29}:
                    this->RotationAngle = 0          // already prone-firing
                elif weapon_idx == 0:
                    vtable[+0x558](this, IsTunnel ? 8 : 4, 0)  // Primary FireUp
                else:
                    vtable[+0x558](this, GarrisonFlag ? 0x29 : 0x28, 0)
            else:
                vtable[+0x558](this, 0x1A, 0)        // NoFiringSequence (TS-legacy-ish)
            this->FireFlag = 1
            FacingClass::UpdateFacing(...)
            if Target == Destination: FootClass::Stop_Moving(); vtable[+0x500]()
        elif status == 5: /* out-of-range nudge */
        elif status == 9: vtable[+0x45C](this)       // Uncloak

    // FRAME GATE — fire-up frame anchor selection
    type = this->TypeData
    fireUp = type[+0xE40]                            // PrimaryFireUp (idle)
    if this->GarrisonFlag: fireUp = type[+0xE44]     // FireProne
    if weapon_idx != 0:
        if this->GarrisonFlag && bullet[+0x5C8] /* Tunnel */: fireUp = type[+0xE4C]
        elif bullet[+0x5A4] /* ProneFire */: fireUp = type[+0xE48]

    if this->FireFlag == 0: return                   // not in firing seq, no bullet
    if this->CurrentSequenceFrame (+0xF8) != fireUp: return   // STRICT EQUALITY

    // BULLET SPAWN
    wpn = vtable[+0x2E4](this)                       // SelectWeapon — second call
    status2 = vtable[+0x3C0](this, Target, wpn)      // re-check eligibility
    if status2 == 0:
        vtable[+0x3CC](this, Target, wpn)            // Fire_At → TechnoClass::Fire_At
    else:
        this->FireFlag = 0
        exitSeq = (was_deployed_fire ? 0x1C : (garrison ? 2 : 0))
        vtable[+0x558](this, exitSeq, 0, 0)

    // Scatter if target moved out of capped range (Rules+0x16C0)
    ...
```

**The FireUp frame check is strict `==`, not `>=`.** Bullet fires only on
the single tick where `current_frame == fireUp_value`. Combined with four
possible fireUp values selected by (`GarrisonFlag`, `weapon_idx`,
`bullet.ProneFire`, `bullet.Tunnel`), this is the most off-by-one-prone
block in the entire firing path.

For GGI deployed fire: `weapon_idx == 1` (Secondary), `GarrisonFlag == 0`
(not garrisoned), and `MissileLauncher`'s `AAHeatSeeker2` projectile has
`Tunnel=0`, `ProneFire=0` (those fields are at `BulletType+0x5A4/+0x5C8`
NOT on the small BulletType layout — Phase 3 noted these may actually be
on `BulletClass` runtime instance, or a different class entirely;
unresolved at MEDIUM confidence). Falls through to `fireUp = type[+0xE40]`
= GGI's `FireUp=2` (from art `[GGI]`). **Bullet spawns on the tick where
`current_frame == 2` within the DeployedFire 6-frame sequence**, i.e., on
the 3rd visible frame.

### 3.4 Eligibility — `TechnoClass__GetFireError @ 0x006fc0b0` (with
InfantryClass override entry at `0x0051c8b0`, vtable+0x3C0)

The InfantryClass vtable+0x3C0 entry at `0x0051c8b0` is a thin override
that branches on `Doing == 0xB..0xE` (death animations) at entry and then
tail-calls or falls through to `TechnoClass__GetFireError`. The shared
return-value enum:

| Code | Meaning | Triggers |
|------|---------|----------|
| 0 | OK — fire | all gates pass |
| 1 | No ammo | `Ammo == 0` (rare on GGI) |
| 3 | Reloading | rearm timer running, burst-index mismatch, teleport-warp, locomotor-pathing-into-target |
| 5 | Cannot target | invalid target, sinking, cloaked-and-disabled, **AA-vs-ground mismatch via Verses==0**, **passenger-in-open-transport-without-CanFireWhilePassengerInOpenTopped (`+0x143==0`)**, IronCurtain, Bunker, depth/bridge mismatch |
| 6 | Out of range | distance > effective range, no weapon for target |
| 8 | Aim error | turret can't lock, IsLocked block |
| 9 | Cloak block | cloak active + `weapon+0x133` NoCloakedFire |

**AA gate (the GGI-critical one):** if target is aircraft (`vtable+0x54
true`) AND `Warhead+0x2A4 == 0` (warhead's "anti-air versus" / Verses[Air]
== 0 marker — note this offset on the Warhead is **separate** from the
BulletType `+0x2A4 AA` flag despite sharing the number), returns 5. So
walking GGI clicked on a Kirov:
1. Cursor stays as red attack reticle (What_Action_OnObject doesn't filter).
2. On click → GetFireError returns 5 → order silently dropped.
3. Player perceives "GGI can't shoot Kirov while standing" without an
   explicit cursor cue.

**Open-transport gate:** when `this+0x82 != 0` (in open transport) AND the
selected weapon's `+0x143` ("CanFireWhilePassengerInOpenTopped") flag is
0, returns 5. For GGI inside BFRT, the weapon is the GGI's own Secondary
(MissileLauncher) via `OpenTransportWeapon=1`, and MissileLauncher's
`+0x143` must be set for the GGI to fire from inside BFRT. The flag isn't
documented in the ini Section 2 dump but the parsing site is at
`WeaponTypeClass+0x143` (in the bool block `+0x140..+0x148`).

### 3.5 Crush gate — `TechnoClass__CanCrushCheck @ 0x005f6cd0`

Two-branch predicate; either branch returning 1 = crush allowed.

```
CanCrushCheck(victim, crusher):
    // Branch A: standard "crusher's Crushable list"
    if crusher != NULL:
        crusherType = crusher->vtable[+0x84]()
        if crusherType[+0xD29] /* CanCrush */ != 0 && victim active:
            victimType = victim->vtable[+0x84]()
            if victimType[+0xD2A] /* Crushable list-flag */ == 0:
                if victim is not Building:
                    if !ally(victim) && !victim->vtable[+0x160]() /* IsImmuneToCrush */:
                        return 1

    // Branch B: AbstractTypeClass-side crush (DeployedCrushable path)
    victimAbsType = victim->vtable[+0x88]()
    if victimAbsType[+0x22D] != 0 &&
       victim active &&
       (byte)victim[+0x2A4] == 0:               // <-- runtime gate
        if !ally(victim) && !victim->vtable[+0x160]():
            return 1

    return 0
```

**Deployed GGI's uncrushability flows through Branch B:**
- At Deploy completion, Sequencer writes `victim[+0x2A4] = 1` (because
  `InfType+0xEC9 == 0` for GGI).
- Branch B then short-circuits on `(byte)victim[+0x2A4] != 0` —
  Branch B's positive case is never reached, returning 0 (not crushable).
- Branch A still runs but Crushable is presumably 1 on a `Crushable=yes`
  unit — but Branch A's positive case requires `victimType[+0xD2A] == 0`
  for the standard list-flag too. The exact interplay here needs in-game
  verification: gamemd's observable behavior is that deployed GGI is NOT
  crushable, so something in Branch A also gates on the deployed state —
  most likely the `IsImmuneToCrush` vtable predicate at `+0x160` checks
  the same `+0x2A4` byte. **Open question: confirm by decompiling
  InfantryClass::IsImmuneToCrush.**

### 3.6 Damage application — `ApplyWarheadDamage @ 0x00489180`

```
ApplyWarheadDamage(damage, warhead, armor, distance):
    if damage == 0 || (ScenarioFlags & 0x20) || warhead == NULL: return 0
    if damage < 0:                                  // healing
        return (distance >= 8 leptons) ? 0 : damage

    dmgF       = (float)damage
    pctAtMaxF  = dmgF * warhead[+0x12C]              // PercentAtMax
    maxDistLep = ftol(warhead[+0x124] * 256.0f)      // CellSpread → leptons
    afterFalloff = damage

    if pctAtMaxF != dmgF && maxDistLep != 0:        // falloff active
        lerp = (dmgF - pctAtMaxF) * (maxDistLep - distance) / maxDistLep + pctAtMaxF
        afterFalloff = ftol(lerp)

    if afterFalloff <= 0: afterFalloff = 0           // zero-floor (no min-1 clamp)
    final = (double)afterFalloff * warhead.Verses[armor]   // wh+0xA0 + armor*8
    result = ftol(final)
    if result >= Rules->MaxDamage (Rules+0x16C8): result = Rules->MaxDamage
    return result
```

**Order:** falloff (linear) → zero-floor → Verses → MaxDamage clamp.

**Verses array** at `WarheadType+0xA0`, 11 doubles, indexed by
`target.Type.Armor` at `typeclass+0x9C`. For GGI weapons:
- `SA` (M60 path): `100, 80, 80, 50, 25, 25, 75, 50, 25, 100, 100`
- `GUARDWH` (MissileLauncher path): `20, 20, 20, 100, 50, 100, 10, 10, 10, 100, 100`

The Verses INI values are percentages (`100%` → `1.0`, `80%` → `0.8` in the
double array). Verified by the parsing path supporting `%` suffix and
otherwise int×0.01.

**ProneDamage application site: UNVERIFIED.** Phase 3-B searched for
`fmul qword [...+0xF8]` byte patterns in the damage chain and found no
hits in `ApplyWarheadDamage`. Hypothesis: ProneDamage (warhead+0xF8) is
applied at the **firer side** — `TechnoClass::Fire_At` multiplies the
weapon's base damage by `warhead.ProneDamage` BEFORE invoking
`ApplyWarheadDamage`. This would explain its absence in the receive
formula. **Confirm in a follow-up trace.**

**`ftol` rounding mode:** the engine relies on `Math__ftol` at
`~0x007cce80`. The MSVC `_ftol` runtime intrinsic typically uses
**truncation toward zero**. If the engine has not changed the FPU mode
elsewhere, damage values truncate (not banker's-round). This is a parity-
critical detail and should be verified by reading the prologue of
`Math__ftol` before any Rust port commits to a rounding rule. **Tracking
open.**

### 3.7 IFV vs BFRT routing for GGI

**GGI in BFRT** (BFRT: `OpenTopped=yes`, no `Gunner=yes`):
- `PerCellProcess` sets `GGI[+0x82] = 1` when the GGI enters/parks inside
  the BFRT.
- BFRT itself uses its **own** `OpenTransportWeapon` (or its weapon list)
  to fire — not the passenger's.
- However, the BFRT's "fire from passenger" path checks each passenger's
  GetFireError. If `weapon[+0x143] == 0`, that passenger can't fire from
  inside. The BFRT fires a combined weapon based on its onboard count.
- The GGI's **own** `OpenTransportWeapon=1` is only consulted if GGI is in
  an open-topped vehicle that has **no** `Gunner=yes` AND fires via
  `SelectWeaponAgainst` returning 1 → MissileLauncher. This is the path
  for non-Gunner open-topped vehicles like FV/Halftrack.

**GGI in IFV** (IFV: `Gunner=yes` + `OpenTopped=yes`):
- IFV's INI iterates through 17 keyed `*TurretIndex`/`*TurretWeapon` pairs.
  For each pair, `FUN_00717890(IndexValue, WeaponValue)` writes
  `*(IFV.Type + 0x814 + IndexValue * 4) = WeaponValue`.
  - For GGI: `GuardianTurretIndex=3, GuardianTurretWeapon=16` →
    `IFV.Type[+0x814 + 12] = 16`.
- When the GGI boards the IFV, `SetGunnerWeapon(IFVMode=16)` runs:
  - Writes `IFV[+0x138] = 16` (current IFVMode).
  - Writes `IFV[+0x124] = IFV.Type[+0x814 + 16*4]` — **WAIT**: indices 0..3
    are the turret-frame indices, so `+0x814 + 16*4 = +0x854` reads past
    the turret-frame table.

  Reconciliation: `IFVMode` is the **passenger's declaration** of which
  IFV mode it picks. The `+0x814` table at index `IFVMode` is the **turret
  SHP frame**. For GGI's IFVMode=16, the frame lookup is at `+0x854`, but
  only frames 0..3 are visually meaningful — the IFV is drawn with the
  turret frame matching the *passenger's IFVMode*, looked up dynamically.
  The frame at index 3 (from `GuardianTurretIndex=3`) is the missile-
  launcher pose. So GuardianTurretIndex=3 separately tells the renderer
  to draw frame 3.

  Actual weapon firing: SelectWeaponAgainst checks `IFV.Type[+0x808]`
  (WeaponCount). IFV has WeaponCount > 0 (17 weapons declared), so it
  takes the gattling-style branch. `CurrentWeaponNumber` is driven to
  match `IFVMode=16` via `FUN_0070e1a0`, calling `vtable+0x3F8(16)`
  = `IFV.WeaponList[16]` = `CRMissileLauncher`.
- **GGI's own `OpenTransportWeapon=1` is dead inside IFV** — the IFV's
  gunner path takes over completely.

### 3.8 Cursor / What_Action

- `What_Action_OnObject @ 0x0051e3b0` — returns 5 (attack reticle) for
  any enemy in WeaponRange, **regardless of AA capability**. No GGI-
  specific branch. Hovering an air target with undeployed GGI shows the
  red attack cursor; click is silently rejected by GetFireError.
- `What_Action_OnCell @ 0x0051f800` — does **NOT** offer "deploy here" as
  a cursor. GGI deploy is the `D` hotkey via `DeployCommandClass`, not a
  cell-click action. Returns plain Move (2) or out-of-range (0x1A) for
  empty cells.
- No auto-deploy on target-acquired exists in `InfantryClass::AI`.
  Deploy is purely player-initiated.

### 3.9 Idle behavior while deployed

`InfantryClass::IdleDispatch @ 0x0051cba0` early-returns if `Mission ==
0x1C` (Mission_Deployed). So no random idle animations cycle while GGI is
deployed. The sequence stays at `Deployed` (Doing 0x1C), advanced by the
art-INI sequence loop. The `DeployedIdle` Doing (0x1E) is selected by the
sequence engine itself based on the `Sequence=` table loop, not by an
explicit timeout in IdleDispatch.

### 3.10 Auto-undeploy on move

When a deployed GGI receives a move command:
1. The command queue posts an event (kind 5 → Assign_Mission(Move)).
2. `FootClass::AI` sees Mission switching while Doing ∈ {0x1B..0x1E}.
3. `Scatter @ 0x0051D0D0` is invoked; if `Doing` is in the deploy range
   and force_arg + override_arg are set, calls `Set_Sequence(0x1F)`
   (Undeploy).
4. Sequencer plays Undeploy frames (`Undeploy=180,2,2` for GGI), then
   on completion enters Doing 0 (Ready), clears `+0x2A4`, and the
   queued move command is accepted.

For player-controlled units in deploy sequences, `Scatter` early-returns
when force_arg or override_arg are not set — so passive scatter doesn't
auto-undeploy; only explicit move orders do.

---

## 4. INI Keys

Only **GGI-specific value-deltas** vs E1 are highlighted. Generic
infantry keys are documented in the E1 dossier.

### 4.1 [GGI] rules section

| Key | Value | Default | Field | Effect |
|-----|-------|---------|-------|--------|
| `Primary` | `M60` | — | TechnoType primary slot | walking weapon |
| `Secondary` | `MissileLauncher` | — | TechnoType secondary slot | deployed weapon |
| `ElitePrimary` | `M60E` | — | | veteran walking weapon |
| `EliteSecondary` | `MissileLauncherE` | — | | veteran deployed |
| `Deployer` | `yes` | no (0) | InfType+0xEC8 | enables deploy state machine |
| `DeployedCrushable` | `no` | **yes (1)** | InfType+0xEC9 | **critical override**: prevents the runtime `+0x2A4 = 1` write being meaningful, AND lets that write actually happen (Sequencer only writes `+0x2A4 = 1` when `InfType+0xEC9 == 0`) |
| `IFVMode` | `16` | 0 | TechnoType+0x688 | IFV passenger mode index — maps to `GuardianTurretIndex/Weapon` via IFV INI |
| `OpenTransportWeapon` | `1` | -1 | TechnoType+0xD50 | when in non-Gunner open transport (BFRT), passenger fires its own Secondary |
| `Occupier` | `no` | yes | InfType+0xEB4 | cannot enter civilian garrison — **GGI delta vs E1=yes** |
| `Pip` | `white` | — | InfType+0xDFC | transport-cargo pip color |
| `DeploySound` | `GuardianGIDeploy` | -1 | InfType+0xEA4 | played at the START of `Do_Action(0x1B)` (before Doing write) |
| `UndeploySound` | `GIUndeploy` | -1 | InfType+0xEA8 | same for 0x1F |
| `IsSelectableCombatant` | `yes` | 0 | TechnoType+0xDBC | included in select-all-combat hotkey — reader unverified |
| `ImmuneToVeins` | `yes` | — | TechnoType (E1 doc) | TS-legacy on RA2; no veins in YR |
| `VoiceSelect`, `VoiceMove`, `VoiceAttack`, `VoiceFeedback`, `VoiceSpecialAttack`, `DieSound`, `CrushSound` | `GuardianGI*` strings | — | TechnoType voc slots | per-voice routing, parsed by TechnoTypeClass__ReadINI |
| `VeteranAbilities` | `STRONGER,FIREPOWER,ROF,SIGHT,FASTER` | — | — | applied at promotion |
| `EliteAbilities` | `SELF_HEAL,STRONGER,FIREPOWER,ROF` | — | — | applied at elite promotion |
| `ThreatPosed` | `10` | — | — | AI threat weight |
| `PixelSelectionBracketDelta` | `-6` | 0 | — | bracket Y offset |

**Keys GGI OMITS** that E1 sets (value-delta = "left at default"):
`VoiceComment`, `DeathAnims`, `DeadBodies`, `OccupyPip`, `OccupyWeapon`,
`EliteOccupyWeapon`. These omissions are observable: GGI cannot fire
from garrison buildings (no OccupyWeapon).

### 4.2 [GGI] art section

| Key | Value | Field | Notes |
|-----|-------|-------|-------|
| `Cameo` | `GDGIICON` | — | sidebar icon |
| `AltCameo` | `GDGIUICO` | — | fog-of-war cameo |
| `Sequence` | `GuardianGISequence` | — | reference to sequence section |
| `Crawls` | `yes` | InfType+0xEBD | prone-while-crawling supported (Crawl Doing 5 active) |
| `Remapable` | `yes` | — | house color remap |
| `FireUp` | `2` | InfType+0xE40 | bullet-spawn frame within Primary FireUp sequence (and reused for deployed fire — see §3.3) |
| `PrimaryFireFLH` | `80,0,105` | TechnoType `+0x8AC/+0x8B0/+0x8B4` (Primary FLH triplet) | muzzle origin for M60 |
| `SecondaryFireFLH` | `80,0,90` | TechnoType `+0x8B8/+0x8BC/+0x8C0` | muzzle origin for MissileLauncher (height 90 vs Primary's 105 — missile spawns slightly lower than M60 to match the prone/deployed-shoulder pose) |

### 4.3 [GuardianGISequence] (artmd.ini)

20 keys defining frame ranges. Key deploy-relevant entries:

| Key | Value | Effect |
|-----|-------|--------|
| `Deploy` | `300,15,0` | start frame 300, 15 frames (deploy animation duration) |
| `Deployed` | `315,1,1` | held frame 315 (deployed idle, 1-frame loop) |
| `DeployedFire` | `323,6,6` | 6-frame fire animation starting at 323 |
| `Undeploy` | `180,2,2` | 2-frame undeploy animation starting at 180 (reuses Up frames) |

The Length values (`15`, `1`, `6`, `2`) end up at sequence-entry `+0x04`
and gate the `current_frame < Length` test in DoType_Sequencer.

### 4.4 Weapons / projectiles / warheads (offset-verified)

`WeaponTypeClass__ReadINI @ 0x00772080`:

| Field | Offset | Type | Default |
|-------|--------|------|---------|
| `Damage` | `+0xA4` | int | 0 |
| `Projectile` (BulletType*) | `+0xA0` | ptr | 0 |
| `Warhead` (WarheadType*) | `+0xAC` | ptr | 0 |
| `Speed` | `+0xA8` | speed | 0 |
| `Range` (cells×256 → **leptons**) | `+0xB4` | int | 0 |
| `MinimumRange` (leptons) | `+0xB8` | int | 0 |
| `Burst` | `+0x9C` | int | 0 |
| `Anim[8]` (muzzle flashes by facing) | `+0xF4..+0x10C` | ptr×8 | empty |
| `OccupantAnim` | `+0x110` | ptr | 0 |
| `Report` (sound list) | `+0xCC..+0xD4` | DynamicVector | empty |
| `RevealOnFire` | `+0x137` | bool | 0 |
| `NeverUse` | `+0x136` | bool | 0 |
| `FireOnce` | `+0x135` | bool | 0 |
| `IsLaser` | `+0x149` | bool | 0 |
| `FireWhileMoving` | `+0x141` | bool | 0 |
| `DrainWeapon` | `+0x142` | bool | 0 |
| `FireInTransport` (`CanFireWhilePassengerInOpenTopped`) | `+0x143` | bool | 0 |
| `IsRadBeam` | `+0x154` | bool | 0 |
| `Lobber` | `+0x12E` | bool | 0 |

**Inaccurate is a BulletType flag, not a WeaponType flag.** Phase 3-A
confirmed.

`BulletTypeClass__ReadINI @ 0x0046bee0` (param_1 is `int`, direct byte
offsets):

| Field | Offset | Type | Default | AAHeatSeeker2 value |
|-------|--------|------|---------|----------------------|
| `Airburst` | `+0x294` | bool | 0 | 0 |
| `Floater` | `+0x295` | bool | 0 | 0 |
| `SubjectToCliffs` | `+0x296` | bool | 0 | 0 |
| `SubjectToElevation` | `+0x297` | bool | 0 | 0 |
| `SubjectToWalls` | `+0x298` | bool | 0 | 0 |
| `VeryHigh` | `+0x299` | bool | 0 | 0 |
| `Shadow` | `+0x29A` | bool | **1 (TRUE!)** | 0 (explicit `Shadow=no`) |
| `Arcing` | `+0x29B` | bool | 0 | 0 |
| `Dropping` | `+0x29C` | bool | 0 | 0 |
| `Level` | `+0x29D` | bool | 0 | 0 |
| `Inviso` | `+0x29E` | bool | 0 | 0 |
| `Proximity` | `+0x29F` | bool | 0 | 0 |
| `Ranged` | `+0x2A0` | bool | 0 | 1 |
| `Rotates` | `+0x2A1` | bool | **1 (TRUE, inverted parse)** | 1 |
| `Inaccurate` | `+0x2A2` | bool | 0 | 0 |
| `FlakScatter` | `+0x2A3` | bool | 0 | 0 |
| **`AA`** | **`+0x2A4`** | bool | **0 (false)** | **1** |
| **`AG`** | **`+0x2A5`** | bool | **1 (TRUE!)** | **1** (redundant with default) |
| `Degenerates` | `+0x2A6` | bool | 0 | 0 |
| `Bouncy` | `+0x2A7` | bool | 0 | 0 |
| `AnimPalette` | `+0x2A8` | bool | 0 | 0 |
| `FirersPalette` | `+0x2A9` | bool | 0 | 0 |
| `Cluster` | `+0x2AC` | int | 1 | 1 |
| `AirburstWeapon` (ptr) | `+0x2B0` | WeaponType* | 0 | — |
| `ShrapnelWeapon` | `+0x2B4` | WeaponType* | 0 | — |
| `ShrapnelCount` | `+0x2B8` | int | 0 | — |
| `DetonationAltitude` | `+0x2BC` | int | 700 | — |
| `Vertical` | `+0x2C0` | bool | 0 | 0 |
| `Elasticity` | `+0x2C8` | double | 0.75 | — |
| `Acceleration` | `+0x2D0` | int | 3 | — |
| `Color` | `+0x2D4` | int | 0 | — |
| `Trailer` (AnimType*) | `+0x2D8` | ptr | 0 | — |
| `ROT` (rotation rate) | `+0x2DC` | int | 0 | **60** |
| `CourseLockDuration` | `+0x2E0` | int | 0 | — |
| `SpawnDelay` | `+0x2E4` | int | 3 | — |
| `Scalable` | `+0x2EC` | bool | 0 | — |
| `Arm` | `+0x2F0` | int | 0 | **2** |
| `AnimLow/High/Rate` | `+0x2F4..+0x2F6` | byte×3 | 0 | — |
| `Flat` | `+0x2F7` | bool | 0 | — |

**`AG` defaults TRUE** — any bullet that doesn't explicitly say `AG=no`
is anti-ground. Be careful in any Rust port.

`Inaccurate` scatter applies only when `Inaccurate && Arcing` (per Phase
3-A trace of `TechnoClass__Fire_At`). For GGI's `AAHeatSeeker2`,
`Inaccurate=0` and `Arcing=0`, so no scatter is applied — the missile
homes purely via `Ranged=1 + ROT=60` in `BulletClass__HomingTrack` (not
analyzed in this pass).

`WarheadTypeClass__ReadINI @ 0x0075d590` (wrapper) / `0x0075d3a0` (body):

| Field | Offset | Type | Default |
|-------|--------|------|---------|
| `Deform` | `+0x98` | double | 0.0 |
| **`Verses` array (11 doubles)** | **`+0xA0..+0xF7`** | double[11] | 1.0 each |
| `ProneDamage` | `+0xF8` | double | **1.0** |
| `DeformThreshhold` | `+0x100` | int | 0 |
| `AnimList` (DynVec<AnimType*>) | `+0x104..+0x11C` | vec | empty |
| `InfDeath` | `+0x120` | int | 0 |
| `Spread` / `CellSpread` | `+0x124` | float (cells) | 0 |
| `CellInset` | `+0x128` | float | 0 |
| `PercentAtMax` | `+0x12C` | float | 1.0 (no falloff) |
| `CausesDelayKill` | `+0x130` | bool | 0 |
| `DelayKillFrames` | `+0x134` | int | 0 |
| `DelayKillAtMax` | `+0x138` | float | 0 |
| `CombatLightSize` | `+0x13C` | float | 0 |
| `Particle` (AnimType*) | `+0x140` | ptr | 0 |
| `Wall` | `+0x144` | bool | 0 |
| `WallAbsoluteDestroyer` | `+0x145` | bool | 0 |
| `PenetratesBunker` | `+0x146` | bool | 0 |
| `Wood` | `+0x147` | bool | 0 |
| `Tiberium` (TS-legacy: gates veins not ore) | `+0x148` | bool | 0 |
| (no-Verses early-out flag) | `+0x149` | bool | computed |
| `Sparky` | `+0x14A` | bool | 0 |
| `Sonic` (mostly TS-legacy) | `+0x14B` | bool | 0 |
| `(fire-bool)` | `+0x14C` | bool | 0 |
| `Conventional` | `+0x14D` | bool | 0 |
| `Rocker` | `+0x14E` | bool | 0 |
| `DirectRocker` | `+0x14F` | bool | 0 |
| `Bright` | `+0x150` | bool | 0 |
| `EMEffect` | `+0x154` | bool | 0 |
| `MindControl` | `+0x155` | bool | 0 |
| `Poison` | `+0x156` | bool | 0 |
| `IvanBomb` | `+0x157` | bool | 0 |
| `ElectricAssault` | `+0x158` | bool | 0 |
| `Parasite` | `+0x159` | bool | 0 |
| `Temporal` | `+0x15A` | bool | 0 |
| `IsLocomotor` | `+0x15B` | bool | 0 |
| `Locomotor` (GUID, 16 bytes) | `+0x15C..+0x16B` | GUID | 0 |
| `Airstrike` | `+0x16C` | bool | 0 |
| `Psychedelic` | `+0x16D` | bool | 0 |
| `BombDisarm` | `+0x16E` | bool | 0 |
| `Paralyzes` (frames) | `+0x170` | int | 0 |
| `Culling` | `+0x174` | bool | 0 |
| `MakesDisguise` | `+0x175` | bool | 0 |
| `NukeMaker` | `+0x176` | bool | 0 |
| `Radiation` | `+0x177` | bool | 0 |
| `PsychicDamage` | `+0x178` | bool | 0 |
| `AffectsAllies` | `+0x179` | bool | 0 |
| `Bullets` | `+0x17A` | bool | 0 |
| `Veinhole` (TS-legacy — no veins in YR) | `+0x17B` | bool | 0 |
| `Shake[X/Y]/[lo/hi]` | `+0x17C..+0x18B` | int×4 | 0 |
| `DebrisTypes` / `DebrisAnims` | `+0x18C..+0x1C3` | vec×2 | empty |
| `MaxDebris` / `MinDebris` | `+0x1C4 / +0x1C8` | int×2 | 0 |

For GGI's two warheads, the canonical Verses arrays (parse target):

| Armor index | SA (M60) | GUARDWH (Missile) |
|------------:|---------:|------------------:|
| 0 (none)    | 1.00     | 0.20              |
| 1           | 0.80     | 0.20              |
| 2           | 0.80     | 0.20              |
| 3           | 0.50     | 1.00              |
| 4           | 0.25     | 0.50              |
| 5           | 0.25     | 1.00              |
| 6 (wood)    | 0.75     | 0.10              |
| 7           | 0.50     | 0.10              |
| 8 (concrete)| 0.25     | 0.10              |
| 9           | 1.00     | 1.00              |
| 10          | 1.00     | 1.00              |

`SA.ProneDamage = 0.70`, `GUARDWH.ProneDamage = 0.50`, `GUARDWH.CellSpread =
0.5 cells (128 leptons)`, `GUARDWH.PercentAtMax = 0.5`, `SA.InfDeath = 1`,
`GUARDWH.InfDeath = 3`.

---

## 5. Integration Points

**Tick order** (within `World::advance_tick` analog):
1. Command intake (player deploy command, move command, attack-target).
2. Locomotor / sequence frame advance — `this[+0xF8]` increments.
3. `InfantryClass__AI @ 0x0051bab0`:
   - `Fire_At_Target` (the FireUp `==` gate runs against the post-
     incremented frame).
   - `DoType_Sequencer` (the end-of-sequence transitions fire when
     `frame == Length`).
   - Idle/Scatter (deploy gates examined).
4. Crush eligibility check — runs from vehicle's Drive_Track path; reads
   the victim's `+0x2A4` byte for Branch B of `CanCrushCheck`.

**Where GGI hooks into surrounding systems:**
- Building (BFRT/IFV) → consumes GGI's `+0x82` open-transport flag and
  GGI's `OpenTransportWeapon` (for BFRT) or its `IFVMode` (for IFV).
- Warhead damage → reads target's Armor at `typeclass+0x9C` then indexes
  Verses at `warhead+0xA0`.
- Crush mechanic → reads `entity+0x2A4` on victim.

**Callers of `Do_Action` (vtable+0x558):**
- `DoType_Sequencer` itself (state transitions).
- `Fire_At_Target` (firing-sequence entry).
- Mission AI (Deploy/Undeploy mission switches).
- Direct `get_function_callers` returns empty for `0x0051d6f0` — every
  call site goes through the vtable.

---

## 6. Current Rust Implementation Status

Per the in-repo scan (Agent C of the plan), what already exists vs what's
missing for GGI parity:

**Already implemented:**
- `DeployPhase` state machine at [src/sim/deploy.rs](../../ra2-rust-game/src/sim/deploy.rs) — but uses hardcoded `DEPLOY_DEFAULT_TICKS=55`.
- `Deployer`, `DeployFire`, `DeployFireWeapon`, `DeploySound`,
  `Secondary`, `IFVMode`, `OccupyWeapon` parsing at [src/rules/object_type.rs](../../ra2-rust-game/src/rules/object_type.rs).
- Sequence variants (`Deploy`, `Undeploy`, `Deployed`, `DeployedFire`,
  `DeployedIdle`) in `SequenceKind` at [src/rules/infantry_sequence.rs](../../ra2-rust-game/src/rules/infantry_sequence.rs).
- Rocket flight (ballistic + homing) at [src/sim/movement/rocket_movement.rs](../../ra2-rust-game/src/sim/movement/rocket_movement.rs).
- Projectile AA/AG flags at [src/rules/projectile_type.rs](../../ra2-rust-game/src/rules/projectile_type.rs).
- `select_weapon()` with AA/AG gating at [src/sim/combat/combat_weapon.rs](../../ra2-rust-game/src/sim/combat/combat_weapon.rs).
- Warhead Verses **parsed** but **not applied** at [src/rules/warhead_type.rs](../../ra2-rust-game/src/rules/warhead_type.rs).

**Gaps surfaced by this investigation:**
1. **`DeployedCrushable` not parsed.** Critical. Must default to `true`
   (matching gamemd ctor), and GGI's `=no` must override to `false` for
   deployed-uncrushable behavior.
2. **`DeploySound`/`UndeploySound` not triggered at phase transition.**
   Order: sound BEFORE state field write (`Do_Action`'s exact order).
3. **Deploy duration is hardcoded.** Should read from art-INI
   `Deploy=300,15,0` (15 frames) — Sequencer drives the
   `frame < Length` test for the transition.
4. **Verses parsed but not applied in damage.** Formula order: falloff →
   zero-floor → Verses → MaxDamage clamp. No min-1 clamp.
5. **`ftol` rounding unverified.** Likely truncation (MSVC `_ftol`
   default). Match this — banker's-rounding would drift parity.
6. **ProneDamage application site unidentified.** Hypothesis: applied
   firer-side in `TechnoClass::Fire_At` to base damage before `ApplyWarheadDamage`.
7. **FireUp frame gate `==` not `>=`.** A Rust port using `>=` will fire
   on every frame from FireUp through end, which is wrong.
8. **AA target preference / no auto-deploy.** GGI does NOT auto-deploy
   on an air target. The cursor stays attack-reticle for any enemy in
   primary range, even if AA-only. Rust should not "help" the player
   here.
9. **BFRT vs IFV routing distinction.** Inside IFV (Gunner=yes), GGI's
   `OpenTransportWeapon=1` is dead and IFV's per-passenger weapon list
   takes over. Inside BFRT (no Gunner), GGI's `OpenTransportWeapon=1`
   actively selects MissileLauncher.
10. **`+0x2A4` only consumed by `CanCrushCheck`.** Don't wire it as a
    movement lock — movement immobility comes from Mission_Deploy not
    relinquishing the entity, not from `+0x2A4`.
11. **`IsSelectableCombatant` consumer not located** (parsed at `+0xDBC`,
    but the select-all-combat dispatch reader is in an unlabeled
    keyboard-handler).
12. **CanFireWhilePassengerInOpenTopped (`Weapon+0x143`)** controls
    whether GGI can fire its own Secondary from inside BFRT. The bit at
    `+0x143` must be checked in the open-transport fire path.

---

## 7. Open Questions

1. **ProneDamage application site.** Where does the firer multiply by
   `warhead+0xF8`? Likely in `TechnoClass::Fire_At @ 0x006fdd50` — needs
   a targeted decompile and a dataflow query on `warhead+0xF8` reads.
2. **`ftol` rounding mode.** Read prologue of `Math__ftol`
   (~`0x007cce80`) and confirm truncation vs banker's-rounding.
3. **CanCrushCheck Branch A interplay with `+0x2A4`.** Branch A returns
   1 if `victimType[+0xD2A]==0` AND `!IsImmuneToCrush`. If
   `IsImmuneToCrush` is the InfantryClass override at `vtable+0x160` and
   it reads `+0x2A4`, that's the second gate path. Decompile
   `InfantryClass::IsImmuneToCrush` to confirm.
4. **Phase 1 noted `BulletType+0x5A4` (`ProneFire`) and `+0x5C8` (`Tunnel`)
   in `Fire_At_Target`'s fireUp-selection branch.** Phase 3-A could not
   find these on `BulletTypeClass` (struct ends near `+0x2F7`). These
   may live on `BulletClass` (the live bullet instance) or on a *different*
   struct accessed through a `bullet[...]` indirection. Re-trace
   `Fire_At_Target`'s `bullet[+0x5A4]` / `bullet[+0x5C8]` reads to
   resolve.
5. **`InfantryTypeClass+0x56C/0x570` vs `+0xEA4/+0xEA8`.** Phase 1 and
   Phase 2 both reference DeploySound/UndeploySound but at different
   offsets in different decompiles. The discrepancy may be: `+0x56C/+0x570`
   are the **VocClass-resolved indices** post-FindByName, while
   `+0xEA4/+0xEA8` are the ReadString slots. Both may exist with the
   final voc index forwarded between them. Decompile `Do_Action`'s sound
   block to resolve which is the runtime read.
6. **`IsSelectableCombatant` consumer.** Confirmed parsed at
   TechnoType+0xDBC but reader not in `What_Action_OnObject`/`OnCell`.
   Likely in `Select_All_Like_This` keyboard handler. Run
   `analyze_struct_field_usage` on `+0xDBC` to locate.
7. **`Tiberium=yes` on a warhead.** Confirmed TS-legacy — gates veins.
   Not relevant to GGI but worth noting in any port of the warhead
   parser to avoid implementing TS vein-destroy logic.
8. **`DeployTime` (TechnoType+0x3C8, double, default 0.0).** Confirmed
   not used for infantry deploy (Sequencer drives infantry deploy).
   Used by `UnitClass` (MCV-to-ConYard transform). Skip for GGI.
9. **`E1`'s `DeployedCrushable=` is omitted, inheriting default 1
   (TRUE).** This means basic GI sandbag is theoretically crushable in
   gamemd. In-game observation: is a deployed E1 actually crushable by
   a tank? If no, there's another gate (likely `IsImmuneToCrush` or the
   sandbag emplacement turning the unit into a non-Crushable type).
   Verify before assuming the gamemd output matches the ini surface.

---

## Sources

**Ghidra addresses decompiled in this investigation:**

| Address | Function |
|---------|----------|
| `0x005240a0` | `InfantryTypeClass__ReadINI` |
| `0x00712170` | `TechnoTypeClass__ReadINI` |
| `0x005236a0` | `InfantryTypeClass__Constructor` (primary) |
| `0x00523980` | `InfantryTypeClass__Constructor` (alt/copy) |
| `0x0051bab0` | `InfantryClass__AI` |
| `0x0051d6f0` | `InfantryClass__Do_Action` (vtable+0x558) |
| `0x00520ae0` | `InfantryClass__DoType_Sequencer` |
| `0x00521b20` | `InfantryClass__Clear_Doing_Action` |
| `0x005206b0` | `InfantryClass__Fire_At_Target` |
| `0x0051df60` | `InfantryClass__Fire_At` (vtable+0x3CC) |
| `0x0051df70` | (tail of Fire_At — not a separate function) |
| `0x005218e0` | `InfantryClass__SelectWeapon` (vtable+0x2E4) |
| `0x006f3330` | `TechnoClass__SelectWeaponAgainst` |
| `0x006fc0b0` | `TechnoClass__GetFireError` (base) |
| `0x0051c8b0` | InfantryClass GetFireError override (vtable+0x3C0; Ghidra has no function boundary, decoded via byte read) |
| `0x006f77b0` | `TechnoClass__CanFireAt` (range geometry wrapper) |
| `0x006f7220` | `TechnoClass__InRange` |
| `0x006f3970` | `TechnoClass__GetWeaponRange` |
| `0x005f6cd0` | `TechnoClass__CanCrushCheck` |
| `0x0051cba0` | `InfantryClass__IdleDispatch` |
| `0x0051cdb0` | `InfantryClass__UpdateIdleAction` |
| `0x0051d0d0` | `InfantryClass__Scatter` |
| `0x0070fec0` | `TechnoClass__IsDeploying` (vehicle-only) |
| `0x0051e3b0` | `InfantryClass__What_Action_OnObject` |
| `0x0051f800` | `InfantryClass__What_Action_OnCell` |
| `0x00772080` | `WeaponTypeClass__ReadINI` |
| `0x0046bee0` | `BulletTypeClass__ReadINI` |
| `0x0075d590` | `WarheadTypeClass__ReadINI` (wrapper) |
| `0x0075d3a0` | `WarheadTypeClass__ReadINI` (body) |
| `0x00489180` | `ApplyWarheadDamage` |
| `0x00710af0` | `TechnoTypeClass__Constructor` |
| `0x00710470` | `TechnoClass__SetInOpenTransport` |
| `0x007104a0` | `TechnoClass__ClearInOpenTransport` |
| `0x007104c0` | `CargoClass__ClearAllInOpenTransport` |
| `0x007eb058` | (data) InfantryClass vtable base |
| `0x007eb418` | (data) InfantryClass vtable + 0x3C0 → `0x0051c8b0` (memory-read confirmed) |
| `0x007eb5b0` | (data) InfantryClass vtable + 0x558 → `0x0051d6f0` |
| `0x00717890` | IFV passenger-mode TurretIndex/Weapon write helper |

**Docs cross-referenced:**
- `GI_GHIDRA_REPORT.md` — E1 dossier; reused for shared infantry layout
  and AI loop; the misleading "Guardian GI / E1" title was confirmed
  E1-only.
- `IFV_AND_OPEN_TOPPED_TRANSPORT_GHIDRA_REPORT.md` — confirmed
  `OpenTransportWeapon` semantics.
- `FIRE_AT_PIPELINE_GHIDRA_REPORT.md` — reused for generic firing.
- `WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md`, `WARHEADTYPECLASS_FULL_STRUCT_LAYOUT.md`,
  `BULLETCLASS_INIT_AND_FIRE_GHIDRA_REPORT.md` — reused for generic
  layout; offset deltas surfaced here where they extend the prior docs.
- `TECHNOCLASS_COMBAT_WEAPON_SYSTEMS_REPORT.md` — reused for
  SelectWeaponAgainst decision tree.

**INI files checked:**
- `ini/rulesmd.ini`:
  - `[GGI]` 3863
  - `[M60]` 22922, `[M60E]` 25281
  - `[MissileLauncher]` 22569, `[MissileLauncherE]` 25123
  - `[SA]` 26466, `[GUARDWH]` 26902
  - `[InvisibleLow]` 25385, `[AAHeatSeeker2]` 25678
- `ini/artmd.ini`:
  - `[GGI]` 291
  - `[GuardianGISequence]` 14166
  - `[MGUN-N..NW]` 16241

**Related plans:**
- `docs/plans/2026-05-17-ggi-guardian-gi-investigation-plan.md` — this
  investigation's scope plan.

---

## 8. Follow-up findings (2026-05-17 same-day extension)

After the initial 3-phase pass, three more functions were investigated:
`BulletClass__HomingTrack` (missile flight curve), `TechnoClass__Fire_At`
(bullet-spawn body), and `Math__ftol` (rounding mode). Findings below
extend §3 and §4 and resolve / re-frame several §7 open questions.

### 8.1 Missile homing flight — `BulletClass__HomingTrack @ 0x005b20f0`

Called by `BulletClass__AI @ 0x004666e0` once per logic tick. **Updates
velocity only; position integration is in the caller** as `pos += ftol(vel)`
(three coordinate stores). For `AAHeatSeeker2` (Ranged=yes, no Arcing, no
Vertical, ROT≥1) every tick takes the homing branch.

**Per-tick body:**

1. If target coord matches the "null target sentinel" globals
   (`DAT_00abef10/14/18`): pitch-only correction, cap `0x2000 BAM (45°)`.
2. Normal path:
   - Compute `desired_yaw = atan2(target.y - pos.y, target.x - pos.x)`,
     convert to BAM.
   - Compute desired pitch from velocity's Z-vs-XY-magnitude.
   - **Apply `Facing__IsWithinROT(cur, desired, ROT_BAM)` test** —
     `abs(cur - desired) <= abs(ROT_BAM)` (inclusive). If within, snap
     to target this tick; otherwise step ±ROT_BAM.
   - Rebuild velocity from new yaw + new pitch (preserves horizontal
     speed `sqrt(vx² + vy²)`).
3. **Cruise-altitude controller** (when `Floater==0 && (high-alt branch
   active) && ROT > 1`):
   - `cruise_alt_cells = min(altitude_in_leptons / 256, 5)` for ground
     target; `= 10` if `Floater` or `VeryHigh`.
   - `dz = bullet.z - cruise_alt_cells * 64 - ground_height`
   - **Dead-band**: `|dz| ≤ 20 leptons` → no clamp.
   - **Snap step**: `±18 leptons` when outside dead-band.
   - **Half-threshold pitch BAMs**: `dz < -32` → tilt up, cap `0x2000`,
     step `ROT/2`; `dz > +32` → tilt down, cap `0x4800`, step `ROT/2`;
     else level off at `0x4000` with step `ROT/2`.
4. **`vz >>= 2` damper every tick when Floater==0.** Z velocity is
   divided by 4 each tick — exponential decay toward cruise altitude
   rather than parabolic arc. **AAHeatSeeker2 has Floater=0 (default),
   so this applies.** This single line shapes the entire "missiles flatten
   quickly" visual feel of YR.
5. `Velocity__ApplyPitch @ 0x005b2a30` commits the new pitch to velocity.

**Stall-detect failsafe** (in `BulletClass__AI`, not HomingTrack itself):
- `bullet[+0x46]` is a "distance-not-decreasing" counter, capped at 60
  frames.
- After 60 frames, EMA on distance change; if not converging AND not
  Airburst/Floater, force self-detonate.
- Separately: stationary bullets (no velocity, no Vertical) past
  `Rules+0x5a0` frames force detonate.

**`Arm` semantics:** `Arm=2` from INI is stored at `BulletClass+0x4b`
(byte). It is incremented by an AnimRate-tick counter at `+0x12d`. So
**`Arm=2` arms after `2 × AnimRate` ticks of `+0x12d` decrement**, not
2 raw frames. Detonation gate uses `<` strict-less comparison.

**Per-tick max turn for AAHeatSeeker2:** if the canonical INI-to-BAM
scaling is `ROT_BAM = ROT_INI × 256`, then `ROT=60 → 0x3C00 BAM` ≈ 84°
per tick — effectively snap-to-target. **The exact scaling factor needs
verification by tracing `WeaponTypeClass__ReadINI`'s ROT key reader.**
(The `WeaponTypeClass` table in §4.4 lists ROT at `+0xCD4` per Phase 3-A
findings; confirm storage units.)

**Magic numbers:**

| Constant | Meaning |
|----------|---------|
| `0x2000` | 45° BAM cap on pitch-only correction (no-target branch) |
| `0x4000` | 90° BAM = horizontal flight target |
| `0x4800` | ~101° BAM cap when tilting down |
| `0x3FFF` | BAM centering offset for sin/cos table |
| `_LAB_007e2810` ≈ `-π/32768` | BAM-to-radians with sign flip |
| `DAT_00abef50` = 64 (canonical) | Lepton-per-cruise-step (one Z quarter-cell) |
| `0x14` (20) | Z dead-band leptons |
| `0x12` (18) | Per-tick Z snap step leptons |
| `0x3c` (60) | Stall-detect window frames |
| `>> 2` | Per-tick vz damper (vz /= 4) when Floater=0 |

**Off-by-ones:**
- `IsWithinROT` is **`<=`** (inclusive snap at boundary).
- Z dead-band is strict-less: `|dz| < 20` no-clamp, `|dz| == 20` is
  treated as outside dead-band.
- Half-cruise threshold is strict-less: `|dz| < 32` falls into level-off;
  exactly ±32 also falls into level-off.
- Stall counter `< 60` (so frames 0..59 accumulate, frame 60 triggers EMA).
- All position-integration `Math__ftol` calls truncate toward zero (§8.3).

**Target tracking:**
- Target relocation: bullet refreshes `+0x4d/+0x4e/+0x4f` (target center
  coord) every tick while target lives. Unconditional follow.
- Target death: target object pointer checked via `(**target+0x2c)()` —
  if returns ≠ 2, target is dead. Bullet continues toward **last-known
  position** stored at `+0x4d..+0x4f`. No "lose target → self-destruct".

**AA vs AG branching inside HomingTrack:** none. The AA/AG decision is
made upstream in `SelectWeaponAgainst` / `GetFireError`; once the missile
is in flight, it doesn't re-check target air/ground status. The
cruise-altitude controller treats Z-velocity uniformly regardless of
whether the target is in the air.

**Open questions from §8.1:**
- ROT INI-to-BAM scaling factor (×256 conventional, not verified by
  reading `WeaponTypeClass__ReadINI`).
- `DAT_00abef50` runtime value (BSS-init, canonical 64 leptons but
  static read returned zeros).
- `CourseLockDuration` (BulletType+0x2e0) — parsed but consumer not seen
  in `HomingTrack`; likely in `BulletClass__UpdateTarget`.

### 8.2 Bullet spawn — `TechnoClass__Fire_At @ 0x006fdd50`

The body is large (~50 step blocks). Key findings most relevant to GGI:

**Order of operations (definitive):**

1. WeaponType / WarheadType resolution (NULL guards).
2. **Spawner / LocomotorWarhead / Particle-system early-exits** — return
   NULL for SpawnManager units (e.g., Carrier), MindControl, IonCannon,
   Railgun, Temporal. **GGI's MissileLauncher hits none of these** —
   falls through to the normal path.
3. **Damage scaling** (in order, all via `Math__ftol`):
   - Base: `WeaponType.Damage (+0xA4)`.
   - Veteran/Elite multiplier (gated by `STRONGER` in VeteranAbilities /
     EliteAbilities at TechnoTypeClass `+0x29e/+0x2b0`).
   - Civilian multiplier (`vtable+0x400`).
   - Mind-control / berserk modifiers (`field_0x2e4`, `field_0x82`).
4. **DiskLaser short-circuit** (`WeaponType+0x149`): allocates
   DiskLaserClass, resets ROF, returns NULL. Not GGI.
5. Compute initial bullet speed via `FUN_00773070` — returns:
   - For MindControl: raw `WeaponType.Speed (+0xa8)`.
   - For others: `Rules+0x16b8` (BallisticScatter) projected through a
     sqrt(damage × c) formula — **a damage-dependent initial speed**, not
     the raw INI Speed. This is a non-obvious detail; `MissileLauncher`
     Speed=30 in INI is not what the bullet flies at initially. The
     Phase 3-A "Speed=30 stored as +0xa8" is the **input** to this
     speed-modulation function.
6. **BulletClass allocation** via `BulletClass__Allocate` (COM
   `CoCreateInstance` → `BulletClass__Init`).
   - Init writes: `+0x10c` projectile, `+0x110` damage, `+0x128` speed,
     `+0xe0` parabolic flag, `+0x6c` firer, `+0xac/+0xb0` damage/speed
     copies, **`+0x12d = Projectile.AA (+0x2f6)`** ← note this is **AA
     read from `BulletType+0x2f6`, not +0x2A4** — the bullet's AA flag
     has a separate storage slot at `+0x12d`. Phase 3-A's `+0x2A4` is the
     **ReadINI write target on BulletType**; `+0x2f6` may be the same
     field aliased, OR a separate cache. Open question.
   - `+0x114 = ColorScheme` if Projectile.IsHoming.
7. `BulletClass::SetOwner` + initial position (`vtable+0xd4`).
8. **Pre-strike damage debit (`FUN_006fdb80`)**: subtracts predicted
   post-armor damage from `target+0x70` (HP) **at fire time, before the
   bullet travels**. This is the anti-overkill mechanic — multiple
   in-flight bullets won't all stack damage on a doomed target. Gate:
   not Civilian path and not MindControl. **Worth replicating in Rust
   if not already.**
9. **Inaccurate scatter** (gate: `Inaccurate && Arcing` — NOT just
   Inaccurate; per Phase 3-A and FU#2 confirmation):
   - Branch A (non-flak): random radius in `[BallisticScatter/2,
     BallisticScatter]`, random angle, polar to target offset.
   - Branch B (flak-style, distance-scaled): radius `= rand(0,
     BallisticScatter) × dist3d / weaponRange`.
   - `Rules+0x1734` = `BallisticScatter`.
   - **AAHeatSeeker2 has neither Inaccurate nor Arcing → no scatter.**
10. Velocity vector built from (yaw, pitch) using sin/cos lookups +
    `_LAB_007e2810` constant (same as HomingTrack §8.1).
11. `BulletClass::MoveTo` (`vtable+0x1f0`) — bullet now "live."
12. **ROF reset** (BEFORE muzzle anim & Report):
    ```
    this->CurrentBurstIndex += 1
    new_rof = vtable[+0x318]()       // GetROF (with vet/firepower mods)
    if (this->field_0x298) new_rof /= 2   // Time-Freeze / stationary halves ROF
    this->field_0x2f8 = new_rof       // ROFTimer.Start
    this->field_0x2ec = g_CurrentFrameCounter  // last_fire_frame
    this->field_0x2f0 = bullet_speed  // last_bullet_speed cache
    this->field_0x2f4 = new_rof       // ROFTimer.Duration
    this->CurrentBurstIndex %= WeaponType.Burst (+0x9c)
    ```
13. **Muzzle anim selection** (driven by `WeaponType.Anim_Count (+0x104)`):
    - `== 8` → 8-direction array: `WT.Anim[((((vtable+0x308 facing >> 12)
      + 1) >> 1) & 7)]`. For GGI's `MGUN-*` array (8 entries), this is
      the active branch.
    - `> 0` → just `WT.Anim[0]`.
    - Civilian + `vtable+0x400 true` → `WT.OccupantAnim (+0x110)` (the
      **garrison/IFV overlay muzzle** — `UCFLASH` for GGI).
14. **Report voc played BEFORE muzzle anim spawn.** Gate: `WeaponType+0xCC
    > 0 && Warhead+0xCD5 (SuppressReport) == 0`. Played at muzzle FLH
    coord, random pick from `Report=` list.
15. **Muzzle AnimClass spawn**: `new AnimClass(animType, &flh_coord, 0,
    1, 0x600, 0, 0)` — `0x600` flag = transient muzzle flash. Civilian
    branch adjusts z-offset to `-200` for garrison-window pose.
    Non-civilian → `AnimClass::SetOwnerObject(this)` so muzzle follows
    the firer.
16. Wave / IsLaser / IsElectricBolt / IsRadBeam / IsRadEruption
    specialty visuals (mutually exclusive cascade). Not GGI.
17. **End-of-fire**: `vtable+0x390` (Set_Stationary) and `vtable+0x124(2)`
    (NotifyOwner: FireEvent — fans out cloak break).
18. **RevealOnFire shroud update** (gate: cloaked-firer OR
    `WeaponType+0x137 RevealOnFire`): `MapClass::RevealShroud(coord,
    radius=3, false, ...)` + `MapClass::UpdateFogBorder(coord, 4)`.
19. Naval/Limpet/Convoy/Coalition bookkeeping.

**FLH resolution path:** the primary muzzle FLH for GGI comes from
`vtable[+0xb0]` (`Get_Fire_Out_Coord`) on the firer — NOT `vtable+0x300`
as the §3.3 Fire_At_Target finding suggested. `+0x300` is
`Get_Firing_Coord` which routes elsewhere. The FLH triplet read on
TechnoTypeClass at `+0x8B8/+0x8BC/+0x8C0` (SecondaryFireFLH) feeds this
vtable call.

**ProneDamage application — DISPROVES §7.1:** Direct decompilation of the
entire `TechnoClass::Fire_At` body shows **NO read of `warhead+0xF8`**.
Searched `immediate_values` list: 0xF8 (248) absent. ProneDamage is
applied **downstream in `BulletClass::Detonate`** (somewhere in
0x468000–0x469000), most likely as a per-victim multiplier at hit-time
based on the defender's prone state. The hypothesis in §7.1 ("firer-side
in `TechnoClass::Fire_At`") is **incorrect**. ProneDamage parity now
depends on a future investigation of `BulletClass::Detonate`.

**Other notable details:**
- `Burst` index advances per shot: `this->CurrentBurstIndex %= Burst`.
  Step `8 / Burst` is used in the 8-direction hardpoint walk; `Burst > 8`
  → division is 0 (slot fixed).
- `field_0x298` flag halves ROF — confirmed Time-Freeze / cryo-frozen
  halving applied AFTER `vtable+0x318` returns the base ROF.
- Speed clamp: if `distance / 2 < bullet_speed`, halve the speed —
  anti-overshoot for close-range arcing. Doesn't affect AAHeatSeeker2 in
  practice unless the target is point-blank.
- `WeaponType+0x142` ("AttachedToTransport"): TS-era IFV-into-building
  flow; returns NULL for that branch. Not GGI.

### 8.3 `Math__ftol @ 0x007c5f00` — rounding mode

Verified by reading the control-word constant at `0x00822d80`:

```
Bytes: 7f 0e 00 00  →  0x0E7F
0x0E7F = 0000 1110 0111 1111
Bits 10-11 (RC, rounding control): 11 → ROUND TOWARD ZERO (truncation)
```

This is the canonical MSVC `_ftol` truncation constant.

**Verdict:** `Math__ftol` **truncates toward zero**. Confidence HIGH.

**Implications:**

- `0.5 → 0`, `0.9 → 0`, `1.5 → 1`, `1.9 → 1`, `-0.5 → 0`, `-1.5 → -1`.
- **Rust's `f64 as i32` / `f32 as i32` cast matches this exactly.** A
  direct cast in the Rust damage / position code preserves gamemd parity.
- In `ApplyWarheadDamage @ 0x00489180`:
  - `afterFalloff = ftol(lerp)` → truncates falloff-interpolated damage.
  - `result = ftol(final)` → truncates final damage after Verses multiply.
  - **Damage computed as e.g. `15 * 0.80 = 12.0` → ftol → 12. But `15 * 0.50 = 7.5` → ftol → 7, NOT 8.**
- In HomingTrack §8.1: position integration `pos += ftol(vel)` truncates
  the velocity each tick. Sub-lepton fractional drift accumulates as
  truncation error rather than averaging out.

### 8.4 §7 open-question status after follow-ups

| §7 item | New status |
|---------|------------|
| §7.1 ProneDamage application site | **Re-framed.** Not in `TechnoClass::Fire_At` (disproved by §8.2). Now suspected in `BulletClass::Detonate`. New investigation target. |
| §7.2 ftol rounding mode | **Resolved.** Truncation toward zero (§8.3). HIGH confidence. |
| §7.3 CanCrushCheck Branch A interplay | Still open. Decompile `InfantryClass::IsImmuneToCrush` (vtable+0x160). |
| §7.4 BulletType+0x5A4/+0x5C8 vs BulletClass | Partially clarified by §8.2: BulletClass `+0x12d` holds the AA flag (read from `BulletType+0x2f6` per Init), separate from `BulletType+0x2A4` parser write. Still need to identify the `+0x5A4`/`+0x5C8` Phase 1 reads. |
| §7.5 DeploySound offset discrepancy | Still open. `+0x56C/+0x570` likely runtime voc indices, `+0xEA4/+0xEA8` likely ReadString slots. Resolve by decompiling `Do_Action`'s sound block more carefully. |
| §7.6 IsSelectableCombatant consumer | Still open. |
| §7.7 Tiberium=yes warhead | Open / noted as TS-legacy not GGI-relevant. |
| §7.8 DeployTime not used by infantry | Confirmed resolved. |
| §7.9 E1 deployed-crushable observable | Still open — needs in-game verification. |

### 8.5 New open questions raised by §8

- **`BulletClass::Detonate` path for ProneDamage.** Where exactly is
  `warhead+0xF8` multiplied into damage at hit time? Bullet stores
  `damage` at `+0x110`; Detonate must re-scale by ProneDamage when
  victim is prone.
- **`FUN_006fdb80` pre-strike damage debit.** This subtracts predicted
  damage from victim HP at fire time. If our Rust impl doesn't have
  this, multiple bullets in flight against the same target won't share
  the "anti-overkill" semantics — overshooting damage on already-doomed
  units.
- **`WeaponType.Speed` is modulated by damage** via `FUN_00773070`
  (returns `sqrt(damage × c) × scatter_factor` rather than raw Speed).
  The Rust port must NOT assume `WeaponType.Speed` is the literal
  initial bullet velocity.
- **`BulletClass+0x12d` AA flag** is set from `BulletType+0x2f6` in Init,
  not `+0x2A4`. The ReadINI writes `+0x2A4` per Phase 3-A; check
  whether `+0x2f6` is a copy/alias or a separate read site.
- **`Burst > 8` edge case:** `8 / Burst = 0` so the muzzle hardpoint
  slot stays fixed across all burst shots. Worth verifying intended
  behavior.
- **ROT INI-to-BAM scaling.** §8.1 assumed ×256 but it was not directly
  verified from `WeaponTypeClass__ReadINI`'s ROT reader. AAHeatSeeker2's
  ROT=60 → snap-or-curve depends entirely on this factor.

---

## 9. Second-round follow-up findings (closure of §7/§8 open items)

A targeted second-round pass investigated: (a) where `ProneDamage` is
actually applied, (b) `InfantryClass`'s vtable+0x160 override, (c)
DeploySound/UndeploySound offset reconciliation, (d) the ROT
INI-to-BAM scaling factor, and (e) the Phase 1 `+0x5A4`/`+0x5C8` reads.
Three of these produced surprises that change the Rust-port guidance.

### 9.1 ⚠ `ProneDamage` is dead data in YR — DO NOT IMPLEMENT

**Resolves §7.1 with a trap finding.**

An exhaustive byte-pattern sweep across `gamemd.exe` for every plausible
x87 encoding of a `+0xF8` read on a WarheadType pointer (`FLD/FSTP/FMUL
qword` at displacement `+0xF8` with all 8 ModR/M bases) found **zero
consumers**. Every hit at offset `0xF8` was:
- `BulletClass+0xE8/F0/F8` (the velocity vector — same offset, different
  class, pure coincidence)
- `HouseClass` / `HouseTypeClass` unrelated fields
- The ReadINI **write** itself at `WarheadTypeClass__ReadINI_Body`
  (`fstp [esi+0xF8]`)

Cross-checked the entire damage chain visually:
`WarheadTypeClass__Detonate @ 0x004690b0` → `Apply_area_damage @
0x00489280` → `ApplyWarheadDamage @ 0x00489180` → `ObjectClass__ReceiveDamage @ 0x005f5390` → `TechnoClass__ReceiveDamage @ 0x00701900` → `InfantryClass__ReceiveDamage @ 0x005227f0`. **None of them
read `warhead+0xF8`.** TechnoClass applies the per-armor multiplier and
veterancy mults; InfantryClass adds an alliance gate. No prone check
ever happens during damage application.

**Verdict:** `ProneDamage=` is a Tiberian Sun holdover. The INI key is
still parsed (so save-files round-trip cleanly), but the runtime
consumer was stripped between TS and RA2/YR. **Prone state does not
modify damage in YR.**

**Parity implication — this is the trap:** a Rust port that "implements
ProneDamage because the INI documents it" will deal **70% of the
correct damage** when GGI's M60 hits a prone infantry, and **50% of
correct damage** when MissileLauncher hits a prone infantry. The fix
is to **drop the multiplier**, not to add it.

Confidence axes: content HIGH (exhaustive sweep), identity HIGH
(matched against existing function labels), binding HIGH (the absence
of consumers is comprehensive, not pipeline-local).

### 9.2 ⚠ `InfantryClass` does NOT override `vtable+0x160` — refactors §3.5

**Resolves §7.3, refutes the prior hypothesis.**

Read InfantryClass vtable at slot `+0x160` (`0x007eb058 + 0x160 =
0x007eb1b8`): bytes `40 bf 41 00` → function at `0x0041bf40` =
**`TechnoClass__IsIronCurtainActive`**, inherited from TechnoClass.
No InfantryClass-specific override.

```c
undefined4 TechnoClass__IsIronCurtainActive(int param_1) {
    int dur = *(int *)(param_1 + 0x194);            // IronCurtain Duration
    if (*(int *)(param_1 + 0x18c) != -1) {          // IronCurtain StartFrame
        int elapsed = g_CurrentFrameCounter - *(int *)(param_1 + 0x18c);
        if (elapsed < dur) {
            dur = dur - elapsed;
            return (dur > 0);                        // immune while active
        }
        dur = 0;
    }
    return (dur > 0);
}
```

This is purely the Iron Curtain immunity gate. The "deployed GGI is
uncrushable" enforcement is **solely** the `(char)param_1[0xA9] == 0`
test in `CanCrushCheck @ 0x005f6cd0` Branch B (which checks
byte `+0x2A4` on the victim). Branch A only blocks crush against
iron-curtained units.

**§3.5 in this report previously hypothesized that `IsImmuneToCrush`
might check `+0x2A4` from inside Branch A's vtable call. That
hypothesis is wrong.** The crush gate path simplifies to:

```
TechnoClass::CanCrushCheck(victim, crusher):
    // Branch A: standard list-based crush, blocked by Iron Curtain
    if crusher exists && crusher.CanCrush && victim.CrushableListed==0:
        if victim not Building && !ally && !IsIronCurtainActive(victim):
            return 1   // crush

    // Branch B: AbstractType-side OmniCrusher + the deploy gate
    if victimAbsType[+0x22D] != 0 && victim active:
        if (byte)victim[+0x2A4] == 0:     // deploy/uncrushable byte
            if !ally && !IsIronCurtainActive(victim):
                return 1   // crush

    return 0   // no crush
```

So **deployed GGI is uncrushable specifically because Branch B's
`+0x2A4` check fails** — and Branch A is gated by `victimType[+0xD2A]`
(`Crushable=yes` flagging in some Crushable-list way). For GGI with
`Crushable=yes` AND deployed:
- Branch A: `Crushable=yes` likely satisfies `CrushableListed==0` (the
  list flagging is inverted), so Branch A would say "crush" → BUT
  Branch A's positive case still requires `victimType[+0xD2A]==0`. If
  that field IS Crushable, then `Crushable=yes` actually sets the
  list-flag to non-zero, blocking Branch A. **The exact semantics of
  `+0xD2A` need a separate verify** — this is a small remaining open
  item but doesn't block GGI parity since Branch B's `+0x2A4` gate
  alone is what gamemd visibly enforces.

### 9.3 DeploySound/UndeploySound — TechnoTypeClass `+0x56C`/`+0x570`

**Resolves §7.5.**

Parsed in **`TechnoTypeClass__ReadINI @ 0x00712170`** (NOT
`InfantryTypeClass__ReadINI` as Phase 1 alt-analysis claimed):

```c
// Near 0x00713550..0x007135c3
int prev = param_1[0x15b];               // existing voc index
if (CCINIClass__ReadString("DeploySound") == 0
    || (idx = VocClass__FindByName(buf), idx == -1)) {
    idx = prev;                          // preserve on absent or unknown
}
param_1[0x15b] = idx;                    // byte 0x15b * 4 = 0x56C

prev = param_1[0x15c];                   // UndeploySound
... same pattern ...
param_1[0x15c] = idx;                    // byte 0x15c * 4 = 0x570
```

The `param_1` here is `int *` so `param_1[N]` is byte offset `N×4`.
**DeploySound runtime slot: TechnoTypeClass+0x56C. UndeploySound:
TechnoTypeClass+0x570.** Both are single-slot — no separate
"parse intermediate vs runtime" distinction. The ReadString result is
resolved by `VocClass__FindByName` and stored directly; on lookup
failure the prior value is preserved.

Runtime read confirmed in `InfantryClass__Do_Action @ 0x0051d6f0`:
- Sequence 0x1B: `iVar2 = vtable+0x84()` (returns TechnoTypeClass*),
  reads `iVar2+0x56C`, skips `VocClass__PlayAt` if `== -1`.
- Sequence 0x1F: same with `+0x570`.

**The `+0xEA4`/`+0xEA8` offsets cited in Phase 1's alt-analysis are
`EnterWaterSound`/`LeaveWaterSound`** on InfantryTypeClass — unrelated
to deploy. Verified by reading the key-string pushes at `0x008259f0`
(`"EnterWaterSound"`) and `0x008259e0` (`"LeaveWaterSound"`).

**Rust-port note:** the deploy sound parser must live on the
TechnoTypeClass equivalent (the shared base where DeploySound is
inherited from), not on the infantry-specific type.

### 9.4 ROT scaling — `BulletTypeClass+0x2DC`, modulated by sidewinder

**Resolves §8.5 ROT scaling question.**

- **Parse site:** `BulletTypeClass__ReadINI @ 0x0046bee0`, key string
  `"ROT"` at `0x0081b164`, `CCINIClass__ReadInt`, stored to
  `BulletTypeClass+0x2DC` (raw int, no scaling at parse).
- **NOT on WeaponTypeClass.** Phase 3-A was correct.

**Per-tick BAM step formula** (in `BulletClass__AI @ 0x004666e0`, lines
`0x00466bd6`–`0x00466cb2`, before the call into `HomingTrack`):

```c
// "Sidewinder" oscillation — 15-frame cosine cycle
double sidewinder = cos(((bullet_id_like_value + frame) % 15) * 2π / 15) * MissileROTVar
                  + MissileROTVar + 1.0;
                  // stock YR rulesmd.ini has MissileROTVar=.25, so
                  // scalar oscillates in [1.0, 1.5]
int delta_far = Math__ftol(sidewinder * ROT_INI);

if (distance < 256 leptons) {
    delta = Math__ftol(delta_far * 1.5);
} else {
    delta = delta_far;
}

// Apply infantry-flag gate (rarely set), then:
uint8 delta_byte = delta & 0xFF;
uint16 ROT_BAM_per_tick = ((uint16)delta_byte) << 8;  // ← param_4 to HomingTrack
```

`RulesClass+0x598` = `MissileROTVar` (double, INI key
`MissileROTVar=` in `[General]`; stock YR `rules.ini`/`rulesmd.ini` set
`.25`. The class fallback when the key is missing is a separate constructor
default and must not be treated as the stock retail rules value).
`_DAT_007e48f8 = 1/15`, `_DAT_007e3cc0 = 2π`.

**For AAHeatSeeker2 ROT=60:**
- Sidewinder yields roughly 60..90 at normal range with stock
  `MissileROTVar=.25`.
- Inside 256 leptons, the binary multiplies that integer by 1.5 before the
  low-byte/shift step, yielding roughly 90..135 before signed 16-bit facing
  wrap effects.
- The final input to `HomingTrack` is `(delta & 0xFF) << 8`, interpreted by
  signed 16-bit facing helpers.

**For low-ROT projectiles (e.g., ROT=3):**
- Sidewinder yields 3..9, `<< 8` → `0x300..0x900 BAM/tick`
- In degrees: 4.2°..12.7° per tick — visible smooth curve with a slight
  oscillation ("sidewinder") visible to the player.

**Close range (distance < 256 leptons = 1 cell):**
- The sidewinder formula is still computed, then gamemd applies an additional
  `* 1.5` close-range multiplier before integer truncation and `<< 8` BAM
  conversion.

**The §8.1 ×256 ceiling assumption is verified, but the realistic
per-tick value uses stock `MissileROTVar=.25`, so the far-range scalar cycles
through roughly 1.0–1.5× that ceiling on a 15-frame cosine; close range raises
that to roughly 1.5–2.25× before wrap/clamp.** For high ROT (≥30 in INI) the modulation is invisible
(saturated snap); for low ROT (≤10) it is visually significant.

**Rust-port note:** the existing rocket flight code at
`src/sim/movement/rocket_movement.rs` needs to reproduce both the
sidewinder modulation and the close-range multiplier to match gamemd visually.
Single ×256 scaling would diverge for low-ROT projectiles.

### 9.5 `+0x5A4`/`+0x5C8` are on `SequenceClass`, not BulletType

**Resolves §7.4.**

Phase 1 found `bullet[+0x5A4]` and `bullet[+0x5C8]` reads in
`Fire_At_Target` and labeled them `ProneFire` and `Tunnel`. Phase 3-A
correctly observed BulletTypeClass struct ends near `+0x2F7`, ruling
out BulletType. Direct re-decompile of `Fire_At_Target @ 0x005206b0`
shows the pointer chain:

```c
// param_1 is TechnoClass*; param_1[1] reaches into the InfantryClass tail
InfantryTypeClass*  type = param_1[1].field_0x1a0;       // TypeData ptr
SequenceClass*      seq  = *(int*)(type + 0xE3C);        // Sequence= table ptr
int                 tunnelDefined   = *(int*)(seq + 0x5C8);
int                 proneFireDefined= *(int*)(seq + 0x5A4);
```

So `+0x5A4` and `+0x5C8` are **`SequenceClass` fields** — per-sequence
"this animation entry is defined" gates. Used in `Fire_At_Target` to
pick which fireUp anchor frame to use:

| weapon | GarrisonFlag | seq.+0x5C8 (Tunnel) | seq.+0x5A4 (ProneFire) | fireUp source |
|--------|--------------|---------------------|------------------------|---------------|
| 0 (Primary) | 0 | — | — | `type+0xE40` (FireUp) |
| 0 (Primary) | 1 | — | — | `type+0xE44` (FireProne) |
| 1 (Secondary) | 1 | non-zero | — | `type+0xE4C` (Tunnel — **TS-legacy**) |
| 1 (Secondary) | any | — | non-zero | `type+0xE48` (ProneFire) |

For GGI's deployed fire path (weapon=1, GarrisonFlag=0), the gate is
**`seq+0x5A4 != 0`** — i.e., is `ProneFire=` defined on
`GuardianGISequence`? Per Section 4.3 of this report, GGI's sequence
does have `FireProne=252,6,6`, so `seq+0x5A4 != 0`, and the fireUp
anchor for deployed fire becomes **`type+0xE48` (SecondaryFire frame)**
— which is the artmd-INI `SecondaryFire=` key (NOT `FireUp=`). For
GGI's art section the `SecondaryFire=` key is absent (per Section 4.2),
so it defaults to 0 → bullet spawn fires on frame 0 of DeployedFire.

**Wait — this re-frames the §3.3 fireUp finding!** §3.3 said deployed
GGI fires at `FireUp=2` (the art `[GGI] FireUp=` value). The §9.5
finding suggests the actual gate picks `SecondaryFire` (type+0xE48)
when `ProneFire` is defined in the sequence. **For GGI specifically,
this means the bullet spawn frame is whatever `SecondaryFire=` resolves
to in the artmd `[GGI]` block.** If absent, default 0.

Recheck: artmd `[GGI]` Section 4.2 lists `FireUp=2` but no
`SecondaryFire=`. So:
- `type+0xE40 (FireUp) = 2`
- `type+0xE48 (SecondaryFire) = 0` (default)
- Deployed GGI fires at frame 0 of DeployedFire sequence (sequence
  starts at SHP frame 323; bullet spawns on tick where
  `current_sequence_frame == 0`).

**This is a change from §3.3's earlier claim** that GGI fires on frame
2 of DeployedFire. The actual frame depends on the SecondaryFire art
value, which is absent for GGI → default 0 → fires on first frame of
DeployedFire animation.

`Tunnel` (TS-era subterranean APC sequence) is mostly dormant in YR —
the `seq+0x5C8` branch almost never triggers. The `SequenceClass` field
names are inferred from context (firing-frame neighbors `+0xE48
SecondaryFire` and `+0xE4C SecondaryProne`) and the corresponding
art-INI sequence keys; not verified against the SequenceClass parser
directly. Field-name confidence: MEDIUM.

**Open question carried forward:** if SecondaryFire is absent on GGI,
which frame does deployed-fire actually spawn the bullet on? Default 0
implies "first frame" but the strict `==` comparison in §3.3 plus the
randomize-start in `Do_Action` could place the firing tick anywhere in
the 6-frame DeployedFire sequence. **In-game test recommended** — pause
on missile spawn and check the SHP frame relative to DeployedFire start.

### 9.6 §7 / §8 final status

| Item | Status |
|------|--------|
| §7.1 ProneDamage application | **RESOLVED, with trap.** Dead data in YR — do NOT implement (§9.1). |
| §7.2 ftol rounding mode | RESOLVED (§8.3) — truncation toward zero. |
| §7.3 CanCrushCheck Branch A | **RESOLVED** (§9.2). vtable+0x160 is Iron Curtain, not deploy. Deploy gate is only Branch B's `+0x2A4`. |
| §7.4 `+0x5A4`/`+0x5C8` | **RESOLVED** (§9.5). SequenceClass fields — ProneFire/Tunnel sequence-defined flags. Changes the GGI deployed-fire frame finding in §3.3 (open follow-up: in-game verify spawn frame). |
| §7.5 DeploySound offset | **RESOLVED** (§9.3). `+0x56C`/`+0x570` on TechnoTypeClass. `+0xEA4`/`+0xEA8` are water sounds. |
| §7.6 IsSelectableCombatant | Open (deferred — non-parity-critical). |
| §7.7 Tiberium warhead | Open (TS-legacy, not GGI). |
| §7.8 DeployTime | RESOLVED (vehicle-only). |
| §7.9 E1 deployed-crushable | Open (needs in-game test). |
| §8.5 ROT scaling | **RESOLVED** (§9.4). ×256 with sidewinder modulation. |
| §8.5 BulletClass+0x12d AA aliasing | Open (non-blocking; the read at +0x2f6 from BulletType+? is unverified). |
| §8.5 ProneDamage in Detonate | **RESOLVED** (§9.1) — does not exist. |
| §8.5 Burst > 8 edge case | Open (not GGI-relevant; GGI Burst=1). |

### 9.7 Net Rust-port guidance summary

The full list of GGI gaps from §6 stands, with these critical updates:

1. **DO NOT implement ProneDamage as a damage multiplier.** Parse the
   INI key for round-trip fidelity, but ignore it during damage
   application. A naive implementation introduces a 30–50% damage drift
   on GGI shots vs prone infantry. (§9.1)
2. **The deploy-uncrushable mechanic is a SINGLE byte check** on the
   victim's `+0x2A4`. There's no inherited "IsImmuneToCrush" predicate
   to chain. (§9.2)
3. **DeploySound/UndeploySound live on the TechnoType base**, not on
   InfantryType — wire them at the shared layer. (§9.3)
4. **ROT requires the sidewinder modulation**: stock YR uses
   `MissileROTVar=.25`, so the far-range scalar oscillates between
   1.0× and 1.5× of `ROT_INI`. At close range (<256 leptons), the
   computed integer is multiplied by 1.5 before low-byte/`<< 8`
   conversion. Single ×256 scaling will diverge visibly for low-ROT
   projectiles. (§9.4; superseded by `AAHEATSEEKER2_HOMINGTRACK_EXACT_MATH_GHIDRA_REPORT.md`)
5. **The deployed-fire frame anchor is `SecondaryFire=` (art-INI)
   when `ProneFire=` is defined in the sequence** — not `FireUp=`. For
   GGI specifically, `SecondaryFire=` is absent → defaults to 0 → fires
   on first frame of DeployedFire. This corrects the §3.3 claim of
   "frame 2." Needs in-game verification. (§9.5)
