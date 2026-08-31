# Phase 14 Exhaustive Disparity Scan

Date: 2026-08-31

Plan hypothesis: `docs/plans/2026-07-30-clean-slate-system-implementation-order.md`, rows 298-320

Compared revision: `origin/main@054696bb91a1daf066915ecdc44364deadfba91e`

Active binary: retail `gamemd.exe`, x86 image base `0x00400000`

Verdict: **OPEN — 5 matches, 14 verified implementation gaps, 2 conditional gaps, and 2 policy/research boundaries**

## Question

Rederive the Phase 14 frontier from current `origin/main`, treating the implementation-order
document's scope, exceptions, skip dispositions, and proposed mechanisms as hypotheses. Preserve
current matches, correct stale claims, and rank the smallest dependency-coherent mechanisms that
must close before the phase can be certified.

This scan is read-only except for this report. It does not authorize code edits, Ghidra metadata
changes, or borrowing unmerged work from another feature branch.

## Reconciliation and evidence method

- `main` and `origin/main` both resolved to `054696bb91a1daf066915ecdc44364deadfba91e` after fetch.
- The main checkout was clean; no Cargo or rustc process was running.
- Existing worktrees and branches belong to other tasks and were not modified.
- Draft PR #172 owns unmerged crate commits `7bf7c261`, `baa52ac8`, and `1984720f`; none is an
  ancestor of `origin/main`, so they are candidate evidence only.
- Three independent read-only audit lanes covered shell/settings, optional gameplay, and
  campaign/media/online. The parent directly spot-checked at least five exact current-Rust claims
  from every lane; no delegated Rust-state claim failed.
- Load-bearing binary corrections and Options flow were independently checked in the active YR
  program. Retail loose settings were also checked directly: the active `RA2MD.INI` contains
  `ScreenWidth=640`, `ScreenHeight=480`, `SoundVolume=0.700000`, `VoiceVolume=0.800000`, and
  `ScoreVolume=0.600000`.

## Scope corrections

1. Rows 303-304 remain in scope despite their Phase 14 placement. The plan explicitly makes
   crates milestone-visible for Phases 6-7, and stock skirmish enables them.
2. Row 306 cannot use the plan's `SKIP/PROVE` disposition. Active YR has a live Patrol assigner
   and a real Patrol handler; the plan's Hunt-alias/no-assigner premise is false.
3. Row 312 subtitles is conditional rather than a stock blocker for the active retail dataset:
   the native path is live, but the installed subtitle data is effectively empty.
4. Rows 319-320 cannot mean restoration of the historical Westwood Online service. Phase closure
   needs an explicit, honest compatibility policy and local shell behavior; a replacement service
   is a separate product decision.
5. Older reports that said the main menu jumped directly to Skirmish are stale. The current Rust
   route includes the stock Single Player shell and return path.

## Row-by-row frontier

| Row | System | Verdict | Current evidence and required disposition |
|---:|---|---|---|
| 298 | Main menu and shell transitions | **PASS** | Main-menu codes 1-6, `MainMenu -> SinglePlayer -> Skirmish`, Back routing, no-hover E2 art, and numeric version fallback match current active-YR evidence. Preserve. |
| 299 | Options, hotkeys, display, audio | **FAIL** | Launcher Options is a zero-field placeholder (`src/ui/main_menu_dialogs.rs:121-186`); configured video size is parsed but not consumed; Sound/Voice settings are disconnected; in-game Keyboard/Sound buttons are no-ops; no hotkey editor/writer exists. Split after a shared profile owner. |
| 300 | Settings/hotkeys/profile persistence | **FAIL** | Startup reads only ScrollRate, DetailLevel, and ScoreVolume through separate paths; Options close writes six `[Options]` keys; quit writes ScoreVolume separately. Active `OptionsClass` reads/writes a process-owned `[Options]`/`[Video]`/`[Audio]` transaction. Existing Skirmish profile persistence is already matched and must not be rebuilt. |
| 301 | Random-map RNG | **PASS** | Separate exact XOR-LFG, seed transform, x87 range math/rejection, and cursor continuation are present. Preserve. |
| 302 | Random-map generation/preview | **PASS** | Production worker publishes the six distinct progress images representing native redraw stages, accepted maps regenerate from `.SED`, and no current-main omission was verified in reviewed offline scope. Preserve. |
| 303 | Crates/powerups | **FAIL — milestone critical** | Current main explicitly owns placement only (`src/sim/crates.rs:17-19`); its 256-slot table is local and discarded. Full rules, persistent slots/timers, regeneration, pickup, effects, save/hash, and arrival continuations are absent. |
| 304 | Crate combat modifiers | **FAIL** | Armor has a stored multiplier but no crate writer; persistent firepower and Foot speed powerup fields/consumers are absent. Depends on crate authority and pickup. |
| 305 | Convoy chains/cohesion | **FAIL — conditional** | Rust move-group minimum-speed sync is not native scenario-authored `Unit+0x6C8` linkage/follower state. Stock `IsTrain=` activation was not found, but map-link frequency remains unproven rather than zero. |
| 306 | Patrol | **FAIL — plan hypothesis falsified** | Active `FootClass__ClickedAction_Cell @ 0x004D7D50`, action `0x33`, queues mission `0x19`; `UnitClass__Mission_Patrol @ 0x00740B10` calls real `FootClass__Mission_Patrol @ 0x004D4280`. Rust collapses the click to actor-anchored Guard and has no Patrol handler/threat branch. |
| 307 | Harmless | **PASS — authored/conditional** | Native base stub returns 450; Rust round-trips Harmless and uses the exact shared base cadence. Preserve. |
| 308 | Rescue | **FAIL — active AI path** | Rust already assigns Rescue for the verified base-defense response, but has no Rescue leaf handler, leaving a live assigned mission unserviced. |
| 309 | Bink/VQA interface | **PARTIAL** | No generic basename resolver that tries BIK then VQA. Retail mounted assets are BIK; typed `UnsupportedVqa` is sufficient for stock compatibility unless a decoder is later added. |
| 310 | Movie playback/audio sync | **FAIL** | Production playback is video-only and elapsed/catch-up-cap paced even though a Bink audio decoder exists. Native delegates frame readiness to Bink wait and stock campaign movies carry audio. |
| 311 | Briefing/cinematic speech | **FAIL** | EVA infrastructure exists, but trigger speech action `0x15` is unrepresented and silently dropped; briefing text has no production presentation consumer. |
| 312 | Subtitles/speech text | **CONDITIONAL GAP** | Active Bink/VQA subtitle lookup and timed caption path exists in YR; Rust has none. Installed stock subtitle data makes ordinary trigger frequency zero, so implement after stock campaign blockers unless populated/addon data is declared in scope. |
| 313 | Campaign catalog/selection | **FAIL** | `ScenarioCatalog` is skirmish-only; campaign content is intentionally excluded from map discovery; the campaign selector exposes no launch action. |
| 314 | Mission selection/briefing/objectives | **FAIL + research prerequisite** | `[Briefing]` is parsed but unused. Objective presentation/state and exact stock restatement semantics require a dedicated live-binary contract before implementation. |
| 315 | Campaign scripting/cinematics | **FAIL — broad stock blocker** | Trigger runtime implements a narrow subset and silently ignores the rest. A census of ordinary actions/events in retail campaign maps must drive bounded mechanism branches. |
| 316 | Campaign progression/carryover | **FAIL** | No standard `BATTLEMD*.INI` campaign controller, next-mission route, or carryover owner exists. Cooperative Skirmish state is not a substitute. |
| 317 | Campaign persistence | **FAIL** | No standard campaign index/progression/carryover persistence owner exists. |
| 318 | Movies/previews/credits | **FAIL** | All three dialog actions are explicit no-ops despite retail movie/credits assets. Depends on source resolution and blocking playback. |
| 319 | Online compatibility/replacement | **POLICY BOUNDARY** | Historical WOL is service-backed and unavailable. Literal operational restoration is not a sensible binary-parity target; closure requires an explicit disabled/replacement policy. |
| 320 | Online account/chat/game shell | **FAIL + policy dependency** | WOL/Network actions only log; there is no socket/protocol/account/lobby/chat service. Local lockstep is not an online shell. |

## Verified gaps, ranked by dependency frontier

1. **Process-owned Options profile transaction** — earliest plan-order gap and shared prerequisite
   for launcher controls, window size, SFX/voice/music, in-game subdialogs, and persistence.
2. **Launcher Options controls and child dialogs** — consume the shared profile; keep resolution,
   audio, and keyboard editing as separately reviewable mechanisms where their evidence differs.
3. **Typed crate rules authority, persistent lifecycle, pickup, then modifiers** — four sequential
   mechanisms; do not cherry-pick another task's unmerged implementation without fresh review.
4. **Rescue**, then **Patrol**, then **convoy linkage** — independent mission/movement mechanisms;
   Patrol must include command payload and threat behavior, not only enum dispatch.
5. **Movie resolver**, **audio/wait-paced playback**, then **Movies/Credits shell**.
6. **Campaign catalog**, retail action/event census, mission launch/briefing, progression, carryover,
   and persistence. Each campaign-script family should be closed as its own dependency-coherent
   mechanism rather than as one oversized branch.
7. **Online compatibility shell policy** after the stock offline Phase 14 surface is closed.

## Not gaps

- Current Single Player/Skirmish shell transitions.
- Main-menu button return codes, E2 hover behavior, and numeric version fallback.
- Existing Skirmish `[MultiPlayer]`/`[Skirmish]` snapshot persistence.
- Random-map RNG, continuation, generation progress, preview, and accepted-map regeneration in
  the reviewed offline scope.
- Harmless serialization and base cadence.
- VQA decoding for stock mounted assets; preserve a typed unsupported boundary instead of
  pretending a decoder exists.

## Implementation boundary

The disparity-scan read-only gate ends with this report. The first implementation candidate is
the process-owned Options profile transaction. Before code, it requires a dedicated active-binary
investigation/implementation contract and an architecture-aware design review. Every subsequent
mechanism must be rederived from the then-current `origin/main`, validated with focused `--lib`
tests, and passed to a fresh read-only critic before publication.

## Ghidra annotation candidates

- Correct the stale research identity for `0x00740B10`: it is `UnitClass__Mission_Patrol`, and it
  calls `FootClass__Mission_Patrol @ 0x004D4280`; it is not a Hunt alias.
- Record the live Patrol assigner at `FootClass__ClickedAction_Cell @ 0x004D7D50`, action `0x33`,
  mission literal `0x19`.

No metadata was changed during this scan. Synchronization requires separate authorization.
