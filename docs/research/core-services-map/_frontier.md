# Core Services Map — Frontier (UN-STUDIED services)

**What this is:** the complement of the 18 substrate study docs. The `core-services-map/`
folder already has 15 profile entries (`abstract-object`, `bridge-helpers`, `cell-map`,
`cell-validation`, `damage-helpers`, `factory-house`, `ini-parsing`, `logicclass`,
`lookup-tables`, `mission-radio`, `pathfinding-helpers`, `random-scenario`, `rules-class`,
`target-scoring`, `techno-foot`) — plus drawing-helpers, gadget-dialog, shell-dialog cover
the remaining studied 18. The `logicclass` profile already names three frontier slugs as
edges (`frontier-render`, `frontier-ai`, `frontier-net`) without a profile behind them.

This file enumerates **every core engine service that has NO substrate study doc yet** so
the final map is complete. Each entry is a CATALOG STUB, not a deep dive: one-line purpose,
key manager + one Ghidra-verified representative function, where it plugs into the per-tick
spine (`LogicClass::PerTickUpdate @ 0x0055AFB0`, sole caller `Main_Tick @ 0x0055D360`) or
the render pass (`TacticalClass_Draw @ 0x006D3D10` / `MainGame_SidebarDraw @ 0x006D0A30`),
and which of the 18 studied services it most depends on.

All addresses verified live via Ghidra `search_functions` / `get_function_callers` this
session (gamemd.exe, image base 0x400000, 10035 functions). Every service below is marked
**UN-STUDIED** (no substrate doc yet).

Authority order: binary → Ghidra → docs. These are catalog stubs — promote to a full
substrate study before implementation.

---

## A. Render / display pipeline

### A1. `frontier-render-tactical` — the tactical (world) draw pass — UN-STUDIED
- **Purpose:** draws the isometric battlefield viewport each frame: terrain layers, the
  z-sorted object layer, shroud, banding/selection overlays; owns world↔screen transforms.
- **Manager / global:** `Tactical` / `TacticalClass` singleton (`g_Tactical`, BSS).
- **Representative fn:** `TacticalClass_Draw @ 0x006D3D10` (per-frame draw entry); object
  z-pass `Tactical_ObjectRenderingLoop @ 0x006D8DB0`; layer heads `Tactical_layer_base_terrain
  @ 0x006D3470`, `Tactical_layer_overlays @ 0x006D3040`, `Tactical_layer_animations
  @ 0x006D3870`, `Tactical_layer_shroud_edges @ 0x006D3660`; transforms
  `TacticalClass__CellToPixel @ 0x006D1FE0`, `Tactical__WorldToScreenSub @ 0x006D1EB0`.
- **Plug point:** the RENDER pass (out-of-sim), NOT PerTickUpdate. Its *AI* counterpart
  `TacticalClass__AI @ 0x006D2540` is LogicClass rung S (`g_Tactical->+0x5C`); the recenter
  helper is rung V. Draw itself runs from `Main_Loop`/frame driver after the tick.
- **Most depends on:** `cell-map` (reads CellClass occupancy/layers per cell),
  `abstract-object` (walks the live object list to z-sort), `lookup-tables`
  (`AdjacentCell`/coord tables for the iso transform).

### A2. `frontier-render-layer` — z-ordered draw list (LayerClass / DisplayClass) — UN-STUDIED
- **Purpose:** the render-only sorted draw list (`g_DisplayLayers @ 0x008A0360`,
  `DynamicVectorClass<ObjectClass*>`, distinct from the LogicClass sim vector). Objects
  submit/remove themselves from a per-layer bucket; the tactical pass walks it in z-order.
- **Manager / global:** `DisplayClass` (a base of `TacticalClass`) + per-layer
  `LayerClass` vectors.
- **Representative fn:** `DisplayClass__Submit_Object @ 0x004A9720`,
  `DisplayClass__RemoveFromLayer @ 0x004A9770`, `LayerClass__Constructor @ 0x004A862A`.
- **Plug point:** render pass; membership churn driven by object reveal/conceal (sim
  lifecycle) but the list itself is render-side. Walked by `Tactical_ObjectRenderingLoop`.
- **Most depends on:** `abstract-object` (lifecycle reveal/conceal toggles layer
  membership), `cell-map` (cell → layer assignment).

### A3. `frontier-blitter` — Surface/Blitter raster back-end — UN-STUDIED
- **Purpose:** the low-level 8/16-bit blitter family (the dozens of `BlitTrans*` /
  `RLEBlitTrans*` template instantiations) plus `Surface`/`DSurface`/`BSurface` framebuffer
  abstraction and the final blit to the DirectDraw primary surface.
- **Manager / global:** `Surface`/`DSurface` (primary + hidden back buffers), `Blitter`,
  the `BlitTrans*<unsigned_short>` / `RLEBlitTrans*` template tables.
- **Representative fn:** `SidebarClass__BlitToScreen @ 0x006A70E0` (a representative blit
  consumer); `Tactical_ZBufferDirtyClear @ 0x006D2B60` (z-buffer dirty management). The
  blitter templates themselves are the `BlitTrans*` class family (no single AI entry).
- **Plug point:** render pass back-end — invoked by every draw consumer (tactical, sidebar,
  radar, UI). Pure output; no sim coupling.
- **Most depends on:** `lookup-tables` (xlat/translucency/shadow remap tables consumed by
  the `*Xlat*` / `*Lucent*` blitters); otherwise self-contained.

---

## B. Sidebar / radar (in-game HUD)

### B1. `frontier-sidebar` — build sidebar / cameo strips — UN-STUDIED
- **Purpose:** the right-hand build bar: cameo strips per tab (building/defense/infantry/
  unit), production progress flash, scroll buttons, hit-testing, tooltips, tab switching.
- **Manager / global:** `SidebarClass` (with nested `StripClass`); `SelectClass`.
- **Representative fn:** `SidebarClass__Draw @ 0x006A6C30`; `SidebarClass__AddCameo
  @ 0x006A6300`; `Sidebar_UpdateFromProduction @ 0x006A6140`; action/hit-test
  `SidebarClass__Action @ 0x006A7780`.
- **Plug point:** render pass via `MainGame_SidebarDraw @ 0x006D0A30`; its cameo state is
  fed from production (factory) state each frame.
- **Most depends on:** `factory-house` (cameo availability/flash mirrors FactoryClass
  production + HouseClass tech/credits), `rules-class` (buildable lists/prereqs).

### B2. `frontier-radar` — radar / minimap — UN-STUDIED
- **Purpose:** the minimap: terrain color surface, per-cell radar pixels, object trackers,
  radar events (pings), jammed/fog modes, click-to-recenter.
- **Manager / global:** `RadarClass` (a base in the `GScreen`→`Display`→`Radar`→`Tactical`
  chain).
- **Representative fn:** `RadarClass__Draw @ 0x00653100`; per-cell color
  `CellClass__GetRadarColor @ 0x0047C060`; trackers `RadarClass__AddObjectToTracker
  @ 0x00655560`; events `CreateRadarEvent @ 0x0065FA70` / `DrawRadarEvent @ 0x00660050`.
- **Plug point:** render pass; tracker/event state updated from sim each frame
  (`BuildingClass__RegisterOnRadar @ 0x00456580`).
- **Most depends on:** `cell-map` (per-cell terrain/ore radar color), `abstract-object`
  (object tracker registration on reveal/conceal).

---

## C. Audio

### C1. `frontier-audio-voc` — SFX engine (VocClass + DirectSound mixer) — UN-STUDIED
- **Purpose:** sound-effect playback: VocClass sample registry, positional volume/pan from
  world coords, the DirectSound channel pool + priority eviction, per-tick voice update.
- **Manager / global:** `VocClass` table; DirectSound channel pool
  (`DSoundChannel`), `SoundEvent` pool.
- **Representative fn:** `VocClass__PlayAtCoord @ 0x00750E20` (positional SFX);
  `VocClass__CalcVolumeAndPan @ 0x00750AC0`; mixer tick `SoundSystem__UpdateTick
  @ 0x004041D0`; channel eviction `DSoundChannel__FindLowestPriority @ 0x00404E20`.
- **Plug point:** OUT-OF-SIM. Sim emits sound *cues* (mission/combat/anim) that the audio
  layer plays; the mixer tick runs from the frame/audio loop, not PerTickUpdate. Cue timing
  must match but the engine is render/audio-side.
- **Most depends on:** `lookup-tables` (coord→pan/volume distance math), `rules-class`
  (`VocClass__ReadINI @ 0x00750440` reads the sound list); `cell-map` for the listener
  position.

### C2. `frontier-audio-eva` — EVA voice queue (VoxClass) — UN-STUDIED
- **Purpose:** the EVA announcer voice queue ("Construction complete", "Unit lost") —
  priority queue, dedup, sequential playout so lines don't overlap.
- **Manager / global:** `VoxClass` queue.
- **Representative fn:** `VoxClass__PlayEVA @ 0x00752700`; `VoxClass__InsertIntoQueue
  @ 0x00752590`; `VoxClass__PlayNextQueued @ 0x00752760`.
- **Plug point:** OUT-OF-SIM; sim emits EVA cue ids, queue drains on the audio loop.
- **Most depends on:** `frontier-audio-voc` (shares the DirectSound back-end), `factory-house`
  / `mission-radio` (most EVA cue sources: build complete, unit ready, base under attack).

### C3. `frontier-audio-theme` — music / theme player — UN-STUDIED
- **Purpose:** in-game music track selection + playback (Score.ini theme list, shuffle/
  repeat, track change on game events). NOTE: no `Theme`/`Music`-labeled functions exist in
  the current Ghidra DB — symbol-naming gap. Likely routed through the same `SoundEvent` /
  AUD streaming path or a CD/Bink audio path; **representative address UNVERIFIED** — must
  be located before any study (search by Score.ini strings / AUD streaming xrefs).
- **Plug point:** OUT-OF-SIM audio loop.
- **Most depends on:** `frontier-audio-voc` (sample/stream back-end), `rules-class` /
  Score.ini theme table.

---

## D. Animation / projectile / particle objects

### D1. `frontier-anim` — AnimClass (sprite animations) — UN-STUDIED
- **Purpose:** transient sprite animations (explosions, muzzle flashes, smoke, build-up):
  frame advance, looping, damage-on-frame, attached-owner detach, trailer spawns. Heavily
  researched in scattered `ANIMCLASS_*_GHIDRA_REPORT.md` docs but NOT consolidated into a
  substrate study.
- **Manager / global:** `AnimClass` live list (`DynamicVectorClass<AnimClass*>`).
- **Representative fn:** `AnimClass__AI @ 0x00423AC0` (per-frame anim update);
  `AnimClass__UpdateLoopingSound @ 0x00750D40`.
- **Plug point:** PerTickUpdate object pass (rung N) — AnimClass is an `ObjectClass`-derived
  element with a `+0x5C` AI head; **but the anim pass is SKIPPED in skirmish modes 0/5**
  per the LogicClass study. Draw via `Tactical_layer_animations @ 0x006D3870`.
- **Most depends on:** `abstract-object` (object lifecycle/registration),
  `damage-helpers` (anim-driven area damage), `cell-map` (anim cell placement).

### D2. `frontier-bullet` — BulletClass (projectiles) — UN-STUDIED
- **Purpose:** in-flight projectiles: homing/ballistic track, arming/proximity, detonation,
  shrapnel spawn. Many `*BULLET*` / `AAHEATSEEKER2_*` reports exist; not a consolidated study.
- **Manager / global:** `BulletClass` live list (`DynamicVectorClass<BulletClass*>`).
- **Representative fn:** `BulletClass__HomingTrack @ 0x005B20F0`;
  `BulletClassBulletDetonationImpactDamage @ 0x00468D80`; alloc `BulletClassAllocate
  @ 0x0046B050`.
- **Plug point:** PerTickUpdate object pass (rung N) as an `ObjectClass`-derived element.
  Draw via the object render loop.
- **Most depends on:** `damage-helpers` (detonation → ReceiveDamage), `target-scoring`
  (retarget on homing), `cell-map` (cell occupancy at impact), `random-scenario`
  (scatter/inaccuracy RNG).

### D3. `frontier-particle` — ParticleSystemClass / ParticleClass — UN-STUDIED
- **Purpose:** particle systems (smoke, fire, gas, sparks, railgun trails): per-type emitter
  AI spawning ParticleClass instances.
- **Manager / global:** `ParticleSystemClass` + `ParticleClass` live lists.
- **Representative fn:** `ParticleSystemClass__AI @ 0x0062FD60` (dispatch);
  per-type `ParticleSystemClass__AI_Smoke @ 0x0062ED40`, `_Fire @ 0x0062F9A0`,
  `_Railgun @ 0x0062F230`; spawn `ParticleSystemClass__SpawnParticle @ 0x0062E380`.
- **Plug point:** PerTickUpdate object pass (rung N) as `ObjectClass`-derived elements.
- **Most depends on:** `abstract-object` (lifecycle), `random-scenario` (particle jitter
  RNG), `rules-class` (`RulesClass__ReadParticleSystems @ 0x00672A70`).

### D4. `frontier-voxelanim` — VoxelAnimClass (voxel debris/effects) — UN-STUDIED
- **Purpose:** voxel-rendered transient effects (debris chunks, meteors): per-frame AI +
  voxel draw.
- **Manager / global:** `VoxelAnimClass` live list.
- **Representative fn:** `VoxelAnimClass__AI @ 0x00749F30`; draw `VoxelAnim__Draw
  @ 0x0046B0C0`; INI `VoxelAnimTypeClass__ReadINI @ 0x0074B050`.
- **Plug point:** PerTickUpdate object pass (rung N); voxel draw in the object render loop.
- **Most depends on:** `abstract-object`, `rules-class` (`RulesClass__ReadVoxelAnims
  @ 0x00672920`), `frontier-render-tactical` (voxel rasterizer).

---

## E. Network / lockstep / command queue

### E1. `frontier-net-eventqueue` — lockstep event/command queue — UN-STUDIED
- **Purpose:** the lockstep command queue: player commands wrap into `EventClass`, are
  buffered (`CommBufferClass`), exchanged over the wire, and EXECUTED at the scheduled frame.
  This is the determinism boundary — event execution order is the lockstep contract.
- **Manager / global:** `CommBufferClass` (send/receive ring); `EventClass` queue.
- **Representative fn:** `Process_QueuedEvents @ 0x0053B560` (**verified caller =
  `Main_Tick @ 0x0055D360`** — runs in the per-tick spine, before/around PerTickUpdate);
  per-event `EventClass__Execute @ 0x004C6CB0`; queue `CommBufferClass__Constructor
  @ 0x0048B0E0`.
- **Plug point:** the per-tick spine via `Main_Tick`, distinct from `LogicClass::PerTickUpdate`.
  Maps to the Rust tick's leading "commands" stage. **Lockstep-critical.**
- **Most depends on:** `logicclass` (events execute at a frame the tick spine schedules),
  `random-scenario` (executed events consume the same RNG cursor → desync if reordered),
  `factory-house` / `techno-foot` (most events mutate house/unit state).

### E2. `frontier-net-transport` — IPX/UDP transport + connection manager — UN-STUDIED
- **Purpose:** the wire transport: IPX and UDP socket layers, connection manager, packet
  framing, retransmit, the null-modem path. Mostly TS legacy plumbing — verify which paths
  are live in YR LAN/WOL before study.
- **Manager / global:** `IPXManagerClass`, `UDPInterfaceClass`, `ConnManClass`,
  `ConnectionClass`, `NullModemClass`.
- **Representative fn:** `IPXManagerClass__Constructor @ 0x005408F0` (other transport AI
  entries unlabeled — locate via ConnManClass/ConnectionClass vtable before study).
- **Plug point:** OUT-OF-SIM transport; feeds E1's CommBuffer. Tick-agnostic.
- **Most depends on:** `frontier-net-eventqueue` (delivers/accepts the serialized events);
  otherwise OS/socket layer, low coupling to sim.

---

## F. Scripted + skirmish AI (triggers / teams / scripts)

### F1. `frontier-trigger` — map triggers/tags (TActionClass / TEventClass) — UN-STUDIED
- **Purpose:** the map scripting engine: trigger conditions (`TEventClass`) fire actions
  (`TActionClass`) via tags. Drives campaign events; some actions exist but were never used
  in YR skirmish maps (TS legacy — verify per-action liveness).
- **Manager / global:** `TriggerClass` / `TriggerTypeClass` + `TagClass` lists;
  `TActionClass`, `TEventClass`.
- **Representative fn:** `TriggerClass__Constructor @ 0x00725FA0`;
  `TActionClass`/`TEventClass` constructors `0x...`/`TEventClass__Constructor @ 0x0071E6A0`
  (per-tick trigger evaluation entry unlabeled — locate via Tag/Trigger update xrefs).
- **Plug point:** per-tick (campaign), likely a global driver near the LogicClass ladder —
  exact rung UNVERIFIED.
- **Most depends on:** `logicclass` (per-tick eval order), `factory-house` (house-scoped
  conditions), `cell-map` (cell/zone conditions). **AI-adjacent — project rule defers AI.**

### F2. `frontier-ai-team` — TeamClass / TaskForce / ScriptClass (team AI) — UN-STUDIED
- **Purpose:** skirmish/campaign team AI: recruit members per TaskForce, run a ScriptType
  list (move/attack/guard/patrol), convoy behaviors. This is LogicClass rung F.
- **Manager / global:** `TeamClass` array; `TaskForceClass`, `ScriptClass`/`ScriptTypeClass`,
  `TeamTypeClass`.
- **Representative fn:** `TeamClass__AI @ 0x006E9140` (**= LogicClass rung F target,
  `g_TeamClass_Array +0x5C`**); recruit `TeamClass__Recruit_Or_Add @ 0x006E9380`; script
  steps `TeamClass__Convoy_Script_Attack_Nearest @ 0x006EE310`.
- **Plug point:** PerTickUpdate rung F (teams array `+0x5C`).
- **Most depends on:** `techno-foot` (member units it commands), `target-scoring`
  (target selection), `pathfinding-helpers` (move scripts). **AI — project rule defers.**

### F3. `frontier-ai-house` — HouseClass AI brain (skirmish economy/production) — UN-STUDIED
- **Purpose:** the per-house AI brain: economy state machine, production choice, build-queue
  management, threat map, rally/target selection. This is LogicClass rung U (house `+0x5C`).
- **Manager / global:** `HouseClass` array (`g_HouseClass_Array`); `BrainClass`.
- **Representative fn:** `HouseClass__AI_EconomyStateMachine @ 0x00509700`;
  `HouseClass__AI_ChooseNextProduction @ 0x00506EF0`;
  `HouseClass__AI_Manage_Build_Queue @ 0x004FDD10`.
- **Plug point:** PerTickUpdate rung U (houses array `+0x5C`, null-guarded, live count).
- **Most depends on:** `factory-house` (its own HouseClass economy/FactoryClass it drives),
  `frontier-ai-team` (dispatches teams), `target-scoring`. **AI — project rule defers.**

### F4. `frontier-ai-trigger` — AITriggerTypeClass (skirmish AI triggers) — UN-STUDIED
- **Purpose:** the skirmish AI trigger system (weighted, condition-gated team production) —
  distinct from F1 map triggers. Drives what the AI builds/sends in skirmish.
- **Manager / global:** `AITriggerTypeClass` list (`DiscreteDistributionClass` weighting).
- **Representative fn:** `AITriggerTypeClass__Constructor @ 0x0041E350` (per-tick eval entry
  unlabeled — locate via HouseClass AI brain xrefs).
- **Plug point:** evaluated within the HouseClass AI brain (rung U).
- **Most depends on:** `frontier-ai-house` (evaluated by the house brain),
  `frontier-ai-team` (produces teams), `rules-class`. **AI — project rule defers.**

---

## G. Superweapons / capture / spawn-manager (object-AI satellites)

### G1. `frontier-super` — SuperClass (superweapon charge/launch) — UN-STUDIED
- **Purpose:** superweapon state machine: charge timer, ready state, launch dispatch,
  recharge, EVA readiness cues. Per-house super instances.
- **Manager / global:** `SuperClass` (per-house `DynamicVectorClass<SuperClass*>`);
  `SuperWeaponTypeClass`.
- **Representative fn:** `SuperClass__AI_Charging @ 0x006CC080`; ready `SuperClass__AI_Ready
  @ 0x006CBCA0`; launch `SuperClass__Launch @ 0x006CC390`.
- **Plug point:** ticked from the HouseClass per-frame update (rung U) — supers are
  per-house members; exact sub-position UNVERIFIED.
- **Most depends on:** `factory-house` (HouseClass owns the supers, power/credits gate),
  `random-scenario` (target/animation RNG), `frontier-audio-eva` (readiness cues).

### G2. `frontier-capture` — CaptureManagerClass (mind-control / capture) — UN-STUDIED
- **Purpose:** mind-control + capture links (Yuri, Master Mind, IFV): link/unlink slaves,
  capacity, fate-on-death, link rendering.
- **Manager / global:** `CaptureManagerClass` (per controller); `SlaveManagerClass`,
  `SpawnManagerClass` (related spawn/slave families).
- **Representative fn:** `CaptureManagerClass__CaptureUnit @ 0x00471D40`; capacity
  `CaptureManagerClass__CanCapture @ 0x00471C90`; fate `CaptureManagerClass__DecideUnitFate
  @ 0x004723B0`; link draw `CaptureManagerClass__DrawLinks @ 0x00472160`.
- **Plug point:** PerTickUpdate object pass (rung N) — runs inside the controlling techno's
  `+0x5C` AI; link draw in the render pass.
- **Most depends on:** `techno-foot` (the controlling + controlled units), `mission-radio`
  (control overrides mission), `damage-helpers` (fate-on-death).

---

## H. Asset / file substrate (load-time, not per-tick)

### H1. `frontier-mix-vfs` — MIX virtual file system + asset loaders — UN-STUDIED
- **Purpose:** the `.mix` archive VFS (mount order, sentinels, neutral mix), file lookup,
  and the from-scratch asset parsers (SHP/VXL/PAL/TMP/AUD/CSF) layered on `FileClass`.
- **Manager / global:** `MixFileClass` list; `CCFileClass`/`FileClass` hierarchy;
  `ConvertClass` (palette remap).
- **Representative fn:** `LoadFileFromMIX @ 0x005B40B0`; `MIX_LoadNeutral @ 0x0072FA10`;
  `MixFileSystem_InitSentinels @ 0x005B3AC0`; side mixes `InitSideMixFiles @ 0x00534FA0`.
- **Plug point:** LOAD-TIME (boot + map load), not per-tick. Upstream of everything.
- **Most depends on:** nothing in the 18 — it is the lowest layer; `ini-parsing` sits beside
  it (CCINIClass reads from the VFS). The Rust `assets/`+`util/` layer is its counterpart.

### H2. `frontier-saveload` — save/load + swizzle serializer — UN-STUDIED
- **Purpose:** whole-game serialization: per-class `Save`/`Load` walk, pointer swizzle
  (`SwizzleManagerClass`), section ordering. The save format is the cross-cut of every
  studied service's persisted fields.
- **Manager / global:** `SwizzleManagerClass`; per-class `*__Save`/`*__Load` methods.
- **Representative fn:** `HouseClass__Save @ 0x00504080`, `CellClass__Save @ 0x00483C10`,
  `AbstractClass__Save @ 0x00410320` (the family); load swizzle helper `FUN_006CF240`
  (per LogicClass study). Top-level orchestrator address UNVERIFIED — locate via the Save
  menu command xrefs.
- **Plug point:** OUT-OF-SIM (menu-triggered), but touches every sim service's state.
- **Most depends on:** ALL studied services (it serializes their state); structurally
  closest to `abstract-object` (the Save/Load vtable contract lives on the object hierarchy)
  and `logicclass` (active-vector save/load order).

---

## I. Input / command dispatch (out-of-sim front of the spine)

### I1. `frontier-input-command` — keyboard/command + mouse action dispatch — UN-STUDIED
- **Purpose:** the front of the input chain: hotkey/`CommandClass` dispatch, mouse action
  determination at the tactical viewport (what cursor, what order a click issues). Click
  results become `EventClass` items for E1.
- **Manager / global:** `CommandClass` family (the many `*CommandClass` entries);
  `DisplayClass` action resolver; `MouseClass`.
- **Representative fn:** `Process_Command @ 0x0055DEE0` (hotkey dispatcher — note the stale
  `LogicClass::AI` label is wrong per the LogicClass study); action resolve
  `DisplayClass__DetermineAction @ 0x00692610`; cursor `DisplayClass__SetCursorFromAction
  @ 0x004AAE90`.
- **Plug point:** OUT-OF-SIM input stage feeding E1's event queue at the front of the tick.
- **Most depends on:** `frontier-net-eventqueue` (emits events into it), `target-scoring` /
  `cell-map` (hit-testing + legal-action resolution), `frontier-render-tactical` (screen→cell).

---

## Coverage note

Deliberately EXCLUDED as already studied (the 18) or as non-services:
- Studied 18: object hierarchy (`abstract-object`, `techno-foot`), LogicClass tick spine,
  RNG/scenario (`random-scenario`), rules (`rules-class`), cell/map (`cell-map`,
  `cell-validation`), factory/house economy (`factory-house`), 7 helper families
  (`bridge-helpers`, `damage-helpers`, `drawing-helpers`, `ini-parsing`,
  `pathfinding-helpers`, `target-scoring`, `mission-radio`), lookup tables
  (`lookup-tables`), two UI frameworks (`gadget-dialog` = in-game GadgetClass dialog
  controls, `shell-dialog` = front-end shell/menu framework).
- Locomotors (`DriveLocomotionClass` etc., `0x004AF540`+) — these are the `techno-foot`
  movement piggyback substrate; folded under the studied techno-foot service, not a separate
  frontier entry (flag if a dedicated locomotor study is wanted).
- Container templates (`DynamicVectorClass`, `VectorClass`, `TypeList`, `List`/`Node`) and
  COM/ATL plumbing (`IUnknown`, `IConnectionPoint`, WOL chat sinks) — infrastructure, not
  gameplay services; covered implicitly by `lookup-tables`/storage decisions.
- Front-end menus, campaign/score screens, movie/Bink players, World Domination Tour — UI
  shell surface adjacent to `shell-dialog`; out of the in-skirmish core scope.

**Total NEW frontier services catalogued: 20** (A1–A3, B1–B2, C1–C3, D1–D4, E1–E2, F1–F4,
G1–G2, H1–H2, I1). All UN-STUDIED. The three slugs the LogicClass profile already references
map to: `frontier-render` → A1/A2, `frontier-ai` → F2/F3/F4, `frontier-net` → E1/E2.
