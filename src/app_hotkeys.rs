//! Retail keyboard binding loading and logical-key dispatch.
//!
//! This app-layer module translates winit's logical keys into the encoded
//! Win32 key words stored in `KEYBOARDMD.INI`. It deliberately stops at
//! semantic command names; gameplay effects remain in `app_input`.

use std::collections::BTreeMap;

use winit::keyboard::{Key, KeyLocation, ModifiersState, NamedKey};

use crate::assets::asset_manager::AssetManager;
use crate::rules::ini_parser::IniFile;

const SHIFT_BIT: u16 = 0x100;
const CTRL_BIT: u16 = 0x200;
const ALT_BIT: u16 = 0x400;
const VK_ESCAPE: u16 = 0x1b;
const VK_SPACE: u16 = 0x20;
const VK_DELETE: u16 = 0x2e;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HotkeyCommand {
    CenterView,
    Options,
    CenterOnRadarEvent,
    TeamSelect(usize),
    TeamAddSelect(usize),
    TeamCreate(usize),
    TeamCenter(usize),
    ToggleAlliance,
    PlaceBeacon,
    AllToCheer,
    DeployObject,
    InfantryTab,
    Follow,
    GuardObject,
    CenterBase,
    ToggleRepair,
    ToggleSell,
    PreviousObject,
    NextObject,
    CombatantSelect,
    StructureTab,
    UnitTab,
    StopObject,
    TypeSelect,
    PageUser,
    DefenseTab,
    ScatterObject,
    VeterancyNav,
    PlanningMode,
    SidebarDown,
    SidebarUp,
    Delete,
    View(usize),
    Taunt(usize),
    ScreenCapture,
    SetView(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HotkeyFallback {
    DiplomacyDialog,
    ArrowLeft,
    ArrowUp,
    ArrowRight,
    ArrowDown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HotkeyResolution {
    Command(HotkeyCommand),
    Fallback(HotkeyFallback),
    Unhandled,
}

impl HotkeyCommand {
    fn accepts_base_modifiers(self, modifiers: ModifiersState) -> bool {
        let _ = self;
        modifier_bits(modifiers) == 0
    }
}

impl HotkeyFallback {
    pub(crate) fn arrow_key_code(self) -> Option<winit::keyboard::KeyCode> {
        use winit::keyboard::KeyCode;
        match self {
            Self::ArrowLeft => Some(KeyCode::ArrowLeft),
            Self::ArrowUp => Some(KeyCode::ArrowUp),
            Self::ArrowRight => Some(KeyCode::ArrowRight),
            Self::ArrowDown => Some(KeyCode::ArrowDown),
            Self::DiplomacyDialog => None,
        }
    }
}

pub(crate) fn fallback_scroll_key(
    resolution: HotkeyResolution,
) -> Option<winit::keyboard::KeyCode> {
    match resolution {
        HotkeyResolution::Fallback(fallback) => fallback.arrow_key_code(),
        HotkeyResolution::Command(_) | HotkeyResolution::Unhandled => None,
    }
}

pub(crate) fn physical_scroll_key(
    physical: winit::keyboard::KeyCode,
) -> Option<winit::keyboard::KeyCode> {
    use winit::keyboard::KeyCode;
    match physical {
        KeyCode::ArrowLeft | KeyCode::Numpad4 => Some(KeyCode::ArrowLeft),
        KeyCode::ArrowUp | KeyCode::Numpad8 => Some(KeyCode::ArrowUp),
        KeyCode::ArrowRight | KeyCode::Numpad6 => Some(KeyCode::ArrowRight),
        KeyCode::ArrowDown | KeyCode::Numpad2 => Some(KeyCode::ArrowDown),
        _ => None,
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct HotkeyBindings {
    by_key: BTreeMap<u16, HotkeyCommand>,
}

impl HotkeyBindings {
    pub(crate) fn load(assets: Option<&AssetManager>) -> Self {
        Self::from_ini_bytes(assets.and_then(|assets| assets.get_ref("KEYBOARDMD.INI")))
    }

    fn from_ini_bytes(bytes: Option<&[u8]>) -> Self {
        let mut bindings = Self::default();
        if let Some(section) = bytes
            .and_then(|bytes| IniFile::from_bytes(bytes).ok())
            .and_then(|ini| ini.section("Hotkey").cloned())
        {
            for name in section.keys() {
                let Some(command) = command_from_name(name) else {
                    continue;
                };
                let Some(encoded) = section
                    .get_i32(name)
                    .filter(|value| *value != 0)
                    .and_then(|value| u16::try_from(value).ok())
                else {
                    continue;
                };
                bindings.by_key.insert(encoded, command);
            }
        }

        bindings.by_key.insert(VK_DELETE, HotkeyCommand::Delete);
        bindings.by_key.insert(VK_ESCAPE, HotkeyCommand::Options);
        bindings
            .by_key
            .insert(VK_SPACE, HotkeyCommand::CenterOnRadarEvent);
        bindings
    }

    pub(crate) fn resolve_event(
        &self,
        logical_key: &Key,
        location: KeyLocation,
        modifiers: ModifiersState,
    ) -> HotkeyResolution {
        // Native removes the 0x800 release bit from both lookup identities and
        // passes the raw edge only to CanExecute; ordinary receivers reject
        // releases. The app therefore resolves one edge-neutral identity and
        // models press/release only for the hardcoded held-arrow fallback.
        let Some(virtual_key) = logical_virtual_key(logical_key, location) else {
            return HotkeyResolution::Unhandled;
        };
        if let Some(command) = self.by_key.get(&virtual_key).copied()
            && command.accepts_base_modifiers(modifiers)
        {
            return HotkeyResolution::Command(command);
        }
        if let Some(command) = self
            .by_key
            .get(&(virtual_key | modifier_bits(modifiers)))
            .copied()
        {
            return HotkeyResolution::Command(command);
        }
        fallback_for_virtual_key(virtual_key)
            .map(HotkeyResolution::Fallback)
            .unwrap_or(HotkeyResolution::Unhandled)
    }

    #[cfg(test)]
    fn resolve(
        &self,
        logical_key: &Key,
        location: KeyLocation,
        modifiers: ModifiersState,
    ) -> Option<HotkeyCommand> {
        match self.resolve_event(logical_key, location, modifiers) {
            HotkeyResolution::Command(command) => Some(command),
            HotkeyResolution::Fallback(_) | HotkeyResolution::Unhandled => None,
        }
    }
}

fn fallback_for_virtual_key(virtual_key: u16) -> Option<HotkeyFallback> {
    Some(match virtual_key {
        0x09 => HotkeyFallback::DiplomacyDialog,
        0x25 => HotkeyFallback::ArrowLeft,
        0x26 => HotkeyFallback::ArrowUp,
        0x27 => HotkeyFallback::ArrowRight,
        0x28 => HotkeyFallback::ArrowDown,
        _ => return None,
    })
}

pub(crate) fn modifier_bits(modifiers: ModifiersState) -> u16 {
    (if modifiers.shift_key() { SHIFT_BIT } else { 0 })
        | (if modifiers.control_key() { CTRL_BIT } else { 0 })
        | (if modifiers.alt_key() { ALT_BIT } else { 0 })
}

pub(crate) fn input_admitted_while_paused(paused: bool, key: &Key) -> bool {
    !paused || matches!(key, Key::Named(NamedKey::Escape))
}

/// Windows derives `key_without_modifiers` with NumLock forced off. Preserve
/// the raw logical keypad identity, while standard keys use the modifier-free
/// value needed for bindings such as Shift+1.
pub(crate) fn binding_logical_key<'a>(
    raw: &'a Key,
    without_modifiers: &'a Key,
    location: KeyLocation,
) -> &'a Key {
    if location == KeyLocation::Numpad {
        raw
    } else {
        without_modifiers
    }
}

pub(crate) fn logical_virtual_key(key: &Key, location: KeyLocation) -> Option<u16> {
    match key {
        Key::Character(text) if text.chars().count() == 1 => {
            let character = text.chars().next()?.to_ascii_uppercase();
            if location == KeyLocation::Numpad {
                if character.is_ascii_digit() {
                    return Some(0x60 + (character as u16 - '0' as u16));
                }
                if character == '.' {
                    return Some(0x6e);
                }
            }
            character
                .is_ascii_alphanumeric()
                .then_some(character as u16)
        }
        Key::Named(named) => named_virtual_key(*named),
        _ => None,
    }
}

fn named_virtual_key(key: NamedKey) -> Option<u16> {
    Some(match key {
        NamedKey::Backspace => 0x08,
        NamedKey::Tab => 0x09,
        NamedKey::Clear => 0x0c,
        NamedKey::Enter => 0x0d,
        NamedKey::Escape => VK_ESCAPE,
        NamedKey::Space => VK_SPACE,
        NamedKey::PageUp => 0x21,
        NamedKey::PageDown => 0x22,
        NamedKey::End => 0x23,
        NamedKey::Home => 0x24,
        NamedKey::ArrowLeft => 0x25,
        NamedKey::ArrowUp => 0x26,
        NamedKey::ArrowRight => 0x27,
        NamedKey::ArrowDown => 0x28,
        NamedKey::Insert => 0x2d,
        NamedKey::Delete => VK_DELETE,
        NamedKey::F1 => 0x70,
        NamedKey::F2 => 0x71,
        NamedKey::F3 => 0x72,
        NamedKey::F4 => 0x73,
        NamedKey::F5 => 0x74,
        NamedKey::F6 => 0x75,
        NamedKey::F7 => 0x76,
        NamedKey::F8 => 0x77,
        NamedKey::F9 => 0x78,
        NamedKey::F10 => 0x79,
        NamedKey::F11 => 0x7a,
        NamedKey::F12 => 0x7b,
        _ => return None,
    })
}

fn command_from_name(name: &str) -> Option<HotkeyCommand> {
    Some(match name {
        "CenterView" => HotkeyCommand::CenterView,
        "Options" => HotkeyCommand::Options,
        "CenterOnRadarEvent" => HotkeyCommand::CenterOnRadarEvent,
        "ToggleAlliance" => HotkeyCommand::ToggleAlliance,
        "PlaceBeacon" => HotkeyCommand::PlaceBeacon,
        "AllToCheer" => HotkeyCommand::AllToCheer,
        "DeployObject" => HotkeyCommand::DeployObject,
        "InfantryTab" => HotkeyCommand::InfantryTab,
        "Follow" => HotkeyCommand::Follow,
        "GuardObject" => HotkeyCommand::GuardObject,
        "CenterBase" => HotkeyCommand::CenterBase,
        "ToggleRepair" => HotkeyCommand::ToggleRepair,
        "ToggleSell" => HotkeyCommand::ToggleSell,
        "PreviousObject" => HotkeyCommand::PreviousObject,
        "NextObject" => HotkeyCommand::NextObject,
        "CombatantSelect" => HotkeyCommand::CombatantSelect,
        "StructureTab" => HotkeyCommand::StructureTab,
        "UnitTab" => HotkeyCommand::UnitTab,
        "StopObject" => HotkeyCommand::StopObject,
        "TypeSelect" => HotkeyCommand::TypeSelect,
        "PageUser" => HotkeyCommand::PageUser,
        "DefenseTab" => HotkeyCommand::DefenseTab,
        "ScatterObject" => HotkeyCommand::ScatterObject,
        "VeterancyNav" => HotkeyCommand::VeterancyNav,
        "PlanningMode" => HotkeyCommand::PlanningMode,
        "SidebarDown" => HotkeyCommand::SidebarDown,
        "SidebarUp" => HotkeyCommand::SidebarUp,
        "Delete" => HotkeyCommand::Delete,
        "ScreenCapture" => HotkeyCommand::ScreenCapture,
        _ => return parse_numbered_command(name),
    })
}

fn parse_numbered_command(name: &str) -> Option<HotkeyCommand> {
    if let Some(slot) = name
        .strip_prefix("View")
        .and_then(|value| value.parse::<usize>().ok())
        .and_then(|number| number.checked_sub(1))
        .filter(|slot| *slot < 4)
    {
        return Some(HotkeyCommand::View(slot));
    }
    if let Some(slot) = name
        .strip_prefix("SetView")
        .and_then(|value| value.parse::<usize>().ok())
        .and_then(|number| number.checked_sub(1))
        .filter(|slot| *slot < 4)
    {
        return Some(HotkeyCommand::SetView(slot));
    }
    let (prefix, suffix) = name.rsplit_once('_')?;
    let number = suffix.parse::<usize>().ok()?;
    let slot = if number == 10 { 0 } else { number };
    if slot > 9 {
        return None;
    }
    match prefix {
        "TeamSelect" => Some(HotkeyCommand::TeamSelect(slot)),
        "TeamAddSelect" => Some(HotkeyCommand::TeamAddSelect(slot)),
        "TeamCreate" => Some(HotkeyCommand::TeamCreate(slot)),
        "TeamCenter" => Some(HotkeyCommand::TeamCenter(slot)),
        "Taunt" if (1..=8).contains(&number) => Some(HotkeyCommand::Taunt(number - 1)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn modifiers(shift: bool, ctrl: bool, alt: bool) -> ModifiersState {
        let mut value = ModifiersState::empty();
        value.set(ModifiersState::SHIFT, shift);
        value.set(ModifiersState::CONTROL, ctrl);
        value.set(ModifiersState::ALT, alt);
        value
    }

    fn character(value: &str) -> Key {
        Key::Character(value.into())
    }

    #[test]
    fn parser_skips_unknown_and_zero_entries() {
        let bindings = HotkeyBindings::from_ini_bytes(Some(
            b"[Hotkey]\nStopObject=83\nUnknownCommand=71\nDeployObject=0\n",
        ));
        assert_eq!(
            bindings.resolve(
                &character("s"),
                KeyLocation::Standard,
                ModifiersState::empty()
            ),
            Some(HotkeyCommand::StopObject)
        );
        assert_eq!(
            bindings.resolve(
                &character("g"),
                KeyLocation::Standard,
                ModifiersState::empty()
            ),
            None
        );
        assert_eq!(
            bindings.resolve(
                &character("d"),
                KeyLocation::Standard,
                ModifiersState::empty()
            ),
            None
        );
    }

    #[test]
    fn inactive_archive_residue_names_are_not_registered_commands() {
        let bindings = HotkeyBindings::from_ini_bytes(Some(
            b"[Hotkey]\nRightSidebarUp=65\nRightSidebarDown=66\nLeftSidebarDown=67\nLeftSidebarUp=68\nRaiseCell=69\nLowerCell=70\nDeleteObject=71\nSidebarPageUp=72\nSidebarPageDown=73\nSidebarDown=98\nSidebarUp=104\n",
        ));
        for letter in ["a", "b", "c", "d", "e", "f", "g", "h", "i"] {
            assert_eq!(
                bindings.resolve_event(
                    &character(letter),
                    KeyLocation::Standard,
                    ModifiersState::empty(),
                ),
                HotkeyResolution::Unhandled
            );
        }
        assert_eq!(
            bindings.resolve_event(
                &character("2"),
                KeyLocation::Numpad,
                ModifiersState::empty(),
            ),
            HotkeyResolution::Command(HotkeyCommand::SidebarDown)
        );
        assert_eq!(
            bindings.resolve_event(
                &character("8"),
                KeyLocation::Numpad,
                ModifiersState::empty(),
            ),
            HotkeyResolution::Command(HotkeyCommand::SidebarUp)
        );
    }

    #[test]
    fn missing_file_has_only_forced_bindings() {
        let bindings = HotkeyBindings::from_ini_bytes(None);
        assert_eq!(
            bindings.resolve(
                &Key::Named(NamedKey::Escape),
                KeyLocation::Standard,
                ModifiersState::empty()
            ),
            Some(HotkeyCommand::Options)
        );
        assert_eq!(
            bindings.resolve(
                &Key::Named(NamedKey::Space),
                KeyLocation::Standard,
                ModifiersState::empty()
            ),
            Some(HotkeyCommand::CenterOnRadarEvent)
        );
        assert_eq!(
            bindings.resolve(
                &character("s"),
                KeyLocation::Standard,
                ModifiersState::empty()
            ),
            None
        );
    }

    #[test]
    fn forced_bindings_override_archive_rows() {
        let bindings = HotkeyBindings::from_ini_bytes(Some(
            b"[Hotkey]\nStopObject=27\nDeployObject=32\nGuardObject=46\n",
        ));
        assert_eq!(
            bindings.resolve(
                &Key::Named(NamedKey::Escape),
                KeyLocation::Standard,
                ModifiersState::empty()
            ),
            Some(HotkeyCommand::Options)
        );
        assert_eq!(
            bindings.resolve(
                &Key::Named(NamedKey::Space),
                KeyLocation::Standard,
                ModifiersState::empty()
            ),
            Some(HotkeyCommand::CenterOnRadarEvent)
        );
        assert_eq!(
            bindings.resolve(
                &Key::Named(NamedKey::Delete),
                KeyLocation::Standard,
                ModifiersState::empty()
            ),
            Some(HotkeyCommand::Delete)
        );
    }

    #[test]
    fn logical_letters_and_numpad_digits_use_distinct_virtual_keys() {
        assert_eq!(
            logical_virtual_key(&character("z"), KeyLocation::Standard),
            Some(0x5a)
        );
        assert_eq!(
            logical_virtual_key(&character("2"), KeyLocation::Standard),
            Some(0x32)
        );
        assert_eq!(
            logical_virtual_key(&character("2"), KeyLocation::Numpad),
            Some(0x62)
        );
        assert_eq!(
            logical_virtual_key(&character("."), KeyLocation::Numpad),
            Some(0x6e)
        );
    }

    #[test]
    fn paused_capture_admits_only_escape() {
        assert!(input_admitted_while_paused(
            true,
            &Key::Named(NamedKey::Escape)
        ));
        assert!(!input_admitted_while_paused(true, &character("s")));
        assert!(input_admitted_while_paused(false, &character("s")));
    }

    #[test]
    fn exact_modifiers_resolve_group_commands_and_reject_two_modifier_chords() {
        let bindings = HotkeyBindings::from_ini_bytes(Some(
            b"[Hotkey]\nTeamSelect_1=49\nTeamAddSelect_1=305\nTeamCreate_1=561\nTeamCenter_1=1073\n",
        ));
        let key = character("1");
        assert_eq!(
            bindings.resolve(&key, KeyLocation::Standard, ModifiersState::empty()),
            Some(HotkeyCommand::TeamSelect(1))
        );
        assert_eq!(
            bindings.resolve(&key, KeyLocation::Standard, modifiers(true, false, false)),
            Some(HotkeyCommand::TeamAddSelect(1))
        );
        assert_eq!(
            bindings.resolve(&key, KeyLocation::Standard, modifiers(false, true, false)),
            Some(HotkeyCommand::TeamCreate(1))
        );
        assert_eq!(
            bindings.resolve(&key, KeyLocation::Standard, modifiers(false, false, true)),
            Some(HotkeyCommand::TeamCenter(1))
        );
        assert_eq!(
            bindings.resolve(&key, KeyLocation::Standard, modifiers(true, true, false)),
            None
        );
    }

    #[test]
    fn shifted_symbol_routes_through_modifier_free_logical_key() {
        let bindings = HotkeyBindings::from_ini_bytes(Some(
            b"[Hotkey]\nTeamAddSelect_1=305\nScreenCapture=339\n",
        ));
        let raw_shifted_digit = character("!");
        let modifier_free_digit = character("1");
        assert_eq!(
            logical_virtual_key(&raw_shifted_digit, KeyLocation::Standard),
            None
        );
        let selected_digit = binding_logical_key(
            &raw_shifted_digit,
            &modifier_free_digit,
            KeyLocation::Standard,
        );
        assert_eq!(
            bindings.resolve_event(
                selected_digit,
                KeyLocation::Standard,
                modifiers(true, false, false),
            ),
            HotkeyResolution::Command(HotkeyCommand::TeamAddSelect(1))
        );

        let raw_shifted_letter = character("S");
        let modifier_free_letter = character("s");
        let selected_letter = binding_logical_key(
            &raw_shifted_letter,
            &modifier_free_letter,
            KeyLocation::Standard,
        );
        assert_eq!(
            bindings.resolve_event(
                selected_letter,
                KeyLocation::Standard,
                modifiers(true, false, false),
            ),
            HotkeyResolution::Command(HotkeyCommand::ScreenCapture)
        );
        assert_eq!(
            logical_virtual_key(&raw_shifted_letter, KeyLocation::Standard),
            logical_virtual_key(&modifier_free_letter, KeyLocation::Standard)
        );
    }

    #[test]
    fn boundary_preserves_numpad_identity_but_strips_standard_shift() {
        let raw_numpad_2 = character("2");
        let modifier_free_down = Key::Named(NamedKey::ArrowDown);
        assert_eq!(
            binding_logical_key(&raw_numpad_2, &modifier_free_down, KeyLocation::Numpad),
            &raw_numpad_2
        );

        let raw_shifted = character("!");
        let modifier_free_1 = character("1");
        assert_eq!(
            binding_logical_key(&raw_shifted, &modifier_free_1, KeyLocation::Standard),
            &modifier_free_1
        );
    }

    #[test]
    fn numlock_on_digits_resolve_sidebar_bindings() {
        let bindings =
            HotkeyBindings::from_ini_bytes(Some(b"[Hotkey]\nSidebarDown=98\nSidebarUp=104\n"));
        for (raw, without_modifiers, expected) in [
            (
                character("2"),
                Key::Named(NamedKey::ArrowDown),
                HotkeyCommand::SidebarDown,
            ),
            (
                character("8"),
                Key::Named(NamedKey::ArrowUp),
                HotkeyCommand::SidebarUp,
            ),
        ] {
            let key = binding_logical_key(&raw, &without_modifiers, KeyLocation::Numpad);
            assert_eq!(
                bindings.resolve_event(key, KeyLocation::Numpad, ModifiersState::empty()),
                HotkeyResolution::Command(expected)
            );
        }
    }

    #[test]
    fn numlock_off_named_arrows_remain_arrow_fallbacks() {
        let bindings = HotkeyBindings::from_ini_bytes(None);
        for (key, expected) in [
            (Key::Named(NamedKey::ArrowDown), HotkeyFallback::ArrowDown),
            (Key::Named(NamedKey::ArrowUp), HotkeyFallback::ArrowUp),
        ] {
            let without_modifiers = character("5");
            let selected = binding_logical_key(&key, &without_modifiers, KeyLocation::Numpad);
            assert_eq!(
                bindings.resolve_event(selected, KeyLocation::Numpad, ModifiersState::empty()),
                HotkeyResolution::Fallback(expected)
            );
        }
    }

    #[test]
    fn held_scroll_keys_canonicalize_keypad_fallbacks_and_preserve_binding_precedence() {
        use winit::keyboard::KeyCode;

        assert_eq!(
            fallback_scroll_key(HotkeyResolution::Fallback(HotkeyFallback::ArrowDown)),
            Some(KeyCode::ArrowDown)
        );
        assert_eq!(
            fallback_scroll_key(HotkeyResolution::Command(HotkeyCommand::SidebarDown)),
            None
        );
        assert_eq!(
            physical_scroll_key(KeyCode::Numpad2),
            Some(KeyCode::ArrowDown)
        );
        assert_eq!(
            physical_scroll_key(KeyCode::ArrowLeft),
            Some(KeyCode::ArrowLeft)
        );
    }

    #[test]
    fn keypad_clear_resolves_center_view_and_standard_arrow_stays_fallback() {
        let bindings = HotkeyBindings::from_ini_bytes(Some(b"[Hotkey]\nCenterView=12\n"));
        assert_eq!(
            bindings.resolve_event(
                &Key::Named(NamedKey::Clear),
                KeyLocation::Numpad,
                ModifiersState::empty(),
            ),
            HotkeyResolution::Command(HotkeyCommand::CenterView)
        );
        assert_eq!(
            bindings.resolve_event(
                &Key::Named(NamedKey::ArrowLeft),
                KeyLocation::Standard,
                ModifiersState::empty(),
            ),
            HotkeyResolution::Fallback(HotkeyFallback::ArrowLeft)
        );
    }

    #[test]
    fn archive_shaped_stock_bindings_resolve_existing_semantic_owners() {
        let bindings = HotkeyBindings::from_ini_bytes(Some(
            b"[Hotkey]\nDeployObject=68\nGuardObject=71\nStructureTab=81\nUnitTab=82\nStopObject=83\nTypeSelect=84\nDefenseTab=87\nDelete=110\nScreenCapture=339\nView1=112\nSetView1=624\n",
        ));
        assert_eq!(
            bindings.resolve(
                &character("d"),
                KeyLocation::Standard,
                ModifiersState::empty()
            ),
            Some(HotkeyCommand::DeployObject)
        );
        assert_eq!(
            bindings.resolve(
                &character("s"),
                KeyLocation::Standard,
                modifiers(true, false, false)
            ),
            Some(HotkeyCommand::ScreenCapture)
        );
        assert_eq!(
            bindings.resolve(
                &Key::Named(NamedKey::F1),
                KeyLocation::Standard,
                ModifiersState::empty()
            ),
            Some(HotkeyCommand::View(0))
        );
        assert_eq!(
            bindings.resolve(
                &Key::Named(NamedKey::F1),
                KeyLocation::Standard,
                modifiers(false, true, false)
            ),
            Some(HotkeyCommand::SetView(0))
        );
        assert_eq!(
            bindings.resolve(
                &character("."),
                KeyLocation::Numpad,
                ModifiersState::empty()
            ),
            Some(HotkeyCommand::Delete)
        );
    }

    #[test]
    fn unclaimed_tab_and_arrows_resolve_to_hardcoded_fallbacks() {
        let bindings = HotkeyBindings::from_ini_bytes(None);
        assert_eq!(
            bindings.resolve_event(
                &Key::Named(NamedKey::Tab),
                KeyLocation::Standard,
                ModifiersState::empty(),
            ),
            HotkeyResolution::Fallback(HotkeyFallback::DiplomacyDialog)
        );
        assert_eq!(
            bindings.resolve_event(
                &Key::Named(NamedKey::ArrowLeft),
                KeyLocation::Standard,
                modifiers(true, false, false),
            ),
            HotkeyResolution::Fallback(HotkeyFallback::ArrowLeft)
        );
    }

    #[test]
    fn ini_binding_claims_tab_or_arrow_before_fallback() {
        let bindings =
            HotkeyBindings::from_ini_bytes(Some(b"[Hotkey]\nStopObject=9\nDeployObject=37\n"));
        assert_eq!(
            bindings.resolve_event(
                &Key::Named(NamedKey::Tab),
                KeyLocation::Standard,
                ModifiersState::empty(),
            ),
            HotkeyResolution::Command(HotkeyCommand::StopObject)
        );
        assert_eq!(
            bindings.resolve_event(
                &Key::Named(NamedKey::ArrowLeft),
                KeyLocation::Standard,
                ModifiersState::empty(),
            ),
            HotkeyResolution::Command(HotkeyCommand::DeployObject)
        );
    }
}
