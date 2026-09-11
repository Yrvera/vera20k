//! The local health commit ends its entity borrow before world callbacks run.
//!
//! This stage retains the existing Object/Techno/Building receiver ordering.
//! Its result carries decisions across the later synchronous lifecycle stages;
//! it does not execute or defer those stages itself.

use super::*;

pub(super) struct ReceiverHealthCommit {
    pub(super) became_fatal: bool,
    pub(super) entered_techno_death: bool,
    pub(super) reached_exact_zero: bool,
    pub(super) postmortem_candidate: Option<i32>,
    pub(super) fatal_category: EntityCategory,
    pub(super) positive_postlude: Option<(u16, bool, bool)>,
    pub(super) synchronous_retaliation: bool,
    pub(super) smoke_maintenance: Option<(EntityCategory, damage::DamageState)>,
    pub(super) healing_only: bool,
    pub(super) latch_hostile_hit: bool,
    pub(super) uncloak_after_damage: bool,
    pub(super) building_damage_cue: Option<(u16, u16)>,
    pub(super) voice_feedback_cue: Option<(InternedId, InternedId, u16, u16)>,
    pub(super) threat_feedback: Option<(InternedId, InternedId, i32, i32, i32)>,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn commit_receiver_health(
    event: &EntityDamageEvent,
    entities: &mut EntityStore,
    rules: &RuleSet,
    interner: &StringInterner,
    alliances: &HouseAllianceMap,
    attacker_owner: Option<InternedId>,
    live_source_owner: Option<InternedId>,
    receiver_outcome: Option<Option<ResolvedReceiveDamage>>,
    current_tick: u64,
) -> Option<ReceiverHealthCommit> {
    let target_id = event.target_id;
    let attacker_id = event.attacker_id;
    let postmortem_duration = receiver_outcome.flatten().and_then(|resolved| {
        let target = entities.get(target_id)?;
        postmortem_duration_for_event(event, target, rules, interner, resolved.outcome)
    });
    let mut became_fatal = false;
    let mut entered_techno_death = false;
    let mut reached_exact_zero = false;
    let mut postmortem_candidate = None;
    let mut fatal_category = EntityCategory::Unit;
    let mut positive_postlude: Option<(u16, bool, bool)> = None;
    let mut synchronous_retaliation = false;
    let mut smoke_maintenance: Option<(EntityCategory, damage::DamageState)> = None;
    let mut healing_only = false;
    let mut latch_hostile_hit = false;
    let mut uncloak_after_damage = false;
    // `BuildingClass::ReceiveDamage`'s damage-state dispatch result: the
    // building coordinate to sound the global struck cue at, or `None`.
    let mut building_damage_cue: Option<(u16, u16)> = None;
    // `TechnoClass::ReceiveDamage`'s result-2 damage-voice arm: the owner,
    // type and coordinate to sound the `VoiceFeedback=` line at, or `None`.
    let mut voice_feedback_cue: Option<(InternedId, InternedId, u16, u16)> = None;
    let mut threat_feedback: Option<(InternedId, InternedId, i32, i32, i32)> = None;
    if let Some(target) = entities.get_mut(target_id) {
        if event.distance_leptons.is_none()
            && crate::sim::superweapon::invulnerability::is_invulnerable(
                target.invulnerability.as_ref(),
                current_tick as u32,
            )
        {
            // Damage fully nullified by IronCurtain/ForceShield.
            // Flash-effect spawn deferred (see design doc Open Questions).
            if attacker_id != RAD_NO_ATTACKER {
                target.last_attacker_id = Some(attacker_id);
            }
            return None;
        }

        let receive_outcome = match receiver_outcome {
            Some(Some(resolved)) => Some(resolved.outcome),
            Some(None) => return None,
            None => None,
        };
        if let Some(value) = receive_outcome.and_then(|outcome| outcome.psychedelic_value) {
            // TechnoClass writes the signed kernel result first. The
            // first inactive->active transition then runs its callbacks
            // in order: optional team-member detach (not represented on
            // GameEntity), archived target clear, deferred Hunt queue.
            // Passenger cargo is unrelated and remains intact.
            target.berserk.timer = value;
            if !target.berserk.active {
                target.berserk.active = true;
                represented_assign_target(target, None);
                queue_entity_mission_deferred(target, MissionId::from_known(MissionType::Hunt));
            }
            return None;
        }
        let reached_survivor_postlude =
            receive_outcome.is_some_and(|outcome| outcome.reached_survivor_postlude);
        let receive_state = receive_outcome.map(|outcome| outcome.state);
        // `TechnoClass::ReceiveDamage @ 0x0070281D` calls vtable `+0xFC`
        // (`StartUncloaking(0) @ 0x00703850`). It sits after the
        // ObjectClass HP commit and after the `ToProtect` response, and
        // before the `if (damage < 0) return` heal early-out, which is why
        // a HEAL surfaces a diving submarine just as reliably as a shell
        // does.
        //
        // Two native conditions guard it, and BOTH are read from the
        // disassembly, not the decompiler — the decompiler renders this
        // dispatch as `switch (uVar7)` after assigning `uStack_a4 = 4`,
        // which hides the selector overwrite below:
        //
        // 1. Every defensive gate in `TechnoClass::ReceiveDamage @
        //    0x00701900` returns ABOVE the `uVar7 =
        //    ObjectClass__ReceiveDamage(this)` join — the `TypeImmune`
        //    (`type+0xC8C`) same-type/same-owner arm, `vt+0x160`
        //    (IronCurtain/ForceShield), `vt+0x1D4` (warping in), the
        //    `AffectsAllies=no` (`warhead+0x179`) allied arm, and the
        //    accepted Psychedelic arm (`return 1`). None of those reaches
        //    `+0xFC`, so an Iron-Curtained, type-immune or ally-shielded
        //    cloaked object stays submerged. `reached_survivor_postlude`
        //    is exactly "the receiver delegated to ObjectClass and came
        //    back through the surviving-object tail", i.e. that join.
        // 2. Post-join HEALTH, not the ObjectClass result code:
        //      0070202e MOV  EAX,[ESI+0x6C]   ; this->Health
        //      00702031 TEST EAX,EAX
        //      00702033 JNZ  0x00702040
        //      00702035 MOV  EDI,0x4          ; overwrites the selector
        //      00702049 JMP  [EDI*4 + 0x702D24]
        //    The table at `0x00702D24` is `[0x007027F7, 0x00702713,
        //    0x00702695, 0x007027F7, 0x00702050]`; case 4 is the death
        //    handler, which returns at `0x00702692` (`RET 0x1C`) without
        //    ever reaching `0x0070281D`. So `Health == 0` after the join
        //    takes the death branch WHATEVER ObjectClass returned — which
        //    covers both "this record killed it" and "it was already a
        //    corpse" (`ObjectClass::ReceiveDamage @ 0x005F5390` opens
        //    `if (Health < 1) return 0`, and 0 is case 0, but the health
        //    test overrides it). The hostile-hit latch six bytes below at
        //    `0x00702812` sits in the same basic block and is guarded
        //    identically.
        //
        // `target.health.current` here is still the PRE-record value, so
        // "post-join health nonzero" is `pre > 0 && state != Dead`:
        // `classify` returns `Dead` exactly when `prev - delta <= 0`.
        uncloak_after_damage = reached_survivor_postlude
            && target.health.current > 0
            && receive_state.is_some_and(|state| state != damage::DamageState::Dead);
        // TechnoClass's persistent hostile-hit byte is written in the
        // shared surviving post-Object tail. The source object must be
        // non-null, and alliance direction is target owner -> captured
        // source house. This is deliberately separate from retaliation's
        // transient `last_attacker_id`.
        let hostile_source = attacker_id != RAD_NO_ATTACKER
            && attacker_owner.is_some_and(|source_owner| {
                !crate::map::houses::is_allied_with(
                    alliances,
                    interner.resolve(target.owner()),
                    interner.resolve(source_owner),
                )
            });
        let resolved_damage = receive_outcome.map_or(event.damage, |outcome| outcome.hp_delta);
        if reached_survivor_postlude
            && let Some(source_owner) = live_source_owner
            && let Some(final_damage) =
                receive_outcome.and_then(|outcome| outcome.post_object_damage)
            && let Some(target_type) = rules.object(interner.resolve(target.type_ref()))
        {
            threat_feedback = Some((
                target.owner(),
                source_owner,
                final_damage,
                target_type.strength,
                receiver_type_value(target, target_type, rules),
            ));
        }
        if resolved_damage == 0 {
            if reached_survivor_postlude && target.health.current > 0 && hostile_source {
                latch_hostile_hit = true;
            }
            if reached_survivor_postlude && target.health.current > 0 {
                smoke_maintenance = receive_state.map(|state| (target.category, state));
            }
            synchronous_retaliation = event.distance_leptons.is_some()
                && event.damage >= 0
                && reached_survivor_postlude
                && target.health.current > 0;
        } else if resolved_damage < 0 {
            let healing = resolved_damage.unsigned_abs().min(u32::from(u16::MAX)) as u16;
            target.health.current = target
                .health
                .current
                .saturating_add(healing)
                .min(target.health.max);
            if reached_survivor_postlude && target.health.current > 0 && hostile_source {
                latch_hostile_hit = true;
            }
            target.refresh_building_damage_state_gate(rules.general.condition_yellow_x1000);
            if reached_survivor_postlude && target.health.current > 0 {
                smoke_maintenance = receive_state.map(|state| (target.category, state));
            }
            healing_only = true;
        } else {
            let damage = resolved_damage.min(i32::from(u16::MAX)) as u16;
            let was_alive = target.health.current > 0;
            target.health.current = target.health.current.saturating_sub(damage);
            target.refresh_building_damage_state_gate(rules.general.condition_yellow_x1000);
            became_fatal = was_alive && target.health.current == 0;
            reached_exact_zero = became_fatal;
            if became_fatal {
                fatal_category = target.category;
            }
            synchronous_retaliation = event.distance_leptons.is_some()
                && event.damage >= 0
                && reached_survivor_postlude
                && target.health.current > 0;
            if reached_survivor_postlude && target.health.current > 0 {
                if hostile_source {
                    latch_hostile_hit = true;
                }
                smoke_maintenance = receive_state.map(|state| (target.category, state));
            }
            positive_postlude = Some((damage, reached_survivor_postlude, hostile_source));
            if became_fatal && let Some(duration_frames) = postmortem_duration {
                // Do not restore here. Native first executes ObjectClass's
                // exact-zero kill/Destroy callbacks, then victim-house anger,
                // and only afterward arms the timer and writes Alive/HP=1.
                postmortem_candidate = Some(duration_frames);
                positive_postlude = None;
                synchronous_retaliation = false;
                smoke_maintenance = None;
                latch_hostile_hit = false;
            }
        }

        // 70202E..702035 selects Techno's fatal branch on post-Object Health0,
        // including a captured successor killed by a prior nested receiver.
        // This does not repeat Object's fresh exact-zero kill/score callbacks.
        // Native comparison: tools/spatial_oracle/bridge_zero_health_receiver.
        entered_techno_death =
            became_fatal || (reached_survivor_postlude && target.health.current == 0);
        if entered_techno_death {
            fatal_category = target.category;
        }

        // gamemd-derived: `BuildingClass::ReceiveDamage @ 0x00442230`'s
        // damage-state dispatch, latched here and emitted below so it
        // lands after the shared Techno receiver's own consequences —
        // native only reaches it once `TechnoClass::ReceiveDamage`
        // (`0x00442425`) has returned.
        //
        // `0x0044242C MOV AL,[ESI+0x90]` is `ObjectClass::IsAlive`: a dead
        // building skips the dispatch and the function returns. Otherwise
        // `0x00442476 JMP [EAX*4 + 0x00442C18]` with `EAX = result - 2`
        // enters `{0x004426AC, 0x004426C8, 0x004424A2, 0x0044247D}`.
        // Entry 0 (result 2) multiplies the `float` at `+0xE8` of the
        // object pointed to by `BuildingClass+0x30C` by `1.5f`
        // (`0x007E4460`) when that pointer is non-null, then falls
        // through into entry 1 (result 3). That object's identity is
        // UNCHECKED — it is written once by `BuildingClass::Unlimbo @
        // 0x00440F5B` from `CALL 0x0062DC50` on `[BuildingTypeClass
        // +0x764]`, and read and rewritten by
        // `BuildingClass::UpdateGapGenerator_Tick` (`0x00454E7F`,
        // `0x0045500C`). Both entries reach
        // `0x004426D2 CMP [type+0x538],-1`, so only a type with **no**
        // `DamageSound=` of its own continues to
        // `0x00442700 MOV ECX,[Rules+0x714]` and `0x00442706 CALL
        // VocClass::PlayAtCoord @ 0x00750E20` at the building's own
        // coordinate (`0x004426DB LEA ECX,[ESI+0x9C]`). Not owner-gated,
        // draws no RNG.
        //
        // Results 2/3 are `DamageState::Yellow`/`Red` — the threshold
        // crossings `ObjectClass::ReceiveDamage @ 0x005F5390` computes
        // (2: HP went from `>= Strength >> 1` to below it; 3: from above
        // `Strength * Rules+0x1708` to below it). A hit that crosses
        // nothing returns 1 and is silent, so this is a per-crossing cue,
        // not a per-hit one.
        if matches!(
            receive_state,
            Some(damage::DamageState::Yellow | damage::DamageState::Red)
        ) && target.category == EntityCategory::Structure
            && target.lifecycle.object_alive
            && rules
                .object(interner.resolve(target.type_ref()))
                .is_some_and(|object| object.damage_sound.is_none())
        {
            building_damage_cue = Some((target.position.rx, target.position.ry));
        }

        // gamemd-derived: `TechnoClass::ReceiveDamage @ 0x00701900`, the
        // damage-voice arm. `0x00702049 JMP [EDI*4 + 0x00702D24]` selects
        // on the damage result (`EDI` forced to 4 at `0x00702035` when
        // `[ESI+0x6C]` Health is zero); index 2 — result 2, the
        // `Strength >> 1` crossing, i.e. `DamageState::Yellow` — is
        // `0x00702695`, which reads the type's `VoiceFeedback=` count at
        // `+0x4E8` and returns without drawing when it is empty
        // (`0x007026A9 JLE`).
        //
        // This is TechnoClass, not BuildingClass: every category reaches
        // it, and result 3 (`Red`) does not — index 3 is `0x007027F7`, the
        // shared tail. Native runs it *inside* `TechnoClass::ReceiveDamage`,
        // so it precedes the BuildingClass cue latched above; the push
        // below preserves that order.
        //
        // Only the "would native draw?" test lives here. The 30% roll
        // (`0x007026B3` `RandomRanged(0, 99)` vs `0x007026BD CMP EAX,0x1E`),
        // the `HouseClass::IsHumanPlayer @ 0x0050B6F0` gate at `0x007026C6`
        // and the `rand % count` pick at `0x007026DE`/`0x007026E7` are all
        // app-side: both draws are on `g_MainRng @ 0x00886B88`, which
        // per-frame draw paths also consume, so they are not lockstep state
        // and must not touch a `sim/` stream. Native spends the roll before
        // the owner gate, so the event is emitted for every house.
        if receive_state == Some(damage::DamageState::Yellow)
            && rules
                .object(interner.resolve(target.type_ref()))
                .is_some_and(|object| {
                    object
                        .voice_feedback
                        .as_deref()
                        .is_some_and(|voice| !voice.is_empty())
                })
        {
            voice_feedback_cue = Some((
                target.owner(),
                target.type_ref(),
                target.position.rx,
                target.position.ry,
            ));
        }
    }

    Some(ReceiverHealthCommit {
        became_fatal,
        entered_techno_death,
        reached_exact_zero,
        postmortem_candidate,
        fatal_category,
        positive_postlude,
        synchronous_retaliation,
        smoke_maintenance,
        healing_only,
        latch_hostile_hit,
        uncloak_after_damage,
        building_damage_cue,
        voice_feedback_cue,
        threat_feedback,
    })
}
