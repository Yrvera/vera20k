# Suicide Weapons & DeathWeapon

This doc is the canonical reference for the **two related self-destruct mechanisms**
in gamemd.exe:

1. **`Suicide=yes` weapon flag** (`weapon+0x144`) — the attacker dies as part of firing the weapon. Used by Crazy Ivan bomb-place, Demo Truck, IFV-Ivan variant.
2. **`DeathWeapon=` TechnoType field** — a weapon fired at the unit's own position when it dies. Used by Terrorist (auto-detonate when killed), Demo Truck (chain-react when destroyed), Kirov (drop bomb on crash), explosive barrels, civilian oil pumps, etc.

The two compose: a Demo Truck has `Suicide=yes` on its weapon AND `DeathWeapon=Demobomb`,
so it explodes both when it arrives at target AND when it's killed mid-route.

Out-of-scope:
- The damage transform → [`damage_formula.md`](damage_formula.md)
- The splash dispatcher that delivers the bomb damage → [`splash_cellspread.md`](splash_cellspread.md)
- Per-warhead detonate dispatch → [`warhead_detonate_dispatch.md`](warhead_detonate_dispatch.md)
- Detailed unit-specific suicide behavior → individual `weapons/DemoTruckSuicide.md`, `weapons/TerroristSuicide.md`, `weapons/CrazyIvanBombs.md` (when written)

---

## 1. Flag layout (verified)

### WeaponTypeClass

| Offset | INI key | String addr | Effect |
|---|---|---|---|
| `weapon+0x144` | `Suicide=` | `0x00843050` (verified live 2026-05-17) | Firer dies on weapon fire (one-shot) |

Parsed in `WeaponTypeClass::ReadINI 0x0077228D` (verified live). 

Cross-reference: existing canonical `FIRE_AT_PIPELINE_GHIDRA_REPORT.md` lists this as the flag that causes "Fire_At short-circuits with vtbl+0x16C SetTarget(self)" — the firer targets itself with the suicide weapon, ensuring the damage hits the firer's own position.

### TechnoTypeClass

| Offset | INI key | String addr | Effect |
|---|---|---|---|
| `type+0x???` | `DeathWeapon=` | `0x0083B11C` (verified live 2026-05-17) | WeaponType pointer fired at unit's position on death |
| `type+0x???` | `DeathWeaponDamageModifier=` | `0x00844488` (verified live 2026-05-17) | Float multiplier applied to DeathWeapon damage |

Parser sites:
- `TechnoTypeClass::ReadINI 0x007122F0` — reads `DeathWeapon=` per-type
- `RulesClass::ReadCombatDamage 0x0066C58A` — reads default `DeathWeapon=` (the global fallback for units that don't set their own)

The Rules-level default is at line 871 in retail rulesmd.ini:
```ini
DeathWeapon=DefaultDeathWeapon ;gs Can't use the unit's weapon anymore now that spread is fixed.  Damage will be based on hitpoints
```

So units without an explicit `DeathWeapon=` get `DefaultDeathWeapon` as their fallback,
and that weapon's damage scales with the unit's HP at death (per the INI comment).

### Confidence (flags)

- **Content: HIGH** — three string xrefs verified live 2026-05-17.
- **Identity: HIGH** — single INI key strings, single parsers per flag.
- **Binding: HIGH** for `Suicide` (parsed + consumer in Fire_At per existing canonical doc). **MEDIUM** for `DeathWeapon` exact dispatch site (not directly traced in this iteration — see open follow-up #1).

---

## 2. Retail INI survey

### Suicide=yes weapons (4 total)

```ini
[IvanBomb]                ; Crazy Ivan placing a bomb
Anim=RING1
Range=1.5
ROF=10
Suicide=yes              ; wait — Ivan doesn't die when placing a bomb!
                          ; (see §3 — Suicide=yes here may mean something specific
                          ;  in the bomb-place mechanic)
[Demobomb]                ; Demo Truck arrival weapon
RadLevel=100
Warhead=DemobombWH
Report=DemoTruckDie
Suicide=yes

[CRIvanBomb]              ; IFV-Ivan variant (probable)
Anim=RING1
Range=1.5
ROF=10
Suicide=yes

[CRNuke]                  ; IFV-Ivan nuke variant
RadLevel=500
Warhead=CRNUKEWH
Report=NukeExplosion
Suicide=yes
```

So 4 retail weapons set `Suicide=yes`:
- `IvanBomb` and `CRIvanBomb` — Crazy Ivan / IFV-Ivan bomb placement
- `Demobomb` — Demo Truck contact detonation
- `CRNuke` — IFV-Ivan special nuke

### DeathWeapon= per-type (sample)

```ini
[TERROR]                  ; Terrorist
DeathWeapon=TerrorBomb     ; Detonates a bomb on death — the suicide-bomber mechanism

[Demotruck]               ; Demo Truck
DeathWeapon=Demobomb       ; Same weapon as Suicide — chain-react if destroyed mid-route

[IFV]                     ; IFV with Ivan passenger
DeathWeapon=CRNuke         ; Only triggers if Ivan-piloted (special case noted in INI comment)

[KIROV]                   ; Kirov Airship
DeathWeapon=BlimpBomb
DeathWeaponDamageModifier=.1   ; 10% damage on crash

[HARRIER], [BEAGLE]       ; Allied air units
DeathWeapon=BlimpBomb
DeathWeaponDamageModifier=.1

[NukeCarrier]             ; (campaign / special unit)
DeathWeapon=NukePayload
DeathWeaponDamageModifier=0.5

[CIV_OIL_DERRICK]         ; civilian oil derrick
DeathWeapon=OilExplosion

[BARREL], [BRL3], [WLAMP] ; explosive barrels & flammable objects
DeathWeapon=BarrelExplosion
```

The pattern: `DeathWeapon=` is a **per-unit-type** "I explode when killed" mechanism,
fired at the unit's own position. `DeathWeaponDamageModifier=` scales the damage,
typically used on aircraft to prevent crash-damage from being lethal to surrounding
units (0.1 = 10% damage).

---

## 3. The mechanisms — what they actually do

### 3.1 `Suicide=yes` weapon

Per existing canonical `FIRE_AT_PIPELINE_GHIDRA_REPORT.md`:

> `Suicide=` `weapon+0x144` — Fire_At short-circuits with vtbl+0x16C SetTarget(self)

Mechanism (working from canonical doc + INI patterns):

```c
// In TechnoClass::Fire_At, after determining a weapon to fire:
if (weapon.Suicide (+0x144) != 0) {
    // Re-target self
    this.SetTarget(this)
    // Standard firing pipeline runs — projectile lands at firer position
    // → Apply_area_damage delivers the warhead damage to self (and surrounding)
    // The C4Warhead self-target gate (splash_cellspread.md §8) is bypassed because
    // weapon != C4Warhead AND self.Type.IsSelfHealing typically not set —
    // BUT the suicide branch sets a special path that allows self-hit
}
```

The exact branch in Fire_At that handles this is not decompiled here. But the INI
pattern is clear: `Suicide=yes` weapons fire AT the firer, the warhead detonates AT
the firer's position, and the firer dies in the explosion. This is how Demo Truck
"arrives at target" works — the truck moves to the target, fires its Demobomb at
itself, and explodes.

### 3.2 The IvanBomb edge case

Crazy Ivan doesn't actually DIE when placing a bomb. So why does `IvanBomb=Suicide=yes`?

Hypothesis (unverified): the `Suicide=yes` flag on IvanBomb may trigger the warhead
dispatch path's special-case for `IvanBomb=yes` warhead (`wh+0x157`) — see
[`mind_control.md`](mind_control.md) §1 cascade priority 2. That special path attaches
a bomb to the target instead of dealing immediate damage, and bypasses the "firer
dies" effect. So the `Suicide=yes` flag may be a no-op when combined with an
IvanBomb warhead, OR it may serve some bomb-placement-specific purpose (e.g.,
"clear the target lock after placing the bomb so Ivan can re-target").

**Status:** LOW confidence on this interpretation. Open follow-up #2 — trace
Fire_At's Suicide branch interaction with IvanBomb warhead.

### 3.3 `DeathWeapon=` mechanism

When a unit dies (Health reaches 0 in ReceiveDamage), the engine fires its
`DeathWeapon=` (if any) at its own position before the unit object is removed.

Inferred flow:

```c
// In TechnoClass::Killed or ObjectClass::ReceiveDamage end-of-life handler:
if (this->Health <= 0):
    type = this->GetType()
    deathWeapon = type.DeathWeapon (+0x???)
    if (deathWeapon == NULL): deathWeapon = Rules.DeathWeapon (+0x???)  // global default

    if (deathWeapon != NULL):
        damageModifier = type.DeathWeaponDamageModifier (+0x???)  // 1.0 default
        scaledDamage = ftol(deathWeapon.Damage × damageModifier)
        Apply_area_damage(this.coords, scaledDamage, NULL/*attacker*/, deathWeapon.Warhead, ...)
```

The damage applies to all surrounding units via standard `Apply_area_damage` dispatch
(see [`splash_cellspread.md`](splash_cellspread.md)). The dying unit's owner is
typically NOT credited as the source (since the unit is dead) — kill credit for
chain-deaths goes to the original attacker.

**Status:** Working hypothesis. The exact dispatch function isn't traced in this
iteration. Open follow-up #1.

### Confidence (mechanisms)

- **Content: MEDIUM** — Suicide flag identity verified, DeathWeapon parser site verified, but the actual dispatch in Fire_At / Killed is not directly decompiled.
- **Identity: HIGH** for the flags themselves.
- **Binding: HIGH** for the parser side (where the flags are stored). **MEDIUM** for the consumer side.

---

## 4. Composition: Suicide + DeathWeapon on the same unit

A Demo Truck (`[Demotruck]`) has:
- Primary weapon `Demobomb` with `Suicide=yes`
- `DeathWeapon=Demobomb` (same weapon)

So when the Demo Truck attack-moves to a target:
- It moves into range
- Fires `Demobomb` at the target (and itself due to Suicide=yes)
- Apply_area_damage delivers Demobomb damage at the Truck's position
- Truck dies (Health reaches 0 from self-damage)
- DeathWeapon=Demobomb fires AGAIN at the Truck's now-dying position
- A SECOND Demobomb detonation occurs

**Net effect:** Demo Truck arrival produces TWO Demobomb explosions, almost in the
same tick. The damage is roughly doubled. This may be the source of the famously
massive Demo Truck explosion.

Alternatively, the engine might gate the DeathWeapon when Suicide=yes already fired —
need to verify whether the dying unit's DeathWeapon fires when the death cause was
its own Suicide weapon. Open follow-up #3.

---

## 5. Mid-route Demo Truck destruction

If a Demo Truck is killed mid-route by enemy fire (not its own Suicide weapon):
1. Standard damage path kills it.
2. `DeathWeapon=Demobomb` fires at the death position.
3. A Demobomb detonation occurs.

So Demo Trucks are ALWAYS dangerous — destroying one in transit triggers its bomb.
This is the well-known "shoot the Demo Truck only when it's far from your base" tactic.

---

## 6. Terrorist (`[TERROR]`) — pure DeathWeapon suicide

Terrorist has NO Suicide weapon — its primary attack IS its DeathWeapon path:
- `[TERROR]` primary weapon attacks normally (probably a melee suicide attack triggered by reaching target)
- When killed, `DeathWeapon=TerrorBomb` fires at the Terrorist's position

The Terrorist's "kamikaze run" works like:
1. Move next to target.
2. Some mechanism triggers self-damage (auto-detonation when adjacent to target — likely a special unit logic, not the Suicide flag).
3. Terrorist dies → DeathWeapon=TerrorBomb fires.

Open follow-up #4 — trace how Terrorist initiates its own death (is it a Suicide weapon, or a special TechnoType behavior gate?).

---

## 7. Aircraft crash DeathWeapon

Kirov, Harrier, Black Eagle, and other aircraft all have `DeathWeapon=BlimpBomb` with
`DeathWeaponDamageModifier=.1`. The intent (per INI comments) is to give them
**controlled crash damage** — without a DeathWeapon, aircraft just disappear on
destruction with no impact damage. With `BlimpBomb × 0.1`, they deal proportional
crash damage to whatever is below them.

The `DeathWeaponDamageModifier=.1` (10%) is calibrated so that:
- Kirov crash deals significant but not-overwhelming damage to a couple of cells
- A laser-blast worth of damage (per the INI comment) is the alternative if you omit the modifier

NukeCarrier (`DeathWeapon=NukePayload × 0.5`) is special — it carries an actual nuke,
so its crash damage is 50% of NukePayload's full damage.

---

## 8. Explosive barrels — chain-reaction with DeathWeapon

`[BARREL]`, `[BRL3]`, `[WLAMP]` (red barrels, oil lamps, etc.) have:
- `Explodes=yes` (a TechnoTypeClass flag — separate mechanism)
- `DeathWeapon=BarrelExplosion`

When a barrel is destroyed:
1. `Explodes=yes` (per [`chain_reaction.md`](chain_reaction.md) §5) triggers the
   IC-barrel chain via `Apply_area_damage(0, Rules.C4Warhead, 1, sourceHouse)`.
2. `DeathWeapon=BarrelExplosion` ALSO fires at the barrel's position.
3. Both damage events apply.

So barrels deliver TWO damage events on destruction — the C4-chain (for chaining to
other barrels) AND the BarrelExplosion DeathWeapon (for damaging surrounding units).
Both are LIVE in retail YR.

---

## 9. Key offsets summary

| Symbol | Offset / Address |
|---|---|
| `weapon.Suicide` | `+0x144` |
| `type.DeathWeapon` | `+0x???` (parsed at `0x007122F0`, exact offset unresolved this pass) |
| `type.DeathWeaponDamageModifier` | `+0x???` (parsed at TechnoTypeClass::ReadINI) |
| `Rules.DeathWeapon` (default) | `Rules+0x???` (parsed at `0x0066C58A`) |
| `"Suicide"` string | `0x00843050` |
| `"DeathWeapon"` string | `0x0083B11C` |
| `"DeathWeaponDamageModifier"` string | `0x00844488` |
| `WeaponTypeClass::ReadINI` Suicide parse | `0x0077228D` |
| `TechnoTypeClass::ReadINI` DeathWeapon parse | `0x007122F0` |
| `RulesClass::ReadCombatDamage` DeathWeapon default | `0x0066C58A` |

---

## 10. TS-legacy filter

- **`Suicide=yes` weapon flag**: LIVE — 4 retail weapons use it.
- **`DeathWeapon=` per-type**: LIVE — many retail units use it.
- **`DeathWeaponDamageModifier=`**: LIVE — aircraft and Kirov calibrate crash damage.
- **`DefaultDeathWeapon` Rules global**: LIVE — fallback for units without explicit DeathWeapon.

No TS-only dead branches. Both mechanisms are fundamental to YR combat (Demo Truck, Terrorist, Crazy Ivan, Kirov are all core units).

---

## 11. Edge cases

| Case | Behavior |
|---|---|
| Demo Truck arrives at target (Suicide weapon fires) | Demobomb explodes, Truck dies, DeathWeapon=Demobomb fires again. Possible double-explosion. (See §4.) |
| Demo Truck destroyed mid-route by enemy fire | DeathWeapon=Demobomb fires at death position. Same Demobomb damage, no Suicide-trigger version. |
| Terrorist killed before reaching target | DeathWeapon=TerrorBomb fires immediately at Terrorist's position. Friendly units near may die. |
| Crazy Ivan placing a bomb (IvanBomb weapon, Suicide=yes) | IvanBomb warhead has `IvanBomb=yes` (priority 2 in detonate cascade) — attaches a bomb to target, doesn't damage immediately. Suicide flag effect unclear (see §3.2). |
| Aircraft shot down | DeathWeapon=BlimpBomb × 0.1 fires at crash position. Damages units below by 10% of BlimpBomb's full damage. |
| Aircraft killed in mid-air with no DeathWeapon | Crash deals "one laser blast's worth of damage" per INI comments — appears to be hardcoded engine default. |
| Mind-controlled Demo Truck attacks its original owner's base | Suicide fires normally; the dying truck's house = mind-controller's house, so DeathWeapon damage attributes to mind-controller. |
| `Rules.DefaultDeathWeapon` is missing or NULL | Untested; probably no DeathWeapon fires for units relying on the default. |
| Two suicide units (e.g., two Demo Trucks) arrive simultaneously | Each fires its own Suicide weapon. Each dies. Each may chain via DeathWeapon. Cell occupancy order determines apply order (see [`splash_cellspread.md`](splash_cellspread.md) §6). |
| Killing a barrel with explosive radius hits more barrels | Apply_area_damage's IC-barrel branch chains via C4Warhead (separate from DeathWeapon). Each chain step also triggers each barrel's DeathWeapon. |

---

## 12. Open follow-ups

1. **DeathWeapon dispatch site.** The exact function that reads `type.DeathWeapon` and dispatches `Apply_area_damage` at the death position is not traced this pass. Likely candidates: `TechnoClass::Killed`, `BuildingClass::Destroy`, or a per-class death handler. Priority: **HIGH** — needed for parity (Demo Truck, Terrorist, Kirov crash damage).
2. **IvanBomb + Suicide=yes interaction.** Does the Suicide flag actually fire for IvanBomb-warhead weapons, or is it a no-op due to the `IvanBomb` warhead-cascade priority? Priority: MEDIUM — affects whether Ivan's bomb placement disables Ivan.
3. **Demo Truck double-explosion verification.** Does the dying Demo Truck's DeathWeapon fire ON TOP of its Suicide weapon's damage, or is there a gate that prevents double-firing when the death cause was the Suicide itself? Priority: HIGH — affects single-truck damage output.
4. **Terrorist self-destruction trigger.** What causes a Terrorist to die when adjacent to target? Is it a Suicide weapon, a special anim trigger, or hardcoded death? Trace `[TERROR]` weapon definition and attack behavior. Priority: MEDIUM.
5. **`DeathWeaponDamageModifier` exact application point.** Is the modifier applied to `weapon.Damage` before Apply_area_damage, or to each per-target damage post-falloff? Priority: MEDIUM (affects damage scaling).
6. **`type.DeathWeapon` exact offset.** Not extracted in this pass. Trace TechnoTypeClass::ReadINI at `0x007122F0`. Priority: LOW.
7. **`Rules.DefaultDeathWeapon` semantics.** What does "Damage will be based on hitpoints" (per INI comment) mean exactly? The default weapon's effective damage scales with the dying unit's HP? Or its base Damage value is replaced with some HP-derived value? Priority: LOW.
8. **Aircraft "default crash damage" without DeathWeapon.** The INI comment mentions a hardcoded default for aircraft crashes if DeathWeapon is unset. Trace where this default damage is applied. Priority: LOW.

---

## 13. Sources

- Live xrefs (2026-05-17):
  - `"Suicide"` at `0x00843050` → `WeaponTypeClass::ReadINI 0x0077228D` (+ 2 other refs)
  - `"DeathWeapon"` at `0x0083B11C` → `RulesClass::ReadCombatDamage 0x0066C58A` + `TechnoTypeClass::ReadINI 0x007122F0`
  - `"DeathWeaponDamageModifier"` at `0x00844488` (parser TBD)
- Existing canonical doc: [`../../FIRE_AT_PIPELINE_GHIDRA_REPORT.md`](../../FIRE_AT_PIPELINE_GHIDRA_REPORT.md) — `weapon+0x144 Suicide` flag identity and Fire_At self-target short-circuit behavior.
- Existing canonical doc: [`../../WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md`](../../WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md).
- INI quotes from `ini/rulesmd.ini`:
  - line 871: `DeathWeapon=DefaultDeathWeapon` Rules default
  - lines 22377/24452/24461/24515: Suicide=yes weapons
  - 16+ retail unit/building DeathWeapon assignments
- Sister system docs: [`damage_formula.md`](damage_formula.md), [`splash_cellspread.md`](splash_cellspread.md) §8 (C4Warhead self-target gate), [`chain_reaction.md`](chain_reaction.md) §5 (barrel chain), [`warhead_detonate_dispatch.md`](warhead_detonate_dispatch.md) (when written, for warhead-priority cascade interactions with IvanBomb).
- Hardcoded-weapon docs (TODO, to cross-link when written):
  - [`weapons/DemoTruckSuicide.md`](../weapons/DemoTruckSuicide.md)
  - [`weapons/TerroristSuicide.md`](../weapons/TerroristSuicide.md)
  - [`weapons/CrazyIvanBombs.md`](../weapons/CrazyIvanBombs.md)
  - [`weapons/KirovBomb.md`](../weapons/KirovBomb.md)
