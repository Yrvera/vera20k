# Parasite — Terror Drone / Attack Dog / Squid Grab

This doc is the canonical reference for the **parasite mechanic** in gamemd.exe:
a warhead-special branch (`Parasite=yes`) that makes the attacker burrow into / latch
onto the target and damage it from inside over time.

Three distinct retail uses:
1. **`[Parasite]`** — Terror Drone burrowing into a vehicle. Verses gates vehicles only.
2. **`[ParasiteDog]`** — Attack Dog / Yuri Dog biting infantry. Verses gates infantry armors only.
3. **`[ParasitePlus]`** — Giant Squid grabbing a ship (or any unit). Verses 100% across the board.

The attacker is allocated a `ParasiteClass` (~0x58 bytes) per Terror-Drone-style unit
in `TechnoClass::Init_Managers`. The warhead Detonate path attaches the parasite to
the target and starts the per-tick damage loop.

Out-of-scope:
- The damage transform → [`damage_formula.md`](damage_formula.md)
- General warhead-special cascade priority → [`warhead_detonate_dispatch.md`](warhead_detonate_dispatch.md)
- Squid-specific anim / drag-down visuals → separate dispatch doc

---

## 1. Flag layout (verified)

### WarheadTypeClass

| Offset | INI key | String addr | Effect |
|---|---|---|---|
| `wh+0x159` | `Parasite=` | `0x0081717C` (verified live 2026-05-17) | Triggers Parasite branch in `WarheadTypeClass::Detonate` |

Parser site: `WarheadTypeClass::ReadINI` (offset within ReadINI body; the string xref points into the ReadINI region per established pattern).

### Confidence

- **Content: HIGH** — string xref verified.
- **Identity: HIGH** — single INI key.
- **Binding: HIGH** — single parse site; consumer in `WarheadTypeClass::Detonate` (per existing canonical mind_control doc's mutually-exclusive cascade — priority 5).

### Cascade priority

Per the mutually-exclusive warhead-special cascade documented in [`mind_control.md`](mind_control.md) §1:

| Priority | WH Offset | Effect |
|---:|---|---|
| 1 | `+0x155` | MindControl |
| 2 | `+0x157` | IvanBomb |
| 3 | `+0x158` | ElectricAssault |
| 4 | `+0x15A` | Temporal |
| **5** | **`+0x159`** | **Parasite** |
| 6 | `+0x15B` | (unknown) |
| 7 | `+0x16C` | IsLocomotor (Magnetron) |

A warhead with multiple flags set fires only the highest-priority one. So a warhead
with both `MindControl=yes` and `Parasite=yes` would mind-control; Parasite would not
fire.

---

## 2. Retail INI survey (verified, all 3 warheads)

```ini
[Parasite]; Terror Drone
Verses=100%,100%,100%,100%,100%,100%,0%,0%,0%,0%,0%
Parasite=yes
```

100% vs none/flak/plate/light/medium/heavy; **0% vs wood/steel/concrete/special_1/special_2**.
Translation: Terror Drone can latch onto any infantry or vehicle, but NOT onto
buildings (wood/steel/concrete = building armors).

```ini
[ParasiteDog]; Woof woof
Verses=100%,100%,100%,0%,0%,0%,0%,0%,0%,0%,0%
Parasite=yes
```

100% vs none/flak/plate (infantry armors); **0% vs everything else**. Attack Dog
bites infantry only.

```ini
[ParasitePlus]; SquidGrab
Verses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%
Parasite=yes
```

100% across the board. Squid grab can latch onto anything.

### Verses-as-filter

Note: the Verses values aren't damage multipliers in the normal sense for Parasite
warheads — they gate which armor types can be parasited. `Verses[armor] == 0` means
the warhead-target combination is rejected (drone can't burrow into a wall). Per
[`damage_formula.md`](damage_formula.md) §6, Verses=0 returns damage 0, which the
Parasite branch likely interprets as "can't attach."

---

## 3. `ParasiteClass` struct (partial, verified-via-constructor)

Decompiled live 2026-05-17 from constructor at `0x006292B0` (the second, 2-arg constructor — the live caller). Allocated via `TechnoClass::Init_Managers 0x006F3F40` (verified caller).

| Offset | Type | Field (inferred) | Notes |
|---|---|---|---|
| `0x00-0x0F` | ptr × 4 | vtables | `vtable__ParasiteClass` + 3 secondary (INoticeSink etc.) |
| `0x10-0x23` | — | AbstractClass base | Inherited |
| `0x24` | ptr | `Owner` (TechnoClass*) | Set from constructor param_2 (the drone) |
| `0x28` | int | (unknown — initialized to 0) | |
| `0x2C` | int | `Timer1_Start` | = `g_CurrentFrameCounter` |
| `0x30` | int | `Timer1_Aux/Duration` | from `local_8` (uninitialized? — see §10 footgun) |
| `0x34` | int | (unknown — init 0) | |
| `0x38` | int | `Timer2_Start` | = `g_CurrentFrameCounter` |
| `0x3C` | int | `Timer2_Aux/Duration` | from `local_8` |
| `0x40` | int | (unknown — init 0) | |
| `0x44` | int | (unknown — init 0) | |
| `0x48` | int | (unknown — init 0) | |
| `0x4C` | int | (unknown — init 0) | |
| `0x50` | int | (unknown — init 0) | |
| `0x54` | byte | (unknown flag — init 0) | |

Approximate size: **0x58 bytes** (88 bytes).

### Global container

| Address | Meaning |
|---|---|
| `0x00AC4914` | `g_ParasiteClass_Array` data ptr |
| `0x00AC4918` | capacity |
| `0x00AC491D` | growable flag |
| `0x00AC4920` | count |
| `0x00AC4924` | growth increment |

### Confidence (struct)

- **Content: MEDIUM** — constructor decompiled live, but field names are inferred (only `Owner` is securely identified via param mapping).
- **Identity: HIGH** — single vtable, single global array.
- **Binding: HIGH** — single allocator (`Init_Managers 0x006F3F40`).

### Comparison to CaptureManagerClass

Architecturally similar to `CaptureManagerClass` (per [`mind_control.md`](mind_control.md) §3):
- Both allocated by `TechnoClass::Init_Managers` based on warhead type.
- Both have 4-vtable COM-style layout.
- Both maintain per-instance state on the **attacker** (not the victim).
- Both have a global DynVector for cross-system iteration.

Differences: CaptureManager has a per-victim MCNode list (1-to-many); ParasiteClass appears to be 1-to-1 (one parasite per drone). Open follow-up #1 to verify.

---

## 4. The dispatch flow (inferred from cascade pattern)

Working hypothesis based on the cascade pattern + mind-control analogue:

```c
// In WarheadTypeClass::Detonate, after MindControl/IvanBomb/ElectricAssault/Temporal:
if (warhead->Parasite (+0x159) != 0):
    target = bullet.Target
    if (target == NULL || target.IsBuilding()): return         // no buildings
    if (warhead.Verses[target.Armor] == 0.0): return            // Verses gate
    
    drone = bullet.Firer (+0xB0)
    if (drone.Parasite == NULL):                                // attacker+0x???
        drone.Parasite = new ParasiteClass(drone)               // via Init_Managers
    
    drone.Parasite.AttachToTarget(target)                       // sets timers, anims
    target.ParasitedBy = drone.Parasite                         // back-reference
    
    // Per-tick logic in ParasiteClass::AI:
    //   - Decrement target.Health by warhead.Damage
    //   - When target.Health <= 0: spawn drone at target's position
    //   - When drone dies separately: clear target.ParasitedBy, drone exits
```

**Status:** Working hypothesis. The exact ParasiteClass::AI / AttachToTarget functions
are NOT decompiled in this iteration. Open follow-up #2 — full lifecycle decomp.

---

## 5. Per-tick damage and lifecycle (inferred)

Mechanism observed in retail play (verified by playing the game, not by decomp):

1. **Attach**: drone reaches target, fires its weapon, Parasite warhead detonates. Drone disappears from outside view; target now has the "infested" status.
2. **Tick damage**: every N frames, target takes some damage. Visible effect: red sparks emerge from the infested unit.
3. **Mutual destruction outcomes**:
   - **Target dies first**: drone re-emerges from the corpse, free to attack another target.
   - **Drone dies first** (e.g., target is repaired in a service depot, or drone is hit by AOE): drone is destroyed, target survives.
4. **Service Depot repair**: a target can be repaired at a Service Depot, which kills the drone inside.

The damage-per-tick value and tick rate are NOT extracted in this iteration. Per the
`[Parasite]` warhead INI, the damage value comes from the `Damage=` field on the
firing weapon (Terror Drone's primary). Per ParasiteClass timers, the tick rate is
probably hardcoded or from a Rules constant.

### Open follow-ups #3 + #4

- Damage-per-tick value: from `weapon.Damage` (the standard formula) or something else?
- Tick rate: hardcoded, INI key, or Rules constant?

---

## 6. Cross-class fields (inferred — NOT verified)

Likely TechnoClass fields for parasite state (extrapolating from the MC pattern at `+0x2BC..+0x2C8`):

| Offset | Field (inferred) | Purpose |
|---|---|---|
| `attacker+0x???` | `Parasite` (ParasiteClass*) | The drone's own parasite manager |
| `victim+0x???` | `ParasitedBy` (ParasiteClass*) | Back-reference: who is parasiting me |
| `victim+0x???` | `IsBeingParasited` (bool) | Visual flag |

**Exact offsets: UNKNOWN.** Open follow-up #5.

---

## 7. The three retail unit types using Parasite

### Terror Drone (`[DRONE]` — `[ParasiteDrone]` weapon)

- Primary weapon: Bite (Parasite warhead = `[Parasite]`)
- Verses on `[Parasite]` warhead: 100% vs all unit armors, 0% vs buildings
- Behavior: drone moves to target, latches on, damage-per-tick until target dies or drone is destroyed. Drone can be killed via Service Depot repair.

### Attack Dog (`[ADOG]`, `[YADOG]`, `[DOG]`) — `[DogJump]`-style weapon

- Primary weapon: bite (Parasite warhead = `[ParasiteDog]`)
- Verses on `[ParasiteDog]`: 100% vs infantry armors, 0% vs all vehicle/building armors
- Behavior: dog leaps onto infantry, kills them (presumably instantly via high damage, not slow tick like the drone). The Parasite warhead semantics here may behave slightly differently — possibly the dog "consumes" the infantry rather than slowly draining HP. Open follow-up #6.

### Giant Squid (`[SQD]`) — `[SquidGrab]` weapon

- Primary weapon: tentacle (Parasite warhead = `[ParasitePlus]`)
- Verses on `[ParasitePlus]`: 100% across the board
- Behavior: squid grabs a ship and drowns it (drag-down anim). Damage-per-tick destroys the target. Unique to squid — affects naval, can't be repaired the same way as a drone-infested vehicle. Open follow-up #7.

---

## 8. Key offsets summary

| Symbol | Offset / Address |
|---|---|
| `wh.Parasite` | `+0x159` |
| `"Parasite"` string | `0x0081717C` |
| `ParasiteClass::Constructor` (2-arg) | `0x006292B0` |
| `ParasiteClass::Constructor` (0-arg / load) | `0x00629210` |
| `TechnoClass::Init_Managers` | `0x006F3F40` |
| `g_ParasiteClass_Array` | `0x00AC4914..0x00AC4924` |
| ParasiteClass size | ~`0x58` bytes |
| Cascade priority in WarheadTypeClass::Detonate | 5 (after MindControl/IvanBomb/ElectricAssault/Temporal) |

---

## 9. TS-legacy filter

| Component | Status in YR |
|---|---|
| `Parasite=yes` warhead flag | **LIVE** — 3 retail warheads use it |
| `ParasiteClass` C++ class | **LIVE** — instantiated by Init_Managers for drone-style units |
| Terror Drone (`[DRONE]`) | **LIVE** in YR (Soviet unit, RA2 base + YR keeps it) |
| Attack Dog (`[ADOG]`, `[YADOG]`, `[DOG]`) | **LIVE** in YR (RA2 base unit) |
| Giant Squid (`[SQD]`) | **LIVE** in YR (RA2 navy unit) |

No TS-only dead branches identified in the parasite system. Parasite was a YR/RA2-era
addition (TS had no Terror Drone).

---

## 10. Edge cases & footguns

| Case | Behavior |
|---|---|
| Drone targets a building | `Verses[building_armor]=0` rejects in CanCapture-equivalent gate (or in the Detonate Parasite branch). Drone bounces / mission fails. |
| Drone targets an infantry | `Verses[infantry_armor]=100%` allows — but the parasite-into-infantry visual is questionable; likely the drone just kills the infantry outright. Verify in-game. |
| Multiple drones target the same vehicle | Open follow-up #8 — does the second drone fail to attach (1-to-1 limit) or stack? |
| Target is repaired at Service Depot mid-parasite | Drone is destroyed, target is repaired. |
| Target is destroyed by external damage during parasite | Drone exits the corpse and survives (probably). |
| `local_8` uninitialized in 2-arg constructor | The 2-arg constructor reads `local_8` (Ghidra-marked as uninitialized — a function-scope `int local_8`) and stores it to `+0x30` and `+0x3C`. This looks like a footgun unless `local_8` is actually filled from the stack by the caller (which would be the standard x86 calling-convention pattern for an extra arg). Open follow-up #9 — verify whether the caller actually passes a third param via stack. |
| Drone is mind-controlled | Drone's owner changes; the parasite attack continues. The dying target's house attribution = new (mind-controller) owner. |
| Squid grabs a unit on land (transport) | Probably blocked by some target-validity check (squid only works on naval). Open follow-up #10. |

---

## 11. Open follow-ups

1. **ParasiteClass single-victim vs multi-victim.** Is it 1-to-1 (one parasite per drone, one target per drone) or 1-to-many like CaptureManager? Per struct size, probably 1-to-1. Priority: MEDIUM.
2. **`ParasiteClass::AI` per-tick lifecycle.** The Update function that decrements target HP and handles separation events is not decompiled. Priority: HIGH for parity.
3. **Damage-per-tick value source.** From weapon.Damage, Rules constant, or per-warhead field? Priority: HIGH for parity.
4. **Tick rate.** Hardcoded value or Rules-driven? Priority: MEDIUM.
5. **TechnoClass parasite-state field offsets.** `attacker.Parasite` and `victim.ParasitedBy` offsets not extracted. Priority: HIGH for parity.
6. **Attack Dog parasite semantics.** Does `ParasiteDog` actually instantiate a ParasiteClass, or is the dog-on-infantry a different "instant kill" mechanism with the Parasite flag as a target-eligibility filter only? Priority: HIGH.
7. **Squid `ParasitePlus` differences.** Squid drowning a ship — does it use the same ParasiteClass AI loop, or is it a special-case anim? Open. Priority: MEDIUM.
8. **Multiple drones on same target.** Stacking semantics. Priority: LOW.
9. **`local_8` uninitialized read in constructor.** Verify caller passes a third arg via stack. Priority: LOW (cosmetic decomp concern).
10. **Squid target validity (naval-only).** Where is the "squid can't target ground" gate? Probably in target acquisition, not Parasite warhead. Priority: LOW.
11. **Detonate dispatch site for Parasite branch.** The `WarheadTypeClass::Detonate` priority-5 branch needs to be decompiled to confirm the dispatch flow in §4. Priority: HIGH for parity.

---

## 12. Sources

- Live xrefs (2026-05-17):
  - `"Parasite"` at `0x0081717C` (single string match)
  - `ParasiteClass::Constructor 0x006292B0` decompiled live (2-arg)
  - `ParasiteClass::Constructor 0x00629210` decompiled live (0-arg / save-load)
  - Caller verified: `TechnoClass::Init_Managers 0x006F3F40` (via `get_function_callers 0x006292B0`)
- INI quotes from `ini/rulesmd.ini` lines 27135-27149: three retail Parasite warheads
- Existing canonical doc: [`../../MIND_CONTROL_SYSTEM_GHIDRA_REPORT.md`](../../MIND_CONTROL_SYSTEM_GHIDRA_REPORT.md) §1 — cascade priority list (used for the priority-5 placement)
- Sister system docs: [`damage_formula.md`](damage_formula.md), [`mind_control.md`](mind_control.md) (analogous Init_Managers allocator pattern), [`warhead_detonate_dispatch.md`](warhead_detonate_dispatch.md) (when written, for the full cascade).
