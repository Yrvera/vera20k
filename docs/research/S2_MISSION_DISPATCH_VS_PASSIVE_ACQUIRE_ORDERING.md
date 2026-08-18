# S2 ordering note — Mission_Dispatch vs passive target acquisition

**Question (for the S2 authority flip):** within gamemd's per-object AI pass, does target
**acquisition** run **before** or **after** `Mission_Dispatch`? This decides whether the
gamemd-faithful dispatch input is the *pre-acquisition* mission (our host-time read at
`object_ai_stage`, top-of-tick) or a *post-acquisition* mission.

**Status:** VERIFIED from binary this session — `decompile_function 0x006F9E50`
(`TechnoClass::AI_Update`). Corroborated by the verified `GRIZZLY_OPPORTUNITYFIRE_*`
report family (independently reached the same order).

---

## Verified answer: DISPATCH FIRST, ACQUIRE AFTER

Reading `TechnoClass::AI_Update` (`0x006F9E50`) top-to-bottom, the load-bearing sequence is:

1. **Target validation / clear suite** (before dispatch). A run of `vtable+0x3C8(0)`
   (Assign_Target → clear) calls: ally-turned-friendly clear, periodic ally recheck
   (`g_CurrentFrameCounter & 0xF`), FireError 5/6 clear, out-of-range clears
   (`GetWeaponRange < 0`), target-RTTI checks. These can **null** the current Target but do
   **not acquire** a new one.
2. **`field_0xc4 += 1;` then `MissionClass__Mission_Dispatch();`** — dispatch routes on
   `CurrentMission` (`+0xAC` / `param_1->field_0xac`) through the 32-case switch
   (`0x005B3060`). The mission handler reads the (possibly-cleared) Target.
3. **AFTER dispatch — passive/opportunity acquisition:**
   ```c
   if ((**(code **)(vtable + 0x4c4))() == 0) {          // not currently firing/over-busy
     iVar7 = param_1->field_0xac;                        // CurrentMission
     if (((iVar7 == 2) || (iVar7 == 10) || (iVar7 == 5)) // Move / Harvest / Guard ONLY
        && FUN_00709290() != 0) {                         // OpportunityFire/CanPassiveAcquire gate
       param_1->field_0x4fc = g_CurrentFrameCounter;      // acquire timestamp
       uVar9 = (**(code **)(vtable + 0x48))(&iStack_60,1);
       cVar6 = (**(code **)(vtable + 0x39c))(uVar9);       // passive scanner (sets ->Target)
       if (cVar6 != 0 && param_1->Target != puVar2) {
         param_1->field_0x50c = 1;                         // target-changed flag
       }
     }
   }
   ```

### Two facts that matter for S2

- **Acquisition is strictly after dispatch.** So the mission value `Mission_Dispatch`
  routes on is the mission *as of before this tick's passive acquisition*. Our host-time
  read in `object_ai_stage` (top-of-tick, post-command, pre-movement, pre-combat) is the
  faithful Rust placement of that input. **Routing S2 by host-time mission is correct.**
- **Passive acquisition does NOT change `CurrentMission` (`+0xAC`).** The block writes only
  `Target`, `+0x4FC` (timestamp), `+0x50C` (changed flag) — never `+0xAC`. A Move/Guard/
  Harvest unit that spots an enemy **stays** Move/Guard/Harvest; its mission *handler* uses
  the new Target. `Mission_Attack(1)` is entered only by an explicit assign-side Attack
  order, never by passive acquisition.
- Acquisition is **mission-gated to {Move(2), Harvest(10), Guard(5)}** — not Enter(7),
  Unload(16), etc. (see `HARV_ARMED_BEHAVIOR_*` for the harvest-cycle corollary).

Evidence: `decompile_function 0x006F9E50`; the dispatch call is the `field_0xc4 += 1;
MissionClass__Mission_Dispatch();` pair; the acquire block is the `{2,10,5}`-gated
`vtable+0x39C` call immediately following it. Corroboration: `GRIZZLY_OPPORTUNITYFIRE_FIRST_SHOT_TIMING_GHIDRA_REPORT.md`
("Passive scan is after mission dispatch inside `TechnoClass::AI_Update`"),
`GRIZZLY_OPPORTUNITYFIRE_CONSUMER_GHIDRA_REPORT.md` (dispatch `0x006FA655` < passive caller
`0x006FA6B7..0x006FA6EE`).

---

## Implication for the S2 authority flip + the churn metric

The S2 host (`object_ai_stage`, top-of-tick) records each Unit's `derived_mission()` and the
end-of-tick churn proof compares it to a fresh tail re-derivation. Reconciling with the
binary:

1. **Host-time dispatch is gamemd-faithful** — dispatch-before-acquire is confirmed, so the
   pre-acquisition mission is exactly what gamemd routes on. The flip should dispatch by the
   host-time mission, not the tail projection. (This was the open S2 question; it is now
   answered: **dispatch by host-time**.)

2. **`derived_mission()`'s `attack_target ⇒ Mission::Attack` rule is the divergence to
   watch.** gamemd keeps a passively-acquiring Move/Guard unit in Move/Guard and only sets
   its Target; it does **not** flip the mission to Attack. Our `derived_mission` conflates
   "has a target" with `Attack`. Consequences:
   - For an **explicit Attack order**, both agree: gamemd writes `CurrentMission=Attack(1)`
     at assign time; our host-time read sees `attack_target` set at command time → `Attack`.
     **Match.**
   - For **passive acquisition** (a Guard unit spotting an enemy), gamemd stays `Guard` and
     dispatches the Guard handler; our `derived_mission` would yield `Attack` once our
     combat phase sets `attack_target`. The **host-time** read is still faithful (no
     `attack_target` yet at top-of-tick → `Guard`/`Move`); the **tail** projection (`Attack`)
     is the port artifact. So the measured Move/Guard→Attack churn is *not* a gamemd mission
     change — it is our family-mapping reacting to a post-dispatch acquisition gamemd models
     as "same mission, new Target."

3. **Churn-metric reading.** The dense measurement's churn was arrival-driven
   (Move→Sleep). Any acquisition-driven churn (Move/Guard→Attack) is, per (2), partly a
   `derived_mission` artifact rather than a gamemd dispatch change — so the S2 design should
   either (a) route by the *committed* mission (Move/Guard/Attack-if-ordered) and feed
   acquisition to the handler post-dispatch, mirroring gamemd, or (b) prove the family
   mapping is observably equivalent. Option (a) is the gamemd-native model.

## Open / not-yet-measured

- **Engagement (acquisition) churn magnitude** is unmeasured — the dense fixture converged
  but did not engage (20/20 survivors; pure-Move auto-acquire did not fire). A fixture that
  forces combat (explicit Attack orders + LOS) is needed to quantify it, and per (2) its
  interpretation must separate "explicit Attack" (faithful) from "passive acquire" (artifact).
- This note covers the **TechnoClass common-work** ordering. The per-leaf `UnitClass::AI`
  fire gate (which actually fires the weapon for ground vehicles, after FootClass::AI) runs
  later still — see the Grizzly first-shot-timing report; not load-bearing for the dispatch
  family decision but relevant when S2 absorbs combat.
