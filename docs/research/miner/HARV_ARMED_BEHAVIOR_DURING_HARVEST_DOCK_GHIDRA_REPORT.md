# HARV Armed Behavior During Harvest/Dock - Ghidra Research Report

Reverse-engineered from `gamemd.exe` via Ghidra MCP, with stock YR data checked
against `ini/rulesmd.ini` and base fallback `ini/rules.ini`.

## Investigation Mode

Focused `/re-investigate` slice for one War Miner question. Read-only binary and
INI evidence only. No Rust, INI, or existing research-doc edits were made.

## Target Question

Verify whether the stock Soviet War Miner (`[HARV]`) can acquire, retaliate, and
fire `Primary=20mmRapid` while:

- searching ore
- moving to ore
- harvesting on ore
- returning to refinery
- queued/approaching dock
- pivoting/unloading
- exiting refinery dock

Focus is mission/weapon gates, not projectile, warhead, damage, or visual math.

## Non-Goals

- Chrono Miner (`CMIN`) weaponless teleport behavior.
- Slave Miner (`SMIN`) behavior.
- `20mmRapid` damage, armor verses, projectile flight, impact, or animation math.
- Player manual attack-command ordering during refinery lock.
- Pixel/timing trace of the turret while physically inside the refinery dock.

## Evidence Needed To Mark COMPLETE

- Mission ids for harvest, enter/dock, unload, guard.
- Active YR path from normal unit AI into mission dispatch, target acquisition,
  fire processing, and damage retaliation.
- Stock `[HARV]` weapon/default flags.
- Weapon gates for passive acquisition, retaliation, and firing.
- Dock/unload gates that disable new passive acquisition or clear opportunistic
  targets.

## Stop Conditions

- Stop after mission/weapon gates are resolved for stock `[HARV]`.
- Stop before implementing Rust behavior.
- Stop before broad manual-target or rendered-pixel validation.

## Stock Data Findings

Verified stock YR `[HARV]` values from `rulesmd.ini`:

- `Turret=yes`
- `Primary=20mmRapid`
- `ElitePrimary=20mmRapidE`
- `Harvester=yes`
- `Storage=40`
- `OpportunityFire=yes`
- `UnloadingClass=HORV`
- `Dock=NAREFN,GAREFN`
- No `CanPassiveAquire=no` override.
- No `CanRetaliate=no` override.

Verified stock YR `[20mmRapid]`:

- `Damage=30`
- `ROF=20`
- `Range=5.5`
- `Projectile=InvisibleLow`
- `Speed=100`
- `Warhead=HARVWH`
- `Report=WarMinerAttack`
- `Anim=GUNFIRE`

Verified mission-control data from `rulesmd.ini`:

- `[Guard]` has no `Retaliate=no`, so it inherits default retaliate allowed.
- `[Enter]` has `Retaliate=no`.
- `[Harvest]` has `Retaliate=no`.
- `[Unload]` has `Retaliate=no`.

Active in YR: yes. These are stock YR rules for the normal Soviet War Miner, not
mod data and not CMIN-specific data.

## Verified Binary Findings

### Mission ids and active AI path

Verified mission ids from the mission table and prior harvest report:

| Id | Name | Relevant use |
|----|------|--------------|
| 5 | Guard | idle/no-ore guard path |
| 7 | Enter | refinery approach/enter handoff |
| 10 | Harvest | ore search, move-to-ore, harvest, return-to-refinery |
| 16 | Unload | refinery dock/unload/depart flow |

Active in YR: yes. `UnitClass::AI` calls `FootClass::AI`, which reaches the
normal techno AI and mission dispatch path for live units, then `UnitClass::AI`
continues into unit-specific fire/facing/turret processing.

### Mission 10 harvest does not disable combat by itself

`UnitClass::Mission_Harvest` is mission id 10. Its state machine handles ore
search, movement to ore, cell harvesting, return-to-refinery, docking request,
and no-ore fallback. It does not call target acquisition directly, but it also
does not clear the target or disable firing.

Active in YR: yes. `[HARV]` has `Harvester=yes`, so the normal harvester branch is
live for the stock War Miner.

### Passive/opportunity acquisition is active on mission 10

`TechnoClass::AI_Update` runs an opportunistic target-acquisition pass only for
missions 2, 10, and 5, guarded by the helper reached through `FUN_00709290` and
`FUN_007091D0`.

The acquisition gate includes:

- `CanPassiveAquire` at the techno type.
- a can-fire/has-weapon style virtual check.
- mission-specific opportunity-fire/movement details.

For stock `[HARV]`, the relevant defaults and data pass this path:

- no `CanPassiveAquire=no` override, so stock default remains enabled.
- `Primary=20mmRapid`, so it has a usable weapon.
- `OpportunityFire=yes`, so the stock War Miner passes opportunity-fire-sensitive
  movement gating.

Active in YR: yes. Mission 10 is explicitly included in the binary's acquisition
mission set.

### Firing is processed globally by UnitClass AI

`UnitClass::AI` calls `UnitClass::Fire_At_Target` every normal tick after
`FootClass::AI`. `UnitClass::Fire_At_Target` checks the current target, selects a
weapon, asks the target/action gate whether firing is possible, and then calls
the actual fire virtual when the result permits it.

There is no `Harvester=yes`, Mission 10, ore-storage, or refinery-return gate in
this fire function. Harvesting is not a special "weapon disabled" state.

Active in YR: yes. This path is the normal live `UnitClass` tick path.

### Mission 7 Enter and mission 16 Unload do not start new passive acquisition

`TechnoClass::AI_Update` only starts the passive/opportunity acquisition pass for
missions 2, 10, and 5. Mission 7 (`Enter`) and mission 16 (`Unload`) are not in
that set.

The same AI update path has a target-clear list for several non-opportunity
missions when the target was marked as opportunistically changed (`field_0x50c`).
That list includes mission 7 and mission 16, and does not include mission 10.

Active in YR: yes. Mission 7 is used by the harvester refinery-approach handoff;
mission 16 is used by the refinery unload/depart state machine.

### Retaliation is disabled by mission-control during harvest/dock/unload

`TechnoClass::ShouldRetaliate` checks both:

- techno type retaliation permission (`CanRetaliate`), and
- current mission-control retaliation permission.

Stock `[HARV]` does not set `CanRetaliate=no`, but mission-control overrides still
matter. In stock YR data:

- `[Harvest]` has `Retaliate=no`.
- `[Enter]` has `Retaliate=no`.
- `[Unload]` has `Retaliate=no`.

Therefore damage-triggered retaliation is disabled while the War Miner is on
mission 10, mission 7, or mission 16, even though passive acquisition/fire may be
active on mission 10.

Active in YR: yes. The damage path calls `TechnoClass::ShouldRetaliate`; the
mission-control table is stock YR data.

### Guard/no-ore behavior is different

`UnitClass::Mission_Guard_Harvester` can put harvesters back into mission 10 for
AI harvesting, but otherwise delegates to `FootClass::Mission_Guard`.

Mission 5 (`Guard`) is included in `TechnoClass::AI_Update` passive acquisition.
Stock `[Guard]` does not set `Retaliate=no`, so damage retaliation is allowed by
mission-control if other target/weapon gates pass.

Active in YR: yes.

## State Answer

| HARV phase | Binary mission | Passive acquire? | Damage retaliate? | Fire `20mmRapid`? |
|------------|----------------|------------------|-------------------|-------------------|
| searching ore | 10 Harvest | Yes | No, `[Harvest] Retaliate=no` | Yes, if target/action/weapon gates pass |
| moving to ore | 10 Harvest | Yes | No, `[Harvest] Retaliate=no` | Yes, if target/action/weapon gates pass |
| harvesting on ore | 10 Harvest | Yes | No, `[Harvest] Retaliate=no` | Yes, if target/action/weapon gates pass |
| returning to refinery | 10 Harvest | Yes | No, `[Harvest] Retaliate=no` | Yes, if target/action/weapon gates pass |
| queued before mission changes to dock | 10 Harvest state 3 | Yes for that Mission 10 tick | No | Yes, if still on mission 10 and gates pass |
| approaching/entering dock | 7 Enter | No new passive acquisition | No, `[Enter] Retaliate=no` | Retained or explicit target path not fully traced; do not assume new opportunistic fire |
| pivoting/unloading | 16 Unload | No new passive acquisition | No, `[Unload] Retaliate=no` | Retained or explicit target path not fully traced; no verified opportunistic fire |
| exiting while still unload/depart mission | 16 Unload | No new passive acquisition | No | Retained or explicit target path not fully traced |
| after unload returns to harvest loop | 10 Harvest | Yes | No | Yes, if target/action/weapon gates pass |
| no-ore/idle guard fallback | 5 Guard | Yes | Yes, if other gates pass | Yes, if target/action/weapon gates pass |

## Inference

The stock War Miner is not "retaliating" while harvesting in the mission-control
sense. It can still shoot during harvest because passive/opportunity acquisition
on mission 10 can assign a target, and `UnitClass::Fire_At_Target` is run by the
global unit AI path. In gameplay terms, this can look like retaliation if an enemy
comes into range during harvesting, but the damage-triggered retaliation path is
disabled for mission 10.

Dock/unload should be treated as a separate phase. The binary does not start new
passive acquisition for mission 7 or 16, and it has an opportunistic target-clear
gate for those missions. That is strong evidence against implementing "auto-acquire
while docked/unloading." The remaining uncertainty is only whether a pre-existing
explicit/manual target can continue to fire through some dock frames.

## Coverage Ledger

| Area | Coverage | Result |
|------|----------|--------|
| Stock `[HARV]` flags | Complete | Armed, turreted, harvester, opportunity fire enabled |
| Stock `[20mmRapid]` | Complete | Weapon exists and has 5.5-cell range |
| Mission ids | Complete | Harvest=10, Enter=7, Unload=16, Guard=5 |
| Mission 10 state machine | Complete for combat gates | No local fire suppression |
| AI passive acquisition | Complete for mission gate | Missions 2/10/5 only |
| Fire path | Complete for harvester gate | Global unit fire path, no harvest-state block found |
| Retaliation | Complete for stock missions | Harvest/Enter/Unload mission-control `Retaliate=no` |
| Dock manual/retained target | Partial | Global fire call exists, but opportunity target clearing makes stock opportunistic firing unverified |

## Implementation Handoff

- Parse and store `CanPassiveAquire`, `CanRetaliate`, and `OpportunityFire` if the
  Rust rules layer does not already expose them.
- Do not disable combat because an entity is in miner states corresponding to
  Mission 10: search ore, move to ore, harvest on ore, or return to refinery.
- Allow stock `[HARV]` to pass passive acquisition during Mission 10-equivalent
  miner states when it has a weapon and `CanPassiveAquire` is enabled.
- Allow `UnitClass::Fire_At_Target` equivalent behavior during Mission 10 miner
  states; the fire gate should be target/action/weapon based, not miner-state
  based.
- Model damage retaliation separately from passive acquisition. For stock mission
  equivalents of Harvest, Enter, and Unload, retaliation should be disabled by
  mission-control even though Mission 10 passive acquisition is enabled.
- During Mission 7/16 equivalents (dock approach, pivot, unload, depart), do not
  start new passive acquisition just because `[HARV]` has `Primary=20mmRapid`.
- Clear or ignore opportunistically acquired targets on Mission 7/16 equivalents
  once Rust models the binary's `field_0x50c` style opportunity-target marker.
- After unload/depart returns to Mission 10-equivalent harvest loop, re-enable
  Mission 10 passive acquisition/fire behavior.

Concrete Rust test-name proposals:

- `war_miner_passively_acquires_enemy_while_searching_ore`
- `war_miner_passively_acquires_enemy_while_moving_to_ore`
- `war_miner_fires_20mmrapid_while_harvesting_cell`
- `war_miner_fires_while_returning_to_refinery_without_canceling_return`
- `war_miner_does_not_damage_retaliate_while_on_harvest_mission`
- `war_miner_does_not_passively_acquire_new_target_during_dock_enter`
- `war_miner_does_not_passively_acquire_new_target_while_unloading`
- `war_miner_reenables_passive_acquire_after_unload_returns_to_harvest`
- `war_miner_guard_fallback_can_retaliate_when_mission_control_allows`
- `can_passive_aquire_no_blocks_modded_war_miner_auto_acquire`
- `opportunity_fire_yes_allows_war_miner_fire_while_moving`

## Negative Facts / Do Not Do

- Do not make miner state disable War Miner combat during Mission 10-equivalent
  harvest/search/return behavior.
- Do not require Mission Attack for stock War Miner harvest firing. The fire call
  is in the normal unit AI tick.
- Do not implement harvest damage retaliation as enabled for stock `[HARV]`;
  `[Harvest] Retaliate=no` disables the damage-triggered retaliation path.
- Do not let dock approach or unload start new passive acquisition. Mission 7 and
  mission 16 are not in the binary passive-acquire mission set.
- Do not use the ore-harvesting active flag as a weapon gate. It is not the fire
  permission flag in the verified path.
- Do not apply this conclusion to Chrono Miner. CMIN has separate teleport and
  weaponless behavior.
- Do not hardcode `20mmRapid` details in simulation logic; use parsed rules.

## Remaining Uncertainty

- Exact same-frame behavior when a Mission 10 opportunistic target exists and the
  War Miner transitions into mission 7 is not pixel-traced here.
- Manual/explicit attack target retention during mission 7 or mission 16 was not
  fully traced. The global `UnitClass::Fire_At_Target` call can still run, but
  opportunistic target clearing for mission 7/16 means stock auto-fire while locked
  into dock/unload is not verified.
- Turret visual cadence, barrel facing, and dock overlay occlusion were not part
  of this investigation.

## Stale-Doc Wording

Use this wording to replace broader or ambiguous claims:

"War Miner can passively acquire and fire during the Mission_Harvest portions of
the ore cycle (search/move/harvest/return) because `TechnoClass::AI_Update`
includes mission 10 in its opportunity-acquire pass and `UnitClass::AI` calls
`UnitClass::Fire_At_Target` every tick. This is not damage retaliation: stock
`[Harvest]` mission-control has `Retaliate=no`. Passive acquisition is not active
during Mission_Enter (7) or Mission_Unload (16); any retained/manual target
behavior during the dock lock needs runtime trace before claiming 'fires while
docked.'"

## Open Questions Final State

- Does a pre-existing manual target keep firing during every refinery dock frame?
  Deferred to trace-action/runtime verification.
- Does target-clear timing occur before or after one possible same-frame dock shot
  on the mission 10 to mission 7 transition? Deferred to a frame trace.

## Status

COMPLETE for stock War Miner mission/weapon gates. Deferred only for retained or
manual target frame behavior during dock/unload visuals.
