# DAT_00AC167C / FUN_005FF250/850/390/FFA0 Effect List - Reswarm Research Report

**Address(es):** `DAT_00AC1678..00AC168C` vector header, `DAT_00AC167C` pointer array, `DAT_00AC1688` count, `FUN_005FF250`, `FUN_005FF2D0`, `FUN_005FF390`, `FUN_005FF850`, `FUN_005FFFA0`, producer helper `FUN_0048A620`, direct producer callers `BuildingLightClass::Draw_It @ 0x00435BE0`, `ParticleSystemClass` one-frame helper `0x0062E280`, `ParticleSystemClass::AI_Spark @ 0x0062E840`.
**Investigation Mode:** exhaustive-slice for direct `0x005FF250/850/390/FFA0` callers and vector mutation semantics; coverage-map for the broader `0x0048A620` producer xref census.
**Claimed Scope:** Classify the `DAT_00AC167C` list owner/class identity, producer families, mutation semantics, and whether non-spark producer timing can make first visible stage differ from persistent SparkSys.
**Non-Scope:** Pixel-perfect blitter math inside `0x005FF850`, full `0x0048A620` INI/parser formulas already covered by the combat-light report, and global PerTick ladder rediscovery.
**Confidence:** High for direct caller census, vector layout/mutation, stage/lifetime, and immediate-vs-persistent producer split. Medium for the full `0x0048A620` indirect caller taxonomy because one raw xref is in an undefined function boundary.
**Status:** COMPLETE for the scoped direct list owner/producers and handoff; the broader raw `0x0048A620` xref is listed as Remaining Uncertainty.

## Working Notes Gate

- **Target question:** What owns `DAT_00AC167C`, who creates/draws/ages/removes entries through `0x005FF250/850/390/FFA0`, and do non-spark producers have first-stage ordering different from persistent SparkSys?
- **Non-goals:** Do not re-prove the global tick ladder; do not patch Rust or published docs; do not rename/mutate Ghidra; do not drain low-level pixel blending beyond producer/list identity.
- **Evidence needed to mark COMPLETE:** direct xrefs/callers for `0x005FF250/850/390/FFA0`, decompile plus disassembly for list mutation, producer decompile for every direct `0x005FF250` caller, and Rust-facing surface scan.
- **Stop conditions:** Stop once every direct list function and direct constructor caller is classified; record indirect `0x0048A620` caller breadth without expanding each caller body unless it changes list ownership or first-stage timing.

## 1. Overview

`DAT_00AC167C` is the data pointer inside a global dynamic vector header rooted at `0x00AC1678`. The entries are heap-allocated 0x18-byte screen-space light/glow primitives, not `AnimClass`, not `SmudgeClass`, and not ordinary map/building `LightSourceClass` objects. The same primitive family serves three producer modes:

1. **Persistent transient entries**: `FUN_0048A620` combat/flash helper and `ParticleSystemClass::AI_Spark` allocate an entry and leave it in the vector. `FUN_005FF390` ages it by `+8` per logic update and removes/frees it when the new stage is `>= 0x50`. `FUN_005FFFA0` draws live vector entries in reverse order.
2. **Immediate draw-and-free entries**: `BuildingLightClass::Draw_It` and `ParticleSystemClass` one-frame spark light allocate, mutate stage where needed, call `FUN_005FF850` directly, remove through `FUN_005FF2D0`, then free. These do not survive to the next `FUN_005FF390`.
3. **Cleanup entries**: scenario cleanup drains `DAT_00AC167C` by repeatedly removing/freeing the first vector entry.

Persistent SparkSys first draws at stage `0` because `FUN_005FF390` already ran earlier in that logic pass before live object particle AI creates the entry. Non-spark persistent combat-light producers share that first-stage `0` only when their producer runs after `FUN_005FF390`; active inspected PerTick producers do. Immediate non-spark `BuildingLightClass` is different: it sets stage `0x50..0x59`/`0x50`, draws immediately during object rendering, and removes the entry before the persistent draw/update lifetime can observe it.

## 2. Vector Header / Entry Layout

| Field | Address / offset | Verified role | Evidence | Active in YR |
|---|---:|---|---|---|
| vector vtable | `0x00AC1678` | dynamic vector helper vtable; init writes `0x007EF6BC`, teardown writes `0x007EF6DC` | assembly `0x005FF1ED`, `0x005FF218` | Yes |
| data pointer | `0x00AC167C` | pointer array of 0x18 light/glow objects | reads/writes `0x005FF2C0`, `0x005FF39F`, `0x005FFFAD` | Yes |
| capacity | `0x00AC1680` | maximum slot count before grow | constructor compare `0x005FF278..0x005FF285`, init clear `0x005FF1DC` | Yes |
| ownership/grow flags | `0x00AC1684/85` | vector allocation flags used by grow/free decisions | init writes `+0x0C=1`, `+0x0D=0`; free guard `0x005FF226..0x005FF237` | Yes |
| count | `0x00AC1688` | live entry count | append increments `0x005FF2B1..0x005FF2C6`; update/draw read `0x005FF391`, `0x005FFFA0` | Yes |
| grow step | `0x00AC168C` | initial/next grow request, initialized to `10` | `0x005FF1F7`; grow call uses `capacity + grow_step` at `0x005FF291..0x005FF2AA` | Yes |
| entry coords | entry `+0x00/+0x04/+0x08` | world coords passed to Tactical coords-to-client conversion | constructor `0x005FF25D..0x005FF26E`; draw `0x005FF87F..0x005FF8B8` | Yes |
| entry stage/index | entry `+0x0C` | draw table index/stage; persistent entries start `0`; update adds `8` | constructor `0x005FF268`; update `0x005FF3A8..0x005FF3B3`; draw index `0x005FF8ED..0x005FF93F` | Yes |
| entry size/radius | entry `+0x10` | scales draw table index when stage byte is `<0x40` | constructor `0x005FF271..0x005FF275`; draw `0x005FF90F..0x005FF93F` | Yes |
| entry flags | entry `+0x14` | low bits select darken/channel-disable branches | constructor clears `0x005FF26B`; `FUN_0048A620` ORs flags at `0x0048A6EC`; draw branches `0x005FF85C..0x005FF87F`, `0x005FFB1C..0x005FFF65` | Yes |

## 3. Direct Function Semantics

| Function | Semantics | Evidence | Active in YR |
|---|---|---|---|
| `FUN_005FF250` | Thiscall-like constructor: copies coords and size, clears stage/flags, grows vector if needed, appends `this` to `DAT_00AC167C[count]`, increments count, returns `this`. | decompile plus assembly `0x005FF250..0x005FF2CC`; xrefs from four producer sites | Yes |
| `FUN_005FF2D0` | Removes `ECX` pointer from the vector using vector find slot `vtable+0x10`; if found, decrements count and shifts later pointers left. Does not free memory. | assembly `0x005FF2D0..0x005FF31B`; callers free separately after removal | Yes |
| `FUN_005FF390` | Logic updater: snapshots count into a reverse index, reads each pointer, adds `8` to `+0x0C`, removes/frees if new stage is `>=0x50`. | decompile plus assembly `0x005FF390..0x005FF413`; only caller `LogicClass::PerTickUpdate @ 0x0055B5BE` | Yes |
| `FUN_005FF850` | Draw/apply one entry from `ECX`: rejects by flag/detail/throttle and visibility gates, converts world coords through Tactical, chooses the static mask/surface table from stage/size, then brightens/darkens primary-surface pixels. | decompile plus assembly `0x005FF850..0x005FFF81`; callers `0x005FFFA0`, `0x00435DE4`, `0x0062E35D` | Yes |
| `FUN_005FFFA0` | Persistent draw-all: snapshots `DAT_00AC1688`, iterates reverse `count-1..0`, loads each pointer from `DAT_00AC167C`, calls `FUN_005FF850`. | decompile plus assembly `0x005FFFA0..0x005FFFBF`; only caller `TacticalClass_Draw @ 0x006D4664` | Yes |

## 4. Producer Taxonomy

| Producer | Direct call(s) | Persistent? | Stage before first draw | Active in YR | Evidence |
|---|---|---:|---|---|---|
| `FUN_0048A620` generic combat/flash helper | calls `FUN_005FF250` at `0x0048A6E7`, ORs flags at `+0x14` | Yes, leaves entry in vector | `0` at construction; first draw is `0` if producer runs after that tick's `FUN_005FF390`, otherwise `8` after same-tick aging | Yes, conditional on helper gates and callers | decompile/asm `0x0048A620..0x0048A6FD`; direct callers include Warhead detonation, AnimClass AI, LightningStorm, Wave splash, damage/locomotion paths |
| `ParticleSystemClass::AI_Spark` persistent spark light | calls `FUN_005FF250` at `0x0062EC5B`; no immediate draw/remove | Yes | `0`; updater already ran earlier in the PerTick pass for live object particle AI | Conditional: stock Spark particle systems and `g_ExtraAnimationsEnabled == 2`, positive `LightSize`, `OneFrameLight=false` | decompile `0x0062E840`; prior spark report; PerTick call ordering `0x0055B5BE` before live object loop |
| `ParticleSystemClass` one-frame light helper | calls `FUN_005FF250` at `0x0062E347`, copies `PSC+0xF4` into entry `+0x0C`, calls `FUN_005FF850`, removes/frees | No, immediate only | copied `PSC+0xF4`, not constructor stage `0` | Conditional: `OneFrameLight=true`, positive `LightSize`, live particle count `>0` | decompile `0x0062E280`; disassembly prior spark report |
| `BuildingLightClass::Draw_It` endpoint glow | calls `FUN_005FF250` at `0x00435CC0`, sets `+0x0C` to `0x50..0x59` or `0x50`, calls `FUN_005FF850`, removes/frees | No, immediate only | `0x50..0x59` for mode 3 within distance gate, otherwise `0x50` | Conditional: `HasSpotlight=yes` BuildingLight object, owner visible/alive/active gates | decompile/disassembly `0x00435BE0..0x00435DFA`; building-light reports |
| cleanup drain | reads first pointer and calls `FUN_005FF2D0`, then frees | N/A | N/A | Yes during scenario/global cleanup | `FUN_00534450` decompile around `0x00534897..0x005348BB` |

Direct `FUN_005FF250` xrefs are only: `0x0048A6E7`, `0x0062EC5B`, `0x00435CC0`, and `0x0062E347`. Direct `FUN_005FF850` xrefs are only: persistent draw-all `0x005FFFB6`, BuildingLight immediate `0x00435DE4`, and one-frame particle immediate `0x0062E35D`. Direct `FUN_005FF390` and `FUN_005FFFA0` each have one caller: PerTick updater and Tactical draw respectively.

## 5. `FUN_0048A620` Indirect Producer Census

`FUN_0048A620` is the broad persistent producer wrapper for combat/light flashes. Caller xrefs include:

| Caller | Producer class | First-stage ordering implication | Active in YR |
|---|---|---|---|
| `WarheadTypeClass::Detonate @ 0x004690B0` | ordinary Bright bullet/warhead impact | Detonation from `BulletClass` live object AI is after `FUN_005FF390`, so first draw stage `0`; helper itself remains schedule-dependent for non-live-object callers | Yes |
| `AnimClass::AI @ 0x00423AC0` | damaging bouncer/impact animation helper | AnimClass AI is through live object vector after updater; first draw stage `0` | Conditional by damaging anim type |
| `LightningStorm::GroundStrike @ 0x0053A300` | storm strike flash | `LightningStorm::Process` is called after `FUN_005FF390`; first draw stage `0` | Conditional on active lightning storm |
| `Wave_splash_forces @ 0x0053CBE0` | wave/splash visual/damage helper | PerTick calls the wave/splash helper after the main object vector and after updater; first draw stage `0` for this path | Conditional |
| `TechnoClass::ReceiveDamage @ 0x00701900` | force-shield/iron-curtain style damage feedback | Schedule depends on caller of damage reception; helper entry starts `0` and may age same tick if invoked before `FUN_005FF390` | Conditional |
| `FlyLocomotionClass::Process`, `UnitClass::PerCellProcess`, `VoxelAnimClass::AI`, `Rocket/V3 detonation helper 0x00663030`, `FUN_006E2390` | object/locomotion/detonation visual impacts | inspected object/locomotion-style producers are normally reached from live object processing after updater; first draw stage `0` in that placement | Conditional |
| `TriggerAction::Execute @ 0x006DD8B0` | map trigger light actions with sizes `0x32/0x64/0x12C` | trigger execution timing is not fully drained here; same helper semantics apply | Conditional |
| `BuildingClass::AdjustWallConnections @ 0x00453060` | wall/building placement or damage related visual | timing not drained here; same helper semantics apply | Conditional |
| raw xref `0x00482705` | undefined function boundary in current Ghidra project | unresolved producer name; assembly shows an explosion/damage/AnimClass-constructor block followed by `FUN_0048A620` | Conditional/Unknown |

## 6. First-Stage Ordering Answer

| Path | Same-tick updater relationship | First visible stage / index | Evidence | Active in YR |
|---|---|---|---|---|
| persistent SparkSys | created in live object particle AI after `FUN_005FF390` | stage `0` first draw | updater call `0x0055B5BE`; live object loop later; constructor clears `+0x0C` | Conditional |
| ordinary persistent combat helper from live object / LightningStorm / Wave placements | active inspected producers run after `FUN_005FF390` | stage `0` first draw | caller xrefs plus known PerTick placements for live object loop, `LightningStorm::Process`, and `FUN_0053D310` | Conditional |
| any persistent helper call before `FUN_005FF390` | would be aged by same-tick updater before Tactical draw | stage `8` first draw | constructor clears `+0x0C`; updater adds `8` before draw; no standard pre-updater `0x0048A620` producer was proven in this slot | Conditional/No proven stock example |
| one-frame ParticleSystem helper | does not wait for persistent draw-all | copied `PSC+0xF4` | `0x0062E347..0x0062E36E` | Conditional |
| BuildingLight endpoint glow | draw call mutates stage before direct draw and removes/free | `0x50..0x59` or `0x50` | `0x00435DB1..0x00435DE4`; `0x00435DED..0x00435DF5` | Conditional |

So the correct implementation rule is not "all `0x005FF250` entries first draw at stage 0." Persistent entries first draw at whatever their stage is when the next persistent draw pass sees them, and that is schedule-dependent. Immediate one-frame/spotlight entries bypass the persistent pass entirely and may draw with nonzero stage values in the same render call that created them.

## 7. Current Rust Surface Scan

| Rust surface | Current shape | Delta |
|---|---|---|
| `src/sim/components.rs:817..855` | `WorldEffect` is an SHP animation record with frame/rate/delay fields | no 0x18 screen light/glow primitive with coords, stage, size, and flags |
| `src/sim/components.rs:896..920` | world effects age by milliseconds/frame count | not native `+8` per logic update with removal at `>=0x50` |
| `src/sim/world/mod.rs:1377`, `1590`, `1828` | many producers push SHP-like `WorldEffect`; tick/remove happens late in `advance_tick` | not the native light-effect vector placement before live objects and persistent draw after object rendering |
| `src/sim/superweapon/lightning_storm.rs:222..305` | lightning storm pushes bolt/warhead anim SHP effects and smudge requests | missing `FUN_0048A620` combat-light primitive for storm/warhead impact |
| `src/map/lighting.rs` / app lighting surfaces | persistent map/cell or point lighting model | wrong owner for this screen-space transient effect list |

## 8. Implementation Handoff

| Verified behavior | Rust delta | Affected surface | Acceptance scenario | Proposed test name | Risk |
|---|---|---|---|---|---|
| `DAT_00AC167C` is a separate dynamic vector of 0x18 screen-space light/glow primitives with coord/stage/size/flag fields, not `WorldEffect` SHP animations | add a dedicated transient light-effect queue/vector or render bridge preserving native fields and insertion order | `src/sim/components.rs`, render/VFX bridge, current `world_effects` producers | create two combat-light entries and one spark entry in one logic frame; persistent draw sees vector reverse order and exact stage/flag payloads | `light_effect_vector_preserves_native_fields_and_reverse_draw_order` | High: folding into SHP `WorldEffect` loses stage math, flags, draw ordering |
| Persistent entries age in `FUN_005FF390` before later producers; creation after that updater first draws stage `0`, but creation before it would first draw stage `8` | schedule light-effect aging at the native global tick spine point, not at the current late world-effect animation phase | `src/sim/world/mod.rs::advance_tick`, future native tick spine | a Bright impact from live object AI first draws stage `0`; a seeded pre-updater entry ages to `8` before draw | `persistent_light_effect_first_stage_depends_on_creation_side_of_updater` | High: blindly aging on creation breaks SparkSys and most combat flashes |
| Immediate one-frame/spotlight callers allocate through the same constructor but draw directly, remove with `FUN_005FF2D0`, and free in the same render/object draw path | support immediate light-effect draw events that do not survive to the persistent vector lifetime | particle render path, BuildingLight/spotlight render path, VFX renderer | BuildingLight endpoint glow draws stage `0x50..0x59` and leaves no persistent vector entry after draw | `immediate_light_effect_draw_removes_entry_before_persistent_pass` | Medium: treating every constructor call as persistent leaves bogus lingering spotlight/spark glows |
| `FUN_0048A620` is a generic producer wrapper with many active callsites; helper output is persistent but its first-stage timing is schedule-dependent | producer call sites should emit native light-effect entries at their actual native tick/draw placement, not a generic end-of-tick event | combat, anim bouncer, lightning storm, damage feedback, trigger visuals | LightningStorm ground strike and AnimClass impact produce persistent flash entries after updater, first draw stage `0` | `combat_light_producers_after_updater_first_draw_stage_zero` | Medium: moving all combat flashes to Rust end-of-tick changes same-tick draw/age order |

## 9. Negative Facts / Do Not Do

- Do not call the `DAT_00AC167C` object an ordinary `LightSourceClass`. Active in YR: Yes. Evidence: entries are raw 0x18 heap blocks filled by `0x005FF250`; ordinary map/building light sources are separate systems and this path draws directly through `0x005FF850`.
- Do not treat every `FUN_005FF250` call as persistent. Active in YR: Conditional. Evidence: `0x0062E280` and `BuildingLightClass::Draw_It` call `FUN_005FF850`, then `FUN_005FF2D0`, then `FUN_007C8B3D` in the same call.
- Do not age newly created persistent SparkSys or post-updater combat-light entries before their first draw. Active in YR: Conditional. Evidence: `FUN_005FF390` at `0x0055B5BE` precedes live object particle/combat producers in the same PerTick pass.
- Do not assume first draw is always stage `0`. Active in YR: Conditional. Evidence: BuildingLight writes `+0x0C = 0x50..0x59`/`0x50` before direct draw; one-frame particle copies `PSC+0xF4` before direct draw.
- Do not implement this through map/cell lighting or SHP animation frame ticking. Active in YR: Yes. Evidence: draw path uses primary-surface mask/table blits from `0x005FF850`, vector draw-all `0x005FFFA0`, and lifetime stage math rather than asset frames.

## 10. Remaining Uncertainty

- One raw `FUN_0048A620` xref at `0x00482705` sits in an undefined/misaligned function boundary in the current Ghidra project. Assembly context shows an explosion/damage/AnimClass-constructor block and then a call to `0x0048A620`, but this report does not assign a class/function name.
- Full timing for trigger-action and wall-adjustment `FUN_0048A620` producers was not drained. The helper semantics are verified; whether those specific producers execute before or after `FUN_005FF390` depends on their caller context.
- Exact presented-frame count under render starvation remains runtime-only; logic stage/order is verified.

## 11. Stale Docs / Replacement Wording

- `docs/research/PERTICKUPDATE_NON_OBJECT_GLOBAL_LOOPS_GHIDRA_REPORT.md`: replace deferred OQ-PTNOG-013 wording with: "`FUN_005FF390` ages the global `0x00AC1678` dynamic vector whose data pointer is `DAT_00AC167C`; entries are 0x18 screen-space light/glow primitives created by persistent combat/spark producers and immediate one-frame/spotlight draw helpers. Persistent entries are drawn later by `FUN_005FFFA0`; immediate helpers draw through `FUN_005FF850` and remove/free the entry in the same call."
- `docs/research/ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md`: replace "creates a `LightSourceClass` instance" around `FUN_0048A620` with: "`FUN_0048A620` creates a 0x18 screen-space light/glow primitive through `FUN_005FF250` and leaves it in the `DAT_00AC167C` transient light-effect vector; it is not `AnimClass`, not `SmudgeClass`, and not the ordinary map/building `LightSourceClass`."
- `docs/research/SPARK_LIGHT_EFFECT_TICK_ROUNDING_AND_FIRST_VISIBLE_STAGE_RESWARM_20260528.md`: add after OQ-15 resolution: "Non-spark direct users split by mode: `FUN_0048A620` persistent combat-light producers share stage-0 first draw when created after `FUN_005FF390`, while `BuildingLightClass::Draw_It` is an immediate draw/remove caller that mutates `+0x0C` to `0x50..0x59`/`0x50` before calling `FUN_005FF850`."
- `docs/research/COMBAT_LIGHT_SPAWN_0X0048A620_BRIGHT_CLDISABLE_GHIDRA_REPORT.md`: add: "The combat-light helper output is persistent in the `DAT_00AC167C` vector, but first visible stage is scheduler-dependent: common live-object/LightningStorm/Wave producers run after `FUN_005FF390` and first draw stage `0`; any producer that executes before that updater would be aged to stage `8` before the next persistent draw."

## 12. Sources

- Ghidra read-only decompile/disassembly: `FUN_005FF250`, `FUN_005FF2D0`, `FUN_005FF390`, `FUN_005FF850`, `FUN_005FFFA0`, `FUN_0048A620`, `BuildingLightClass::Draw_It @ 0x00435BE0`, `ParticleSystemClass` one-frame helper `0x0062E280`, `ParticleSystemClass::AI_Spark @ 0x0062E840`, `FUN_00534450`.
- Ghidra read-only xrefs: bulk xrefs to `0x00AC167C`, `0x00AC1688`, `0x005FF250`, `0x005FF850`, `0x005FF390`, `0x005FFFA0`; callers of `0x0048A620`.
- Prior docs checked: `PERTICKUPDATE_NON_OBJECT_GLOBAL_LOOPS_GHIDRA_REPORT.md`, `SPARK_LIGHT_EFFECT_TICK_ROUNDING_AND_FIRST_VISIBLE_STAGE_RESWARM_20260528.md`, `COMBAT_LIGHT_SPAWN_0X0048A620_BRIGHT_CLDISABLE_GHIDRA_REPORT.md`, `BUILDINGLIGHTCLASS_BEAM_RASTERIZATION_AND_CELLACTION_0X23_GHIDRA_REPORT.md`, `ANIMCLASS_BOUNCER_APPLY_AREA_DAMAGE_CALLROW_GHIDRA_REPORT.md`.
- Rust read-only scan: `src/sim/components.rs`, `src/sim/world/mod.rs`, `src/sim/superweapon/lightning_storm.rs`.
