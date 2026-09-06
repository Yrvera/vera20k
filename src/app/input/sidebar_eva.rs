//! App-local EVA lines spoken by the sidebar itself — `SelectClass::Action @
//! 0x006AAD00` (cameo clicks) and `SidebarClass::AddCameo @ 0x006A63E0` /
//! `StripClass::AddEntry @ 0x006A87F0` (a new cameo). These are spoken only
//! on the clicking machine (`PlayEVA` runs inside the click handler, before
//! the `EventClass` is queued), so they are app events, not sim events.
//!
//! Every call site passes type `-1` (`OR EDX,0xffffffff` at `0x006AAE31`,
//! `0x006AAF9F`, `0x006AAFFF`, `0x006AB100`, `0x006AB3A9`, `0x006AB490`,
//! `0x006AB68B`, `0x006AB6C1`, `0x006A640D`, `0x006A882F`): the entry's own
//! `Type=` / `Priority=` route them (stock: all STANDARD LOW except
//! `EVA_NewConstructionOptions`, QUEUE LOW). Strings: `0x83FB38` Building,
//! `0x83FB48` Training, `0x83FB58` UnableToComply, `0x83FB6C` OnHold,
//! `0x83FB78` SelectTarget, `0x83FB8C` Canceled, `0x83FA64`
//! NewConstructionOptions.
//!
//! `EVA_SelectTarget` (`0x006AAFA7`) is the superweapon cameo click — VERA's
//! `SidebarAction::ArmSuperWeapon` (`select_target_line`).

use std::collections::BTreeSet;

use crate::sim::intern::InternedId;
use crate::sim::production::{BuildQueueState, ProductionCategory, QueueItemView};

pub(crate) const EVA_BUILDING: &str = "EVA_Building";
pub(crate) const EVA_TRAINING: &str = "EVA_Training";
pub(crate) const EVA_ON_HOLD: &str = "EVA_OnHold";
pub(crate) const EVA_CANCELED: &str = "EVA_Canceled";
pub(crate) const EVA_UNABLE_TO_COMPLY: &str = "EVA_UnableToComply";
pub(crate) const EVA_SELECT_TARGET: &str = "EVA_SelectTarget";
pub(crate) const EVA_NEW_CONSTRUCTION_OPTIONS: &str = "EVA_NewConstructionOptions";
pub(crate) const EVA_CANNOT_DEPLOY_HERE: &str = "EVA_CannotDeployHere";

/// What a fresh left click on a build cameo does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BuildClickOutcome {
    /// The line spoken, if any.
    pub eva: Option<&'static str>,
    /// Whether the build command is issued at all.
    pub queue: bool,
}

/// `SelectClass::Action 0x006AB5F0..0x006AB6CE`, the fresh-click branch.
///
/// `iVar6 = HouseClass::GetFactory(rtti, naval)`; the factory is *busy* when
/// it exists and is not (`object == 0 || IsDone`) with an empty queue
/// (`0x006AB61A..0x006AB67B`). Busy + `RTTI == 7` (BuildingType, both the
/// structure and defense strips) → `EVA_UnableToComply` and **no event**
/// (`0x006AB67F CMP EBP,0x7 ; ... 0x006AB693 CALL PlayEVA ; JMP end`).
/// Busy + a unit strip → queued silently (`local_8d = 1`, no line). Idle →
/// `EVA_Training` for `RTTI == 0x10` (InfantryType, `0x006AB6A1`) else
/// `EVA_Building`, both only when `HouseClass::CheckBuildLimit` passed
/// (`0x006AB6BB TEST AL,AL ; JNZ skip`) — the caller feeds that gate as
/// `buildable` (the cameo is only clickable when the option is enabled).
pub(crate) fn build_click_outcome(
    category: ProductionCategory,
    factory_busy: bool,
    buildable: bool,
) -> BuildClickOutcome {
    if !buildable {
        return BuildClickOutcome {
            eva: None,
            queue: false,
        };
    }
    let structure_strip = matches!(
        category,
        ProductionCategory::Building | ProductionCategory::Defense
    );
    if factory_busy {
        return if structure_strip {
            BuildClickOutcome {
                eva: Some(EVA_UNABLE_TO_COMPLY),
                queue: false,
            }
        } else {
            BuildClickOutcome {
                eva: None,
                queue: true,
            }
        };
    }
    BuildClickOutcome {
        eva: Some(start_line_for(category)),
        queue: true,
    }
}

/// `0x006AB484..0x006AB498` / `0x006AB6A1..0x006AB6C9`: infantry trains,
/// everything else builds.
pub(crate) fn start_line_for(category: ProductionCategory) -> &'static str {
    if category == ProductionCategory::Infantry {
        EVA_TRAINING
    } else {
        EVA_BUILDING
    }
}

/// The hold/resume toggle. Native has no toggle gadget: a right click on a
/// building cameo suspends it (`0x006AB007 EVA_OnHold`, event `0xF`), and a
/// left click on a held cameo resumes it (`0x006AB498 EVA_Building` /
/// `EVA_Training`, event `0xE`). VERA's pause button is that pair.
pub(crate) fn pause_toggle_line(
    category: ProductionCategory,
    currently_paused: bool,
) -> &'static str {
    if currently_paused {
        start_line_for(category)
    } else {
        EVA_ON_HOLD
    }
}

/// What a left click on a cameo does when the category's active build is
/// stalled (`SelectClass::Action 0x006AB5B3`: `local_94` — the factory
/// producing THIS cameo's type — exists and `nRate == 0 || IsSuspended`, and
/// `!IsComplete`): the click resumes it and speaks `EVA_Building` /
/// `EVA_Training` (`0x006AB498`) with a `0xE` (produce) event — no new item.
/// Only the same type takes that branch; another cameo in the category
/// goes through the fresh-click branch with a busy factory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HeldClick {
    /// Player-paused active item of this type: resume it and speak the start
    /// line.
    Resume,
    /// The active item of this type is stalled for money (`nRate == 0`):
    /// speak the start line; VERA's `NoFunds` resumes by itself, so no
    /// command is issued.
    NoFundsSameType,
    /// The fresh-click branch (`0x006AB5F0..`).
    Fresh,
}

pub(crate) fn held_click(
    queue: &[QueueItemView],
    category: ProductionCategory,
    type_id: Option<InternedId>,
) -> HeldClick {
    let Some(type_id) = type_id else {
        return HeldClick::Fresh;
    };
    let active = queue
        .iter()
        .find(|item| item.queue_category == category && item.state != BuildQueueState::Done);
    match active {
        Some(item) if item.type_id == type_id && item.state == BuildQueueState::Paused => {
            HeldClick::Resume
        }
        Some(item) if item.type_id == type_id && item.state == BuildQueueState::NoFunds => {
            HeldClick::NoFundsSameType
        }
        _ => HeldClick::Fresh,
    }
}

/// The right-click / cancel-button line. Native right-click
/// (`0x006AADF0..0x006AB1FF`): the cameo's own factory running → `EVA_OnHold`
/// + suspend; already suspended or `nRate == 0` (a completed factory has
/// `Set_Rate(0)`) → `EVA_Canceled` (`0x006AAE39`) + abandon; a copy that only
/// sits in the queue behind the active build (`FactoryClass::IsInQueue`) is
/// removed silently. VERA's cancel skips the hold step and abandons at once,
/// so cancelling the ACTIVE item of its category speaks `EVA_Canceled` and a
/// tail copy stays silent (VERA-internal mapping of the two-step native
/// flow, gamemd equivalent UNCHECKED for the hold step).
///
/// `type_id == None` is the sidebar cancel button (`cancel_last`: the tail
/// item if any, else the active build).
pub(crate) fn cancel_line(
    queue: &[QueueItemView],
    type_id: Option<InternedId>,
) -> Option<&'static str> {
    match type_id {
        Some(type_id) => {
            let category = queue
                .iter()
                .find(|item| item.type_id == type_id)?
                .queue_category;
            let mut in_category = queue.iter().filter(|item| item.queue_category == category);
            let active = in_category.next()?;
            if in_category.any(|item| item.type_id == type_id) {
                return None;
            }
            (active.type_id == type_id).then_some(EVA_CANCELED)
        }
        None => {
            let last = queue.last()?;
            let only_one = queue
                .iter()
                .filter(|item| item.queue_category == last.queue_category)
                .count()
                == 1;
            only_one.then_some(EVA_CANCELED)
        }
    }
}

/// `HouseClass::GetFactory` "busy" per category: any queue entry that is
/// not finished. `NoFunds`/`Paused` are the native `nRate == 0 ||
/// IsSuspended` factory, still a factory with an object.
pub(crate) fn factory_busy(queue: &[QueueItemView], category: ProductionCategory) -> bool {
    queue
        .iter()
        .any(|item| item.queue_category == category && item.state != BuildQueueState::Done)
}

/// Whether the category's active build is player-suspended (`FactoryClass::
/// IsSuspended`, the state the resume branch tests at `0x006AB5B3`).
pub(crate) fn factory_paused(queue: &[QueueItemView], category: ProductionCategory) -> bool {
    queue
        .iter()
        .any(|item| item.queue_category == category && item.state == BuildQueueState::Paused)
}

/// `SelectClass::Action 0x006AAED5..0x006AAFAC`, the superweapon strip
/// (`RTTI == 0x1F`) on a left click (`param_2 & 1`) of a cameo below the
/// house's super count (`[PlayerPtr+0x264]`). `0x006AAEEA CALL 0x006CC360`
/// is the readiness test: `SuperClass+0x70` (IsSuspended) set → false;
/// `Type+0xE5` (`UseChargeDrain=`) → `+0x7C != 0`; else `+0x6F` (IsReady).
/// Not ready → `0x006AAFB1` (a no-op) and nothing is spoken. Then
/// `0x006AAF08 CMP [Type+0xBC], 0` — `Action=` (`SuperWeaponTypeClass::
/// ReadINI 0x006CEA20`, `CCINIClass::ReadAction`, `None` = 0): a targeted
/// weapon arms the pending-super state (`[0x8809A0] = index`),
/// `Unselect_All`, and speaks `EVA_SelectTarget` (`0x006AAFA7`); an
/// untargeted one fires at once through event `0x12` (`0x006AAF3C`) and
/// stays silent. Every stock `[*Special]` carries an `Action=`.
///
/// `is_online` is VERA's `!is_suspended`; `is_ready` stands in for both the
/// charged flag and the charge-drain `+0x7C` state.
pub(crate) fn select_target_line(
    is_ready: bool,
    is_online: bool,
    action: Option<&str>,
) -> Option<&'static str> {
    let targeted = action.is_some_and(|action| !action.eq_ignore_ascii_case("none"));
    (is_ready && is_online && targeted).then_some(EVA_SELECT_TARGET)
}

/// `SidebarClass::AddCameo 0x006A63F0..0x006A6415`: an entry not already in
/// the strip (`piVar4[1] == rtti && *piVar4 == index` scan), when
/// `[0xA8E7AC] == 0` (not inside scenario init) and the RTTI is not
/// `0x1F` (SuperWeaponType, `0x006A6406 CMP ESI,0x1F`), speaks
/// `EVA_NewConstructionOptions` once per insertion; the VoxClass same-type
/// duplicate rule folds a burst into one line.
///
/// `previous` is `None` on the first observation after a scenario starts —
/// that is the init-nesting window, so the seed is silent. Returns the new
/// set and whether at least one non-superweapon cameo was inserted.
pub(crate) fn new_construction_options(
    previous: Option<&BTreeSet<InternedId>>,
    current: BTreeSet<InternedId>,
) -> (BTreeSet<InternedId>, bool) {
    let inserted = previous.is_some_and(|prev| current.iter().any(|id| !prev.contains(id)));
    (current, inserted)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(category: ProductionCategory, state: BuildQueueState) -> QueueItemView {
        QueueItemView {
            type_id: InternedId::default(),
            display_name: String::new(),
            queue_category: category,
            state,
            remaining_ms: 0,
            total_ms: 1,
        }
    }

    #[test]
    fn idle_structure_strip_says_building_and_queues() {
        let out = build_click_outcome(ProductionCategory::Building, false, true);
        assert_eq!(out.eva, Some(EVA_BUILDING));
        assert!(out.queue);
        let out = build_click_outcome(ProductionCategory::Defense, false, true);
        assert_eq!(out.eva, Some(EVA_BUILDING));
    }

    #[test]
    fn idle_infantry_strip_says_training() {
        let out = build_click_outcome(ProductionCategory::Infantry, false, true);
        assert_eq!(out.eva, Some(EVA_TRAINING));
        assert!(out.queue);
        for category in [
            ProductionCategory::Vehicle,
            ProductionCategory::Aircraft,
            ProductionCategory::Ship,
        ] {
            assert_eq!(
                build_click_outcome(category, false, true).eva,
                Some(EVA_BUILDING),
                "{category:?}"
            );
        }
    }

    #[test]
    fn busy_structure_strip_refuses_with_unable_to_comply() {
        for category in [ProductionCategory::Building, ProductionCategory::Defense] {
            let out = build_click_outcome(category, true, true);
            assert_eq!(out.eva, Some(EVA_UNABLE_TO_COMPLY));
            assert!(!out.queue, "{category:?}");
        }
    }

    #[test]
    fn busy_unit_strip_queues_silently() {
        for category in [
            ProductionCategory::Infantry,
            ProductionCategory::Vehicle,
            ProductionCategory::Aircraft,
            ProductionCategory::Ship,
        ] {
            let out = build_click_outcome(category, true, true);
            assert_eq!(out.eva, None, "{category:?}");
            assert!(out.queue);
        }
    }

    #[test]
    fn build_limit_reached_is_silent_and_does_nothing() {
        let out = build_click_outcome(ProductionCategory::Infantry, false, false);
        assert_eq!(
            out,
            BuildClickOutcome {
                eva: None,
                queue: false
            }
        );
    }

    #[test]
    fn pause_toggle_holds_then_resumes_with_the_start_line() {
        assert_eq!(
            pause_toggle_line(ProductionCategory::Vehicle, false),
            EVA_ON_HOLD
        );
        assert_eq!(
            pause_toggle_line(ProductionCategory::Vehicle, true),
            EVA_BUILDING
        );
        assert_eq!(
            pause_toggle_line(ProductionCategory::Infantry, true),
            EVA_TRAINING
        );
    }

    #[test]
    fn factory_busy_and_paused_read_only_their_own_category() {
        let queue = vec![
            item(ProductionCategory::Vehicle, BuildQueueState::Paused),
            item(ProductionCategory::Building, BuildQueueState::Done),
        ];
        assert!(factory_busy(&queue, ProductionCategory::Vehicle));
        assert!(factory_paused(&queue, ProductionCategory::Vehicle));
        assert!(!factory_busy(&queue, ProductionCategory::Building));
        assert!(!factory_busy(&queue, ProductionCategory::Infantry));
        assert!(!factory_paused(&queue, ProductionCategory::Building));
        let building = vec![item(ProductionCategory::Building, BuildQueueState::NoFunds)];
        assert!(factory_busy(&building, ProductionCategory::Building));
        assert!(!factory_paused(&building, ProductionCategory::Building));
    }

    #[test]
    fn held_click_resumes_only_the_paused_active_type() {
        let a = InternedId::from_index(1);
        let b = InternedId::from_index(2);
        let mut paused = item(ProductionCategory::Vehicle, BuildQueueState::Paused);
        paused.type_id = a;
        let mut tail = item(ProductionCategory::Vehicle, BuildQueueState::Queued);
        tail.type_id = b;
        let queue = vec![paused, tail];
        assert_eq!(
            held_click(&queue, ProductionCategory::Vehicle, Some(a)),
            HeldClick::Resume
        );
        assert_eq!(
            held_click(&queue, ProductionCategory::Vehicle, Some(b)),
            HeldClick::Fresh
        );
        assert_eq!(
            held_click(&queue, ProductionCategory::Infantry, Some(a)),
            HeldClick::Fresh
        );
        let mut broke = item(ProductionCategory::Vehicle, BuildQueueState::NoFunds);
        broke.type_id = a;
        assert_eq!(
            held_click(&[broke], ProductionCategory::Vehicle, Some(a)),
            HeldClick::NoFundsSameType
        );
        assert_eq!(
            held_click(&queue, ProductionCategory::Vehicle, None),
            HeldClick::Fresh
        );
    }

    #[test]
    fn cancel_line_speaks_for_the_active_item_only() {
        let a = InternedId::from_index(1);
        let b = InternedId::from_index(2);
        let mut active = item(ProductionCategory::Infantry, BuildQueueState::Building);
        active.type_id = a;
        let mut tail = item(ProductionCategory::Infantry, BuildQueueState::Queued);
        tail.type_id = b;
        let mut tail_same = item(ProductionCategory::Infantry, BuildQueueState::Queued);
        tail_same.type_id = a;
        // Active item with no tail copy: Canceled.
        assert_eq!(cancel_line(&[active.clone()], Some(a)), Some(EVA_CANCELED));
        // A tail copy of the same type is what gets removed: silent.
        assert_eq!(cancel_line(&[active.clone(), tail_same], Some(a)), None);
        // A different tail item: silent.
        assert_eq!(cancel_line(&[active.clone(), tail.clone()], Some(b)), None);
        // Unknown type: silent.
        assert_eq!(cancel_line(&[active.clone()], Some(b)), None);
        // Cancel button: the tail goes first (silent), then the active build.
        assert_eq!(cancel_line(&[active.clone(), tail], None), None);
        assert_eq!(cancel_line(&[active], None), Some(EVA_CANCELED));
        assert_eq!(cancel_line(&[], None), None);
        // A completed building awaiting placement is the active item.
        let mut ready = item(ProductionCategory::Building, BuildQueueState::Done);
        ready.type_id = a;
        assert_eq!(cancel_line(&[ready], Some(a)), Some(EVA_CANCELED));
    }

    #[test]
    fn super_weapon_click_speaks_select_target_only_when_ready_and_targeted() {
        assert_eq!(
            select_target_line(true, true, Some("LightningStorm")),
            Some(EVA_SELECT_TARGET)
        );
        assert_eq!(select_target_line(false, true, Some("Nuke")), None);
        assert_eq!(
            select_target_line(true, false, Some("IronCurtain")),
            None,
            "`+0x70` IsSuspended refuses the click"
        );
        assert_eq!(select_target_line(true, true, None), None);
        assert_eq!(
            select_target_line(true, true, Some("None")),
            None,
            "`Action=None` fires through event 0x12 and stays silent"
        );
    }

    #[test]
    fn new_cameo_speaks_only_after_the_silent_seed() {
        let a = InternedId::from_index(1);
        let b = InternedId::from_index(2);
        let (seed, spoke) = new_construction_options(None, BTreeSet::from([a]));
        assert!(!spoke, "the scenario-init window is silent");
        let (same, spoke) = new_construction_options(Some(&seed), BTreeSet::from([a]));
        assert!(!spoke, "an unchanged strip inserts nothing");
        let (grown, spoke) = new_construction_options(Some(&same), BTreeSet::from([a, b]));
        assert!(spoke);
        // A cameo that leaves and comes back is a fresh insertion.
        let (shrunk, spoke) = new_construction_options(Some(&grown), BTreeSet::from([a]));
        assert!(!spoke);
        let (_, spoke) = new_construction_options(Some(&shrunk), BTreeSet::from([a, b]));
        assert!(spoke);
    }
}
