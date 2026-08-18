# YAPOWR InfantryAbsorb ExtraPower Bonus — Design

## Goal

Reproduce gamemd's `GetPowerOutput` InfantryAbsorb/UnitAbsorb path so a Yuri
Bio-Reactor (YAPOWR) with `ExtraPower=100`, `InfantryAbsorb=yes`,
`Passengers=5` contributes `(Power + ExtraPower × OccupantCount) × HealthRatio`
instead of the current `Power × HealthRatio`.

## Architecture Context

### Stock-YR power pipeline (verified)

`BuildingClass::GetPowerOutput` at `0x44E7B0`
(verified via `decompile_function 0x0044E7B0`, 2026-05-20):

```
base = Type->Power (+0xee0)
if not InLimbo:
    if this->IsOverpowered (+0x668):                # upgrade-attached flag
        base += Type->ExtraPower (+0xee8)
    if (Type->UnitAbsorb || Type->InfantryAbsorb)   # +0x16ae / +0x16af
       && Type->ExtraPower > 0
       && this->OccupantCount (+0x114) > 0:
        base += Type->ExtraPower * OccupantCount
    if this->UpgradeLevel != 0:
        for slot in 0..3:
            if slot != NULL: base += slot->Power
    if base > 0 && this->HasPower (+0x660):
        return ftol(base * GetHealthRatio())
return 0
```

`BuildingClass::IsOperational` at `0x4555D0`
(verified via `decompile_function 0x004555D0`, 2026-05-20):

```
if !HasPower && UpgradeCount (+0x67C) < 2:  return false
... (EMP, health, power-deficit, PoweredSpecial, NeedsEngineer, mission)
```

### Why this scope is correct for stock YR

Stock `ini/rulesmd.ini` defines **no** `PowersUpBuilding=` add-on buildings —
only the commented-out template at line 3659. The Battle Lab, War Factory,
Barracks, Power Plant, and every other building reach `UpgradeCount = 0`
forever in normal play.

Consequence: the four lines of GetPowerOutput marked
`if this->IsOverpowered` and `if UpgradeLevel != 0` are dormant TS/RA2-era
plumbing. The only live extra-power path in stock YR is the
`InfantryAbsorb/UnitAbsorb × OccupantCount` branch, fired by YAPOWR every Yuri
match where the Bio-Reactor is garrisoned.

`IsOperational`'s `UpgradeCount >= 2` bypass is similarly dormant in stock YR.

### Rust state today

- `src/sim/power_system.rs:58-107` (`recalculate_power_for_owner`) reads only
  `obj.power` and health-scales it.
- `src/rules/object_type.rs:222` has `pub power: i32`, `pub powered: bool`
  (line 659), `pub infantry_absorb: bool` (line 598), `pub unit_absorb: bool`
  (line 602), `pub max_number_occupants: u32` (line 566). No `extra_power`
  field.
- `src/sim/game_entity.rs:224` has `pub passenger_role: PassengerRole`. Cargo
  count via `entity.passenger_role.cargo().map_or(0, |c| c.count())`.
- INI parser in `ObjectType::from_section` already covers `InfantryAbsorb`
  and `UnitAbsorb` (`object_type.rs:1017-1018`); `ExtraPower` is unread.

## Impact Analysis

- **Files changed**:
  - `src/rules/object_type.rs` — add `extra_power: i32` field, parse
    `ExtraPower=` as signed integer (default 0).
  - `src/sim/power_system.rs` — extend `recalculate_power_for_owner` with the
    occupant-bonus computation; add unit tests.
- **Downstream**:
  - `is_building_powered`, `has_active_radar`, `tick_power_states`: no
    signature change; consumes updated `total_output`.
  - `theoretical_total_power` (sidebar curve input): intentionally untouched.
    It already sums `|Power=|` from TypeClass only; the actual `total_output`
    reflects the bonus and feeds the green-bar fill via the asymptotic curve.
- **Determinism**: pure integer math, no float, no map iteration order change.
  State hash will shift only for ticks where a YAPOWR has at least one
  passenger. No existing test exercises that scenario, so the 2435-test suite
  should remain green.
- **Migration**: none. New field defaults to 0, so any TypeClass that doesn't
  parse `ExtraPower=` behaves identically to today.

## Chosen Approach

**Approach A — inline in `recalculate_power_for_owner`.** The bonus has a
single consumer; the gate conditions are explicit; the surrounding loop is
short enough to keep the parity-critical math co-located with the drain
branch. Per CLAUDE.md "don't add abstractions beyond what the task requires."

Rejected alternative: extracting a `building_power_output(entity, obj) -> i32`
helper. Worth it only if the upgrade-slot iteration ever goes live (mods).
Stock YR has nothing to consume it, so the helper is premature.

## Tiny-Detail Ledger

Source key: `[GHIDRA 0xADDR]` = verified via `decompile_function` in this
session. `[doc §X]` = `POWER_SYSTEM_GHIDRA_REPORT.md` (GREEN, 2026-05-20).

- **Gate**: bonus fires iff
  `(obj.infantry_absorb || obj.unit_absorb) && obj.extra_power > 0 && occupant_count > 0`.
  All three are strict (`!= 0` for the bools, `> 0` for the two integers).
  `[GHIDRA 0x44E7B0]`
- **Formula**: `bonus = extra_power × occupant_count` (signed `i32 * i32`).
  `[GHIDRA 0x44E7B0]`
- **Ordering**: bonus added to `base` **before** health scaling.
  `(base + bonus) × hp / max_hp` integer-divided. `[GHIDRA 0x44E7B0]`
- **Health-scaling truncation**: gamemd uses
  `ftol((float80)Health/(float80)Strength × base)`; Rust's
  `base × hp / max_hp` integer division rounds toward zero, equivalent for
  positive operands. `[doc §Health ratio precision]`
- **`ExtraPower=` is signed i32**, default 0 absent, source
  `BuildingTypeClass+0xee8`. `[doc §BuildingTypeClass Upgrade Fields]`
- **`InfantryAbsorb` / `UnitAbsorb` byte flags** at `TypeClass+0x16AF` /
  `+0x16AE`. Already parsed as `obj.infantry_absorb` / `obj.unit_absorb`.
  `[GHIDRA 0x44E7B0; existing Rust]`
- **OccupantCount source**: gamemd `BuildingClass+0x114` is one counter for
  both InfantryAbsorb and UnitAbsorb buildings. Rust uses one
  `PassengerCargo.passengers: Vec<u64>` per building; both infantry and
  vehicle passengers go in the same list. Equivalent. `[GHIDRA + existing Rust]`
- **GetPowerOutput returns 0** if `base + bonus <= 0` OR building is offline.
  Rust's existing `if entity.building_up.is_some() { continue; }` skip
  matches "not yet online" for the duration of construction.
  `[GHIDRA 0x44E7B0]`
- **Drain branch unaffected**: gamemd's GetPowerDrain has no occupant path in
  stock YR (only IsChargeDraining for GapGenerator and upgrade slots).
  Drain in Rust stays `power.saturating_abs()` when `power < 0`.
  `[doc §GetPowerDrain]`
- **Stock-YR concrete fixture**: YAPOWR `Power=150`, `ExtraPower=100`,
  `Passengers=5`:
  - 100% HP, 0 passengers → `150`
  - 100% HP, 5 passengers → `150 + 500 = 650`
  - 50% HP, 5 passengers → `650 × 200/400 = 325`
  - 100% HP, 3 passengers → `150 + 300 = 450`
- **No stock-YR building has `InfantryAbsorb=yes` AND `Power < 0`.** Rust's
  single-direction `power` field model (positive = output, negative = drain)
  is sufficient for stock parity. `[grep of rulesmd.ini, 2026-05-20]`
- **No upgrade-slot scaffolding** added in this design. `UpgradeCount`,
  `IsOverpowered`, the 3 slot pointers, and `PowersUpBuilding=` parsing
  remain absent. The disparity-scan G2/G3 items will be demoted to
  TS-ghost-not-in-stock in a separate doc patch (see Follow-ups).

## Design

### Components

- `ObjectType::extra_power: i32` — new field, parsed from `ExtraPower=`
  signed integer.
- `recalculate_power_for_owner` — extended with the bonus branch.

### Data Flow

Per structure entity owned by the player, each tick:

```
base_output = max(obj.power, 0)
if (obj.infantry_absorb || obj.unit_absorb) && obj.extra_power > 0:
    occupants = entity.passenger_role.cargo().map_or(0, |c| c.count()) as i32
    if occupants > 0:
        base_output += obj.extra_power * occupants
if base_output > 0:
    produced += base_output * hp / max_hp
if obj.power < 0:
    drained += obj.power.saturating_abs()
```

The `theoretical_total_power` accumulator stays on `|obj.power|` from
TypeClass (no occupant contribution).

The spy-blackout `produced = 0` override at the end of the function still
runs (forces output to 0 regardless of any bonus).

### Interfaces / Contracts

No public signature changes. `recalculate_power_for_owner` is private; only
its caller `tick_power_states` uses it.

### Error Handling

None needed. Missing `ExtraPower=` defaults to 0 (no bonus). Missing
`infantry_absorb`/`unit_absorb` defaults to `false` (gate fails).

### Testing Strategy

Add unit tests in `power_system.rs` covering:

1. **YAPOWR empty** — `Power=150`, `ExtraPower=100`, `InfantryAbsorb=yes`,
   no passengers → `total_output = 150`.
2. **YAPOWR garrisoned at full HP** — 5 passengers → `total_output = 650`.
3. **YAPOWR garrisoned at half HP** — 5 passengers, HP=200/400 →
   `total_output = 325`.
4. **InfantryAbsorb=no but ExtraPower set** — bonus suppressed
   (gate fails) → output stays at `power`.
5. **InfantryAbsorb=yes but ExtraPower=0** — no bonus (strict `> 0`).
6. **InfantryAbsorb=yes but ExtraPower=-50** — no bonus (strict `> 0`).
7. **UnitAbsorb=yes path** — same bonus formula (verify gate alternative).
8. **Pin test against existing YAPOWR-shaped fixture during construction** —
   `building_up.is_some()` continues to suppress all output including bonus.

Test fixtures use `make_building` helper + `PassengerCargo` injection on the
entity's `passenger_role`.

### Determinism Considerations

- Integer math throughout.
- Per-entity iteration order in `recalculate_power_for_owner` already
  deterministic (EntityStore = BTreeMap).
- The bonus is a function of `(obj.power, obj.extra_power, obj.infantry_absorb,
  obj.unit_absorb, entity.passenger_role.cargo().count(), hp, max_hp)` —
  all already part of the deterministic sim state. State hash will move on
  the next tick after a passenger enters/exits a YAPOWR, which is the
  correct behavior.

## Architectural Decisions

- **Inline the bonus, don't extract a helper.** Single consumer; mirroring
  gamemd's `GetPowerOutput` function shape via a Rust helper would be
  speculative scope (no current need for upgrade slots).
- **Skip upgrade-slot scaffolding entirely.** Dormant in stock YR. Per
  CLAUDE.md "don't design for hypothetical future requirements" and "Watch
  for Tiberian Sun ghosts."
- **Skip UpgradeCount bypass in `is_building_powered`.** Same reason: in
  stock YR, `UpgradeCount` cannot reach 1 let alone 2. The bypass is
  unreachable code.
- **Tech debt explicitly accepted**: when/if a future YR mod with live
  `PowersUpBuilding=` add-ons is in scope, the upgrade-slot iteration and
  bypass will need to be added. The disparity-scan doc records this so it's
  not forgotten.

## Alternatives Considered

- **Approach B (extract `building_power_output` helper)** — rejected for
  premature abstraction. Single call site; gamemd-mirror shape pays off only
  with upgrade-slot work which is out of scope.
- **Full upgrade-system implementation (G2 + G3 as originally written)** —
  rejected. Stock YR has no `PowersUpBuilding=` add-ons. Building the
  production attach flow, 3 slot pointers, UpgradeCount tracking, and
  IsOverpowered flag would add hundreds of lines with zero observable effect
  in stock skirmish. Per CLAUDE.md, the parity bar is observable behavior,
  not internal completeness.

## Follow-ups (deferred, NOT cut)

- **Patch `docs/gap-scans/2026-05-20-disparity-scan-power-system.md`**: demote
  G2 and G3 from HIGH-active to "TS-ghost not active in stock YR" with
  Ghidra citations. Promote a new finding describing the YAPOWR
  InfantryAbsorb ExtraPower bonus as the actually-live gap (this design
  closes it).
- **G5 (PoweredSpecial HasOccupiedPowerPlant gate)** — separate brainstorm;
  live in stock YR (disables superweapons when your own reactor is
  garrisoned).
- **`theoretical_total_power` semantics with garrisoned reactor** — needs a
  Ghidra check of what the sidebar bar fill curve reads; deferred until G5
  or sidebar-bar work surfaces it.
