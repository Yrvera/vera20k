# Parity Gap Scan — Non-Combat Surfaces (RAW, 2026-05-29)

Second-pass scan scoped to UI/HUD, economy/production, audio, animation/facing, map/terrain — the surfaces the combat-heavy first scan under-covered. **63 raw findings across 5 surfaces.**

> ⚠️ **RAW / UNRANKED.** The adversarial Verify pass and the severity Rank pass both failed (transient platform issue — every schema-forced agent bailed without emitting structured output). These are the survey agents' findings as-reported. Each surveyor *did* grep `src/` for a first-pass `rust_status`, but they have **not** been adversarially re-checked against current code, so some may already be implemented or be false positives. Confirm before acting. Companion verified combat list: `2026-05-29-parity-gap-scan-shortlist.md`.

Confidence is the surveyor's own (high/medium/low), not a verified severity.

## Cross-surface overlaps (strong signals)

- Rally-line + shift-queued waypoint overlays — flagged by both **UI** and **map/terrain**.
- EVA ~500ms inter-announcement gap — flagged by both **UI** and **audio**.
- Ore overlay variant + spread density — flagged by both **economy** and **map/terrain**.

## Determinism flags (lockstep risk, not cosmetic)

- Smudge scorch/crater 50/50 selector draws from a different RNG stream → desyncs all subsequent draws (map/terrain).
- Ore-spread germination RNG-order divergence (map/terrain).


## AUDIO BREADTH (excluding the channel-management policy item already owned)  — 15 findings

### [HIGH] EVA announcement queue — STANDARD-type discard policy
- **type:** behavior-not-acted-on
- **effect:** Most EVA cues (insufficient funds, low power, base under attack, building captured, superweapon ready, etc.) are not routed at all yet; the ones that are use a hardcoded Type guess rather than the INI-declared Type, so queueing/dropping behavior diverges from gamemd when announcements overlap.
- **frequency:** Every match — EVA announcements fire constantly (production, power, combat, capture).
- **rust today:** play_standard_eva_sound exists and drops STANDARD when voice busy, but Rust never parses VoxClass Type/Priority from evamd.ini — it hardcodes which app events are 'STANDARD' vs 'QUEUE' in app_building_anim.rs:677-696. Any EVA event not in that hardcoded match (most of the 120 events) has no correct Type/discard behavior.
- **evidence:** docs/research/EVA_SYSTEM_GHIDRA_REPORT.md:308-317 (STANDARD/INTERRUPT silently discarded if anything queued/playing); docs/research/traces/CURRENT_APP_EVA_QUEUE_POSTFIX_TRACE.md:55-63 (Stage 5 FAIL); src/audio/sfx.rs:312-339 play_standard_eva_sound only checks idle, but Rust collapses Type metadata; ini/evamd.ini EVA_UnitReady/EVA_CannotDeployHere have Priority=LOW, no Type=QUEUE (default STANDARD)

### [HIGH] EVA inter-announcement 500ms delay
- **type:** wrong-formula-or-timing
- **effect:** Queued EVA lines play back-to-back with no gap; gamemd inserts a 500ms pause between announcements. Player hears EVA cues spoken faster/more crowded than retail.
- **frequency:** Every time two or more EVA cues queue near each other (common in busy mid-game).
- **rust today:** none found — SfxPlayer has no InterAnnouncementDelay, no last-end timestamp; next queued EVA starts immediately when the slot frees.
- **evidence:** docs/research/EVA_SYSTEM_GHIDRA_REPORT.md:286-290 (DAT_00b1d4d0=0x1F4=500ms gap after each EVA); docs/research/traces/CURRENT_APP_EVA_QUEUE_POSTFIX_TRACE.md:75-83 (Stage 7 FAIL); src/audio/sfx.rs:363-378 advance_voice_queue starts next clip the instant voice_player.empty(), no stored end-time or delay

### [HIGH] EVA priority queue tiers
- **type:** partial-implementation
- **effect:** When multiple EVA cues compete, Rust plays them in arrival order; gamemd plays higher-priority/interrupt cues first and can flush lower ones. Important announcements (e.g. CRITICAL) can be delayed behind trivial ones in Rust.
- **frequency:** Whenever 3+ EVA events of mixed priority queue together (combat + production bursts).
- **rust today:** Single FIFO queued_voice VecDeque storing only sound_id/decoded/volume — no priority tiers, no interrupt queue, no critical queue.
- **evidence:** docs/research/EVA_SYSTEM_GHIDRA_REPORT.md:164-201 (InterruptQueue, CriticalQueue, PendingImmediate, 4 PriorityQueues drained in priority order); docs/research/traces/CURRENT_APP_EVA_QUEUE_POSTFIX_TRACE.md:65-73 (Stage 6 FAIL); src/audio/sfx.rs:131 single VecDeque<QueuedVoice> FIFO

### [HIGH] VocClass Control= flags (random/sequential/interrupt/loop sample selection)
- **type:** unparsed-ini
- **effect:** Sounds declared sequential play randomized and vice versa; LOOP sounds (engine loops, tesla charge) won't loop; ALL (layered) sounds play only one sample. Voice/SFX variation cadence does not match retail.
- **frequency:** Every sound with multiple Sounds= variations or a non-default Control (most unit voices, many weapons, all ambient loops).
- **rust today:** SoundEntry has no control field. sound_ini.rs parses Sounds/Volume/Priority/Range/MinVolume only. Sample picking in sfx.rs:178-181 always does random_counter%len regardless of declared mode.
- **evidence:** docs/research/AUDIO_CHANNEL_MANAGEMENT_GHIDRA_REPORT.md:209-240 (Control bits RANDOM 0x02, ALL 0x04, LOOP 0x01, etc. drive SoundEvent__LoadSamples/AdvancePlaylist); src/rules/sound_ini.rs:88-138 never reads Control=; ini/soundmd.ini:898-997 entries use Control=random/interrupt/loop/all

### [HIGH] VocClass sample selection is sequential, not random
- **type:** wrong-formula-or-timing
- **effect:** Unit selection/attack/death voices and multi-variant SFX rotate in a fixed cycle instead of randomly; a player clicking the same unit repeatedly hears a predictable round-robin rather than gamemd's random pick, and the first variant (index 0) is skipped on first play.
- **frequency:** Every multi-sample voice/SFX, i.e. nearly every unit selection and weapon fire in a match.
- **rust today:** random_counter is a monotonic +1 counter, so multi-sample sounds cycle deterministically (1,2,0,1,2,...) starting at index 1, never random. Also not seeded from the sim RNG.
- **evidence:** docs/research/AUDIO_CHANNEL_MANAGEMENT_GHIDRA_REPORT.md:224-240 (RANDOM picks a random sample; SoundEvent__SelectNextSample @0x00404bb0); src/audio/sfx.rs:178-181,215-218,352-354 use self.random_counter.wrapping_add(1) % len

### [HIGH] Sound [Defaults]: Priority/Limit/MinVolume defaults
- **type:** unparsed-ini
- **effect:** Spatial sounds attenuate to silence at the edge of range instead of holding the MinVolume floor (50% in soundmd); per-sound priority is effectively ignored so eviction order is wrong; per-sound concurrency Limit is unenforced so e.g. a sound capped at Limit=1 (ichrmova) can stack.
- **frequency:** Every positional sound near screen edge (MinVolume) and every repeated/looping sound (Limit) — continuous in any battle.
- **rust today:** Rust reads [Defaults] MinVolume/Range/Volume but defaults MinVolume to 0 when absent (gamemd uses [Defaults] then 20.0 global). Priority parsed as numeric unwrap_or(1) but soundmd uses string NORMAL/high/low which get_i32 cannot parse (yields default for every entry). Limit= is never parsed.
- **evidence:** ini/soundmd.ini:22-28 [Defaults] MinVolume=50 Range=10 Limit=5 Priority=NORMAL; docs/research/SPATIAL_AUDIO_GHIDRA_REPORT.md:27,40 (Limit default 5, MinVolume default 20.0 from globals/[AudioVisual]); src/rules/sound_ini.rs:70-74 (default_min_volume unwrap_or(0)), :117 (priority unwrap_or(1)), no Limit parsed

### [HIGH] Spatial audio stereo pan
- **type:** behavior-not-acted-on
- **effect:** All sounds are dead-center; gamemd pans sounds left/right by horizontal screen position. A tank firing on the left of the screen sounds centered instead of from the left.
- **frequency:** Every positional sound off the screen's horizontal center — continuous.
- **rust today:** none found — calc_spatial_volume computes only a mono volume scalar; play_sound_with_volume sets player.set_volume() with no left/right panning. Decoded audio is upmixed to identical L/R.
- **evidence:** docs/research/SPATIAL_AUDIO_GHIDRA_REPORT.md:179-219 (CalcVolumeAndPan outputs pan 0..16384, center 8192, applied to DirectSound buffer); docs/research/AUDIO_CHANNEL_MANAGEMENT_GHIDRA_REPORT.md:271-287; src/audio/sfx.rs:55-97 calc_spatial_volume returns a single f32 volume, no pan

### [HIGH] VShift pitch randomization and FShift frequency shift
- **type:** unparsed-ini
- **effect:** Repeated sounds (weapon fire, footsteps, ambient loops) play at identical pitch each time; gamemd randomizes pitch slightly via VShift/FShift so repeated shots sound less mechanically identical.
- **frequency:** Every sound entry that declares VShift/FShift (many weapons and movement loops) — repeated firing in any battle.
- **rust today:** none found — VShift and FShift are not parsed (SoundEntry has no such field) and no per-instance pitch/rate randomization is applied; samples always play at their native rate.
- **evidence:** docs/research/SPATIAL_AUDIO_GHIDRA_REPORT.md:20,31,289-291 (VShift offset 0x68 random pitch variation; FShift offset 0x60 freq-shift range); ini/soundmd.ini:949,1686-1693 (VShift/FShift values present); src/audio/sfx.rs play_decoded/play_voice set volume only, no playback-rate adjustment

### [HIGH] Per-unit MoveSound
- **type:** behavior-not-acted-on
- **effect:** Vehicles/ships make no movement sound when crossing cells; gamemd plays MoveSound on each cell entry. Silent vehicle movement vs retail's engine/tread sounds.
- **frequency:** Continuously whenever any unit with a MoveSound is moving — every match.
- **rust today:** move_sound is parsed into ObjectType but never emitted — no GameSoundEvent variant and no caller in movement code.
- **evidence:** docs/research/SOUND_TRIGGERS_COMPLETE_GHIDRA_REPORT.md:253-271,584-597 (MoveSound fires once per cell entry from DriveLocomotion/ShipLocomotion); src/rules/object_type.rs:934 move_sound parsed; grep shows no emit site for MoveSound anywhere in src/

### [HIGH] Building WorkingSound / NotWorkingSound / AmbientSound loop
- **type:** behavior-not-acted-on
- **effect:** Active structures (power plants, refineries, tesla reactors) and ambient map objects are silent; gamemd plays a continuous working/ambient loop. The base soundscape is missing.
- **frequency:** Continuous — every powered structure with a working/ambient sound, the whole match.
- **rust today:** none found — AmbientSound/WorkingSound/NotWorkingSound are not parsed or emitted; no looping ambient SFX system exists.
- **evidence:** docs/research/SOUND_TRIGGERS_COMPLETE_GHIDRA_REPORT.md:235-251,600-619 (AmbientSound/WorkingSound re-triggered each frame in AI_Update via field 0x4f0 loop); grep src/ shows no WorkingSound/AmbientSound emit

### [HIGH] TurretRotateSound / DamageSound / CreateSound / DeploySound resolution
- **type:** behavior-not-acted-on
- **effect:** No sound when a turret rotates, when a unit takes damage, when a unit is created from a factory, or when entering/leaving a transport; gamemd plays each. Several routine feedback cues are silent.
- **frequency:** TurretRotate every time a turreted unit re-aims (very frequent in combat); CreateSound every unit built; DamageSound on every hit.
- **rust today:** DeploySound/UndeploySound have events but DockDeploy resolution is a TODO; TurretRotateSound, DamageSound, CreateSound, EnterTransportSound, LeaveTransportSound are neither parsed nor emitted.
- **evidence:** docs/research/SOUND_TRIGGERS_COMPLETE_GHIDRA_REPORT.md:471-486 (TurretRotateSound, DamageSound, CreateSound, EnterTransportSound, LeaveTransportSound); src/app_sim_tick.rs:383-387 DockDeploy TODO 'resolve building's deploy sound'; grep shows TurretRotateSound/DamageSound/CreateSound never parsed or emitted

### [MEDIUM] Spatial volume falloff axis test + on-screen center math
- **type:** wrong-formula-or-timing
- **effect:** Sounds attenuate from an offset reference point and can be audible/inaudible at the wrong distances; a sound far off one axis but near on the other plays in Rust where gamemd would silence it.
- **frequency:** Every off-center positional sound; center-point bug affects all positional SFX whenever the camera is not at origin (i.e. always).
- **rust today:** calc_spatial_volume uses Chebyshev max() like gamemd but applies the cutoff to the max axis only (gamemd requires BOTH axes in range), and computes screen center as camera_x + half-viewport while screen_pos is already screen-space — double-counting the camera offset, so the falloff is measured from the wrong point.
- **evidence:** docs/research/SPATIAL_AUDIO_GHIDRA_REPORT.md:156-163 (gamemd: volume 0 unless distX<maxRange AND distY<maxRange; effectiveDist=max(distX,distY)); src/audio/sfx.rs:65-96 computes center as camera_x+viewport_w*0.5 and uses dist=dx.max(dy) with single dist>=max_range test

### [MEDIUM] In-game music track order: sequential vs random shuffle
- **type:** wrong-formula-or-timing
- **effect:** Gameplay music plays tracks in a fixed list order each session; gamemd shuffles to a random next track when the current one ends. Players notice the predictable, always-same track sequence.
- **frequency:** Every track transition during gameplay (every few minutes, every match).
- **rust today:** play_next walks the playlist in fixed index order (idx+1 % len). No random selection; the gameplay 'next track' is the next list entry, not a random pick.
- **evidence:** docs/research/MAIN_MENU_MUSIC_TRACK_AND_LOOP_GHIDRA_REPORT.md:167-180 (Theme::AI pending==-2 -> FUN_00720a80 random-from-playlist for next song); src/audio/music.rs:182-199 play_next advances playlist_index sequentially with wraparound

### [MEDIUM] EVA pause/suspend/audio-off guards
- **type:** behavior-not-acted-on
- **effect:** EVA lines can advance/play while the game is paused or during moments gamemd suppresses voice (radar movies), instead of freezing the queue. EVA timing drifts after any pause.
- **frequency:** Whenever the player pauses or a suspend window occurs (pause every match; radar movies in campaign).
- **rust today:** none found — SfxPlayer voice queue has no suspend counter or pause gate; EVA would continue dequeuing while game is paused or during a radar-movie/suspend window.
- **evidence:** docs/research/EVA_SYSTEM_GHIDRA_REPORT.md:179-185,336-352 (PlayNextQueued checks GamePaused/AudioOff/PauseCounter; QueueVoice checks SuspendCounter; suspend during radar movies); docs/research/traces/CURRENT_APP_EVA_QUEUE_POSTFIX_TRACE.md:85-91 (Stage 8 UNCHECKED); src/audio/sfx.rs advance_voice_queue has no pause/suspend gate

### [MEDIUM] Voice response interrupt semantics (priority-2 vs separate slot)
- **type:** partial-implementation
- **effect:** Selecting a unit while an EVA line is queued/playing may not interrupt/flush the EVA exactly as gamemd does (single shared channel); overlap and cutoff ordering between unit voices and EVA differs.
- **frequency:** Whenever the player selects/orders a unit while an EVA announcement is active — frequent in active play.
- **rust today:** Rust splits unit voices and EVA into the same voice_player slot but treats them via separate entry points; unit voice cuts the current voice but the queued_voice EVA FIFO is a separate structure, so a unit voice does not flush a queued EVA the way gamemd's shared priority-2 StreamPlayer does.
- **evidence:** docs/research/SOUND_TRIGGERS_COMPLETE_GHIDRA_REPORT.md:547-564 (unit voice uses priority==2 which flushes the SAME StreamPlayer shared with EVA, interrupting a playing EVA); src/audio/sfx.rs:239-269 play_voice_sound uses a dedicated voice_player separate from EVA queue, only cutting the previous voice not the EVA queue


## UI / HUD / SIDEBAR / CURSOR / MINIMAP  — 14 findings

### [HIGH] Credits ticker animation (CreditsClass::AI)
- **type:** wrong-formula-or-timing
- **effect:** Credits counter rolls up and down at the same speed (gamemd rolls DOWN 3x slower than UP), and the roll speed is tied to render FPS instead of the 15Hz logic clock, so it animates faster/slower depending on framerate
- **frequency:** Every match, continuously — fires on every harvester dump, purchase, and refund
- **rust today:** Implemented but: ticks per render-frame not game-tick, and uses identical speed for count-up and count-down
- **evidence:** src/app_sidebar_render.rs:61-75 steps once per RENDER frame with one delay for both directions; gamemd CreditsClass::AI @0x004A2600 ticks per GAME frame at delay=1 frame/step UP, 3 frames/step DOWN (ORE_VALUE_CREDIT_DEPOSIT_GHIDRA_REPORT.md §10; SIDEBAR_SYSTEM §30)

### [HIGH] Credit tick sound (CreditUp/CreditDown via CreditTicks[])
- **type:** behavior-not-acted-on
- **effect:** The rapid 'tick-tick-tick' money-counter sound that plays while credits roll up/down is entirely silent
- **frequency:** Every match, very frequent — plays whenever the credits counter is mid-animation (every harvest deposit, every purchase)
- **rust today:** none found
- **evidence:** No credit tick sound anywhere in src/audio (grep for credit/CreditUp/tick_sound = no matches); gamemd CreditsClass__Draw @0x004A2370 asm 0x004A24F4 plays VocClass__PlayAtPos(CreditTicks[counting_up?0:1], 0.5f) every frame while animating (GLOBAL_SOUNDS_GHIDRA_REPORT.md credit-tick deep dive)

### [HIGH] Sidebar cameo unbuildable DARKEN.SHP overlay
- **type:** behavior-not-acted-on
- **effect:** Cameos for items you cannot currently build (prereq missing, can't afford) appear fully lit instead of dimmed, so the player loses the at-a-glance 'greyed out / unavailable' read
- **frequency:** Every match, constantly — most cameos on the build palette are unbuildable at any given moment
- **rust today:** item.enabled=false suppresses the build action in hit_test but the cameo still renders at full brightness
- **evidence:** src/app_sidebar_build.rs:491-506 draws non-building/disabled cameos at full tint=[1,1,1] alpha=1 with no darken; gamemd StripClass::Draw @0x006A9540 step 3 draws DARKEN.SHP (DAT_00B07BC0) frame 0 flags 0x401 over any unbuildable cameo (SIDEBAR_SYSTEM §12; SIDEBAR_LAYER_PALETTE §3)

### [HIGH] Sidebar cameo 'new item' flash pulse (FlashEndFrame)
- **type:** researched-not-implemented
- **effect:** A newly-unlocked buildable cameo does not pulse dim/bright to draw the eye; players miss the cue that a new tech/upgrade just became available
- **frequency:** Per-match, occasional — fires each time a new buildable appears (tech unlock, prereq met)
- **rust today:** none found
- **evidence:** No FlashEndFrame / 16-frame darken-pulse logic in src (grep = only in chrome/build files for unrelated uses); gamemd StripClass::Draw @0x006A9540 darkens cameo on frames 9-15 of every 16 while g_CurrentFrameCounter<FlashEndFrame (SIDEBAR_TIMING_AND_TOOLTIPS_GHIDRA_REPORT.md §3.1; CAMEO_FLASH_END_FRAME_WRITER)

### [HIGH] Power bar transition blink (PowerClass::Draw frame 4)
- **type:** wrong-draw-order
- **effect:** On a power transition that lands at zero surplus (e.g. dropping from surplus into exactly balanced/deficit), the boundary blink segment is missing, so the power-change flash looks incomplete
- **frequency:** Per-match, occasional — fires on power-state changes that hit the zero-surplus boundary (building destroyed/sold, power plant lost)
- **rust today:** Implemented but blink suppressed when surplus segment count is zero
- **evidence:** src/app_sidebar_build.rs:273 gates the frame-4 blink with `if flashing && n_surplus > 0`; gamemd PowerClass__Draw @0x0063FB20 draws frame 4 whenever the flash counter +0x151C is positive and even, NOT gated on surplus>0 (SIDEBAR_POWER_CREDITS_READY_TEXT_LAYOUT §2; Negative Facts)

### [HIGH] Selected-factory rally line (FUN_006DA9D0)
- **type:** researched-not-implemented
- **effect:** Selecting a war factory/barracks/refinery with a rally point set shows no pulsing line from the building to the rally cell; the player gets no visual confirmation of where new units will gather
- **frequency:** Every match, frequent — any time a production building with a rally point is selected
- **rust today:** Rally point data stored, no on-screen line rendered
- **evidence:** PLACEMENT_RALLY_WAYPOINT_VISUALS_GHIDRA_REPORT.md §6: rally line NOT IMPLEMENTED — HouseState.rally_point stored but no selected-building line drawn; gamemd FUN_006DA9D0 draws factory→rally line with house-color and pulse phase (0x7FFFFFFF-g_CurrentFrameCounter)%0xF pattern DAT_00842930

### [HIGH] Queued waypoint planning-path overlay (FUN_006DAD60)
- **type:** researched-not-implemented
- **effect:** Shift-queuing move/attack waypoints shows no connecting path line or per-waypoint markers on the tactical view, so the player can't see the queued route
- **frequency:** Per-match, frequent for micro-heavy players — every shift-click multi-waypoint order
- **rust today:** none found (no MOUSE.SHA planning marker, no path-segment overlay)
- **evidence:** PLACEMENT_RALLY_WAYPOINT_VISUALS_GHIDRA_REPORT.md §6: planning path lines/markers NOT IMPLEMENTED; gamemd FUN_006DAD60 draws all adjacent WaypointPathClass segments with MOUSE.SHA action index 0x3C marker, MaxWaypointPathLength=15

### [HIGH] EVA voice inter-announcement delay (VoxClass queue)
- **type:** researched-not-implemented
- **effect:** Back-to-back EVA announcements (e.g. 'Construction complete' then 'New construction options', or several 'Unit ready') play with no pause, running together instead of with gamemd's ~500ms breath between lines
- **frequency:** Every match, frequent — any time two EVA events fire close together (very common during base build-up and combat)
- **rust today:** Queue advances with zero gap between consecutive EVA clips
- **evidence:** src/audio/sfx.rs:271-306 queue_eva_sound comment explicitly defers 'inter-announcement delay'; advance_voice_queue starts the next clip the instant voice_player.empty(); gamemd VoxArray InterAnnouncementDelay_Lo=500 (EVA_SYSTEM_GHIDRA_REPORT.md global @0x00b1d4d0)

### [MEDIUM] Sidebar tooltip delay state machine (ToolTipManager)
- **type:** researched-not-implemented
- **effect:** Sidebar cameo/button tooltips either appear instantly or with egui's default timing instead of the 1000ms-after-stop / 10000ms-auto-hide behavior; tooltips feel wrong to muscle memory and never time out the same way
- **frequency:** Every match, frequent — any time the cursor rests over a cameo or sidebar button
- **rust today:** No hover-delay or auto-hide anywhere; egui default tooltip timing if any
- **evidence:** SIDEBAR_TIMING_AND_TOOLTIPS_GHIDRA_REPORT.md §8 lists tooltip delay timer and auto-hide as NOT IMPLEMENTED in src; gamemd ToolTipManager @0x00724000 uses DelayMs=1000, DurationMs=10000 real Win32 ms, mousemove restarts the delay (TOOLTIP_TEXT_SOURCE_AND_DELAY_TIMERS)

### [MEDIUM] Power bar flash + segment-slide timing (PowerClass animation)
- **type:** wrong-formula-or-timing
- **effect:** The power-bar flash blinks at a different rate and the colored segments slide toward their new level faster/slower than gamemd, so the bar's reaction to a power change feels off
- **frequency:** Per-match, occasional — every power-state change animates the bar
- **rust today:** Implemented with placeholder tick intervals (2 and 9) that do not match the verified 3-tick flash / 1-tick slide cadence
- **evidence:** src/sidebar/power_bar_anim.rs:20-27 uses FLASH_TICKS_PER_STEP=2 and SLIDE_TICKS_PER_STEP=9 self-labeled placeholders; gamemd is a 10-count flash at 3-tick intervals (30 ticks total) and ±1 segment lerp per tick (POWER_SYSTEM_GHIDRA_REPORT.md Animation System)

### [MEDIUM] Power bar position/scaling (PowerClass::Draw origin)
- **type:** wrong-draw-order
- **effect:** Power bar sits at a slightly different x/y and its segment tiles are stretched rather than drawn at native pixel size, and the Allied 5px / Soviet 0px x-offset distinction is not honored
- **frequency:** Every match, always visible — power bar is on screen the entire game
- **rust today:** Implemented but positions from tab-layout coords and stretches segment frames to a configured width instead of native SHP size
- **evidence:** SIDEBAR_POWER_CREDITS_READY_TEXT_LAYOUT §1 + handoff: gamemd draws POWERP.SHP at sidebar-surface x=5 (Allied) / x=0 (Soviet), y=g_SidebarWidth+0x45=227, native frame size, 3px y advance; Rust uses layout.tabs_y+spec.power_bar_top_y and stretches frames to power_bar_width (src/app_sidebar_build.rs render_power_bar, layout_spec.rs)

### [MEDIUM] Minimap local-object dot flash inversion (RadarClass::RenderCellPixel)
- **type:** behavior-not-acted-on
- **effect:** When a local-player object has an active radar flash (newly registered / event-pinged), its minimap dot does not blink by inverting its color the way gamemd's does
- **frequency:** Per-match, occasional — fires on local-object radar registration / flash-timer events
- **rust today:** Dot color is static per owner; no per-object flash timer / color inversion
- **evidence:** src/render/minimap.rs:285-339 stamps a static owner-color dot with no flash-phase inversion; gamemd RenderCellPixel @0x00655C50 inverts (~packed) the dot on odd phase = ((remaining-1)/RulesClass[0x88]) while a local object's radar flash timer is active (HOUSE_COLORSCHEME_TO_RADAR_DOT_PACKED_COLOR_GHIDRA_REPORT.md §2)

### [MEDIUM] Minimap dot overlap winner ordering (RadarClass::AddObjectToTracker)
- **type:** wrong-draw-order
- **effect:** When a friendly and enemy unit map to the same minimap pixel, the wrong one can show on top — a local unit can be hidden under an enemy dot (or vice versa) depending on entity id rather than ownership
- **frequency:** Per-match, situational — fires when friendly+enemy objects share a minimap pixel (common in mixed engagements on small/zoomed maps)
- **rust today:** Dots written in EntityStore id-iteration order, last-writer-wins; no local-front/enemy-back priority
- **evidence:** src/render/minimap.rs:285 iterates EntityStore.values() (sorted by entity id) and overwrites pixels in id order; gamemd AddObjectToTracker @0x00655560 inserts local-player objects at the FRONT of the per-pixel bucket and non-local at the back so the local object wins the shared pixel (RADAR_OBJECT_DOT_PRIORITY_VISIBILITY_GATES §10; MINIMAP_GENERATED_PIXEL_COLOR_PIPELINE §8)

### [MEDIUM] Sidebar hit-test tie-break ordering (GadgetClass smallest-area-wins)
- **type:** partial-implementation
- **effect:** Currently benign because in-game sidebar rects do not overlap, but any future overlapping gadget (tooltip rect over a tab button) would resolve to the wrong target; the tie-break rule itself diverges from gamemd
- **frequency:** Rare in current build (no overlapping sidebar rects today); would surface if overlapping gadgets are added
- **rust today:** First-match ordering, not smallest-area-wins
- **evidence:** src/sidebar/mod.rs:379-425 hit_test returns first rect that contains the point (tabs, then repair/sell, then items, then control buttons); gamemd GadgetClass dispatch uses smallest-gadget-area-wins on overlap (GADGET_UI_FRAMEWORK_GHIDRA_REPORT.md §10 item 1 explicitly flags the Rust sidebar 'returns the first-in-vec match')


## ECONOMY / PRODUCTION  — 12 findings

### [HIGH] Building repair tick (toggle-repair / wrench)
- **type:** wrong-formula-or-timing
- **effect:** Self-repairing buildings heal ~7x too fast (4 HP/tick continuous vs 8 HP every ~14 ticks = ~8.6 HP/s) and total repair cost is 25% of build cost instead of 15%; the wrench bar fills visibly faster and money drains at the wrong rate.
- **frequency:** Every time a player repairs a damaged building — routine in nearly every combat skirmish.
- **rust today:** tick_repairs heals 4 HP every sim tick (~60 HP/s) and charges cost*25%/maxHP per HP; never reads parsed RepairStep/RepairPercent/RepairRate.
- **evidence:** src/sim/production/production_sell.rs:757-760 hardcodes REPAIR_COST_PERCENT=25 and REPAIR_HP_PER_TICK=4 healing EVERY tick (tick_repairs :798 heals min(4,...) per call). ini/rulesmd.ini:27-29 RepairPercent=15%, RepairStep=8, RepairRate=.016 min (~14 ticks). rules/ruleset.rs:432-435 parses repair_step=8/repair_percent=15/unit_repair_rate_ticks=14 but the building path ignores them.

### [HIGH] Building sell refund formula
- **type:** wrong-formula-or-timing
- **effect:** Selling a damaged building refunds far less than gamemd (e.g. a 50%-HP building refunds 25% of cost in Rust vs full 50% in gamemd). If a mod changes RefundPercent, Rust ignores it. Credit delta shown after a sell is wrong.
- **frequency:** Every building sell, especially common when selling damaged structures to deny capture or recoup losses.
- **rust today:** Refund hardcoded to 50% AND multiplied by current health percentage; INI RefundPercent= is never read for the refund percentage.
- **evidence:** src/sim/production/production_sell.rs:23-24 SELL_REFUND_PERCENT=50 hardcoded; sell_refund_for_building :45-54 multiplies by health%/100. BUILDINGCLASS_SELL_AND_REPAIR_GHIDRA_REPORT.md sec 3: refund = Cost * Rules.SellBack/100 via GetCost_Adjusted, and 'binary does NOT scale the refund by current health ratio' (same % at 100% or 1% HP). ini/rulesmd.ini:25 RefundPercent=50% is the actual percentage key (SellBack=2 at :3168 is a bool gate, not %).

### [HIGH] Building sell (Mission_Selling state machine + deconstruction anim)
- **type:** partial-implementation
- **effect:** Sold buildings vanish instantly instead of playing the buildup-in-reverse deconstruction animation; the credit refund and any survivor ejection land on the wrong frame relative to the visual. A core, very visible economy interaction looks nothing like gamemd.
- **frequency:** Every building sell in every skirmish.
- **rust today:** Sell despawns the building and pays the refund the same tick the command runs; no reverse-construction sell animation, no deferred state-2 payout.
- **evidence:** src/sim/world/world_commands.rs:655-660 Command::SellBuilding -> production::sell_building which removes the entity immediately (production_sell.rs:687-737 sim.entities.remove + instant refund). BUILDINGCLASS_SELL_AND_REPAIR_GHIDRA_REPORT.md sec 1-2,7: Mission_Selling is a 3-state machine (Init/Eject+Anim/Payout); state 1 plays GrandOpening(0) reverse-construction (e.g. POWRMAKE.SHP reversed); refund/MCV-spawn happen in state 2 only after the anim's last frame sets field_0x6DD.

### [MEDIUM] Building sell — stored ore conversion
- **type:** behavior-not-acted-on
- **effect:** Selling a refinery (or silo) with unspent stored ore in gamemd returns extra credits for that ore; Rust under-pays by the stored-ore value, so the credit gain from selling storage buildings is too low.
- **frequency:** Occurs whenever a player sells a refinery or silo holding ore — moderately common mid/late game base teardown or relocation.
- **rust today:** sell_building grants only the SellBack-style refund; no conversion of ore held in a refinery/silo StorageClass on sell.
- **evidence:** BUILDINGCLASS_SELL_AND_REPAIR_GHIDRA_REPORT.md sec 3 'Stored ore: All tiberium in the building's StorageClass is converted to credits at the configured per-type rate and added to the owner. This is on top of the SellBack refund.' production_sell.rs sell_building :687-737 only adds the cost-based refund, never reads any stored-ore on the building.

### [MEDIUM] Refinery ore-dump cadence (unload deposit timing)
- **type:** wrong-formula-or-timing
- **effect:** Credits jump in two big lumps (ore slot, then gem slot) instead of ticking up bale-by-bale, and the refinery dump animation/smoke fires once per slot instead of per bale — the money counter and the dump VFX cadence differ from gamemd. NOTE: a prior REFINERY_STORAGE_FLOW finding claims gamemd itself drains whole-slot in ~15 frames; this contradiction should be resolved against the binary before fixing.
- **frequency:** Every harvester unload — fires constantly throughout every skirmish.
- **rust today:** One deposit event per slot drains all same-type bales' credits at once and emits one bale-deposit anim/smoke per slot, rather than one credit grant + one anim per bale at 14.4-frame intervals.
- **evidence:** src/sim/miner/miner_dock_sequence.rs phase_unloading :1064-1131 drains an ENTIRE resource-type SLOT (all bales of that type) in one ~14-tick threshold crossing. UNIT_MISSION_DEPLOY_BUILDING_GHIDRA_REPORT.md Dump State 3 and REFINERY_STORAGE_FLOW_GHIDRA_REPORT.md sec 4: gamemd transfers ONE bale per HarvesterDumpRate*900=14.4-frame gate (Mission_Deploy_Building state 3 per-bale credit transfer); the prior Rust per-bale model was rewritten to whole-slot.

### [MEDIUM] Cancel/cancel-by-type queue refund
- **type:** behavior-not-acted-on
- **effect:** Cancelling a partially-built unit/building refunds the full cost in Rust; gamemd only refunds the unspent portion (cost minus progress already paid), so Rust returns more credits than gamemd on a mid-progress cancel.
- **frequency:** Every right-click cancel of an in-progress queue item — a common UI action.
- **rust today:** Whole cost is deducted up-front at enqueue (enqueue_by_type :217) and the full cost is refunded on cancel — net-neutral for a queued-but-unstarted item, but for a partially-built front item gamemd has already spent the progressed fraction, so a full refund over-pays.
- **evidence:** src/sim/production/production_queue.rs cancel_last_for_owner :745-781 and cancel_by_type_for_owner :785-836 refund full obj.cost for the cancelled item. FACTORY_CREDIT_SYSTEM_GHIDRA_REPORT.md Add_Credits: AbandonProduction 'refunds TypeCost - Balance (unspent portion)'. gamemd charges credits incrementally as production progresses; cancelling refunds only what is left to spend, not the full cost.

### [MEDIUM] Production credit charging model (up-front vs incremental)
- **type:** wrong-formula-or-timing
- **effect:** Credits drop to the full cost the moment you click a cameo, rather than draining smoothly as the bar fills; a player who queues something then loses income never sees the build pause for funds (gamemd would 'On Hold - need credits'). The money counter animation during a build is visibly different.
- **frequency:** Every queued item; the visible difference is the credit-counter behavior during every build.
- **rust today:** Cost is fully spent at enqueue time; build never stalls for lack of funds mid-progress and the credit counter drops all at once instead of draining over the build.
- **evidence:** src/sim/production/production_queue.rs enqueue_by_type :217 deducts full obj.cost the instant an item is queued. FACTORY_CREDIT_SYSTEM_GHIDRA_REPORT.md / BUILD_QUEUE_GHIDRA_REPORT.md: gamemd FactoryClass draws credits incrementally as the build timer advances (the bar can stall when the player runs out of cash mid-build), refunding the unspent remainder on abandon.

### [LOW] BuildLimit negative-value semantics
- **type:** wrong-formula-or-timing
- **effect:** Units configured with negative BuildLimit (intended one-shot / total-ever caps) become re-buildable after the existing one dies, when gamemd would keep them permanently disabled in the sidebar. The cameo disabled-state differs.
- **frequency:** Rare in stock YR (few negative-BuildLimit types) but always wrong when one is present; mod-dependent.
- **rust today:** Negative and positive BuildLimit both block at abs(limit) simultaneous instances; no cumulative-ever-produced tracking.
- **evidence:** src/sim/production/production_tech.rs:179-184 effective_build_limit uses build_limit.unsigned_abs() for both signs — negative treated identically to positive (simultaneous cap). gamemd HouseClass__CheckBuildLimit (decompiled 0x50b370): the BuildLimit field param_2[0xee] is sign-branched; negative limits take the abs() path that counts via CountOwnedInstances+queued differently from the positive simultaneous path (YR documents negative BuildLimit as cumulative-ever, positive as simultaneous).

### [LOW] Multi-factory queue speed bonus exponent
- **type:** wrong-formula-or-timing
- **effect:** If the two paths disagree by one MultipleFactory factor, the sidebar build-timer countdown drifts from the actual completion moment when a player owns 2+ factories of one type — the clock and the green fill desync.
- **frequency:** Any time a player has 2+ war factories / barracks of the same category, common in mid-late game; needs a concrete trace to confirm the two paths agree.
- **rust today:** Two separate code paths compute the MultipleFactory damp (one in effective_progress_rate_ppm via matching_factory_time_multiplier_ppm, one in effective_time_to_build via apply_multiple_factory_scaling_ppm); the live progress rate uses the multiplier-PPM path while the UI estimate uses the other — risk of off-by-one exponent between displayed time and actual progress.
- **evidence:** src/sim/production/production_tech.rs:496-516 matching_factory_time_multiplier_ppm seeds result=mf_ppm then loops 1..(factory_count-1), giving mf^(n-1) for n factories; but apply_multiple_factory_scaling_ppm :429-443 (used by the other display path) loops 1..queue_factory_count giving mf^(n-1) too. ini/rulesmd.ini:368 MultipleFactory=0.8 ('cumulative ... 1, .8, .64, .512'). FACTORY_CLASS_BUILD_SPEED_GHIDRA_REPORT.md GetBuildStepTime 0x6F47A0 applies MultipleFactory once per extra factory in a loop over the house's matching factories.

### [LOW] Starting credits source
- **type:** partial-implementation
- **effect:** Starting cash shown on the credit counter at match start may not match the lobby-selected money level (e.g. 10000 starting credits maps), and AI handicap multipliers on starting cash are not applied.
- **frequency:** Once per match at start, but visible immediately on the HUD credit counter for every player.
- **rust today:** House credits default to a compile-time 5000 constant in the fallback paths; not driven by the skirmish lobby Credits setting / handicap.
- **evidence:** src/sim/production/production_types.rs:17 STARTING_CREDITS=5000 hardcoded constant; production_queue.rs:46,87 fall back to it. Real skirmish starting credits come from MP dialog/game-options (Credits= setting, default varies) and per-house handicap, not a fixed 5000.

### [LOW] Harvested-credit income multiplier / handicap
- **type:** behavior-not-acted-on
- **effect:** Harvester income per dump is the flat bale value; if a house has an income multiplier/handicap (notably AI difficulty economy boost, separate from AIVirtualPurifiers which IS handled), the credit gain per unload is wrong. The money counter climbs at the wrong rate for handicapped players.
- **frequency:** Every harvester unload for any house carrying a non-1.0 income multiplier; AI-economy-handicap matches every tick.
- **rust today:** Deposit credits the raw ore/gem bale value (25/50 per bale defaults); no per-house income multiplier or AI economy handicap applied to harvested credits.
- **evidence:** src/sim/miner/miner_dock_sequence.rs phase_unloading :1099-1102 adds slot_value (= sum of bale.value) straight to credits with no income/handicap multiplier. gamemd HouseClass__Add_Tiberium_Credits (decompiled 0x004F9610) takes a float param run through ftol — the caller HouseClass::DepositOreCredits (0x004F9610 chain, ORE_VALUE_CREDIT_DEPOSIT_GHIDRA_REPORT.md sec 3) multiplies ore value by an income/IncomeMultiplier factor before depositing.

### [LOW] Ore spread germination density (RA1-scan fallback path)
- **type:** partial-implementation
- **effect:** When the fallback ore-growth path runs, newly spread ore cells appear at density 1 (sparse, low-value) instead of density 3, so harvesters extract less per visit and the ore field regrows visibly thinner. Mostly masked by the native path being active in normal play.
- **frequency:** Only when native tiberium queues are not yet initialized (early load / edge); rare in steady-state skirmish.
- **rust today:** Native growth/spread driver uses density-3 germination (correct); the older scan/reservoir fallback at :1519 still germinates at level 1.
- **evidence:** src/sim/ore_growth.rs:1514-1520 legacy spread path inserts new ore node with remaining=ORE_BASE_PER_LEVEL (density level 1). PLACETIBERIUM_SPREAD_GERMINATION_CONSTRAINTS_AND_OVERLAY_FRAME_GHIDRA_REPORT.md: runtime spread calls PlaceTiberium(tib_type, 3) — new cells start at density 3, not 1. The native driver path (ore_growth.rs:1282 SPREAD_GERMINATION_DENSITY) already uses level 3, but the RA1-scan fallback (when native queues are not ready) still seeds level 1.


## ANIMATION + FACING FIDELITY  — 11 findings

### [HIGH] Vehicle body in-place rotation (turn before move)
- **type:** wrong-formula-or-timing
- **effect:** Vehicle body turn-in-place speed is wrong (about 2x too fast) and tick-rate-dependent rather than binary-frame-locked; every unit that turns from a non-aligned heading rotates faster/jerkier than gamemd
- **frequency:** Every match, every time any vehicle (MCV, tank, etc.) starts moving from a non-aligned facing
- **rust today:** ms-based per-tick step via rot_to_facing_delta; barrel/turret already uses correct FacingClass but body does not
- **evidence:** src/sim/movement/movement_step.rs:221-274 handle_vehicle_rotation() uses rot_to_facing_delta(rot, tick_ms) (turret.rs:32-43) — a per-45Hz-tick ms-integrated 8-bit delta. gamemd body facing is the timer-based FacingClass @ TechnoClass+0x370 (ROT=3 default), Start=g_CurrentFrameCounter, Duration=abs(delta)/ROT in binary frames. Documented as explicit DRIFT in FRAME_BASIS_MOVEMENT_TURRET_GHIDRA_REPORT.md §5.2 + H-1; the MCV trace shows the body turn is ~2x too fast.

### [HIGH] Idle turret return-to-body
- **type:** behavior-not-acted-on
- **effect:** After a tank/turreted unit loses its target, the Rust turret swings back to align with the hull; in gamemd the turret freezes pointing where it last aimed. Visible re-centering animation that should not occur
- **frequency:** Every match, every time a turreted unit finishes/loses a target without a new one (very common in combat lulls)
- **rust today:** actively rotates turret back to body facing when idle (opposite of gamemd)
- **evidence:** src/sim/movement/turret.rs:140-143 sets desired_facing = body_facing_to_turret(entity.facing) when entity has no attack_target, and combat_turret_facing_tests.rs:149 'idle_turret_returns_to_body_facing' asserts this. gamemd Facing_Update Section A (UNITCLASS_TURRET_TRACKING_AND_FIRE_TIMING_GHIDRA_REPORT.md §5.1) only aims the turret when there IS a Target; TurretAI (§3.2) only acquires targets, it never rotates the turret. With no target the turret holds its last facing.

### [HIGH] Infantry animation cadence (walk/fire/idle/etc.)
- **type:** wrong-formula-or-timing
- **effect:** Infantry walk/fire/idle/death animation speeds and the exact frame at which the shot fires differ from gamemd; visible as wrong-paced infantry motion and slightly off muzzle/fire timing on every infantryman
- **frequency:** Every match, continuously, for every infantry unit
- **rust today:** hardcoded ms-per-frame per SequenceKind; no action-delay table, no Normalized game-speed scaling, fire not frame-gated
- **evidence:** src/sim/animation.rs uses fixed per-sequence tick_ms (DEFAULT_WALK_TICK_MS=100, DEFAULT_STAND_TICK_MS=200, DEFAULT_IDLE_TICK_MS=120, DEFAULT_DIE_TICK_MS=80) advanced by dt_ms in advance_animation(). gamemd cadence comes from a binary action-delay table indexed by action id + ActionTimer (TICK_ANIMATION_FRAME_TIMING_EXTENSION_GHIDRA_REPORT.md Finding 7; TICK_AND_ANIMATION_SPEED §8) where six action ids {9,10,0x12,0x13,0x17,0x20} get Normalized() game-speed scaling; weapon fire is tied to a specific sequence FRAME, not a generic ROF tick.

### [HIGH] SHP-bodied vehicle frame cadence (Dolphin/Terror Drone/Giant Squid)
- **type:** wrong-formula-or-timing
- **effect:** Dolphin, Terror Drone, and Giant Squid body animation speed and firing-frame hold differ from gamemd; visible mis-paced swim/move/fire animation for those units
- **frequency:** Per match whenever a Dolphin/Terror Drone/Squid is present and moving or firing
- **rust today:** ms-per-frame model; WalkRate/IdleRate/BodyFrameCounter and firing_counter/2 not modeled
- **evidence:** src/rules/shp_vehicle_sequence.rs:19-24 hardcodes DEFAULT_VEHICLE_WALK_TICK_MS=100 / FIRE=80 / STAND=200 ms. gamemd drives SHP body frame from FootClass.BodyFrameCounter (this+0x538) gated by g_CurrentFrameCounter % WalkRate (moving) / % IdleRate (idle) with WalkRate default 1, IdleRate default 0 (TICK_AND_ANIMATION_SPEED §7; TICK_ANIMATION_FRAME_TIMING_EXTENSION Finding 6). No 900/Rate conversion, no ms. Firing body frame also persists 2 counter steps (firing_counter/2). WalkRate/IdleRate are not parsed by Rust (grep: only art_data WalkFrames count).

### [HIGH] Floating Disk (DISK) turret permaspin
- **type:** researched-not-implemented
- **effect:** Yuri's Floating Disk turret does not continuously spin at 32 ticks/rev with the discrete 256-step quantization; it would instead snap to body when idle (wrong)
- **frequency:** Per match only when a Floating Disk is present (single unit type)
- **rust today:** none found (only a forward-test in facing_class.rs; no runtime permaspin branch)
- **evidence:** UNITCLASS_TURRET_TRACKING_AND_FIRE_TIMING_GHIDRA_REPORT.md §5.2: TurretSpins=yes (Type+0xD21, only [DISK] in vanilla YR) makes the barrel target advance +8 8-bit units/tick = 11.25 deg/tick = 32 ticks/revolution, quantized to 256 discrete facings (NOT smooth). facing_class.rs has only a test 'turret_spins_formula_smoke_test'; turret.rs tick_turret_rotation has no TurretSpins branch — idle DISK turret would instead return to body (see idle-return gap).

### [MEDIUM] Turret idle scan (TurretScansNearby)
- **type:** researched-not-implemented
- **effect:** Units with TurretScansNearby (e.g. some defenses/turreted units) acquire adjacent targets on a different cadence/radius than gamemd, changing when the turret first swings toward a passing enemy
- **frequency:** Per match for units flagged TurretScansNearby when an enemy passes within 1 cell while idle
- **rust today:** none found (uses general retaliation targeting, not the TS-style 1-cell 1-in-8-frame idle scan)
- **evidence:** UNITCLASS_TURRET_TRACKING_AND_FIRE_TIMING_GHIDRA_REPORT.md §3 + §9 item 7-8: TurretAI does a 1-cell-radius (8 neighbor cells) idle target scan gated by g_CurrentFrameCounter & 0x80000007 == 0 (1-in-8 frames, ~0.53s at 15fps), enabled by TurretScansNearby (Type+0xD32). Rust uses broader tick_retaliation (turret.rs has no 8-cell 1-in-8 idle scan). §10 item 5 flags this as missing.

### [MEDIUM] Body-facing render smoothing (no interpolation struct)
- **type:** partial-implementation
- **effect:** Even high-turn-rate vehicles should render a body that smooths over a few frames; Rust snaps or steps differently, so body re-orientation looks subtly more abrupt than gamemd
- **frequency:** Every match on every body re-facing (turns, post-arrival re-orient)
- **rust today:** u8 body facing with per-tick step / instant snap on some paths; no ROT=3 FacingClass smoothing
- **evidence:** Body facing in Rust is stored as a u8 and snapped/stepped (movement_step.rs handle_vehicle_rotation; configure_motion_after_transition sets *facing = new_face for ROT<=0/infantry). gamemd body uses a 16-bit FacingClass @ TechnoClass+0x370 with ROT=3 default that smooths every body turn over abs(delta)/3 frames (FRAME_BASIS §7 negative fact #5; UNITCLASS_TURRET_TRACKING §10 item 9).

### [MEDIUM] Building build-up / build-down (make) animation timing
- **type:** wrong-formula-or-timing
- **effect:** Build-up/sell animations can play at the wrong speed or skip/duplicate frames when total_ticks != frame_count * anim_rate; the construction/teardown animation does not match gamemd's cadence
- **frequency:** Every match, every time a building finishes construction or is sold (frequent)
- **rust today:** proportional frame mapping over total_ticks, not fixed-Rate AnimClass playback
- **evidence:** src/app_instances/shp.rs:108-136 maps make-anim frames by linear progress = elapsed_ticks / total_ticks then frame = progress * total_make_frames (and reverse for build-down). gamemd Mission_Construction plays the _MAKE AnimClass at its own fixed Rate (900/Rate frame delay) and completes when the anim finishes (BUILDING_LIGHTSOURCE_POST_LOAD_REHYDRATE §4; BUILDINGCLASS_SELL_AND_REPAIR §7 GrandOpening), not a proportional stretch of frames over an arbitrary total_ticks.

### [MEDIUM] Harvester OREGATH mining-arm overlay cadence
- **type:** wrong-formula-or-timing
- **effect:** The ore-gathering arm overlay animates on a different phase/cadence and may not pick the facing-correct 15-frame block; harvester mining arm looks subtly off vs gamemd
- **frequency:** Per match while any harvester is actively mining ore
- **rust today:** flat 15-frame %15 loop at 67ms with no BodyFrameCounter+global coupling, no facing-block index, no 5-tick step timer
- **evidence:** src/sim/animation.rs:519-551 tick_harvest_overlays advances frame = (frame+1)%15 at HARVEST_OVERLAY_FRAME_MS=67 (one per 15Hz tick), decoupled from facing and body counter. gamemd OREGATH frame = (unit+0x538 BodyFrameCounter + g_CurrentFrameCounter) % 15 + facing-block where facing block = (7 - ((facing>>12)+1>>1 & 7))*15 (OREGATH_RENDERING_GHIDRA_REPORT.md §3); on FIRE-OK the harvester turn-anim uses a facing-indexed StepCounter with StepTimer Step=5/Rate=5 (UNITCLASS_GHIDRA_REPORT §6c; UNITCLASS_TURRET_TRACKING §9 items 15-16).

### [MEDIUM] FacingClass step_size<1 snap vs body path
- **type:** partial-implementation
- **effect:** Tiny body re-facing requests (smaller than one frame of ROT) should snap instantly in gamemd; Rust body path instead steps gradually, a 1-frame difference on small turns
- **frequency:** Per match on small heading corrections during pathing (common but subtle)
- **rust today:** snap-on-small-rotation present for barrel only; absent on body rotation path
- **evidence:** facing_class.rs:97-101 correctly implements the step_size<1 snap for the barrel FacingClass, but the BODY rotation path (movement_step.rs handle_vehicle_rotation via rot_to_facing_delta) does not use FacingClass at all, so it lacks the §2.2 'abs(diff)<ROT snaps instantly' edge-case behavior (UNITCLASS_TURRET_TRACKING §2.2; §10 item 3 lists 'No step_size<1 snap behavior' as missing).

### [LOW] compute_facing_to_target atan2 convention + ftol truncation
- **type:** wrong-formula-or-timing
- **effect:** Turret aim facing can differ by up to ~1 facing unit near cardinal directions due to truncate-vs-round and arg-order, producing a sub-degree aim bias on every turret aim
- **frequency:** Every match on every turret aim computation (sub-degree, but pervasive)
- **rust today:** uses facing_from_delta_int_u16; arg-order/truncation parity with gamemd's atan2(-dy,dx)+ftol not confirmed
- **evidence:** UNITCLASS_TURRET_TRACKING_AND_FIRE_TIMING_GHIDRA_REPORT.md §6 + §9 items 10-11: gamemd uses atan2(self_y - target_y, target_x - self_x) (= atan2(-dy, dx)) then Math::ftol truncate-toward-zero (NOT round). Rust facing_toward_lepton (turret.rs:47-64) routes through facing_from_delta_int_u16; §10 item 7 explicitly asks to verify the convention matches, and the doc notes the original convention was mis-documented (OPERATOR_OR_ORDER_DRIFT), so the exact arg order + truncation are unverified against the Rust helper.


## MAP / TERRAIN / BRIDGE / OVERLAY render+behavior  — 11 findings

### [HIGH] Ore/tiberium overlay rendering — position-deterministic SHP variant selection
- **type:** wrong-formula-or-timing
- **effect:** Ore field tile-to-tile visual variety (which of the 12 flat TIB SHPs each cell uses) does not match gamemd's deterministic checkerboard pattern; newly grown/spread/germinated ore shows the wrong variant. Field looks subtly different cell-by-cell across the whole ore patch.
- **frequency:** Every frame on every map with ore — ore is on essentially every skirmish map, so the divergent pattern is on screen constantly.
- **rust today:** Renders each ore cell using the overlay_id name stored in the map's OverlayPack, never recomputing the per-position SHP variant; ore that grows/spreads/germinates keeps a single frozen variant (spread also copies source overlay id per PLACETIBERIUM_SPREAD report).
- **evidence:** Ghidra CellClass__DrawOverlay_Body @0x0047F6A0 (verified this session): flat tiberium variant = g_OverlayTypeClass_Array[((cell+0x26 MapY * cell+0x24 MapX) % tib+0xe8 NumImages) + Image.ArrayIndex]; frame = cell+0x11e OverlayData. Rust src/app_instances/overlays.rs:262-295 keys the sprite by the map-stored overlay NAME (entry.overlay_id, frozen at load) with frame=live overlay_data. No (MapY*MapX)%NumImages variant is ever computed; OVERLAY_CLASS_SYSTEM_GHIDRA_REPORT.md:284.

### [HIGH] Smudge anim scorch/crater 50/50 RNG selector
- **type:** wrong-formula-or-timing
- **effect:** Same probability but different branch stream — whether a given impact/death leaves a scorch vs crater diverges from gamemd, and the RNG state desyncs from the reference for all subsequent draws (determinism/replay). Wrong smudge type appears under explosions.
- **frequency:** Fires on every animation with both Scorch=yes and Crater=yes (common explosion anims) — many times per combat engagement.
- **rust today:** Uses a raw u32 high-bit test instead of the RandomRanged(0,0x7FFFFFFE) ranged draw with 31-bit normalized < 0.5 compare and the 0x7FFFFFFF rejection retry.
- **evidence:** SMUDGE_RNG_CLASSIFICATION_GHIDRA_REPORT.md:124,157 (binary 0x0042507A..0x004250A6): gamemd calls RandomRanged(0,0x7FFFFFFE), rejects masked 0x7FFFFFFF, takes scorch only when result < 0x40000000. Rust src/sim/combat/smudge_dispatch.rs:211-213 rng_below_half_normalized uses `rng.next_u32() < 0x8000_0000` (raw high-bit test) — flagged RED.

### [HIGH] Runtime ore spread germination — density, overlay variant, target validation
- **type:** partial-implementation
- **effect:** When an ore patch spreads to an empty cell, the new ore appears at the wrong density level (1 vs 3 = sparse vs medium) and wrong/frozen sprite variant; ore can germinate on cells gamemd would reject (AllowTiberium=false tiles, TIBTRE cells).
- **frequency:** Fires whenever ore spreads near depleted/harvested patches — continuously on any map with TiberiumSpreads=yes (stock default), every spread interval.
- **rust today:** Two ore systems coexist: a partial native-queue model (NativeTiberium*, places density 3 via place_native_spread_tiberium) and the legacy RA1 scan try_spread_ore (level-1, source-id copy). The native path is gated behind native_growth_ready (world/mod.rs:1886); the legacy fallback still produces level-1/data-0 spread when native isn't ready. Theater AllowTiberium not parsed.
- **evidence:** PLACETIBERIUM_SPREAD_GERMINATION_CONSTRAINTS_AND_OVERLAY_FRAME_GHIDRA_REPORT.md:12,92-114,260-262 (binary CellClass::SpreadTiberium 0x00483780, PlaceTiberium 0x00487190): new spread cell = PlaceTiberium(type,3) → OverlayData=3 + random flat variant RandomRanged(0,11); target validated by CanPlaceTiberium (bridge mask 0x500, live building, SpawnsTiberium terrain, land Buildable, no overlay, flat, theater AllowTiberium). Rust legacy try_spread_ore (ore_growth.rs:1489-1522) inserts remaining=120 (level 1), copies source overlay id, overlay_data 0, and can_germinate only checks resource node + path-grid walkability.

### [HIGH] Factory rally line — pulse animation
- **type:** partial-implementation
- **effect:** Selected factory's rally line is a static solid line instead of the animated pulsing/marching pattern gamemd draws; the visual cadence when checking a rally point differs.
- **frequency:** Visible whenever a producing structure with a rally point is selected — frequent during base macro.
- **rust today:** Rally line is drawn (owner-tinted, src/app_target_lines.rs:137-174) but the pulse/dash phase is computed and thrown away; line is fully solid.
- **evidence:** PLACEMENT_RALLY_WAYPOINT_VISUALS_GHIDRA_REPORT.md:313,328-330 (renderer FUN_006DA9D0 uses g_CurrentFrameCounter phase). Rust src/app_target_lines.rs:352-361 emit_rally_line computes `let _phase = (0x7fff_ffff - tick) % 15;` but discards it and calls emit_solid_line — a static, non-pulsing line.

### [MEDIUM] Map overlay seed richness → density (11 vs 12 levels)
- **type:** wrong-formula-or-timing
- **effect:** Max-density ore cells yield one extra bale of credits vs gamemd (12 vs 11 levels at base 120 = 120 extra credits per fully-rich cell when fully harvested); cumulative economy drift across a full ore field.
- **frequency:** Triggers every time a max-density (frame 11) ore cell is fully harvested — common on rich starting ore patches, every match.
- **rust today:** seed_resource_nodes_from_overlays converts stored frame 0..11 into richness 1..12 and stock = base*richness, treating max-density ore as 12 bales rather than 11.
- **evidence:** REDUCE_TIBERIUM_FULL_REMOVAL_SIDE_EFFECTS_AND_RETURN_VALUE_GHIDRA_REPORT.md:191-203: binary full removal returns the pre-removal OverlayData byte; OverlayData=11 yields 11, not 12. Rust src/sim/production/production_queue.rs:155 seeds richness = entry.frame.min(11)+1, so OverlayData=11 becomes 12 levels of stock (base*12).

### [MEDIUM] Building placement ghost — per-cell invalid variants and wall ghost
- **type:** partial-implementation
- **effect:** Placement preview shows uniform 'invalid' where gamemd shows two distinct invalid colors (physically blocked vs outside build radius), and wall placement lacks the per-cell wall silhouette — placement feedback is less precise.
- **frequency:** Visible every time a building/wall is armed for placement — many times per match.
- **rust today:** Per-cell PLACE.SHP diamonds are drawn with a single 'invalid' state; no frame-2-vs-3 distinction (blocked vs out-of-build-radius) and no wall-segment silhouette (frame 1).
- **evidence:** PLACEMENT_RALLY_WAYPOINT_VISUALS_GHIDRA_REPORT.md:282-302,309-310 (PLACE.SHP frames 0=valid,1=wall,2=invalid-A,3=invalid-B; per-cell frame chosen by cell+0x11C byte). Rust src/render/selection_overlay.rs:507-552 uses only valid/invalid (frames 0/2-ish) per the report's table; frame-1 wall ghost and frame-2-vs-3 distinction NOT IMPLEMENTED.

### [MEDIUM] Planning/queued waypoint lines and markers
- **type:** researched-not-implemented
- **effect:** When the player plans a multi-waypoint route, the connecting line segments and per-waypoint markers gamemd shows are absent — the route shape isn't visible.
- **frequency:** Triggers only when the player uses planning/queued-waypoint mode (shift-queue) — moderate use, mostly micro-heavy players.
- **rust today:** No planning-mode path overlay; multi-segment queued waypoint routes and per-point markers are not rendered.
- **evidence:** PLACEMENT_RALLY_WAYPOINT_VISUALS_GHIDRA_REPORT.md:314,332-338 and PLANNING_QUEUED_WAYPOINT_LINES_AND_FLAGS_GHIDRA_REPORT.md (renderer FUN_006DAD60 draws all adjacent WaypointPathClass segments + MOUSE.SHA markers, can close a loop). Rust: report marks 'NOT IMPLEMENTED' — no WaypointPathClass-style overlay; only final endpoint at best.

### [MEDIUM] Bridge damage — per-cell damage-variant stepping vs whole-group destruction
- **type:** partial-implementation
- **effect:** A damaged-but-alive bridge does not show progressive per-cell damage art (cracked/damaged deck frames) before collapsing — it stays pristine then snaps to destroyed, instead of gamemd's incremental damage appearance.
- **frequency:** Triggers whenever a bridge takes weapon damage on a bridge map — situational, common on river/bridge maps under attack.
- **rust today:** RE-correct per-cell damage-step and ramp-collapse helpers exist in bridge_specs.rs and a damaged-variant flood-fill exists (bridge_state/mod.rs:1184-1344), but the live apply_damage path destroys the whole bridge group at once when group HP reaches 0 rather than stepping intermediate damage frames per cell.
- **evidence:** LAT_RETRIGGER_AND_BRIDGE_DAMAGE_VARIANT_GHIDRA_REPORT.md:315-322: gamemd flips per-cell damage bits (ProcessBridgeDamageStateMachine, UpdateRamp_*_DamageA/B) before any collapse; Rust src/sim/bridge_state.rs apply_damage() does 'whole-group binary destruction (all cells flip to destroyed when group HP hits 0)'. bridge_specs.rs has the RE-correct low_bridge_overlay_damage_step_ra2 + ramp collapse states (893-999, 1187-1288) but apply_damage uses whole-group binary flip.

### [MEDIUM] Cliff/rock/shore tile classification
- **type:** wrong-formula-or-timing
- **effect:** Cells that gamemd treats as cliff (impassable / cliff-back-impassable) vs non-cliff rock/shore can be misclassified — affecting which cells block movement, fire, and ore at cliff/shore edges; visible as units/ore behaving differently at terrain boundaries.
- **frequency:** Triggers on maps with cliff/shore terrain near pathing/ore — common on most theater maps with elevation or water.
- **rust today:** Cliff detection is name-substring based; theater CliffSet/CliffRamps/WaterCliffs/DestroyableCliffs ordinal keys are not parsed into tile-id ranges.
- **evidence:** THEATER_CLIFF_RAMP_TILE_CLASSIFICATION_GHIDRA_REPORT.md:130-141 and CLIFF_OBJECTS_GHIDRA_REPORT.md:446-463: gamemd classifies cliff/ramp via numeric theater tile-id ranges from [General] CliffSet/CliffRamps/WaterCliffs/DestroyableCliffs (0x00545150). Rust src/map/theater.rs:214 and resolved_terrain.rs:940 use string heuristics (name.contains('cliff')/'rock'/'shore'), conflating rocks and shore with cliffs; numeric keys not parsed.

### [LOW] Ore-on-slope removal during recalc
- **type:** behavior-not-acted-on
- **effect:** Ore that ends up on a steep-slope cell (map-authored or via terrain change) stays rendered/harvestable where gamemd would have removed it — a visible ore cell that shouldn't exist.
- **frequency:** Rare — only fires on cells whose slope index is >=5 with tiberium present; edge case on hand-authored maps.
- **rust today:** Spread/germination checks flat slope for NEW placement (cell_is_flat, slope_type==0), but there is no recalc path that removes pre-existing tiberium overlay when a cell's slope becomes >=5.
- **evidence:** ORE_OVERLAY_SYSTEM_GHIDRA_REPORT.md:817-831 (CellClass::RecalcAttributes 0x0047d2b0): when overlay is tiberium and recalculated SlopeIndex >= 5, gamemd REMOVES the overlay (ore can't sit on steep slopes). No corresponding slope>=5 ore-removal found in Rust overlay recalc; ore_growth spread checks slope==0 for placement but recalc-driven removal of already-present ore on steep slope is not modeled.

### [LOW] Wall overlay damage-level rendering and isolated-wall auto-destruct
- **type:** partial-implementation
- **effect:** Walls under fire may not show the correct intermediate damage frames, and the chain-destruction of adjacent connected wall segments and auto-collapse of isolated damaged segments may differ from gamemd — visible wall-grid teardown looks different.
- **frequency:** Triggers when walls are attacked — situational; walls are common base defenses, so fires in most matches with wall usage.
- **rust today:** Walls render the live overlay_data frame and have a destroy path, but the verified upper-nibble damage-level progression, penultimate-level 4-neighbor chain reaction (damage 200), and hardcoded isolated-fully-damaged auto-destruct per wall type are not clearly modeled.
- **evidence:** OVERLAY_CLASS_SYSTEM_GHIDRA_REPORT.md:247-251,352-461 (DestroyOverlay 0x00480CB0 + PostDestructionWallCleanup 0x00480630): wall OverlayData packs upper-nibble damage level + lower-nibble connectivity; chain-reaction at penultimate damage (chain damage 200), and hardcoded isolated+fully-damaged auto-destruct for GASAND/CYCL/GAWALL/BARB/FENC. Rust overlay render (app_instances/overlays.rs:254,341-357) treats wall frame = live overlay_data and nudges depth, but the damage-nibble visual stepping and isolated-auto-destruct rules need confirmation in sim/overlay_grid.rs (damage_wall_overlay referenced at overlay_grid.rs:1054 destroys leftmost cell directly).
