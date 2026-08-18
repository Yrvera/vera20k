# Guardian GI Deployed Crush Resistance and Movement Lock Trace

Scenario: Rhino Tank ordered through a Guardian GI at cell (50,50), first with the
GGI undeployed and then with the GGI fully deployed; then issue a move order to
the deployed GGI. Scope is stock Yuri's Revenge rules only.

Status: COMPLETE for code/doc/INI trace. No live Ghidra calls were made because
the checked docs already contain verified GGI-specific gamemd.exe evidence.
No Rust, INI, or in-repo docs were modified.

## Evidence Base

- Standard YR INI:
  - `ini/rulesmd.ini:3863` `[GGI]`
  - `ini/rulesmd.ini:3905` `Crushable=yes`
  - `ini/rulesmd.ini:3906` `DeployedCrushable=no`
  - `ini/rulesmd.ini:3898` `Deployer=yes`
  - `ini/rulesmd.ini:3899` `DeployFire=yes`
  - `ini/rulesmd.ini:7683` `[HTNK]`
  - `ini/rulesmd.ini:7699` `Crusher=yes`
  - `ini/rulesmd.ini:7717` `MovementZone=Destroyer`
- Art:
  - `ini/artmd.ini:291` `[GGI]`, `Sequence=GuardianGISequence`
  - `ini/artmd.ini:14156` `Deploy=300,15,0`
  - `ini/artmd.ini:14157` `Deployed=315,1,1`
  - `ini/artmd.ini:14160` `Undeploy=180,2,2`
- Verified docs:
  - `GGI_GHIDRA_REPORT.md:36-45`: GGI is stock YR, deployed GGI is uncrushable
    because `DeployedCrushable=no` drives runtime byte `InfantryClass+0x2A4`.
  - `GGI_GHIDRA_REPORT.md:73-80`: `Deployer=1`, `DeployedCrushable=0`, default
    is `DeployedCrushable=1`.
  - `GGI_GHIDRA_REPORT.md:171-175`: entering Deploy sets `+0x2A4=1` when
    `DeployedCrushable=no`; Undeploy completion clears it.
  - `GGI_GHIDRA_REPORT.md:535-549`: explicit move order to deployed GGI invokes
    undeploy sequence, then the queued move is accepted; passive scatter does not
    auto-undeploy.
  - `GGI_GHIDRA_REPORT.md:1342-1366`: deployed crush gate is the victim
    `+0x2A4` byte in `TechnoClass::CanCrushCheck @ 0x005f6cd0`.
  - `CRUSH_SYSTEM_GHIDRA_REPORT.md:18-29`: gamemd parser fields for
    `Crushable`, `Crusher`, `OmniCrusher`, `OmniCrushResistant`.
  - `CRUSH_SYSTEM_GHIDRA_REPORT.md:397-416`: allies not crushed by default,
    deployed infantry crush gate, distance gate 128 leptons.

## Pipeline

Command input -> rule/art data -> spawn runtime flags -> movement/path/cell
entry -> crush decision -> entity state/sound/death -> animation sequence ->
screen result.

## Stage Results

| Stage | Boundary | gamemd output | VERA output | Verdict |
|---|---|---:|---:|---|
| 1. INI constants | GGI/Rhino booleans | GGI `Crushable=1`, `DeployedCrushable=0`; HTNK `Crusher=1` | same parser inputs; `ObjectType::from_ini_section` reads `Crushable` and default-true `DeployedCrushable` at `src/rules/object_type.rs:965-968` | PASS |
| 2. Spawn/runtime copy | entity crush flags | GGI type has `+0xEC9=0`, runtime deploy gate can set `+0x2A4` | `world_spawn.rs:163-168` and `:363-366` copy `crushable` and `deployed_crushable` into `GameEntity` | UNCHECKED |
| 3. Undeployed crush eligibility | Rhino entering (50,50) | `CanCrushCheck` returns crush allowed when GGI `+0x2A4=0`, non-ally, not iron-curtained | `can_crush(Destroyer,false,Infantry,true,false,false)=true` at `src/sim/movement/bump_crush.rs:382-395` | PASS |
| 4. Undeployed visible result | survival/block | GGI is crushed; Rhino can pass through cell | `collect_crush_victims` includes the GGI and `movement_tick.rs:963-977` removes victim | PASS |
| 5. Fully deployed crush eligibility | Rhino entering (50,50) | `CanCrushCheck` returns 0 when deployed-uncrushable byte `+0x2A4=1` | `is_low_silhouette_for_crush` returns true for `DeployPhase::Deployed` and `deployed_crushable=false`; `can_crush` returns false | PASS |
| 6. Fully deployed blocking/survival | survival/block | Deployed GGI survives and blocks regular Rhino crush; tank cannot pass by crushing | `cell_passable_after_crush` returns false for uncrushable deployed GGI; enemy blocker path sets attack/wait at `movement_occupancy.rs:372-405` | PASS |
| 7. Move order to deployed GGI | command acceptance | Explicit move auto-starts Undeploy, then queued move is accepted after `Undeploy=180,2,2` completes | `world_commands.rs:131-137` returns false for any deployed phase before movement command logic; `deploy_tests.rs:276-297` asserts this rejection | FAIL |
| 8. Undeploy timing after move | move-order timing | 2-frame GGI undeploy sequence, then Ready and movement accepted | VERA computes GGI undeploy as 7 ticks for explicit toggle (`deploy_tests.rs:776-802`) but never starts it on Move | FAIL |
| 9. Render-driving deployed frame | visual sequence | fully deployed idle is GuardianGISequence `Deployed=315,1,1` | `animation.rs:442-454` switches deployed state to `SequenceKind::Deployed`; exact rendered frame/pixel output not computed | UNCHECKED |
| 10. Final screen result | player-visible comparison | undeployed: squished; deployed: survives/blocks; move order: undeploys then walks | VERA: undeployed squished, deployed survives/blocks, move order produces no visible undeploy or movement | FAIL |

## Findings

### FAIL - Move order does not auto-undeploy deployed GGI

Player-visible difference: in gamemd.exe, a move order on a deployed GGI starts
Undeploy and then the GGI moves. In VERA, the command is rejected immediately, so
the GGI remains deployed and stationary.

Code evidence:

- `src/sim/world/world_commands.rs:131-137` rejects `Command::Move` when
  `e.is_deployed()` is true.
- `src/sim/game_entity.rs:469-472` defines `is_deployed()` as any deploy phase.
- `src/sim/deploy_tests.rs:276-297` locks this behavior in with
  `move_silently_ignored_on_deployed`.

gamemd evidence:

- `GGI_GHIDRA_REPORT.md:535-545`: explicit move command to deployed GGI invokes
  Scatter, switches to `Set_Sequence(0x1F)` Undeploy, clears the deployed crush
  byte on completion, and then accepts the queued move.

### FAIL - Move-triggered undeploy timing is absent

Player-visible difference: gamemd shows the `Undeploy=180,2,2` animation before
movement. VERA only runs that 7-tick undeploy timing for explicit
`ToggleInfantryDeploy`, not for a normal move order.

Code evidence:

- `src/sim/world/world_commands.rs:538-545` starts `Undeploying` only on
  `Command::ToggleInfantryDeploy`.
- `src/sim/deploy_tests.rs:776-802` computes GGI explicit-toggle undeploy as
  7 ticks from 2 frames.

gamemd evidence:

- `GGI_GHIDRA_REPORT.md:543-545`: GGI move-order undeploy uses
  `Undeploy=180,2,2`, then Ready, then movement.

### PASS - Deployed crush resistance for the concrete Rhino/GGI case

Player-visible match: Rhino can crush undeployed GGI, but cannot crush a fully
deployed GGI with `DeployedCrushable=no`.

Code evidence:

- `src/rules/object_type.rs:965-968` parses `Crushable` and `DeployedCrushable`
  with the correct default.
- `src/sim/world/world_spawn.rs:163-168` and `:363-366` copy those values into
  runtime entities.
- `src/sim/movement/bump_crush.rs:382-395` lets regular crusher zones crush only
  crushable infantry that are not low-silhouette.
- `src/sim/movement/bump_crush.rs:401-413` treats a fully deployed,
  `deployed_crushable=false` infantry entity as low-silhouette for crush.

gamemd evidence:

- `GGI_GHIDRA_REPORT.md:41-44` and `:1342-1366` verify the deployed GGI
  uncrushable gate.

## Adjacent Findings

- Potential adjacent issue, not traced here: gamemd sets the deployed-uncrushable
  byte when entering Doing 0x1B Deploy, while VERA's `is_low_silhouette_for_crush`
  does not include `DeployPhase::Deploying`. A Rhino arriving during the deploy
  animation may therefore crush in VERA when gamemd would not. This is outside
  the concrete "fully deployed GGI at (50,50)" scenario.
- E1/GI deployed crushability remains an older open doc question and was not
  traced here.
- Friendly-crush behavior was not traced; standard gamemd avoids allied crush by
  default, but this scenario did not specify same-owner Rhino/GGI.

## Verdict Tally

PASS: 5 | FAIL: 3 | UNCHECKED: 2 | NOT-IMPLEMENTED: 0

## Status

COMPLETE
