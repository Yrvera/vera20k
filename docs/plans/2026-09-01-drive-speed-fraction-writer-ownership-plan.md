# Drive Speed-Fraction Writer Ownership Implementation Plan

> **For Codex:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Remove the non-native Drive move-command speed-fraction mutation so
path installation preserves the existing fractions and the scheduled movement
visit remains their ordinary producer.

**Architecture:** `Simulation::apply_command` dispatches `Command::Move` into
`movement::issue_move_command_with_layered`, which owns path/destination
installation. `movement_tick` already owns the scheduled Drive target/current
fraction update and speed projection. This transaction deletes the duplicate
command-time write without changing interfaces, state layout, or tick order.

**Design Doc:**
`docs/plans/2026-09-01-drive-speed-fraction-writer-ownership-design.md`

---

## Grounding Summary

- Active-retail `DriveLocomotionClass::Process_Movement @ 0x004B2630` produces
  the target fraction; `Process_Drive_Track @ 0x004B0F20` applies the
  target/current transition and queries current speed.
- `PHASE3_ACTIVE_RETAIL_CRATE_RUNTIME_GHIDRA_REPORT.md:1537-1546` explicitly
  reports no native Move-command path-install fraction writer and calls for the
  Rust update at `movement_commands.rs:717-726` to be deleted.
- `FOOTCLASS_GET_CURRENT_SPEED_EXACT_GHIDRA_REPORT.md:306-329,403-430`
  verifies constructor zero, `SetSpeedFraction` ownership, and the scheduled
  DriveTrack call order.
- The reports are active-YR verified, and the active executable remains
  `gamemd.exe` SHA-256
  `1CDD1180E49024FBDA8AD568CAAC2E86E856063FF67AB38F62B7D2C7BB84298C`.
- No consequential native uncertainty remains, so a ritual live decompile would
  not change this plan. Ghidra metadata mutation is not authorized or needed.
- `src/sim/movement/movement_commands.rs:686-737` currently installs Drive
  destination/path/turn state and then eagerly calls
  `update_drive_speed_fraction` with target one and zero ramp inputs.
- `src/sim/movement/movement_tick.rs:1950-2058` already performs the scheduled
  target computation, fraction transition, and owner-speed projection before
  DriveTrack budget consumption.
- `src/sim/world/world_commands.rs:540-730` is the production command
  dispatcher and stamps parsed ramp parameters only after successful path
  installation.
- `DriveLocomotionRuntime` owns and hashes the target/current fractions and
  cached owner speed; no schema or snapshot version changes.
- The production test fixture parses only active movement keys and deliberately
  omits `SlowdownDistance`, exercising the existing verified default of 500.
  No new INI key, asset, RNG draw, or external data path is introduced.
- ForceTrack and Tube contain separate known fraction drift. They are excluded
  from this diff and remain open; existing tests for them are regression checks,
  not parity proof.

## Key Technical Decisions

- Delete the eager call; do not replace it with another command-time helper. —
  **Confidence:** high
  - **Source:** `PHASE3_ACTIVE_RETAIL_CRATE_RUNTIME_GHIDRA_REPORT.md:1537-1546`
- Preserve `DriveLocomotionRuntime` as the Rust-native state owner and
  `movement_tick` as the scheduled producer. — **Confidence:** high
  - **Source:** `src/sim/components.rs:440-508`,
    `src/sim/movement/movement_tick.rs:1950-2058`
- Prove the production dispatch boundary with `Simulation::apply_command` and
  prove the next scheduled mutation with `Simulation::advance_tick`. —
  **Confidence:** high
  - **Source:** `src/sim/world/world_commands.rs:540-730`,
    `src/sim/world/mod.rs:6424-6608`
- Leave ForceTrack, Stop, Ship, and Tube source untouched; do not add assertions
  that bless their known discrepancies as native. — **Confidence:** high
  - **Source:** `PHASE3_ACTIVE_RETAIL_CRATE_RUNTIME_GHIDRA_REPORT.md:930-955`

No low-confidence implementation decision remains.

## Open Questions

### Resolved During Planning

- **Does a move command legitimately initialize either fraction?** No. The
  active-retail producer/caller scan confines ordinary production/application
  to the scheduled Drive bodies.
- **Should cached `owner_current_speed` be recomputed at command time?** No. It
  is a Rust projection of the applied fraction and remains unchanged until the
  same scheduled visit that updates the applied fraction.
- **Must the known ForceTrack and Tube drift be fixed in this transaction?**
  No. They are separate writer/lifecycle mechanisms with different callers and
  ordering. Preserving their files in the diff prevents scope confusion while
  keeping those residuals explicitly open.

### Deferred to Later Mechanisms

- Exact ForceTrack selector and target-only writes, Tube fraction ordering,
  crush clamp, passive/selector bypass, second brake band, full
  `GetCurrentSpeed`, and crate Speed multiplier/consumer integration remain
  separate dependency-coherent mechanisms.

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Modify | `src/sim/movement/movement_commands.rs` | Remove the disproved command-time fraction writer and document the native ownership boundary. |
| Modify | `src/sim/movement/movement_tests.rs` | Pin default and mid-curve path-install preservation at the direct command boundary. |
| Modify | `src/sim/world/world_tests.rs` | Prove production command dispatch preserves sentinels and the next scheduled visit mutates them. |
| Add | `docs/plans/2026-09-01-drive-speed-fraction-writer-ownership-design.md` | Record the approved evidence-backed design. |
| Add | `docs/plans/2026-09-01-drive-speed-fraction-writer-ownership-plan.md` | Record this reviewed execution plan. |

All modified Rust files remain within deterministic `sim/`; no dependency on
render, UI, sidebar, audio, or net is introduced.

## Interface Changes

None. Function signatures, structs, serialization, state hashing, snapshot
version, and command schemas remain unchanged. Only the postcondition of an
existing Drive path installation is corrected: it preserves the two fraction
slots and cached owner-speed value.

## Sim Checklist

- [x] No new math; existing `SimFixed` values only.
- [x] No new state; hash/schema work is unnecessary.
- [x] No render/UI/sidebar/audio/net dependency.
- [x] Tick order is unchanged; one non-native earlier mutation is removed.
- [x] No iteration or `BTreeMap` ordering change.

## Risk Areas

- A test may have accidentally depended on the eager target-one write. Update
  only assertions about the disproved ownership boundary; do not broadly
  rebaseline movement behavior.
- A fresh Drive runtime must remain zero until the first scheduled visit.
- An in-flight Drive reissue must preserve nonzero fractions in addition to its
  already-tested curve/path/occupation state.
- The production command test must call `Simulation::apply_command`, not only
  the helper, or it cannot prove delivery.
- ForceTrack/Stop/Ship/Tube files must not appear in the implementation diff.

## Player-Experience Critical Items

| Task | Class | Item | Why it matters | Verification |
|---|---|---|---|---|
| 1-2 | MILESTONE-BLOCKING | Command-time preservation | Every ordinary Drive move/reissue currently mutates deterministic speed state too early. | Direct sentinel tests plus `Simulation::apply_command` production test. |
| 2 | MILESTONE-BLOCKING | Scheduled first mutation | Terrain/health and acceleration must be read by the live movement visit, not snapshotted by the command. | Follow production command with `Simulation::advance_tick`; assert scheduled snap/ramp result. |
| 1-2 | COMPOUNDING | Fresh `Accelerates=true/false` | Wrong early target/current values affect every departure and state hash. | Fresh-default and parsed true/false fixture cases. |
| 1 | COMPOUNDING | Mid-curve reissue | Frequent player retasking must not reset live speed state. | Extend the existing in-flight curve test with exact sentinel assertions. |
| Audit | EXACTIFICATION-RESIDUAL | ForceTrack/Stop/Tube writers | Their triggers are narrower or independently ordered; folding them here would mix mechanisms. | Confirm their files are absent from the diff and keep the residual named. |
| Later | EXACTIFICATION-RESIDUAL | Full `GetCurrentSpeed` and crate multiplier | Ordinary and crate movement remain incomplete after this prerequisite. | Remains open in the crate disparity scan; no closure claim in this PR. |

Representative production scenario: an ordinary stock Drive vehicle is moving
or stationary, receives a player/team/AI Move order, and reaches its next live
movement visit. The command installs the new path without touching speed
fractions; the movement visit then computes and applies them.

---

## Tasks

### Task 1: Pin the path-install preservation contract

**Why:** Tests must fail on the current eager writer and distinguish fresh
constructor state from a reissued in-flight state before implementation.

**Files:**

- Modify: `src/sim/movement/movement_tests.rs:1526-1587`
- Modify: `src/sim/movement/movement_tests.rs:1623-1701`

**Pattern:** Extend the existing Drive command and mid-curve tests; introduce no
new fixture abstraction.

**Step 1: Extend the fresh Drive command test**

After the existing `let drive = ...` assertion block in
`test_issue_move_command_starts_drive_track_for_drive_locomotor`, assert:

```rust
assert_eq!(drive.target_speed_fraction, SIM_ZERO);
assert_eq!(drive.current_speed_fraction, SIM_ZERO);
assert_eq!(drive.owner_current_speed, 0);
```

This must fail before Task 2 because the current eager call writes the target to
one.

**Step 2: Extend the mid-curve reissue test**

After the first order and before the second order, write exact sentinels:

```rust
{
    let drive = entities
        .get_mut(1)
        .and_then(|entity| entity.drive_locomotion.as_mut())
        .expect("drive state");
    drive.target_speed_fraction = SimFixed::lit("0.4");
    drive.current_speed_fraction = SimFixed::lit("0.25");
    drive.owner_current_speed = 7;
}
```

After the second order, add:

```rust
assert_eq!(drive.target_speed_fraction, SimFixed::lit("0.4"));
assert_eq!(drive.current_speed_fraction, SimFixed::lit("0.25"));
assert_eq!(drive.owner_current_speed, 7);
```

These assertions must fail before Task 2 and pass after it.

### Task 2: Remove the command-time writer

**Why:** This is the complete implementation delta proved by the active binary.

**Files:**

- Modify: `src/sim/movement/movement_commands.rs:702-727`

**Pattern:** Keep command handling as intent/path installation and leave
scheduled deterministic mutation in `movement_tick`.

**Step 1: Delete the eager update call**

Replace the entire `update_drive_speed_fraction` call at the end of the Drive
path-replay block with the nearest ownership/provenance comment:

```rust
// Drive path installation preserves the target/current speed fractions.
// Active YR produces the target only in DriveLocomotionClass::Process_Movement
// @ 0x004B2630 and applies it in Process_Drive_Track @ 0x004B0F20.
```

Do not change destination, path replay, turn fields, first-track selection,
occupation, Ship handling, or any helper signature.

### Task 3: Prove production dispatch and scheduled ownership

**Why:** A helper-only assertion cannot prove the player/team/AI command path
reaches the corrected code or that scheduled movement remains connected.

**Files:**

- Modify: `src/sim/world/world_tests.rs` near the existing Move command tests

**Pattern:** Use the existing `Simulation`, `RuleSet::from_ini`, `spawn_object`,
`apply_command`, and `advance_tick` test patterns.

**Step 1: Add a minimal parsed rules helper**

```rust
fn drive_fraction_writer_rules(accelerates: bool) -> RuleSet {
    let ini = IniFile::from_str(&format!(
        "[InfantryTypes]\n\
         [VehicleTypes]\n\
         0=DRIVE\n\
         [AircraftTypes]\n\
         [BuildingTypes]\n\
         [DRIVE]\n\
         Strength=300\n\
         Speed=6\n\
         Locomotor={{4A582741-9839-11d1-B709-00A024DDAFD1}}\n\
         MovementZone=Normal\n\
         Accelerates={}\n\
         AccelerationFactor=0.03\n\
         DeaccelerationFactor=0.002\n",
        if accelerates { "yes" } else { "no" }
    ));
    RuleSet::from_ini(&ini).expect("Drive fraction writer rules")
}
```

**Step 2: Add the production-route test**

For both `Accelerates=false` and `Accelerates=true`:

1. create `Simulation::new()` and the rules fixture;
2. `spawn_object("DRIVE", "Americans", 2, 3, 64, ...)`;
3. materialize Rust's lazily-created `DriveLocomotionRuntime`, then set
   target/current/owner sentinels to `0.4`, `0.25`, and `7` (the direct fresh
   command test separately proves the missing-runtime zero-default path);
4. call `Simulation::apply_command` with `Command::Move` to `(7, 3)` and a
   `16x16` `PathGrid`;
5. assert all sentinels remain exact immediately after the call;
6. call `Simulation::advance_tick` once with no commands and the same rules/grid;
7. assert target becomes one; current becomes one for nonaccelerating Drive and
   `0.28` for accelerating Drive; compute the expected cached owner speed from
   the `MovementTarget.speed` integer stage and the expected current fraction,
   then assert exact equality.

The expected owner-speed calculation in the test is:

```rust
let raw_stage = (movement_speed / SimFixed::from_num(15)).to_num::<i32>();
let expected_owner =
    (SimFixed::from_num(raw_stage) * expected_current).to_num::<i32>();
```

If the first scheduled visit is gated by an unexpected turn/track setup, fix the
fixture alignment so it reaches the existing production DriveTrack visit; do
not weaken the assertion to an arbitrary multi-tick wait.

### Task 4: Format and run focused validation

**Why:** Prove the exact mechanism without running unrelated side binaries or
parallel Cargo.

**Files:** Only the three edited Rust files.

**Step 1: Inspect ownership and diff**

Confirm the branch/worktree is task-owned, `origin/main` is the base, and only
the planned files plus the two plan documents are changed.

**Step 2: Format only edited leaf files**

Run direct `rustfmt --edition 2024` only when it produces no unrelated churn.
Never run crate-wide `cargo fmt` and never format a `mod.rs`.

**Step 3: Check Cargo ownership**

Run `Get-Process cargo,rustc -ErrorAction SilentlyContinue`. If another session
owns Cargo, wait; never kill it.

**Step 4: Run focused tests**

```text
cargo test -p vera20k --lib sim::movement::movement_tests::test_issue_move_command_starts_drive_track_for_drive_locomotor -- --exact
cargo test -p vera20k --lib sim::movement::movement_tests::test_reissue_mid_curve_keeps_track_and_anchors_path_at_head -- --exact
cargo test -p vera20k --lib sim::world::tests::phase14_drive_move_command_preserves_fractions_until_scheduled_visit -- --exact
```

Record each literal `test result:` line. If exact module paths differ after
compilation identifies the registered name, correct the filter rather than
running a bare or side-binary test.

**Step 5: Focused module sweep**

```text
cargo test -p vera20k --lib sim::movement::movement_tests::
```

Record the literal result. Do not run the full library suite during iteration.

### Task 5: Commit, obtain fresh read-only review, and publish only green

**Why:** The active goal requires one reviewed dependency-coherent mechanism
per feature branch/PR.

**Step 1: Audit the final diff**

- Compare against the exact `origin/main` base.
- Confirm no ForceTrack, Stop, Ship, Tube, unrelated test, generated, or local
  evidence file is included.
- Confirm the sim-behavior commit names
  `PHASE3_ACTIVE_RETAIL_CRATE_RUNTIME_GHIDRA_REPORT.md` and the two native
  function addresses.

**Step 2: Commit the coherent mechanism**

Use a descriptive commit such as:

```text
Phase 14: preserve Drive speed fractions through path install
```

The commit body cites the active Phase 3 report and
`Process_Movement @ 0x004B2630` / `Process_Drive_Track @ 0x004B0F20`.

**Step 3: Fresh read-only critic**

Give a critic who did not build the change the exact base/head SHAs, design,
plan, native report sections, full diff, and literal focused validation output.
Require findings-first review of semantics, production delivery, scope,
provenance, and test strength.

If the critic finds a confirmed issue, fix it, rerun affected focused tests,
commit, and submit the new head to a different fresh critic. Recheck every prior
finding. Stop only on a green verdict.

**Step 4: PR certification and publication**

After critic green, check Cargo ownership and run the one PR-certification
command required by AGENTS/ENGINE:

```text
cargo test -p vera20k --lib
```

Record the literal result, push the feature branch, open a PR targeting `main`,
wait for checks, merge only green, and re-review any conflict resolution before
merging. Refresh `origin/main` after merge before selecting the next Phase 14
mechanism.

## Sources & References

- **Design:**
  `docs/plans/2026-09-01-drive-speed-fraction-writer-ownership-design.md`
- **Primary active-binary report:**
  `docs/research/PHASE3_ACTIVE_RETAIL_CRATE_RUNTIME_GHIDRA_REPORT.md:817-968,1537-1546`
- **Current-speed report:**
  `docs/research/FOOTCLASS_GET_CURRENT_SPEED_EXACT_GHIDRA_REPORT.md:232-252,306-329,403-430`
- **Native addresses:** Drive `Process_Movement @ 0x004B2630`, Drive
  `Process_Drive_Track @ 0x004B0F20`, owner
  `TechnoClass::SetSpeedFraction @ 0x004D3710`
- **Rust command boundary:** `src/sim/movement/movement_commands.rs:686-737`
- **Rust scheduled owner:** `src/sim/movement/movement_tick.rs:1950-2058`
- **Production command route:** `src/sim/world/world_commands.rs:540-730`
- **State owner:** `src/sim/components.rs:440-508`
- **Test patterns:** `src/sim/movement/movement_tests.rs:1493-1701`,
  `src/sim/world/world_tests.rs:5555-5697`
- **Fixture INI keys:** `[DRIVE] Speed=`, `Locomotor=`, `MovementZone=`,
  `Accelerates=`, `AccelerationFactor=`, `DeaccelerationFactor=`,
  plus the existing `ObjectType` `SlowdownDistance` default of `500`

## Post-Plan Self-Review

- [x] Every design requirement maps to a task.
- [x] No placeholders or vague implementation steps remain.
- [x] Existing module ownership and interfaces are preserved.
- [x] No interface task is needed because there is no interface change.
- [x] Fresh/default/reissue/production/scheduled risks have explicit tests.
- [x] Each task is self-contained and ordered tests → implementation → proof.
- [x] Sim boundary, fixed-point, state-hash, tick-order, and iteration checks pass.
- [x] Grounding cites current Rust and the decisive active-YR reports.
- [x] Every technical decision is high confidence; no live binary gap remains.
- [x] Deferred mechanisms are explicit and not mislabeled as complete.
- [x] The production scenario and every blocking/compounding item are populated.
