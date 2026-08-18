# GATE D3 — Country-Armor DIVIDE + Per-Unit +0x158/+0x160 Mults + Full Multiplier Order & ftol Boundaries — RESOLUTION

**Status:** CLOSED. Every sub-gate (a)(b)(c) bit-VERIFIED by live Ghidra disassembly/decompile this run (2026-06-04). No inherited claim reused without re-reading the function body this session.
**Bar:** parity on player-observable OUTPUT (damage to the last decimal). Default verdict for any unproven equivalence is DRIFT.
**Scope:** the multiply/divide order and ftol-truncation boundaries from attacker-side `Fire_At` through receiver-side `TechnoClass::ReceiveDamage` → `ObjectClass::ReceiveDamage` → `ApplyWarheadDamage`. Companion: `DAMAGE_HELPERS_ENGINE_SUBSTRATE_SERVICE_STUDY.md` (D7–D10, D13–D18).

> **2026-07-13 active-binary correction (supersedes conflicting attacker prose
> below):** `disassemble_function(address="0x006fdd50",
> program="gamemd.exe")` shows `TEST EDI,EDI; JLE 0x006fe3e3` at
> `0x006fe32f..0x006fe331`, so ordinary `Damage <= 0` skips country/per-unit and
> veterancy scaling. The positive path is exactly `FLD house+0x188; FMUL
> techno+0x160; FIMUL weapon Damage; CALL Math__ftol` at
> `0x006fe33d..0x006fe34d`: grouping is `(house * unit) * integer Damage` before
> one conversion. The Wave/special branch `XOR EDI,EDI; JMP 0x006fe3df` at
> `0x006fe328` stores zero but does not return. Both it and ordinary non-positive
> damage can continue into the containment stages at `0x006fe3e3..0x006fe455`,
> including their own `Math__ftol` calls. The older deploy/gattling labels in
> this report are also superseded by the audited civilian-garrison → tank-bunker
> → open-topped identities in `TANK_BUNKER_COMBAT_SURFACE_GHIDRA_REPORT.md`.

`ftol` = `Math__ftol @ 0x007c5f00` = truncate-toward-zero (FPU CW 0x0E7F). Every "→ftol" below is one truncation to int.

---

## (a) Country/house armor IS a DIVIDE — VERIFIED

**Confirmed: the defender country-armor multiplier divides incoming damage (it does NOT multiply).** Tougher country = larger mult = `damage ÷ mult` = less damage taken.

Assembly, `TechnoClass::ReceiveDamage @ 0x00701900` (verified `disassemble_function 0x00701900` this run):
```
00701939: FILD dword ptr [ESP + 0x14]      ; promote incoming int damage to float
00701941: FSTP double ptr [ESP + 0x14]     ; stash as double
00701945: CALL dword ptr [EAX + 0x84]      ; vtable+0x84 -> target's HouseTypeClass getter (EAX=this->vtable)
0070194b: MOV ECX,dword ptr [ESI + 0x21c]  ; ECX = defender HouseClass (this+0x21c)
00701952: CALL 0x0050bd30                  ; HouseClass::GetArmorMultForType(target) -> ST0 = country armor mult
00701957: FMUL double ptr [ESI + 0x158]    ; *= TechnoClass+0x158 (per-unit ArmorMultiplier)
0070195d: FDIVR double ptr [ESP + 0x14]    ; FDIVR = reverse divide => ST0 = damage / (countryMult * unitMult)
00701961: CALL 0x007c5f00                  ; ftol(...)  (truncate)
0070196c: MOV dword ptr [EBX],EAX          ; *damage = result
```
`FDIVR mem` computes `mem / ST0`, i.e. `damage / product` — a DIVIDE. **(a) CLOSED.**

**Field / source of the mult — `HouseClass::GetArmorMultForType @ 0x0050bd30`** (verified `decompile_function 0x0050bd30` this run). The float is read from the **defender's HouseTypeClass** (`param_1+0x34` deref), switched on the target's `WhatAmI()` (`vtable+0x2c`):

| WhatAmI (case) | Target kind | HouseTypeClass offset | INI (ArmorMult family) |
|---|---|---|---|
| 3 | Infantry | +0x108 | `[Country] ArmorInfantryMult` |
| 0x10 | Aircraft | +0x100 | `ArmorAircraftMult` |
| 0x28 | Building | +0x104 | `ArmorBuildingsMult` |
| 7 + `param_2[0x382]==5` (byte 0xE08, FlyLocomotor) | flying Unit | +0x110 | `ArmorAircraftMult` (flying-unit slot) |
| 7 (default) | ground Unit | +0x10c | `ArmorUnitsMult` |
| default | — | `_DAT_007e2ac8` = **1.0** | (no mult) |

**Where applied:** receiver side, inside `TechnoClass::ReceiveDamage`, BEFORE the kernel `ApplyWarheadDamage` runs (the kernel runs later, in `ObjectClass::ReceiveDamage`). Default 1.0 ⇒ no-op when no country bonus is set.

---

## (b) Per-unit fields +0x158 / +0x160 — VERIFIED identity + operation

| Offset (TechnoClass) | Identity | Side | Operation | Stage it folds into |
|---|---|---|---|---|
| **+0x158** | **ArmorMultiplier** (per-unit, `double`) | receiver | folded into the country-armor product, then incoming is **DIVIDED** by it | D7 country-armor divide |
| **+0x160** | **FirepowerMultiplier** (per-unit, `double`) | attacker | **MULTIPLY** | D10 Fire_At FirePower stage |

- **+0x158 (receiver, ÷):** `00701957: FMUL double ptr [ESI+0x158]` — multiplied INTO the country mult, then `FDIVR` divides damage by the product (see (a)). So `damage ÷ (countryArmorMult × unitArmorMult)`. Larger ⇒ tougher.
- **+0x160 (attacker, ×):** `006fe343: FMUL double ptr [ESI+0x160]` inside `Fire_At` (verified `get_assembly_context 0x006fe337` this run), multiplied with country FirePower and the base damage in the same `ftol` stage (see (c) Stage 1).

Neither is a vet/upgrade field — vet is handled separately by Rules+0x670/+0x688 with `IsVeteran/IsElite` gates. **(b) CLOSED.**

---

## (c) COMPLETE end-to-end multiplier ORDER + ftol boundaries — VERIFIED

Two independent chains: attacker-side stored on the projectile, receiver-side applied on impact. Each numbered stage is one `ftol` truncation to int (except where noted "stays float / 80-bit x87").

### ATTACKER side — `Fire_At @ 0x006fdd50` (verified `get_assembly_context` @ 0x006fe337/0x3c8/0x3f1 + `disassemble` this run)

Gate: Wave/special sets working damage to zero and jumps past A1/A2, but rejoins
A3. Ordinary base `Damage <= 0` also skips A1/A2 and rejoins A3. Only strictly
positive ordinary Damage executes A1/A2.

| # | Stage | Op | Address | Rounds? |
|---|---|---|---|---|
| A0 | Wave/special → 0; otherwise base = `weapon+0xa4` (Damage) | — | `006fe328..006fe331` | int |
| A1 | strictly positive only: `(HouseClass+0x188 country FirePower × TechnoClass+0x160 per-unit FirepowerMult) × integer base` | MUL | `006fe33d/343/349` → `006fe34d` | **→ftol** |
| A2 | strictly positive only: × `VeteranCombat = Rules+0x670` (≈1.1), gated by verified vet/elite firepower ability | MUL | `006fe3d2` → `006fe3d8` | **→ftol** |
| A3 | civilian-garrison × `OccupyDamageMultiplier` (`Rules+0xf40`) | MUL | `006fe3e3..006fe400` | **→ftol** |
| A4 | tank-bunker × `BunkerDamageMultiplier` | MUL | tail through `006fe455` | **→ftol** |
| A5 | open-topped × `OpenToppedDamageMultiplier` | MUL | tail through `006fe455` | **→ftol** |

Result is the integer `base_damage` stored on the bullet and passed to the receiver chain.

### RECEIVER side — `TechnoClass::ReceiveDamage @ 0x00701900` then `ObjectClass::ReceiveDamage @ 0x005f5390`

| # | Stage | Op | Address (verified this run) | Rounds? |
|---|---|---|---|---|
| R1 | **Country armor ÷** `(GetArmorMultForType × TechnoClass+0x158)` | **DIV** | `0070195d FDIVR` → `00701961` | **→ftol** |
| R2 | **Vet/elite ARMOR ÷** `VeteranArmor = Rules+0x688` (≈1.5) — gated: defender vet+`type+0x29d` OR elite+(`0x29d` or `0x2af`) | **DIV** | `007019cb FDIV [EAX+0x688]` → `007019d1` | **→ftol** |
| R3 | **Min-1 floor** (positive only): `if (*dmg < 1) *dmg = 1` | clamp | `007019d8 CMP/JGE/MOV 1` | int |
| R4 | **TypeImmune** zero: attacker present, `type+0xc8c`, same `WhatAmI`, same owner `+0x21c` → return 0 | gate | `007019e3…00701a1c` | — |
| R5 | **Immunity gates** (each short-circuits to 0), in this order: WarpingOut `vtable+0x160` (`00701a3f`); ForceShield/invuln `vtable+0x1d4` (`00701ab1`); Bunker/wall `field_0x2e4`+`WhatAmI==6`→`warhead+0x146` (`00701b87`); Radiation `wh+0x177`&`type+0xd37` (`00701bfe`); PsychicDamage `wh+0x178`&`type+0xd36` (`00701c31`); Poison `wh+0x156`&`type+0xd3b` (`00701c64`); `!AffectsAllies` `wh+0x179==0`&`IsAlliedWith` (`00701c97`); Psychedelic `wh+0x16d` (`00701cd7`, MC path → kernel w/ NULL warhead @ `00701d64`, returns 1, zero HP). | gate | as cited | — |
| R6 | Fall through → `CALL 0x005f5390` ObjectClass::ReceiveDamage | — | `00701df8` | — |
| R7 | **Verses falloff kernel** `ApplyWarheadDamage @ 0x00489180`: `cellSpreadLeptons=ftol(CellSpread×256)`; falloff = `ftol(lerp(1.0, PercentAtMax, dist/csLeptons)*dmg)` ONLY if `dmg*PAM != dmg && csLeptons!=0` else flat; `scaled = ftol(falloff_int × Verses[armor] double @ wh+0xA0+armor*8)`. Three ftol: `004891e4`, `00489220`, `00489244`. | falloff×verses | `00489180` | **→ftol ×3** (`ftol(ftol(lerp)*Verses)`) |
| R8 | **MaxDamage cap**: `if (scaled >= Rules+0x16C8 [10000]) scaled = cap` | clamp | `0048924f CMP/JL` | int |
| R9 | **Building min-1**: WhatAmI==6 w/o CanC4 (`+0x1577==0`) → `if (*dmg<1) *dmg=1` | clamp | `005f5390` body | int |
| R10 | **Overkill clamp**: `if (*dmg >= Health) *dmg = Health` | clamp | `005f5390` `else { *dmg = iVar6 }` | int |
| R11 | **State classify** (return code): Yellow = **integer `Strength>>1`** crossing (`iVar3>>1 <= prevHP && prevHP-dmg < iVar3>>1`); Red = `(double)Strength × Rules+0x1708` crossing; Dead = HP==0; PostMortem = IsAlive==false. | — | `005f5390` body | int |

Healing path (R7 D2): `dmg < 0` → `ApplyWarheadDamage` returns `(armorIndex >= 8) ? 0 : dmg` (special_1/special_2 cannot heal), bypassing falloff/Verses; ObjectClass then `Health -= dmg` clamped to Strength.

ScenarioFlags&0x20 (`[0x00a8b230] & 0x20`) early-out present in BOTH the kernel (`0048919a`) and `Apply_area_damage` → returns 0.

**The exact ordered formula the Rust damage service must reproduce (positive, non-immune hit):**
```
# attacker (Fire_At), each enabled later line truncates toward zero:
d = 0 if Wave/special else weapon.Damage
if ordinary and d > 0:
    d = ftol((country_firepower(House+0x188) * unit_firepower(Techno+0x160)) * d)
    if vet/elite firepower ability: d = ftol(d * VeteranCombat[Rules+0x670])
# These remain reachable for ordinary d <= 0 and Wave/special d == 0:
if civilian garrison:            d = ftol(d * OccupyDamageMultiplier)
if tank bunker:                  d = ftol(d * BunkerDamageMultiplier)
if open-topped:                  d = ftol(d * OpenToppedDamageMultiplier)
# receiver (TechnoClass::ReceiveDamage):
d = ftol(d / (country_armor(GetArmorMultForType) * unit_armor(Techno+0x158)))
if vet/elite armor ability:      d = ftol(d / VeteranArmor[Rules+0x688])     # ~1.5
d = max(d, 1)                                                                # positive only
# ... immunity gates may zero d here ...
# kernel (ApplyWarheadDamage, via ObjectClass::ReceiveDamage):
csL  = ftol(CellSpread * 256)
fall = (d*PAM != d && csL != 0) ? ftol(d * lerp(1.0, PercentAtMax, dist/csL)) : d
fall = max(fall, 0)
d    = ftol(fall * Verses[armor])            # Verses is double, at wh+0xA0+armor*8
d    = min(d, MaxDamage[Rules+0x16C8])       # 10000
# ObjectClass HP apply:
if building && !CanC4: d = max(d, 1)
d = min(d, currentHealth)                    # overkill clamp
Health -= d ; classify Yellow(Strength>>1)/Red(Strength*Rules+0x1708)/Dead
```
**(c) CLOSED.**

---

## YR-active vs TS note

The positive attacker stages and receiver stages above are ACTIVE in a standard
YR skirmish; country multipliers default to identity and veterancy gates on the
stock abilities. The attacker country/per-unit/veteran stages specifically do
not run for ordinary non-positive Damage, while enabled containment stages can.
TS-legacy / NOT to model: ProneDamage (+0xF8, dead — proven by whole-image byte
sweep in the companion doc), VeinholeMonster `WhatAmI==0xF` nuke-survival clamp
inside `005f5390`, Deform/rocking (separate subsystem). Bullet-ammo decrement
(`00701adb`, `type+0x6b1`) and threat-score feedback (`00701e0c`) consume the
final clamped `*dmg` but are side-effects, not HP-number stages.

---

## Rust handoff — what current Rust gets WRONG

Current Rust damage apply is a single `i32` multiply, missing the entire attacker chain (except garrison) and the entire receiver divide/clamp/gate chain. Verified against:
- `src/sim/combat/combat_aoe.rs::aoe_damage_at_distance` (L327–350): `base*verses_pct*falloff_pct/10000`, one divide, no ftol order, no MaxDamage cap, no country/vet/per-unit mults; Verses is `u8` percent (loses gamemd `double`).
- `src/sim/combat/mod.rs` direct-hit (L2223): `base_damage * verses_pct / 100` — duplicate twin, same DRIFT; no min-1 floor.
- `src/sim/combat/mod.rs` HP apply (L1642–1653): `health.current.saturating_sub(*damage)`; coarse `is_invulnerable` nullify (L1644) — no overkill clamp (R10), no D11 gate set (TypeImmune/WarpingOut/Radiation/Psionic/Poison/AffectsAllies), no state-return enum.
- Attacker mults: only `OccupyDamageMultiplier` is folded into current
  `base_damage`; MISSING the exact positive gate/grouped country+per-unit stage,
  VeteranCombat, tank-bunker, open-topped, and special-zero continuation.
- Receiver mults: MISSING country armor DIVIDE (GetArmorMultForType), per-unit ArmorMult (Techno+0x158), VeteranArmor DIVIDE (Rules+0x688), and the min-1 floor.
- `apply_prone_damage_modifier` (`mod.rs`) + `prone_damage_basis_points` (`warhead_type.rs`): WRONG — applied on every infantry hit; gamemd never reads ProneDamage in YR. Deals 50–70% wrong damage to prone infantry. Retire the APPLY.
- `src/rules/combat_damage.rs`: out of scope (particle-system defaults only — confirmed by reading the file header this run); leave untouched.

**Net:** Rust must replace both formula copies with one kernel that reproduces the ordered/ftol-truncated pipeline above, add the attacker `CombatMods` chain, the receiver country/vet/per-unit DIVIDE chain with min-1, the D11 immunity gate ordering, the overkill clamp + integer-Strength>>1 state classify, migrate Verses `u8 → SimFixed`, and stop applying ProneDamage.

---

## Verification ledger (this run, 2026-06-04)
- `disassemble_function 0x00701900` — R1 FDIVR divide + Techno+0x158, R2 FDIV Rules+0x688, R3 min-1, R4 TypeImmune, R5 gate order, R6 call 0x005f5390 (all addresses cited inline).
- `decompile_function 0x0050bd30` — GetArmorMultForType switch + HouseTypeClass+0x100/0x104/0x108/0x10c/0x110, default 1.0.
- `disassemble_function 0x00489180` — kernel D1/D2 healing `(7<armor)-1 & dmg`, three ftol (0x004891e4/0x00489220/0x00489244), Verses double @ +0xA0+armor*8, MaxDamage cap @ Rules+0x16c8.
- `decompile_function 0x005f5390` — kernel call `FUN_00489180(+0x9c armor, warhead)`, building min-1, overkill clamp `else{*dmg=Health}`, Yellow integer `iVar3>>1`, Red double `*Rules+0x1708`, Dead vtable +0xE0/+0xE4/+0xDC.
- `disassemble_function 0x006fdd50` (2026-07-13 recheck) — positive gate,
  exact A1 grouping, A2, Wave/special zero rejoin, and containment tail at
  `0x006fe328..0x006fe455`; containment identities cross-checked against
  `TANK_BUNKER_COMBAT_SURFACE_GHIDRA_REPORT.md`.
- Rust: read `src/sim/combat/combat_aoe.rs`, `src/sim/combat/mod.rs` (L1600–1710, L2120–2280), `src/rules/combat_damage.rs` header.
