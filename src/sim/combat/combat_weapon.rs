//! Weapon selection logic for the combat system.
//!
//! Determines which weapon (Primary or Secondary) a unit should use against
//! a given target. Selection is based on:
//! 1. Projectile targeting flags (AA/AG) — can the projectile reach this target type?
//! 2. Warhead Verses value — is damage > 0% against this armor type?
//!
//! If the Primary weapon fails either check, the Secondary is tried. If both
//! fail, the unit cannot engage the target at all.
//!
//! ## Verses behavioral thresholds
//! - **0%**: Weapon completely blocked — cannot target even with force-fire.
//! - **1%**: No passive acquire, no retaliation. Force-fire still works at 1% damage.
//! - **>1%**: Normal engagement.
//!
//! ## Dependency rules
//! - Part of sim/ — depends on rules/ (RuleSet, ObjectType, WeaponType, etc.)
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use super::armor_index;
use crate::map::entities::EntityCategory;
use crate::rules::object_type::ObjectType;
use crate::rules::ruleset::RuleSet;
use crate::rules::warhead_type::WarheadType;
use crate::rules::weapon_type::WeaponType;

/// Which weapon slot the unit is using for this engagement.
///
/// Used to resolve the correct FLH (firing offset) from art.ini:
/// Primary → `PrimaryFireFLH`, Secondary → `SecondaryFireFLH`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum WeaponSlot {
    Primary,
    Secondary,
}

/// Weapon-selection override used by transport passengers.
///
/// Two transport semantics are distinguished:
///
/// - **`IfvSlot(idx)`** — `Gunner=yes` transports (e.g., IFV). The transport
///   fires its own `weapon_list[idx]` where `idx` is the passenger's IFVMode.
///   The attacker passed to `select_weapon_*` is the TRANSPORT's ObjectType.
///
/// - **`OpenTransport(slot)`** — Open-topped non-Gunner transports (e.g., BFRT).
///   The transport fires the passenger's own Primary (slot=0) or Secondary
///   (slot=1) per the passenger's `OpenTransportWeapon=` INI value. The
///   attacker passed to `select_weapon_*` is the PASSENGER's ObjectType.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum WeaponOverride {
    /// Transport's weapon_list[idx], used when transport is `Gunner=yes`.
    IfvSlot(u32),
    /// Passenger's own primary (0) or secondary (1), used for open-topped
    /// non-Gunner transports with `OpenTransportWeapon != -1`.
    OpenTransport(u32),
}

/// Result of weapon selection: the chosen weapon, its warhead, and the
/// effective Verses percentage against the target's armor.
pub(crate) struct SelectedWeapon<'a> {
    /// Section id of the selected weapon.
    pub weapon_id: &'a str,
    pub weapon: &'a WeaponType,
    pub warhead: &'a WarheadType,
    /// Damage percentage for target armor (0–200). Already looked up from Verses.
    /// 100 = full damage, 0 = immune.
    pub verses_pct: u8,
    /// Which weapon slot (Primary or Secondary) was selected.
    pub slot: WeaponSlot,
}

/// Behavioral gate derived from the Verses damage percentage.
/// Controls whether a weapon can passively acquire or retaliate against a target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VersesGate {
    /// 0% — weapon cannot target this armor type at all, even force-fire.
    Blocked,
    /// 1% — no passive acquire, no retaliation. Force-fire works at 1% damage.
    Suppressed,
    /// >1% — normal engagement allowed.
    Normal,
}

/// Classify a Verses percentage into its behavioral gate.
///
/// RA2 uses these thresholds to control targeting:
/// - 0 blocks the weapon entirely (falls back to Secondary).
/// - 1 (1%) suppresses auto-targeting but allows force-fire.
/// - >1 is normal combat.
pub(crate) fn verses_gate(verses_pct: u8) -> VersesGate {
    match verses_pct {
        0 => VersesGate::Blocked,
        1 => VersesGate::Suppressed,
        _ => VersesGate::Normal,
    }
}

/// Resolve the primary weapon ID for this unit at the given veterancy.
///
/// Elite tier (veterancy >= 200) prefers `elite_primary` with fallback to
/// `primary`. Veteran tier (100..199) does NOT swap.
pub(crate) fn primary_for_tier(obj: &ObjectType, veterancy: u16) -> Option<&str> {
    if veterancy >= 200 {
        obj.elite_primary.as_deref().or(obj.primary.as_deref())
    } else {
        obj.primary.as_deref()
    }
}

/// Same as `primary_for_tier` for the secondary slot.
pub(crate) fn secondary_for_tier(obj: &ObjectType, veterancy: u16) -> Option<&str> {
    if veterancy >= 200 {
        obj.elite_secondary.as_deref().or(obj.secondary.as_deref())
    } else {
        obj.secondary.as_deref()
    }
}

const DEFAULT_DEPLOY_FIRE_WEAPON_INDEX: i32 = 1;

/// Select the best weapon (Primary or Secondary) for an attacker against
/// a specific target. Returns None if no weapon can engage.
///
/// Selection logic:
/// 1. If a `WeaponOverride` is set, dispatch on the variant (see
///    `select_weapon_with_override`).
/// 2. Try Primary (elite-aware) — check projectile AA/AG flags + Verses > 0%.
/// 3. If Primary fails, try Secondary (elite-aware) with same checks.
/// 4. If both fail, return None.
#[allow(dead_code)] // Used by tests; production callers go through with_override.
pub(crate) fn select_weapon<'a>(
    rules: &'a RuleSet,
    attacker_obj: &'a ObjectType,
    target_category: EntityCategory,
    target_armor: &str,
    veterancy: u16,
) -> Option<SelectedWeapon<'a>> {
    select_weapon_with_override(
        rules,
        attacker_obj,
        target_category,
        target_armor,
        veterancy,
        None,
    )
}

/// Veterancy-aware weapon selection with optional transport override.
///
/// Replaces the legacy `select_weapon_with_ifv` Option<u32> shape.
pub(crate) fn select_weapon_with_override<'a>(
    rules: &'a RuleSet,
    attacker_obj: &'a ObjectType,
    target_category: EntityCategory,
    target_armor: &str,
    veterancy: u16,
    override_: Option<WeaponOverride>,
) -> Option<SelectedWeapon<'a>> {
    match override_ {
        Some(WeaponOverride::IfvSlot(idx)) => {
            // Gunner=yes transport: attacker_obj is the TRANSPORT; fire its
            // weapon_list[idx]. If the slotted weapon can't engage, fall
            // through to the transport's own Primary/Secondary.
            if let Some(weapon_id) = attacker_obj.weapon_list.get(idx as usize) {
                if let Some(result) = try_weapon(
                    rules,
                    weapon_id,
                    target_category,
                    target_armor,
                    WeaponSlot::Primary,
                ) {
                    return Some(result);
                }
            }
            // Fall through to base Primary/Secondary on transport.
        }
        Some(WeaponOverride::OpenTransport(slot)) => {
            // Open-topped non-Gunner transport: attacker_obj is the PASSENGER;
            // fire its own Primary (slot=0) or Secondary (slot=1). No fallback
            // — if the chosen slot can't engage, the passenger doesn't fire.
            let (weapon_id, weapon_slot) = match slot {
                0 => (
                    primary_for_tier(attacker_obj, veterancy),
                    WeaponSlot::Primary,
                ),
                1 => (
                    secondary_for_tier(attacker_obj, veterancy),
                    WeaponSlot::Secondary,
                ),
                _ => return None,
            };
            return weapon_id.and_then(|wid| {
                try_weapon(rules, wid, target_category, target_armor, weapon_slot)
            });
        }
        None => {}
    }

    // Default Primary -> Secondary, with tier-aware ID lookup.
    if let Some(wid) = primary_for_tier(attacker_obj, veterancy) {
        if let Some(result) = try_weapon(
            rules,
            wid,
            target_category,
            target_armor,
            WeaponSlot::Primary,
        ) {
            return Some(result);
        }
    }
    if let Some(wid) = secondary_for_tier(attacker_obj, veterancy) {
        if let Some(result) = try_weapon(
            rules,
            wid,
            target_category,
            target_armor,
            WeaponSlot::Secondary,
        ) {
            return Some(result);
        }
    }
    None
}

/// Select the fixed weapon slot used while a `DeployFire=yes` infantry is in
/// its deployed-fire combat state.
///
/// `DeployFireWeapon=` defaults to slot 1 (Secondary) in RA2/YR. Unlike the
/// normal Primary -> Secondary selection path, deployed-fire selection is a
/// direct slot lookup: if that slot cannot engage, the unit does not fall back.
pub(crate) fn select_deploy_fire_weapon<'a>(
    rules: &'a RuleSet,
    attacker_obj: &'a ObjectType,
    target_category: EntityCategory,
    target_armor: &str,
    veterancy: u16,
    override_: Option<WeaponOverride>,
) -> Option<SelectedWeapon<'a>> {
    if override_.is_some() || !attacker_obj.deploy_fire {
        return select_weapon_with_override(
            rules,
            attacker_obj,
            target_category,
            target_armor,
            veterancy,
            override_,
        );
    }

    let (weapon_id, weapon_slot) = weapon_for_slot_index(
        attacker_obj,
        veterancy,
        attacker_obj
            .deploy_fire_weapon
            .unwrap_or(DEFAULT_DEPLOY_FIRE_WEAPON_INDEX),
    )?;
    try_weapon(rules, weapon_id, target_category, target_armor, weapon_slot)
}

fn weapon_for_slot_index(
    obj: &ObjectType,
    veterancy: u16,
    index: i32,
) -> Option<(&str, WeaponSlot)> {
    match index {
        0 => primary_for_tier(obj, veterancy).map(|weapon_id| (weapon_id, WeaponSlot::Primary)),
        1 => secondary_for_tier(obj, veterancy).map(|weapon_id| (weapon_id, WeaponSlot::Secondary)),
        _ => None,
    }
}

/// Select the weapon used by a garrisoned occupant firing from a building.
///
/// Priority chain (matching gamemd `BuildingClass::GetWeapon` 0x004526F0):
/// 1. Elite occupant → `EliteOccupyWeapon` (fall back to `OccupyWeapon`)
/// 2. Normal occupant → `OccupyWeapon`
/// 3. Fallback → occupant's Primary weapon
///
/// Returns None if no weapon can engage the target type.
pub(crate) fn select_garrison_weapon<'a>(
    rules: &'a RuleSet,
    occupant_type_ref: &str,
    occupant_veterancy: u16,
    target_category: EntityCategory,
    target_armor: &str,
) -> Option<SelectedWeapon<'a>> {
    let occupant_obj = rules.object(occupant_type_ref)?;
    let is_elite = occupant_veterancy >= 200;

    // Try elite/normal OccupyWeapon.
    let occupy_weapon_id = if is_elite {
        occupant_obj
            .elite_occupy_weapon
            .as_deref()
            .or(occupant_obj.occupy_weapon.as_deref())
    } else {
        occupant_obj.occupy_weapon.as_deref()
    };

    if let Some(wid) = occupy_weapon_id {
        if let Some(sw) = try_weapon(
            rules,
            wid,
            target_category,
            target_armor,
            WeaponSlot::Primary,
        ) {
            return Some(sw);
        }
    }

    // Fallback: occupant's primary weapon.
    if let Some(ref primary) = occupant_obj.primary {
        return try_weapon(
            rules,
            primary,
            target_category,
            target_armor,
            WeaponSlot::Primary,
        );
    }
    None
}

/// Try a single weapon against a target. Returns Some if the weapon can engage.
pub(crate) fn try_weapon<'a>(
    rules: &'a RuleSet,
    weapon_id: &'a str,
    target_category: EntityCategory,
    target_armor: &str,
    slot: WeaponSlot,
) -> Option<SelectedWeapon<'a>> {
    let weapon: &WeaponType = rules.weapon(weapon_id)?;

    // Check projectile targeting flags (AA/AG) against target category.
    if !can_projectile_engage(rules, weapon, target_category) {
        return None;
    }

    // Check warhead Verses against target armor — 0% blocks entirely.
    let warhead: &WarheadType = weapon.warhead.as_ref().and_then(|id| rules.warhead(id))?;
    let idx: usize = armor_index(target_armor);
    let verses_pct: u8 = warhead.verses.get(idx).copied().unwrap_or(100);

    if verses_gate(verses_pct) == VersesGate::Blocked {
        return None;
    }

    Some(SelectedWeapon {
        weapon_id,
        weapon,
        warhead,
        verses_pct,
        slot,
    })
}

/// Check whether a weapon's projectile can target the given entity category.
///
/// Aircraft require AA=yes on the projectile. Ground units, infantry, and
/// buildings require AG=yes (which defaults to true for most projectiles).
/// If the weapon has no projectile defined, we assume it can hit ground only.
fn can_projectile_engage(
    rules: &RuleSet,
    weapon: &WeaponType,
    target_category: EntityCategory,
) -> bool {
    let proj = weapon
        .projectile
        .as_ref()
        .and_then(|id| rules.projectile(id));

    match target_category {
        EntityCategory::Aircraft => proj.is_some_and(|p| p.aa),
        // Ground units, infantry, buildings all need AG.
        EntityCategory::Unit | EntityCategory::Infantry | EntityCategory::Structure => {
            proj.is_none_or(|p| p.ag)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::ini_parser::IniFile;

    #[test]
    fn test_weapon_override_variants() {
        let ifv = WeaponOverride::IfvSlot(16);
        let open = WeaponOverride::OpenTransport(1);
        assert_ne!(ifv, open);
        assert_eq!(ifv, WeaponOverride::IfvSlot(16));
        assert_eq!(open, WeaponOverride::OpenTransport(1));
    }

    /// Build a test RuleSet with a dual-weapon unit (AG primary, AA secondary).
    fn make_dual_weapon_rules() -> RuleSet {
        let ini_str: &str = "\
[InfantryTypes]
[VehicleTypes]
0=IFV
[AircraftTypes]
[BuildingTypes]

[IFV]
Name=IFV
Cost=600
Strength=200
Armor=light
Speed=8
Primary=Missiles
Secondary=FlakGun

[Missiles]
Damage=50
ROF=40
Range=6
Projectile=MissileGround
Warhead=HE

[FlakGun]
Damage=30
ROF=20
Range=8
Projectile=FlakProj
Warhead=Flak

[MissileGround]
AG=yes
AA=no

[FlakProj]
AG=no
AA=yes

[HE]
Verses=100%,100%,100%,80%,60%,40%,100%,40%,20%,0%,0%

[Flak]
Verses=100%,100%,100%,80%,60%,40%,100%,40%,20%,0%,0%
";
        let ini: IniFile = IniFile::from_str(ini_str);
        RuleSet::from_ini(&ini).expect("Should parse test rules")
    }

    #[test]
    fn test_primary_selected_for_ground() {
        let rules: RuleSet = make_dual_weapon_rules();
        let ifv = rules.object("IFV").unwrap();

        let result = select_weapon(&rules, ifv, EntityCategory::Unit, "light", 0);
        assert!(result.is_some());
        let selected = result.unwrap();
        assert_eq!(selected.weapon_id, "Missiles");
        assert_eq!(selected.weapon.id, "Missiles");
        assert_eq!(selected.slot, WeaponSlot::Primary);
    }

    #[test]
    fn test_secondary_selected_for_aircraft() {
        let rules: RuleSet = make_dual_weapon_rules();
        let ifv = rules.object("IFV").unwrap();

        let result = select_weapon(&rules, ifv, EntityCategory::Aircraft, "light", 0);
        assert!(result.is_some());
        let selected = result.unwrap();
        assert_eq!(selected.weapon_id, "FlakGun");
        assert_eq!(selected.weapon.id, "FlakGun");
        assert_eq!(selected.slot, WeaponSlot::Secondary);
    }

    #[test]
    fn test_zero_verses_blocks_and_falls_back() {
        // special_1 armor (index 9) has 0% in both HE and Flak warheads.
        let rules: RuleSet = make_dual_weapon_rules();
        let ifv = rules.object("IFV").unwrap();

        let result = select_weapon(&rules, ifv, EntityCategory::Unit, "special_1", 0);
        assert!(result.is_none(), "Both weapons have 0% vs special_1");
    }

    #[test]
    fn test_verses_gate_thresholds() {
        assert_eq!(verses_gate(0), VersesGate::Blocked);
        assert_eq!(verses_gate(1), VersesGate::Suppressed);
        assert_eq!(verses_gate(2), VersesGate::Normal);
        assert_eq!(verses_gate(100), VersesGate::Normal);
        assert_eq!(verses_gate(200), VersesGate::Normal);
    }

    #[test]
    fn test_no_weapons_returns_none() {
        let ini_str: &str = "\
[InfantryTypes]
[VehicleTypes]
0=CIV
[AircraftTypes]
[BuildingTypes]

[CIV]
Name=Civilian
Cost=0
Strength=50
Armor=none
";
        let ini: IniFile = IniFile::from_str(ini_str);
        let rules: RuleSet = RuleSet::from_ini(&ini).expect("parse");
        let civ = rules.object("CIV").unwrap();

        let result = select_weapon(&rules, civ, EntityCategory::Unit, "none", 0);
        assert!(result.is_none());
    }

    /// GGI-style fixture: M60 primary (AG), MissileLauncher secondary (AA+AG),
    /// elite variants for both, plus an OpenTransportWeapon=1 slot selector.
    fn make_ggi_rules() -> RuleSet {
        let ini_str: &str = "\
[InfantryTypes]
0=GGI
[VehicleTypes]
[AircraftTypes]
[BuildingTypes]

[General]
MissileROTVar=.25

[GGI]
Name=Guardian GI
Cost=400
Strength=100
Armor=none
Primary=M60
Secondary=MissileLauncher
ElitePrimary=M60E
EliteSecondary=MissileLauncherE
OpenTransportWeapon=1
DeployFire=yes

[M60]
Damage=15
ROF=20
Range=4
Projectile=InvisibleLow
Warhead=SA

[M60E]
Damage=25
ROF=20
Range=4
Projectile=InvisibleLow
Warhead=SA

[MissileLauncher]
Damage=40
ROF=40
Range=8
Burst=1
Projectile=AAHeatSeeker2
Speed=30
Warhead=GUARDWH
Report=GuardianGIDeployedAttack
MinimumRange=1

[MissileLauncherE]
Damage=50
ROF=20
Range=8
Burst=1
Projectile=AAHeatSeeker2
Speed=40
Warhead=GUARDWH
Report=GuardianGIDeployedAttack
MinimumRange=1

[InvisibleLow]
AG=yes
AA=no

[AAHeatSeeker2]
Arm=2
Shadow=no
Proximity=no
Ranged=yes
AA=yes
AG=yes
Image=DRAGON
ROT=60
SubjectToCliffs=no
SubjectToElevation=no
SubjectToWalls=no

[SA]
Verses=100%,80%,80%,50%,25%,25%,75%,50%,25%,100%,100%

[GUARDWH]
Verses=20%,20%,20%,100%,50%,100%,10%,10%,10%,100%,100%
";
        let ini: IniFile = IniFile::from_str(ini_str);
        RuleSet::from_ini(&ini).expect("Should parse GGI test rules")
    }

    #[test]
    fn test_rookie_uses_base_primary() {
        let rules = make_ggi_rules();
        let ggi = rules.object("GGI").unwrap();
        let sel = select_weapon(&rules, ggi, EntityCategory::Infantry, "none", 0).unwrap();
        assert_eq!(sel.weapon_id, "M60");
        assert_eq!(sel.slot, WeaponSlot::Primary);
    }

    #[test]
    fn test_deploy_fire_defaults_to_secondary_slot() {
        let rules = make_ggi_rules();
        let ggi = rules.object("GGI").unwrap();
        let sel =
            select_deploy_fire_weapon(&rules, ggi, EntityCategory::Unit, "heavy", 0, None).unwrap();
        assert_eq!(sel.weapon_id, "MissileLauncher");
        assert_eq!(sel.slot, WeaponSlot::Secondary);
    }

    #[test]
    fn test_veteran_still_uses_base_primary() {
        // Veteran tier (100..199) does NOT swap weapons — only Elite does.
        let rules = make_ggi_rules();
        let ggi = rules.object("GGI").unwrap();
        let sel = select_weapon(&rules, ggi, EntityCategory::Infantry, "none", 199).unwrap();
        assert_eq!(sel.weapon_id, "M60", "veteran at v=199 must still fire M60");
    }

    #[test]
    fn test_elite_uses_elite_primary_at_threshold() {
        let rules = make_ggi_rules();
        let ggi = rules.object("GGI").unwrap();
        let sel = select_weapon(&rules, ggi, EntityCategory::Infantry, "none", 200).unwrap();
        assert_eq!(sel.weapon_id, "M60E", "elite at v=200 must fire M60E");
    }

    #[test]
    fn test_elite_secondary_via_open_transport() {
        // GGI inside BFRT at elite tier (Secondary slot): MissileLauncherE.
        let rules = make_ggi_rules();
        let ggi = rules.object("GGI").unwrap();
        let sel = select_weapon_with_override(
            &rules,
            ggi,
            EntityCategory::Aircraft,
            "light",
            200,
            Some(WeaponOverride::OpenTransport(1)),
        )
        .unwrap();
        assert_eq!(sel.weapon_id, "MissileLauncherE");
    }

    #[test]
    fn test_open_transport_routes_to_passenger_secondary() {
        // GGI inside BFRT (no Gunner): OpenTransport(1) -> passenger's own
        // Secondary = MissileLauncher (AA-capable). Without override, GGI's
        // Primary (M60) wouldn't engage Aircraft (AA=no).
        let rules = make_ggi_rules();
        let ggi = rules.object("GGI").unwrap();
        let sel = select_weapon_with_override(
            &rules,
            ggi,
            EntityCategory::Aircraft,
            "light",
            0,
            Some(WeaponOverride::OpenTransport(1)),
        )
        .unwrap();
        assert_eq!(sel.weapon_id, "MissileLauncher");
    }

    #[test]
    fn test_open_transport_primary_fires_passenger_primary() {
        let rules = make_ggi_rules();
        let ggi = rules.object("GGI").unwrap();
        let sel = select_weapon_with_override(
            &rules,
            ggi,
            EntityCategory::Infantry,
            "none",
            0,
            Some(WeaponOverride::OpenTransport(0)),
        )
        .unwrap();
        assert_eq!(sel.weapon_id, "M60");
    }

    #[test]
    fn test_open_transport_no_fallback_on_engagement_fail() {
        // OpenTransport(0) routes to Primary (M60 / AG=yes only) for an
        // Aircraft target. Engagement check fails; no fallback to Secondary
        // — unlike IfvSlot, OpenTransport returns None instead.
        let rules = make_ggi_rules();
        let ggi = rules.object("GGI").unwrap();
        let sel = select_weapon_with_override(
            &rules,
            ggi,
            EntityCategory::Aircraft,
            "light",
            0,
            Some(WeaponOverride::OpenTransport(0)),
        );
        assert!(
            sel.is_none(),
            "OpenTransport must NOT fall back to passenger secondary"
        );
    }
}
