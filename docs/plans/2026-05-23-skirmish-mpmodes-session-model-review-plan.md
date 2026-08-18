# Skirmish MPModes Session Model Review Plan

Review target:
`docs/plans/2026-05-23-skirmish-mpmodes-session-model-plan.md`

Purpose: catch parity drift while implementing selected MPMode launch/session
handoff, Team row gating/defaults, and MIX-backed common `MP*.ini` override
parsing. This is a code-review and verification plan for after implementation,
not another research pass.

---

## Review Stance

Treat this as a parity-risk review. Findings should focus on player-visible
setup behavior, wrong selected mode identity, incorrect Team rows/defaults,
data-loading shortcuts, accidental stock Siege exposure, and unverified
post-launch mode behavior.

Do not require literal native globals or vtable plumbing. Do require the
observable outcomes verified by the reports: selected mode identity survives
Start, Team Game hides `None`, FFA/Coop still expose `A-D`, inactive AI defaults
follow `AlliesAllowed`, and explicit teams mutual-ally without treating sentinels
as teams.

---

## Primary Review Sources

- `docs/plans/2026-05-23-skirmish-mpmodes-session-model-design.md`
- `docs/plans/2026-05-23-skirmish-mpmodes-session-model-plan.md`
- `docs/research/skirmish-ui/SKIRMISH_SELECTED_MPMODE_START_LAUNCH_SESSION_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_MPMODES_MIX_OVERRIDE_PAYLOADS_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_MPMODES_MUSTALLY_ALLIESALLOWED_ROW_BEHAVIOR_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_SELECTED_MODE_START_VALIDATION_CONTRACT_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_TEAM_ADJUNCT_HOUSE_ALLIANCE_HANDOFF_GHIDRA_REPORT.md`
- `ini/mpmodesmd.ini`

---

## Review Checkpoints

### 1. Architecture Boundaries

Check:

- `sim/` does not import `ui/`, `render/`, `sidebar/`, `audio/`, `net/`, or
  app-level shell modules.
- `skirmish_modes` owns MPMode roster/override parsing; UI/app code does not
  parse `MP*.ini` payloads ad hoc.
- `skirmish_launch` stays plain app-level data. It must not depend on render,
  audio, WOL/network, or shell-control types.
- Existing `app_skirmish` behavior remains a consumer of normalized launch slots,
  not a place where shell UI state leaks in.

Reject if:

- A new mode behavior framework is added that implements unresearched
  non-Battle `+0x80/+0x84` semantics.
- `SkirmishLaunchMode` remains Battle-only or selected mode id is stored only in
  UI state and not in `SkirmishLaunchSession`.
- The implementation routes mode parsing through stringly app code rather than
  `skirmish_modes`.

### 2. MPMode Override Loading

Check:

- Asset-backed mode loading uses `AssetManager` and `IniFile::from_bytes`, matching
  existing rules/art loader style.
- Native common defaults are applied before override reads:
  `AlliesAllowed=true`, `MustAlly=false`, tournament flags true if stored.
- Only the verified common keys affect `SkirmishGameMode`:
  `WonlineTournamentAllowed`, `WonlineClanTournamentAllowed`, `AlliesAllowed`,
  `MustAlly`.
- `MustAlly` is cleared when `AlliesAllowed=false`.
- Missing or malformed override files retain native defaults rather than crashing.
- `stock_skirmish_modes()` remains deterministic and asset-less.
- Stock ids are still exactly `1..9`, and no stock Siege row is synthesized.

Reject if:

- `AllyChangeAllowed`, money, unit count, tech level, game speed, checkboxes,
  `FogOfWar`, or `MCVRedeploys` are folded into common `SkirmishGameMode`.
- The asset-backed path still relies on the stock filename switch as the source of
  truth when override bytes are available.
- Missing override files produce empty or partially invalid mode rows instead of
  native defaults.

### 3. Launch Session Identity

Check:

- `launch_session` receives the mode roster and resolves
  `state.selected_mode_id`.
- `SkirmishLaunchSession.mode` carries at least the numeric selected mode id.
- Team Game id `9` and FFA id `2` survive through `pending_skirmish_launch_session`.
- The selected map behavior is unchanged except for selected mode preservation.
- Ordinary validation order stays intact: selected map, map capacity, no opponent,
  same explicit team, then launch packing.

Reject if:

- Non-Battle stock modes still become Battle in the session.
- A missing selected mode silently falls back to Battle in normal stock tests.
- Selected-mode false return is implemented as automatic rejection without the
  `output_dword == 0x617` gate.
- `0x469` or `TXT_OK` is used as a modal body for selected-mode rejection.

### 4. Team Combo Rows

Check:

- Battle exposes Team rows `None,A,B,C,D` with item data `-2,0,1,2,3`.
- Team Game exposes only `A,B,C,D` with item data `0,1,2,3`.
- FFA/Coop still expose `None,A,B,C,D`; `AlliesAllowed=false` must not remove
  explicit team choices.
- No offline Team `Auto/-1` row is added.
- Dropdown rendering, hit testing, hover item identity, and selection code all use
  the same mode-aware item list.

Reject if:

- Team `None` remains visible in Team Game.
- FFA/Coop remove or disable Team `A-D`.
- Different call sites compute different Team item lists, causing click/render
  mismatch.

### 5. Team Repair And Defaults

Check:

- After mode changes, stale `-2` values are repaired when selected mode
  `must_ally=true`.
- Local Team Game default/repair lands on Team A `0`; AI `-2` values repair to
  Team D `3` where applicable.
- Inactive AI row default follows selected-mode `AlliesAllowed`: true -> Team D
  `3`, false -> `-2`.
- Explicit `0..3` selections survive mode repair.
- Repair runs before launch packing and after Choose Map mode commits.

Reject if:

- Team Game can still launch with local or active AI Team `-2`.
- Inactive Battle/Team Game AI rows remain `-2` after refresh when native would
  default them to Team D.
- Inactive FFA/Coop AI rows default to Team D.
- Repair overwrites valid explicit teams unnecessarily.

### 6. Start/Team/Alliance Handoff

Check:

- `LaunchTeam::Team(_)` still drives bidirectional alliances for matching
  explicit teams.
- `LaunchTeam::None` does not ally just because multiple rows share sentinel
  defaults.
- Start position and Team remain separate fields and separate consumers.
- Tests cover explicit same-team mutual alliance, sentinel non-alliance, and
  start/team independence.

Reject if:

- Start position is collapsed with Team, or Team is used as a start-position
  input.
- Matching negative Team values become allies.
- Same-team alliance is one-way or delayed behind normal gameplay startup.

### 7. Negative Facts / Do Not Do

Check:

- No stock Siege row is exposed.
- No WOL/network lobby behavior is added.
- No Create Random Map implementation is mixed into this patch.
- No full non-Battle mode callback behavior is invented.
- No broad refactor of Skirmish shell rendering or unrelated launch options is
  included.

Reject if:

- The patch uses this session model work to implement unrelated RMG, preview,
  modal, or rules/dialog settings.
- It changes unrelated game options or default rules behavior outside the verified
  common MPMode fields.

---

## Required Test Gates

Run focused tests first:

```powershell
cargo test skirmish_modes --lib
cargo test launch_session --lib
cargo test combo --lib
cargo test team --lib
cargo test launch_alliance --lib
cargo fmt
```

Expected new or updated coverage:

- asset-backed/common override parser defaults and clamp;
- stock ids `1..9`, no stock Siege;
- Team Game id `9` and FFA id `2` preserved in launch session;
- Team Game omits `None`, Battle/FFA expose `None,A-D`;
- inactive AI Team defaults follow `AlliesAllowed`;
- explicit team alliances are mutual;
- Team sentinels do not auto-ally;
- start position and team are independent.

Known possible blocker:

- Broad `cargo test --lib` may still fail from unrelated dirty unit-rendering
  work in `src/app_instances/units.rs` and `src/app_render/build_instances.rs`.
  Treat those as external unless this implementation touched those files.

---

## Manual Review Scenario

At 800x600 in the native Skirmish shell:

1. Open Choose Map.
2. Select Battle. Team combo shows `None,A,B,C,D`.
3. Select Team Game. Team combo shows `A,B,C,D`; `None` is absent.
4. With Team Game selected, verify stale `None` selection is repaired before
   launch.
5. Select FFA. Team combo shows `None,A,B,C,D`.
6. With FFA selected, inactive AI Team defaults to `None`.
7. Return to Battle or Team Game. Inactive AI Team defaults to Team D.
8. Start Team Game and verify the pending/session mode id is `9`.
9. Start FFA and verify the pending/session mode id is `2`.
10. Do not claim full non-Battle gameplay parity from this scenario; it validates
    shell/session handoff and visible Team row behavior only.

---

## Review Output Format

Use code-review style:

1. Findings first, ordered by severity.
2. Each finding must include file and line reference.
3. Focus on player-visible risk and parity evidence.
4. Then list open questions.
5. Then summarize tests run and residual risk.

If no issues are found, say that directly and still mention remaining unimplemented
areas: non-Battle post-launch callbacks, custom selected-mode `false + 0x617`
modal behavior, Create Random Map, and exact Unholy disabled-byte setup.
