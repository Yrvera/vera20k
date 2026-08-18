# GGI fidelity audit — iteration scratchpad

**Purpose:** Track progress across `/loop` iterations verifying the post-26-task
GGI integration against gamemd.exe. Each iteration covers ONE surface from the
10-surface list and appends an entry below.

**Stop conditions:** all 10 surfaces covered, OR three consecutive
NO-DISPARITY iterations, OR user interrupt.

**Artifact convention:** detailed per-surface traces go to
`docs/fidelity-checks/<system>.md`. This file is the index + verdict log.

---

## Iteration 1 — Surface #1: DeploySound/UndeploySound emit timing

- **Date:** 2026-05-17
- **Verdict:** PASS (HIGH overall confidence — content HIGH, identity MED, binding MED)
- **Artifact:** [ggi_deploy_sound_timing.md](../fidelity-checks/ggi_deploy_sound_timing.md)
- **Evidence:** live Ghidra decompile of `InfantryClass__Do_Action @ 0x0051d6f0` shows
  `VocClass__PlayAt` precedes `param_1[0x1b1] = iVar6` (Doing-field write at offset 0x6C4).
  Rust at [src/sim/world/world_commands.rs:553-576](../../src/sim/world/world_commands.rs#L553-L576)
  pushes the `SimSoundEvent::Entity{De,Un}deployed` event then writes
  `entity.deploy_state = new_phase`. Ordering identical for all 4 traced scenarios
  (normal deploy, normal undeploy, DeploySound unset, retoggle mid-anim).
- **Candidate flagged, not a finding:** L544 `entity.movement_target = None`
  clear is gamemd-absent but operates only on `Deployed` state, where movement_target
  is guaranteed-None, so observable output is unaffected.
- **Next action:** none. Move to surface #2.

---

<!-- subsequent iterations append below -->
