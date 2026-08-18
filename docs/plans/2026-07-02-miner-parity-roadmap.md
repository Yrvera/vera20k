# Miner Parity Roadmap — everything to reach gamemd parity

Source: docs/gap-scans/2026-07-02-disparity-scan-miner.md (140 confirmed gaps: G1-G19 HIGH,
M1-M33 MED, L1-L67 LOW, S1-S20 slave/OREGATH; NV1-NV26 open questions; per-lane evidence in
docs/gap-scans/_miner-scan-2026-07-02-lanes/). Every finding ID maps to exactly one workstream
below. Order within the list = recommended execution order. Each workstream is one
brainstorm->plan->implement arc unless marked trivial.

## W0 — Quick wins (no research gate, independent, do first)
- G1 AI-difficulty/purifier index inversion (one mapping family + tests). Follow-on: M14
  per-house AI difficulty (small plumbing).
- L6 no-ore retry 106->105.
- G13 CreditTicks sound + M15 counter cadence (game-frame stepping, 1-up/3-down delay).
- G18 VoiceHarvest + M13 VoiceEnter + S-voices (VoiceDeploy/MoveSound) — parse keys, extend the
  order-voice service.
- L48 ChronoIn/OutSound read from [AudioVisual] (+ correct missing-key default).
- L40 wire tiberium Value= into bale values; L41 PurifierBonus full precision; L42 DumpRate
  full-precision gate (all mod-only, trivial).
- L21 RadioMessage "name inferred" annotations (doc-hygiene in code).
- HOLD: the 18-vs-19 harvest gate (L5) until NV26 settles which is right.

## W1 — Mission-cadence service (single highest-leverage arc)
Root cause: parsed MissionControl.rate_frames has zero consumers; miner FSM + radio retries run
per-tick. Build one Rate-gated dispatch seam (fits the substrate scheduler track; keep
advance_tick phase order).
- Closed (LANDED 2026-07-06): G5 (close-return collapse), G6 (per-tick HELLO/dispatch), L1
  (hardcoded 14/14/5 + round-vs-ftol), L9/L10 (missing RandomRanged(0,2) draws), L20 (EnterDock
  dupes); M1-timing half + M5 cadence half advanced by the shared seam.
- **S12 (slave 10-frame cadence) NOT in W1 — deferred to W11** (unverified, no SlaveManager doc).
- Also restores gamemd RNG-draw pattern (lockstep-stream parity).

## W2 — Dock radio protocol pass (docs now conflict-free after GH-1/GH-2)
- G7 replace bypass_grid beeline with pathfound drive to the accepted cell.
- G8 far-return staging stop (stop overriding the QueueingCell move).
- M1 HARV close-radio HELLO at HarvesterTooFarDistance.
- M4 0x13 NEED_TO_MOVE probe + GetDockCoord(+0x5A4) side-check redirect (adds the NW+(2,1) frame).
- M5 per-dispatch 0x0E/0x12 re-issue while driving.
- M6 denied-miner movement per GH-2 (full refinery still directs prober to (3,1)).
- M7 GH-1 contact-liveness gate + scatter/latch-clear cleanup in Mission_Deploy passes.
- L11 power NEGATORY gate + BREAK-on-hard-reject; L14 remove pad_clear_or_self invented gate;
  L15 CanDock reply code; L16 remove IsOccupied receiver; L17 0x0E sound cue; L19 conditional
  BREAK; L28 same-pass 0x16->0x15 cascade; L31/L32 extra 0x15 sources; L33 PerCellProcess(2)
  side effects (crowd counters may defer to substrate); L13 eviction BREAK; L12 Is_Ally gate.

## W3 — Dock search + reservation model
- G9 two-pass Find_Docking_Bay (free-slot-first arg3=0, then arg3=1 fallback).
- M3 re-run search every state-2 evaluation (kill sticky reserved_refinery).
- M2 refused-close chrono warps to staging (teleport locomotor route).
- L4 selection metric (3D lepton to building coord); L36 evaluator details (+0x3D3 -> NV20 first);
  L2/L3 threshold boundary + anchor (gate on NV8/NV9); L30 QueueingCell ctor default.

## W4 — FNPC cutover (plan already written — execute)
docs/plans/2026-06-04-cell-validation-facade-implementation-plan.md
- M31 (radius 32, 24-candidate pool, direct/indirect, binary_frame index, clear-on-failure),
  L67 occupancy input, bunker_link.rs consumer, then the remaining ~39-caller cutover ladder
  (T7/T8 shadow->invert->authoritative).

## W5 — Dock pivot & facing (GATE: NV7 Ghidra read of the 0x16 receiver first)
- G2 East pivot rendered (write the shared unit facing timer, not the private one; PF-C2
  ownership fix).
- L22 re-evaluate facing window in states 3/4; L29 +0x6AF bypass; S7 OREGATH facing rounding
  (half-bucket); L60 drive-track facing via FacingClass interpolation (verify vs track-point
  semantics first).

## W6 — Combat integration (slots into planned object-AI S5 slice)
- G3 passive/opportunity acquisition (parse CanPassiveAquire, consume OpportunityFire; mission
  eligibility {Move,Harvest,Guard}; excludes Enter/Unload — needs the real per-miner mission
  visible to the scanner, ties W1).
- G4 retaliation honors MissionControl Retaliate + type CanRetaliate.
- M19 warp stop-targeting/anim-detach; TL-C15 post-warp re-acquisition falls out of G3.

## W7 — Interrupt family: radio 0x17 end-to-end
- Implement 0x17: building death/sell broadcast to contacts -> unit reaction (immediate latch +
  display clear, scatter, Mission Harvest) = M11; unit-side QUEUED handler for factory/depot =
  L18; delete invented force-track-on-sell and unify sell/destroy/temporal = M12 (+L23 head
  offset dies with it, L24 stays stock-unreachable); M10 player-order early-exit mid-unload;
  L37 ForcedReturn->Harvest after abort; L38 scatter for interrupted miners; L39 conditional
  abort BREAK.

## W8 — Building-anim + world-effect render services
- Anim-slot control service (set/clear/skip-while-playing/damaged-variant): G10 empty-gate cut,
  M9 restart guard (NV22 first), M8 final N+1 burst, L26/L27 slot 7/8 triggers (parse
  PreProductionAnim), L25 slot-8 depart guard, S17 emission order, UM-C9 damaged variants (NV14).
- World-effect fidelity: G12 translucency consumption, L51 Flat/Layer/YSortAdjust, NV19 centering.
- M33 dock-cell ore-eat (squish + DestroyOverlay + rocking).
- L46 storage-tier pile display (gate on NV24); S18 RefinerySmokeFrames; S8/M28 OREGATH phase
  (per-unit offset, no reset, global clock); S20 OREGATH gates + z-bias (NV25).

## W9 — Teleport/warp fidelity
- G11 remove arrival shimmer (user-verified; keep both sounds).
- G16 chrono lock: block fire + movement orders; render 50% translucency.
- G17 sounds on player-ordered warps; M17 sound timing (emit at relocate, not issue).
- M18 arrival z/bridge handling; M29 PostWarpValidation (terrain death + occupant displacement)
  + issue-time destination validation; M30 route player path through the Set_Destination bridge
  (gate on NV2).
- L49 sound z anchor; L50 cleanup row; L52/L53 timer model; L54 3D delay distance; L55 infantry
  sub-cell; L56 Warpable=; L57 piggyback restore gates; L58 commit gates; L59 cancel-warp in
  SearchOre; L44/L45 (CompEasyBonus, award gate) sit in W0-adjacent trivia.
- GATES: NV2 (player CMIN order), NV3 (WAKE2) before touching warp anim composition.

## W10 — Ore scan/harvest FSM fidelity
- G14 density-0 ore trap (clear-or-consume rule — decide vs L8 husk semantics).
- G15 commit-to-target while destination set (kills mid-drive retarget); M20 archive return
  survives; M23 stuck recovery = short-scan-6 then return; M24 remove map-wide fallback ->
  M25 Mission_Guard_Harvester complex (repair-bay, move-off, re-trigger) + L64 ore-depleted
  flag; M26 no-refinery hold-state; M21/M22 timing (gate on NV4/NV26 extraction granularity);
  M27 ring value formula; L61 zone-0 scans; L62 archive validation removal; L63 direct-move/
  cost exemptions; L65 campaign shroud gate; S10 slave scan anchor.

## W11 — Slave miner program (GATE: research first)
/re-investigate SlaveManager 0x006AFD60 + MasterDestroyed liberation (no doc exists), then:
- S1 manager state machine (deploy-seek, retry, damage relocation, auto-undeploy/redeploy,
  kick-scan); S2 liberation flow + SlavesFreeSound; S3 deploy validation + CannotDeployHere;
  S4 enslaved-unit gates (no-fire/no-command/drag-box/dual voice); S5 buildup/reverse anims;
  S6 spawn/respawn geometry; S9 dock limbo/reload/heal; S11 undeploy gates; S13 exact-cell
  deposit; S14 leash; S15 mind-control semantics; S16 free-miner refund + cell formula;
  M16 whole-storage deposit at dock cell; G19 deploy sounds + respawn rates; L47 DockUnload=
  parse; S19 DockAnim.

## W12 — Ghidra NV burn-down (half-day, unblocks W3/W5/W8/W9/W10 details)
NV1 DONE (Harvester flag — no disparity). Remaining, in value order:
NV2 player-CMIN-order teleport (0x741970) [gates W9/M30]; NV7 0x16 pivot mechanism [gates W5];
NV3 WAKE2 semantics [gates W9 anims]; NV4/NV26 extraction + step-gate granularity
(Harvest_Ore_Tick / StepTimer) [gates W10 timing + L5]; NV21/NV22 anim pulse cadence + slot-10
guard (0x73E384) [gates W8]; NV5 Mission_Enter teleport branch (0x004D9290); NV6 RecalcBonuses
power gating; NV8/NV9 threshold anchor + building coord; NV23 smoke anchor; NV24 storage-tier
visibility; the rest (NV10-NV20, NV25) opportunistically.

## W13 — Research-doc correction pass
13 doc errors listed in the report (incl. the +0xE0E "Teleporter" mislabel in 5 locomotion docs,
the Has_Valid_Steps label, NEXT_DOCKER pseudocode, ZoneType-6 trace claim). Run
/verify-doc-fix-swarm scoped to those docs, citing GH-1..GH-4.

## Deferred (correctly absent — needs other systems first)
Chronosphere warp pipeline + WarpAttach (superweapon system); crate pickup (crate system);
observer display (observer mode); SpentCredits UI (score screen); L66 tiberium slots 2/3
(TS-adjacent); carryall/0x0B/0x1A/0x1B radio paths (dormant).

## Suggested sequence
1. W0 (day of quick wins) + W12 NV burn-down in parallel sessions.
2. W1 mission-cadence (unblocks honest timing everywhere) -> W2 dock protocol -> W3 search.
3. W4 FNPC cutover (plan exists, mechanical).
4. W5 pivot (after NV7) + W8 render services — the two biggest visible-fidelity wins.
5. W6 combat (with the S5 slice), W7 interrupts, W9 teleport, W10 scan FSM.
6. W11 slave program last (research-gated, self-contained).
7. W13 doc corrections whenever Ghidra findings accumulate.

## Status (updated 2026-07-06)

### W0 — quick wins: LANDED (9 findings on `dev`, one commit each, each with a focused test)
- `66a9891f` **G18** VoiceHarvest — parse + harvest-order dispatch (order-voice service).
- `2fe90587` **M13** VoiceEnter — manual refinery-return voice, same seam.
- `46e9214a` **L21** RadioMessage — 5 inferred-name annotations (comment-only).
- `9d7bd12c` **L6** no-ore retry 106→105 (dropped the `+1` fencepost at both arm sites; boundary test).
- `3aa97140` **G1** AI-difficulty→virtual-purifier inversion fixed at the single lookup (remap
  `2 - difficulty`); stored convention unchanged → no state-hash bump.
- `319c3134` **L48** ChronoIn/OutSound read from `[AudioVisual]`, fabricated default dropped
  (+ a follow-up commit fixing the fallback test helper).
- `9606745a` **L42** HarvesterDumpRate full-precision gate (`ceil(rate×900)`, dropped tenths
  quantization; field renamed `harvester_dump_tenths`→`harvester_dump_frames`).
- `f3ea03bc` **L40** ore/gem bale credits sourced from `[Tiberiums] Value=` via
  `MinerConfig::from_rules` (5 call sites); stock 25/50 byte-identical.
- `e783d91c` **L41** PurifierBonus full fixed-point precision (`purifier_bonus_ppm: i64`, single
  i128 truncation); stock `.25`→250_000 byte-identical.

Verified: focused `cargo test -p vera20k <name>` per finding; full miner suite green (155 passed).
Stock-parity note: G1 is the only one that changes stock output (it fixes an inversion); L42/L40/L41
are provably stock byte-identical (mod-visible only); the audio/annotation fixes don't touch the hash.

**W0 skipped (logged, not done):**
- **M14** (per-house AI difficulty) — needs a new `HouseState.difficulty` field threaded through
  house construction + a state-hash contribution; follow-on to G1 (must feed the same `2 - difficulty`
  remap). Own slice.
- **VoiceDeploy** (M32) — crosses the sim→audio seam (needs a `SimSoundEvent` carrier) AND has an
  unresolved undeploy-voice-source question (SMIN vs YAREFN); not a clean quick win.
- **MoveSound** (M32) — a looping movement sound, needs a per-entity looping-audio channel, not the
  voice service. Own feature.
- **G13 + M15** (CreditTicks sound + counter cadence) — the "credits-ticker service": needs a new
  counter-anim struct in app state + a game-frame step gate (1-up/3-down) + a render-path sound cue
  across 4 files with borrow-check care. HIGH-value but not a one-liner; own slice. (Note: `CreditTicks`
  is in `[AudioVisual]`, not `[General]` as the scan said.)
- **L5** (harvest gate 18-vs-19) — HOLD per plan, gated on NV26/NV4 (StepTimer granularity).

### W12 — Ghidra NV burn-down: NV2, NV3, NV7 RESOLVED (written into the scan's NV section, inline-cited)
- **NV2 — Rust CORRECT.** A player move-to-ground does NOT teleport a CMIN; the `Set_Destination`
  teleport-locomotor swap requires a building destination (RTTI 6 + dock flag `+0x16b3`), i.e. the
  inbound-to-refinery harvest leg. The teleport report §21 "Set_Destination warps it" claim is DRIFT.
  (`[CMIN] Teleporter=yes` rulesmd:7396; `decompile_function 0x00741970`.)
- **NV3 — Rust CORRECT.** gamemd strips the inline `;WAKE2` at INI load: `CCINIClass::ReadString`
  only `strtrim`s and `AnimTypeClass::FindOrAllocate` whole-name-matches (blank-allocates on miss),
  so a retained `;WAKE2` would render nothing — contradicting the observed WARPOUT shimmer. `;` is a
  comment; no second anim. The "WAKE2 ring" trace is DRIFT. (`0x00528a10`, `0x00428b80`, read `0x0066e1c6`.)
- **NV7 — G2 root cause confirmed.** The dock `0x16` pivot is `DriveLocomotion::Do_Turn(0x4000)`
  (DriveLocomotion vtable slot `0x4c`, base `0x007e7eb0` read live) gated on `Is_Moving_Now` (slot
  `0x10`) — the SHARED locomotor body facing the renderer reads, not a private FacingClass. G2 must
  route the pivot through the locomotor turn, not the miner-private `dock_pivot_facing`.
- NV1 remains DONE (Harvester flag — no disparity).

### W1 — mission-cadence service: **LANDED 2026-07-06** (branch `w1-mission-cadence` off `dev` 769f18f7)
Shipped task-by-task (A1 ftol, A2 table-wired Enter/Unload cadence, B1/G6 approach-HELLO gate,
B2/G5 accepted-HELLO arms the cadence, B3/L20 0x18-per-due-pass, C1/L9 + C2/L10 the two missing
`RandomRanged(0,2)` draws on `Scen->Random`). 8 commits, one focused test each; full lib suite green
(4051 passed). **Golden re-baseline was a NO-OP**: `GLOBAL_HARNESS_FINAL_HASH` did not move because the
global harness harvester never completes a dock handshake (its coverage tripwire only asserts
ore-target acquisition; dock coverage is delegated to the miner-dock suite) — so none of the new dock
draws fire in that scenario. Documented in `global_parity_harness_tests.rs` (W1 UNSHIFTED note);
`SNAPSHOT_VERSION` unchanged. **Open follow-up (F1):** the first CAN_DOCK / SearchOre resume may land
1 frame early vs gamemd's `queue→commence→dispatch` (+1) — the plan targeted `14..16`, ledger prose
says `15..17`; needs a `Mission_Dispatch @0x005b3060` decompile to settle (see
`docs/fidelity-checks/miner-close-return-cadence-w1.md`). Follow-up: the global harness does not
determinism-ratchet the dock path — consider extending it (or a dedicated dock-cycle harness).

### W1 — (prior design/plan record)
DESIGN + PLAN: gates resolved, implemented as above.
`docs/plans/2026-07-06-mission-cadence-service-design.md` (design) +
`docs/plans/2026-07-06-mission-cadence-service-plan.md` (implementation plan, 3 steps A/B/C + rollout).
Both Ghidra gates RESOLVED 2026-07-06: **U-consumer** — the mission timer gates the WHOLE handler
(`Mission_Dispatch 0x005b3060` calls the handler only when due, stores its return as the next
duration); **U1** — all cadence `RandomRanged(0,2)` draws use `Scen->Random`
(`g_ScenarioClass_Instance+0x218`), and Rust's `miner_jitter_rng()` already routes there
(test-pinned) so no RNG-routing fix is needed. Implementation changes the replay hash → one
documented `GLOBAL_HARNESS_FINAL_HASH` re-baseline (no `SNAPSHOT_VERSION` bump). Summary of the seam:
hybrid Rate-consumer seam wiring
`MissionControl.rate_frames` into the miner Enter/Unload timers, gating the per-tick approach-HELLO
(G6) + accepted-HELLO always-due collapse (G5), fixing the `.round()`→ftol conversion (L1), the two
missing `RandomRanged(0,2)` draws (L9/L10), and the `0x18` idempotency (L20). Key findings from
research: `MissionControl` is fully parsed with ZERO consumers; the mission/radio substrate already
shipped (`MissionCom` authoritative+hashed, `MissionTimer` is a direct port of gamemd's inclusive due
semantics). **W1 changes the state hash** (cadence + new RNG draws) → needs ONE documented re-baseline.
Gates before the authoritative flip: RNG-instance per draw (U1), Rate-gates-whole-handler-vs-rescan
(U-consumer). S12 (slave 10-frame) deferred to W11 (unverified, no SlaveManager doc).

### Next (recommended)
1. W1 LANDED — settle F1 (1-frame commence question) with a `Mission_Dispatch @0x005b3060` decompile,
   then W2 dock-protocol pass (M4/M5/M6) builds on the W1 cadence seam.
2. Optional: extend the global determinism harness (or add a dock-cycle harness) so the W1 dock-path
   RNG draws are ratcheted; today only the miner-dock suite covers them.
3. W5 pivot (G2) is now unblocked by NV7 — small fix: write the shared locomotor facing via Do_Turn.
4. G13+M15 credits-ticker service and M14 per-house difficulty are the two cleanest deferred W0
   follow-ons.
