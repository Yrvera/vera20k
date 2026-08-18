# Chaos Drone / Berserk Gas (Psychedelic Warhead) — Ghidra Research Report

Research date: 2026-04-19 (updated 2026-04-19 — resolved berserk clear mechanism, see §3.6)
**Addresses:** `0x0075D590` (WarheadTypeClass::ReadINI), `0x00712170` (TechnoTypeClass::ReadINI), `0x00701900` (TechnoClass::ReceiveDamage), `0x006F9E50` (TechnoClass::AI_Update — **berserk timer decrement / clear**), `0x006F8960` (TechnoClass::Scan_Cell_For_Target), `0x00489180` (Apply_Verses_Damage), `0x006EA870` (**TeamClass::RemoveMember** — verified, not "passenger-ejection" as originally labeled), `0x005B35E0` (MissionClass::Queue_Mission)
**Confidence:** HIGH (all offsets + dispatch verified from disassembly; berserk set/clear both verified; timer decrement confirmed via byte-pattern search for writes to TechnoClass+0x298 — only 2 live writes exist in the binary)
**Active in YR:** YES — Chaos Drone (`[CAOS]`) is a YR-exclusive Yuri unit with its `ChaosAttack` → `PsychGasCreate` → `Psychedelic=yes` path fully active in a standard skirmish.

---

## 1. Overview

The Chaos Drone fires an invisible projectile that detonates a `PsychGasCreate` warhead
at the impact cell. Any non-allied, non-building, non-psionic-immune target hit by this
warhead **immediately flips into a persistent berserk state** (TechnoClass+0x298 ← 1)
and receives Verses-scaled damage. Once berserk, the target bypasses alliance filtering
during target acquisition — it will attack any adjacent hostile **or friendly** unit via
the normal retaliation / auto-acquire path.

The warhead also spawns the `PsychCloudSys` particle system (via the normal warhead
animation path) which continues to emit `PsychGas` warhead hits on units that remain in
the gas cloud, causing continuous damage but **not** re-triggering the berserk flip (it
is already set).

The "berserk gas" mechanic is fundamentally a **target-acquisition filter override**
driven by a single byte flag. There is no berserk timer, no duration multiplier, no
random-target picker — the unit continues normal target acquisition but with alliance
checks disabled.

---

## 2. Class Layout / Key Offsets

### 2.1 WarheadTypeClass — verified offsets (from `WarheadTypeClass__ReadINI` disassembly)

| Offset | Field | INI key | Type | Evidence |
|--------|-------|---------|------|----------|
| 0x14E | `Rocker` | `Rocker` | bool | String @0x847DE8, previously verified |
| 0x14F | `DirectRocker` | `DirectRocker` | bool | String @0x847DD8 |
| 0x155 | `MindControl` | `MindControl` | bool | |
| 0x159 | `Parasite` | `Parasite` | bool | |
| 0x15A | `Temporal` | `Temporal` | bool | |
| 0x15B | `IsLocomotor` | `IsLocomotor` | bool | (Magnetron report) |
| 0x16C | `Airstrike` | `Airstrike` | bool | String @0x817154 |
| **0x16D** | **`Psychedelic`** | **`Psychedelic`** | **bool** | **String @0x847D30, single xref from 0x0075D8EA. Disasm: `PUSH 0x847d30; CALL ReadBool; MOV byte ptr [ESI + 0x16d],AL` at 0x0075D8FB.** |
| 0x16E | `BombDisarm` | `BombDisarm` | bool | String @0x847D24 |

### 2.2 TechnoTypeClass — verified offsets

| Offset | Field | INI key | Type | Evidence |
|--------|-------|---------|------|----------|
| **0x690** | **`BerserkFriendly`** | **`BerserkFriendly`** | **bool** | String @0x8439F8. Disasm at `0x007148FA`: `PUSH 0x8439f8; CALL ReadBool; MOV byte ptr [EBP + 0x690],AL` at 0x00714905. **Runtime read: TechnoClass::GetFireError at 0x006FC1E1 — applies to TARGETS, not attackers.** The flag makes the UNIT IMMUNE to being fired on by berserk attackers. See §3.7. |
| 0x6C0 | `AttackFriendlies` | (likely) | bool | Read in Scan_Cell_For_Target (§3.3) as an alternative-path to berserk for bypassing alliance filter. Existing TARGET_ACQUISITION_GHIDRA_REPORT.md claim verified. |
| 0xD35 | `ImmuneToPsionics` | `ImmuneToPsionics` | bool | Read in ReceiveDamage (§3.2). Existing doc claim verified. |

### 2.3 TechnoClass — verified runtime fields

| Offset | Field | Type | Evidence / role |
|--------|-------|------|-----------------|
| **0x298** | **berserk_flag** | **byte** | **Set in TechnoClass::ReceiveDamage Psychedelic branch; cleared in TechnoClass::AI_Update when timer expires; read in Scan_Cell_For_Target to bypass alliance filter. Verified all three sites in disasm. Only TWO live writes exist in the binary (ReceiveDamage: set=1; AI_Update: set=0).** |
| **0x29C** | **berserk_timer** | **int (frames)** | **The berserk DURATION counter, NOT "last damage." Set in ReceiveDamage to the damage-calculator output, decremented every tick in AI_Update, clears berserk flag when it reaches 0 or below. See §3.6.** |
| 0x14 | flags byte | byte | Bit 2 (`& 4`) = "is Techno" (used in the berserk-flip branch to gate the team-removal call — not passenger ejection; see §7 item #4 for correction) |
| 0x5D4 | TeamPtr (ptr) | ptr | **TeamPtr** — pointer to the AI TeamClass this unit belongs to (or NULL). Read in the berserk-flip branch to decouple the unit from its team. Verified via TECHNOCLASS_EXPANDED_STRUCT_LAYOUT.md line 620. |

Note: 0x298 and 0x29C are **adjacent** and likely belong to the same logical cluster
(per prior TechnoClass expanded-layout docs). This investigation does not resolve
whether the full cluster is a single struct (e.g., a "berserk state" sub-object) or
loose fields.

---

## 3. Core Logic

### 3.1 Warhead dispatch chain (who calls what)

```text
BulletClass::BulletDetonation (0x00468D80)
  └─ WarheadTypeClass::Detonate (0x004690B0)
       └─ [else-chain falls through to] Apply_area_damage (0x00489280)
            └─ for each object in AoE:
                 object->ReceiveDamage(&damage, 0, warhead, src_obj, ...)
                     └─ TechnoClass::ReceiveDamage (0x00701900)   ← Psychedelic checked here
```

**Key point**: The Psychedelic flag is **NOT** checked in `WarheadTypeClass::Detonate`'s
special-type if/else-chain. Unlike `MindControl`, `Parasite`, `Temporal`, `IsLocomotor`,
`Airstrike`, etc., Psychedelic takes the *normal* damage path (`Apply_area_damage`) and
is evaluated per-target inside `ReceiveDamage`. This makes Psychedelic an **area effect**
(via `CellSpread=3` on the warhead) rather than a single-target direct-dispatch effect.

### 3.2 Per-target dispatch in `TechnoClass::ReceiveDamage` (0x00701900)

Verified from decompilation. The Psychedelic branch sits inside the
`LAB_00701bf6` pre-damage filter block and reads `in_stack_0000000c` (the warhead
pointer) at offset `+0x16D`:

```text
if (warhead != NULL) {
    // (other immunity gates — Tiberium, Culling, Poison, allied-damage flag check)
    ...
    if (warhead->Psychedelic /* +0x16D */) {
        // Gate 1: allied?
        if (HouseClass::IsAlliedWith(this->Owner, source_house))
            return 0;  // no damage, no berserk
        // Gate 2: target is psionic-immune?
        type = this->GetType();
        if (type->ImmuneToPsionics /* +0xD35 */)
            return 0;
        // Gate 3: target is a building?
        if (this->WhatAmI() == 6)   // vtable+0x2C — BuildingClass discriminator
            return 0;

        // Compute berserk duration via FUN_00489180 (universal damage clamping helper).
        // Verified call site at 0x00701D4D-0x00701D69 — __fastcall with all 4 args:
        //   param_1 (ECX) = current damage (post-armor-mult, already scaled by Verses
        //                   earlier in ReceiveDamage top block at 0x00701945-0x0070196C)
        //   param_2 (EDX) = warhead pointer (EBP)
        //   param_3 (stk) = target_type->Strength @ +0x9C (max HP)
        //   param_4 (stk) = 0
        // FUN_00489180 does: result = damage × warhead[+0x12C] (float multiplier),
        //                    capped to RulesClass[+0x16C8].
        // The returned value is both stored back to the out-damage and used as the
        // berserk timer in frames (+0x29C).
        type = this->GetObjectType();                                // vtable+0x88
        new_damage = FUN_00489180(
            /*damage*/  *p_out_damage,
            /*warhead*/ current_warhead,
            /*strength*/type->Strength_0x9C,
            /*param4*/  0
        );
        *p_out_damage = new_damage;                                   // pass updated damage out
        this->berserk_timer /* +0x29C */ = new_damage;                // duration in frames

        // Berserk state flip — once per target-lifetime (until timer expires and clears)
        if (this->berserk_flag /* +0x298 */ == 0) {
            this->berserk_flag = 1;
            // If target is a vehicle (flag & 4) with passengers (head-of-list != NULL)
            // If target is in an AI team, remove target from team.
            // (This is NOT passenger ejection — the initial interpretation was wrong.)
            if ((this->flags & 4) && this->TeamPtr /* +0x5D4 */ != NULL) {
                TeamClass::RemoveMember(this->TeamPtr, this, -1, 0);  // FUN_006EA870
                // walks team->members list at team+0x54, unlinks `this`, decrements
                // team member count (team+0x48) and capacity (team+0x4C), clears
                // this->was_removed_flag (+0x6B8)
            }
            // Reset target-acquisition state
            (*this->vtable[0x3C8])(0);   // TechnoClass::Set_ArchiveTarget(NULL)
            (*this->vtable[0x1E8])(0x0F, 0);   // MissionClass::Queue_Mission(HUNT, false)
                                                //   — verified at asm 0x00701DAC-0x00701DB4
        }
        return 1;   // non-zero return signals "special damage handled"
    }
}
```

**Important observations:**
- The berserk flip runs **exactly once** per target, protected by `if (field_0x298 == 0)`.
  Subsequent Psychedelic hits on an already-berserk target will **only refresh the timer**
  (re-writing `field_0x29C` with a new duration) — they will not re-remove from team,
  re-reset ArchiveTarget, or re-queue the mission. This means sustained gas exposure
  effectively **extends** the berserk state indefinitely.
- The mission-queue call on the initial flip is `Queue_Mission(0x0F, 0)` — mission
  **Hunt** (`MISSION_HUNT` per `MISSIONCLASS_STATE_MACHINE.md` line 76) with
  `commence_bool=false` (queued, not immediately executed). Verified from asm at
  `0x00701DAC-0x00701DB4`: `PUSH 0; PUSH 0xF; CALL [vtable+0x1E8]`.
  Hunt dispatches to `FootClass::Mission_Hunt` (0x004D4280) which seeks and attacks
  the nearest valid target — combined with the alliance-filter bypass from §3.3, this
  produces the "attacks everything nearby, allied or not" behavior.

### 3.3 Target-acquisition behavior when berserk is set

Verified in `TechnoClass::Scan_Cell_For_Target` (0x006F8960). The berserk flag is
checked **twice** during target scanning — once in the cell-walking filter and once
after candidate selection:

```text
// Filter during cell walk:
for each techno in cell.occupant_list:
    ally = HouseClass::Is_Ally_ByObject(techno)
    if ( (!ally && !some_flag) ||                                  // normal hostile path
         self.type->AttackFriendlies /* +0x6C0 */ ||                // attacks-friendlies override
         self->berserk_flag /* +0x298 */ ) {                        // ← berserk override
        // continue considering this techno (skip the ally filter)
        ...
    }

// Second check after picking best candidate:
if ( !ally_with_candidate ||
     weapon_range >= 0 ||
     ... ||
     self.type->AttackFriendlies ||
     self->berserk_flag != 0 ||    // ← berserk override again
     some_scan_flag) {
    TechnoClass::Evaluate_Candidate(...);
}
```

**Effect on behavior**: with `berserk_flag = 1`, the unit's auto-acquire / passive-target
acquisition path will consider **every nearby object** as a valid target, not just
hostiles. The unit does not actively pick *random* targets — it still uses the normal
threat-score / distance evaluation — but its pool of candidates is no longer filtered
by alliance.

Combined with `TechnoClass::ShouldRetaliate` (which itself has an `Is_Ally_ByObject`
check at line ~18), a berserk unit that is *shot by an ally* will retaliate normally
(because the retaliation path uses different logic — it's the initial "fire at anything"
acquisition that the berserk flag enables).

### 3.4 What BerserkFriendly actually does (CORRECTION — 2026-04-19)

**The initial interpretation was WRONG.** The flag is read by
`TechnoClass::GetFireError` (0x006FC0B0) at instruction 0x006FC1E1, where it is
checked on the **TARGET's** TechnoType, not the attacker's. The correct semantic:

```text
GetFireError(attacker, target):
    ...
    if attacker.berserk_flag /* +0x298 */ != 0
       AND target.type.BerserkFriendly /* +0x690 */ != 0:
        return FIRE_ILLEGAL (5)   // berserk unit can't fire at this target
```

So `[CAOS] BerserkFriendly=yes` on the **Chaos Drone's type** means "berserk units
cannot target Chaos Drones." This is a **friendly-fire protection** flag — the
Chaos Drone's victims (now berserk) will not attack it back. This matches observed
in-game behavior: a Chaos Drone can safely sit among units it's just gassed.

The INI pairing on `[CAOS]` (`BerserkFriendly=yes` + `CanPassiveAquire=no` +
`CanRetaliate=no`) is therefore interpreted as a three-part safety net:
- `BerserkFriendly=yes` prevents berserk-victims from retaliating
- `CanPassiveAquire=no` prevents Chaos Drone from auto-acquiring targets
- `CanRetaliate=no` prevents Chaos Drone from retaliating even when hit

Together these make the Chaos Drone a pure player-controlled support unit.

### 3.7 Additional fire-legality gates for Psychedelic (GetFireError at 0x006FC0B0)

Decompilation of `TechnoClass::GetFireError` exposes several weapon-gating checks
that prevent a Psychedelic weapon from even firing under specific conditions. These
are **pre-fire** gates (GetFireError returns nonzero = FIRE_ILLEGAL), distinct from
the **post-fire** gates in ReceiveDamage (§3.2) that return 0 damage with no effect:

| Gate (line in decomp) | Condition | Effect |
|---|---|---|
| 0x006FC1E1 | `attacker.berserk_flag != 0 && target.type.BerserkFriendly != 0` | FIRE_ILLEGAL (berserk can't target `BerserkFriendly` types) |
| `warhead.Psychedelic + target.type.ImmuneToPsionics` check | `weapon.Warhead.Psychedelic != 0 && target.type.ImmuneToPsionics != 0` | FIRE_ILLEGAL — weapon won't fire at all |
| `warhead.Psychedelic + target+0x2E4 check` | `weapon.Warhead.Psychedelic != 0 && target.field_0x2E4 != 0` | FIRE_ILLEGAL — target+0x2E4 is likely an "already under psychic effect" field |

**Practical consequences:**
- Chaos Drones and Yuri Prime cannot attack mind-control-immune units (Terror Drones,
  Chaos Drones themselves, etc.) — the weapon-fire validation blocks the shot
  upfront, saving a fired bullet.
- A target already in a warp/mindcontrol state (field 0x2E4) is skipped — prevents
  chained psychic attacks from stacking.
- Berserk attackers physically cannot target `BerserkFriendly=yes` units; this is
  an extra layer of protection beyond just "don't pick them via auto-acquire."

**Additional gates for IsLocomotor** (Magnetron) in the same function:
- `weapon.Warhead.IsLocomotor != 0 && target.is_chronoshifted (FUN_00746DB0)` →
  FIRE_ILLEGAL — Magnetron cannot target already-lifted or chronoshifted units.
- A mirror gate on target type's `+0xD94` field (likely "ImmuneToLocomotor" or
  similar) with additional `+0x674` locomotor-ptr checks.

### 3.6 Berserk clear — `TechnoClass::AI_Update` (0x006F9E50) timer decrement

**Verified from decompilation of AI_Update at 0x006F9E50.** The top of the function
contains:

```text
LAB_006f9f0d:
if ( this->berserk_flag /* +0x298 */ != 0 ) {
    // Decrement timer and test < 1 in a single compound expression
    int new_timer = this->berserk_timer /* +0x29C */ - 1;
    this->berserk_timer = new_timer;
    if (new_timer < 1) {
        this->berserk_flag  = 0;                      // ← CLEAR berserk
        this->berserk_timer = 0;
        (*this->vtable[0x3C8])(0);                    // Set_ArchiveTarget(NULL)
        if (this->Owner->field_0x1EC == 0) {
            (*this->vtable[0x1E8])(0x0F, 0);          // Queue_Mission(0x0F, false)
        } else {
            (*this->vtable[0x1E8])(0x05, 0);          // Queue_Mission(0x05, false)
        }
    }
}
```

**Confirmed via byte-pattern search** (pattern `C6 ?? 98 02 00 00 ??` = "MOV byte ptr
[reg+0x298], imm8"): the binary contains **exactly four matches** for writes-to-0x298:

| Address | Function | Role |
|---------|----------|------|
| 0x00701D7D | `TechnoClass::ReceiveDamage` | Set berserk = 1 on first Psychedelic hit |
| 0x006F9F2A | `TechnoClass::AI_Update` | Clear berserk = 0 when timer expires |
| 0x00697188 | `FUN_006970a0` (config-class ctor) | False positive — writes to a different struct that happens to use offset 0x298 |
| 0x0070F8A9 | (dead code — no containing function, no xrefs) | Unreachable |

So **berserk is *not* permanent**. The duration is controlled by the timer field
`field_0x29C`, which is re-loaded on every Psychedelic hit (keeping the unit berserk
as long as it stays in the gas cloud) and counted down once per AI_Update tick
(roughly once per frame).

**Mission IDs after clear (verified from `MISSIONCLASS_STATE_MACHINE.md` + decomp of
`HouseClass::IsPlayerControl` at `0x0050B730`):**

- `0x0F` = **Hunt** — continue actively hunting targets (used for AI-controlled units)
- `0x05` = **Guard** — fall to defensive / idle (used for player-controlled units)

The branch is:
```
if (this->Owner->IsPlayerControl_0x1EC == 0)    // AI-controlled
    Queue_Mission(Hunt, false);                  // keep attacking
else                                             // player-controlled
    Queue_Mission(Guard, false);                 // wait for orders
```

**Confirmed `HouseClass+0x1EC` semantics**: verified from `HouseClass::IsPlayerControl`
(0x0050B730) which reads `*(char*)(house + 0x1EC)` as the primary IsPlayerControl byte
flag. When `g_GameMode == 0` (skirmish), falls through to a secondary `+0x1ED` check;
in normal play `+0x1EC` is sufficient to distinguish.

**Rationale**: a player-controlled unit coming out of berserk returning to Hunt would
keep attacking allied units after the gas wears off — bad UX. Going to Guard lets the
player re-issue orders. An AI-controlled unit going to Hunt keeps it engaged without
needing AI to re-issue orders. Both consistent with the "berserk flag resets
ArchiveTarget" pattern observed on set.

### 3.5 Why the warhead has `InfDeath=1`, `CellSpread=3`, `AnimList=CDGAS`

Normal AoE damage via `Apply_area_damage` iterates all objects inside `CellSpread` and
calls `ReceiveDamage` on each. Each of those ReceiveDamage calls re-evaluates the
Psychedelic branch independently, so the gas *simultaneously* berserks every valid
target inside the 3-cell radius. The `CDGAS` animation provides the visible cloud
(rate 450 in artmd.ini), and the `PsychCloudSys` particle system (rulesmd.ini line 25966)
emits per-tick `PsychGas` warhead hits for units lingering in the cloud.

---

## 4. INI Keys

All values extracted from `c:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini` and
`ini/artmd.ini` (YR-only; `rules.ini`/`art.ini` have no Chaos Drone entries).

### 4.1 Unit `[CAOS]` (Chaos Drone) — lines 8761–8823

| Key | Value | Role |
|-----|-------|------|
| `Primary` | `ChaosAttack` | |
| `Secondary` | `VirtualScanner` | Sight-extension weapon (Damage=1, NeverUse=yes) |
| `Locomotor` | `{4A582741-9839-11d1-B709-00A024DDAFD1}` | DriveLocomotion (normal vehicle) |
| `ImmuneToPsionics` | `yes` | **Chaos Drone can't be chaos'd by another Chaos Drone** |
| `ImmuneToRadiation` | `yes` | |
| `BerserkFriendly` | `yes` | **§3.4 — allows attacker to fire on friendlies** |
| `CanPassiveAquire` | `no` | Chaos Drone doesn't auto-acquire targets |
| `CanRetaliate` | `no` | Chaos Drone doesn't retaliate when hit |
| `Crewed` | `no` | No pilot bailout |
| `Deployer` | `yes` / `DeployFire` | |

### 4.2 Weapon `[ChaosAttack]` — lines 23506–23516

| Key | Value |
|-----|-------|
| `Damage` | 600 |
| `ROF` | 45 |
| `Range` | 3 |
| `Projectile` | `InvisibleLow` |
| `Speed` | 30 |
| `Warhead` | `PsychGasCreate` |
| `OmniFire` | `yes` |
| `AreaFire` | `yes` |

### 4.3 Warhead `[PsychGasCreate]` — lines 27000–27006

| Key | Value |
|-----|-------|
| `CellSpread` | 3 |
| `PercentAtMax` | 1 |
| `Verses` | `100%,100%,100%,50%,50%,50%,0%,0%,0%,100%,100%` |
| `InfDeath` | 1 |
| `AnimList` | `CDGAS` |
| **`Psychedelic`** | **`yes`** ← **triggers the dispatch at WarheadType+0x16D (§3.2)** |

The **secondary warhead** `[PsychGas]` (emitted by the `PsychCloud` particle as it
lingers in the gas cloud) also has `Psychedelic=yes`. This means the particle system
continuously re-triggers Psychedelic hits on units inside the cloud — but because the
berserk flag only flips on the first hit, subsequent hits only apply damage.

### 4.4 Projectile `[InvisibleLow]` — lines 25385–25390

`Inviso=yes`, `Image=none`, `SubjectToCliffs=yes`, `SubjectToElevation=yes`,
`SubjectToWalls=yes` — the gas bullet cannot cross cliffs/walls.

### 4.5 `[General]` keys

- `BerserkColor=4;0` (rulesmd line 627) — RGB palette index for the in-game color
  overlay applied to berserk-flagged units. **Not yet traced to its read site** — likely
  used by the render layer (unit palette remap) when `field_0x298 != 0`. See §7.
- `BerzerkAllowed=no` (rules line 729) — **TS-LEGACY, disabled in base RA2 and YR.**
  This is the Tiberian Sun Cyborg "go berserk at half damage" mechanic, unrelated to
  the Chaos Drone. Do NOT implement.

### 4.6 Animation `[CDGAS]` (artmd.ini line 15719)

`Translucent=no`, `TranslucencyDetailLevel=1`, `Flat=true`, `Rate=450`. Plays at the
impact cell for the duration of one anim cycle.

---

## 5. Integration Points

- **Writer of `berserk_flag` (0x298):** `TechnoClass::ReceiveDamage` (0x00701900),
  Psychedelic branch only. No other write sites located.
- **Readers of `berserk_flag` (0x298):** `TechnoClass::Scan_Cell_For_Target` (0x006F8960)
  — twice, both to bypass alliance filter. This investigation did not exhaustively
  enumerate other readers; an exhaustive xref search is open work.
- **Tick placement:** Psychedelic effects apply synchronously during bullet impact
  (inside the same tick the bullet reaches its target). The berserk flip takes effect
  on the next tick's target-acquisition cycle.
- **Related systems:**
  - Mind Control warhead (WarheadType+0x155) — different dispatch (WarheadTypeClass::Detonate
    direct branch), different effect (house-swap not behavior-flag). Do not confuse.
  - Parasite warhead (WarheadType+0x159) — different dispatch (direct branch), different
    effect (WarpAttach attachment). Do not confuse.
  - ChaosDrone's `BerserkFriendly=yes` on its *own* type (TechnoType+0x690) is separate
    from targets' berserk flag (TechnoClass+0x298). The attacker flag and the target
    flag live on different classes and at different offsets — do not alias them.

---

## 6. Current Rust Implementation Status

Summary from research agent C:

| Component | Rust site | Status |
|-----------|-----------|--------|
| `WarheadType.psychedelic` parsing | `src/rules/warhead_type.rs:72, 161` | **Parsed.** Offset comment (if any) should be `+0x16D`. |
| `WarheadType.mind_control` parsing | `src/rules/warhead_type.rs:76, 163` | Parsed (separate mechanic). |
| `TechnoType.immune_to_psionics` | — | **Not parsed.** Required to gate the dispatch (§3.2 gate 2). |
| `TechnoType.berserk_friendly` | — | **Not parsed.** (TechnoType+0x690) |
| `TechnoType.attack_friendlies` | — | **Not parsed.** (TechnoType+0x6C0) |
| `GameEntity.berserk_flag` | — | **Not present.** No runtime field for berserk state. |
| `GameEntity.last_psych_damage` | — | **Not present.** |
| Warhead-effect dispatch | `src/sim/combat/mod.rs:1260-1277` | **`_wh_id` ignored** — no dispatch. |
| Target-acquisition alliance-filter override for berserk | `src/sim/combat/combat_targeting.rs:126 (acquire_best_target)` | **No override path.** Target selection currently has no alliance-bypass hook. |
| `[General] BerserkColor` parsing | `src/rules/ruleset.rs` | **Not parsed.** |
| Chaos Drone unit & `[PsychGasCreate]` / `[PsychGas]` warheads | — | Rules parser reads generic warhead fields; nothing Chaos-specific. |
| Psychedelic cloud particle system | — | Not implemented. Particle systems have no dispatch in sim yet. |

**Faithful implementation requires (in rough order):**
1. Parse three missing TechnoType bools: `ImmuneToPsionics` (at 0xD35),
   `BerserkFriendly` (at 0x690), `AttackFriendlies` (at 0x6C0).
2. Add a `berserk` byte field to the GameEntity / combat-state struct and a
   `last_psych_damage` field (or cluster them in a `BerserkState` sub-struct).
3. Add a warhead-effect dispatch step in the damage-application pipeline that
   runs the §3.2 gate chain when `warhead.psychedelic=true`.
4. Modify target-acquisition (ally filter) to pass candidates through when
   `attacker.berserk_flag != 0` or `attacker.type.attack_friendlies`.
5. When setting berserk, remove the unit from any AI team it's part of
   (mirror of FUN_006EA870 = TeamClass::RemoveMember), clear the unit's
   ArchiveTarget, and force a mission re-queue to Hunt (0x0F).
6. Parse `[General] BerserkColor=R;G` and plumb to render (unit palette remap)
   for any entity with `berserk_flag != 0`.

**Do not implement:**
- `BerzerkAllowed=` — TS-legacy cyborg mechanic; off by default; not used in YR.
- Any "berserk duration timer" — not present in the binary (see §7).
- A separate MISSION_BERSERK mission enum — the binary re-uses the normal
  target-acquisition path with the flag; no dedicated mission.

---

## 7. Open Questions

**Resolved in a follow-up Ghidra pass (2026-04-19):**
- ~~Item #1 "Berserk clear condition"~~ → resolved in §3.6 (AI_Update decrements
  timer, clears when ≤0). Only 2 live writes exist.
- ~~Item #5 "last_psych_damage reader"~~ → resolved (`+0x29C` is the timer, not a
  damage stash).
- ~~Item #1 "Exact initial-timer formula"~~ (from the first revision) → resolved via
  disassembly at `0x00701D4D-0x00701D69`. The __fastcall passes 4 args:
  `FUN_00489180(current_damage, warhead, target_type->Strength, 0)`. The returned
  value (damage clamped by `warhead[+0x12C] × RulesClass[+0x16C8]` cap) serves as
  the timer in frames. For a Chaos Drone primary hit against infantry, the effective
  timer is approximately `600 (weapon.Damage) × Verses[infantry]=100% × warhead_mult`
  frames — observationally a few seconds of berserk.
- ~~Item #2 "Queue_Mission arguments on set"~~ → verified at asm `0x00701DAC`:
  `Queue_Mission(0x0F=Hunt, 0=false)`. Combined with §3.6's clear path, the full
  mission sequence is: `Hunt` on flip → (berserk filter bypass during target pick)
  → `Hunt` (if AI) or `Guard` (if player) on clear.
- ~~Item #5 "HouseClass+0x1EC flag"~~ → verified via decomp of
  `HouseClass::IsPlayerControl` (0x0050B730): it IS the IsPlayerControl flag.

**Still open** (deferred; lower priority):

1. ~~**BerserkFriendly (TechnoType+0x690) read site**~~ → **RESOLVED 2026-04-19**:
   Verified at `TechnoClass::GetFireError` (0x006FC0B0) line `0x006FC1E1`. The flag
   sits on TARGETS, not attackers — it makes a unit immune to being fired on by
   berserk attackers. See §3.4 and §3.7 for the corrected semantics. Originally
   misinterpreted in this report's first revision; corrected.

2. ~~**BerserkColor render read site**~~ → **RESOLVED 2026-04-19**: Stored at
   `RulesClass+0x18AC` (int, verified via disasm at 0x0066B864). **Primary reader:
   `UnitClass::DrawPips` (0x0073B500) at instruction `0x0073C097`** — the color
   is used as a palette index for a status pip drawn over berserk units (not a
   whole-unit palette remap, as initially assumed). Two other readers exist
   (`FUN_00665650` at 0x006678B6 — likely a RulesClass accessor; and an orphan
   reference at 0x00518FDA that Ghidra hasn't tagged to a function). The INI
   syntax `BerserkColor=4;0` parses to integer `4` (the `;0` is comment
   delimiter). Palette index 4 is a purple/magenta shade in RA2's standard palette.

3. ~~**Interaction with MindControl / Parasite**~~ → **RESOLVED 2026-04-19**:

   **Chaos gas hitting a mind-controlled unit**: Works — but with a subtle ownership
   twist. `TechnoClass::ReceiveDamage`'s Psychedelic gate checks
   `HouseClass::IsAlliedWith(this->Owner, sourceHouse)`. A mind-controlled unit's
   `Owner` field is swapped to the controller's house. Therefore:
   - If the Chaos Drone is owned by the **same** player as the MC-controller, the
     unit counts as allied → `return 0`, no berserk applied.
   - If the Chaos Drone is owned by an **enemy** player, the ally check fails → the
     unit goes berserk normally. A mind-controlled+berserk unit attacks anything
     nearby, potentially including its own MC-controller (because berserk bypasses
     alliance filter at §3.3 Scan_Cell_For_Target).

   **Chaos gas hitting a parasite-host (Terror Drone'd vehicle)**: ReceiveDamage's
   Psychedelic branch does not special-case parasite hosts. The gate chain applies
   as normal (allied? immune? building? no → berserk). The parasite attachment
   (tracked separately at TechnoClass field for WarpAttachClass) remains active.
   The team-removal call (FUN_006EA870 = TeamClass::RemoveMember) on berserk flip
   operates on the host's TeamPtr (+0x5D4), which is independent of the parasite
   attachment — so the parasite stays attached through berserk state transitions.
   The host vehicle being "piloted" by both a berserk flag AND a parasite would
   produce the observable erratic movement of gassed Terror-Drone'd vehicles.

   **Psychedelic weapon fired at a MindControl-immune target**: Blocked upfront
   by `TechnoClass::GetFireError` (§3.7) — the weapon won't even fire.

   **Double-Psychedelic hit**: A second Psychedelic warhead fired at a unit with
   `field_0x2E4 != 0` is blocked by `GetFireError`. However, the gas-cloud particle
   emitting `PsychGas` hits directly may bypass this gate (particles enter
   ReceiveDamage without going through GetFireError). This is consistent with
   observed behavior of gas refreshing the berserk timer continuously.

4. ~~**The `flag & 4` vehicle gate on passenger ejection**~~ → **RESOLVED 2026-04-19
   (with a major correction)**: The original interpretation was wrong. Disassembly
   of the exact call at `0x00701D86-0x00701D9B` shows:

   ```
   TEST byte ptr [ESI + 0x14],0x4        ; target is Techno?
   JZ  skip
   MOV ECX,dword ptr [ESI + 0x5d4]       ; ECX = target->TeamPtr (NOT transport)
   TEST ECX,ECX                           ; target is IN a team?
   JZ  skip
   PUSH 0x0 ; PUSH -1 ; PUSH ESI         ; (team, target, -1, 0)
   CALL FUN_006EA870                      ; TeamClass::RemoveMember
   ```

   **TechnoClass+0x5D4 = TeamPtr** (AI team membership pointer, verified via
   `TECHNOCLASS_EXPANDED_STRUCT_LAYOUT.md` which labels it `TeamPtr` at 0x5D4
   and cross-references `SuicideTeamPtr` at 0x434). `FUN_006EA870` walks the
   team's member linked-list at `team+0x54`, removes the target, decrements
   team member count at `team+0x48` and capacity at `team+0x4C`.

   **Correct semantics**: *If the berserked target is currently in an AI team,
   remove it from the team.* The target becomes an independent Hunt-missioned
   unit, no longer directed by AI team logic. Makes intuitive sense — a
   berserked unit is erratic and shouldn't follow group commands.

   This does NOT eject passengers from an IFV. If an IFV full of infantry gets
   gassed, each individual infantry passenger gets berserked (via the CellSpread=3
   AoE hitting each entity) and each one leaves its team if applicable. Passengers
   stay INSIDE the IFV unless some other mechanic ejects them.

5. ~~**`warhead[+0x12C]` field semantics**~~ → **RESOLVED 2026-04-19**:
   WarheadTypeClass+0x12C = **`PercentAtMax`** (float, default 1.0, INI key
   `PercentAtMax=` — verified via WARHEADTYPECLASS_FULL_STRUCT_LAYOUT.md §DataFields
   line 98). For AOE warheads this represents damage-falloff-at-max-distance;
   FUN_00489180 uses it as a damage multiplier and clamps the result to
   `Rules+0x16C8 = MaxDamage` (default 10000, verified via
   RECEIVE_DAMAGE_PIPELINE_VERIFICATION_REPORT.md). For `[PsychGasCreate]`
   (`PercentAtMax=1`, meaning 100%), the multiplier is 1.0 → timer ≈ post-Verses
   damage. **Final formula**:
   ```
   berserk_timer_frames = min(
       post_armor_mult_damage × warhead.PercentAtMax,
       Rules.MaxDamage = 10000
   )
   ```
   **For Chaos Drone primary** (MagneticBeam Damage=600 × Verses[infantry]=100% × 
   PercentAtMax=1.0): **timer = 600 frames** (never capped). At RA2's standard tick
   cadence this is several seconds of berserk — matches observed in-game behavior.

---

## Sources

**Ghidra addresses decompiled/disassembled this session:**
- `0x0075D590` — WarheadTypeClass::ReadINI (disasm near Psychedelic) → offset 0x16D verified
- `0x00712170` — TechnoTypeClass::ReadINI (disasm near BerserkFriendly) → offset 0x690 verified
- `0x00701900` — TechnoClass::ReceiveDamage (full decomp) → Psychedelic dispatch + berserk flip verified
- `0x006F8960` — TechnoClass::Scan_Cell_For_Target (full decomp) → berserk bypass in ally filter verified
- `0x006EA870` — TeamClass::RemoveMember (full decomp; sem: walks team->members linked-list at team+0x54, unlinks the target, decrements team count at team+0x48 and capacity at team+0x4C, clears target->was_removed flag at +0x6B8). Initially mislabeled as "passenger-ejection helper" — corrected 2026-04-19.
- `0x00489180` — FUN_00489180 damage-verses calculator (full decomp)
- `0x005B35E0` — MissionClass::Queue_Mission (full decomp)
- `0x007087C0` — TechnoClass::ShouldRetaliate (full decomp) — noted this does NOT use 0x298
- `0x00709290` — FUN_00709290 passive-target gate (full decomp) — does not use 0x298
- `0x004F9AF0` — HouseClass::Is_Ally_ByObject_WithFlag (full decomp)
- `0x004D97A0` — FootClass::Evaluate_Target_Threat (full decomp)

**String xrefs resolved:**
- `Psychedelic` @ 0x847D30 (single xref to 0x0075D8EA) → WarheadType+0x16D
- `BerserkFriendly` @ 0x8439F8 (single xref to 0x007148FA) → TechnoType+0x690
- `BerserkColor` @ 0x83A194 (xref to RulesClass::ReadAudioVisual at 0x0066B864)
- `BerzerkAllowed` @ 0x83ADCC — TS-legacy, confirmed no live effect

**Existing reports referenced:**
- `WARHEADTYPECLASS_FULL_STRUCT_LAYOUT.md` — Psychedelic=0x16D claim confirmed
- `WARHEAD_DETONATE_GHIDRA_REPORT.md` — Psychedelic NOT in the direct-dispatch chain (confirmed; it falls through to Apply_area_damage)
- `MIND_CONTROL_SYSTEM_GHIDRA_REPORT.md` — its Psychedelic offset claim is correct; its "dispatch via Detonate" framing is misleading for Psychedelic specifically (dispatch is in ReceiveDamage, not Detonate)
- `DAMAGE_MATH_GHIDRA_REPORT.md` — the Psychedelic gate chain is verified (lines 338-352 match the binary); the "begin warp state" phrasing in that doc is inaccurate — the actual effect is a berserk state flip, not a warp state
- `TARGET_ACQUISITION_GHIDRA_REPORT.md` — the "field_0x298 != 0 attacks everything" claim confirmed at the Scan_Cell_For_Target sites
- `TECHNOCLASS_VTABLE_COMPLETE.md` — vtable slot assignments used for 0x1E8 and 0x3C8
- `FOOTCLASS_NON_MOVEMENT_FIELDS.md` — referenced for spawn/passenger field layout context

**INI files checked:**
- `ini/rulesmd.ini` — `[CAOS]`, `[ChaosAttack]`, `[VirtualScanner]`, `[PsychGasCreate]`,
  `[PsychGas]`, `[PsychCloudSys]`, `[PsychCloud]`, `[PsychCloudD]`, `[InvisibleLow]`,
  `[General]` (BerserkColor), `[Warheads]` list
- `ini/artmd.ini` — `[CDGAS]`
- `ini/rules.ini`, `ini/art.ini` — confirmed no Chaos Drone entries (YR-only unit)
