//! Sidebar view builder: constructs `SidebarView` from production state.
//!
//! Extracted from sidebar/mod.rs for file-size limits.

use crate::sim::intern::InternedId;
use crate::sim::production::{
    BuildOption, BuildQueueState, ProducerFocusView, ProductionCategory, QueueItemView,
    ReadyBuildingView,
};
use crate::sim::superweapon::SuperWeaponView;

use super::gadget_flash::SidebarGadgetState;

/// The armed targeting selection as the sidebar consumes it (F06): exactly
/// one of building placement or superweapon may be armed at a time. This is
/// the sidebar-owned projection; the app converts its targeting state at the
/// refresh seam so presentation never imports app vocabulary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArmedSidebarEntry {
    /// Ready building awaiting placement (building INI section name).
    BuildingPlacement(String),
    /// Charged superweapon awaiting a target cell (SW INI section name).
    SuperWeapon(String),
}

impl ArmedSidebarEntry {
    pub fn as_building_placement(&self) -> Option<&str> {
        match self {
            Self::BuildingPlacement(section) => Some(section.as_str()),
            _ => None,
        }
    }

    pub fn as_super_weapon(&self) -> Option<&str> {
        match self {
            Self::SuperWeapon(section) => Some(section.as_str()),
            _ => None,
        }
    }
}
use super::{
    CAMEO_COLUMNS, Rect, SidebarAction, SidebarChromeLayoutSpec, SidebarControlButton, SidebarItem,
    SidebarScrollButton, SidebarTab, SidebarTabButton, SidebarToggleButton, SidebarView,
    compute_layout_with_spec, scroll_button_rects,
};

#[cfg(test)]
pub(crate) fn build_sidebar_view(
    screen_w: f32,
    screen_h: f32,
    active_tab: SidebarTab,
    credits: i32,
    power_produced: i32,
    power_drained: i32,
    tab_button_size: Option<[f32; 2]>,
    queue_items: &[QueueItemView],
    build_options: &[BuildOption],
    ready_buildings: &[ReadyBuildingView],
    armed: Option<&ArmedSidebarEntry>,
    producer_focus: &[ProducerFocusView],
    scroll_rows: usize,
    interner: Option<&crate::sim::intern::StringInterner>,
    gadget_state: &SidebarGadgetState,
    repair_button_size: Option<[f32; 2]>,
    sell_button_size: Option<[f32; 2]>,
) -> SidebarView {
    build_sidebar_view_with_spec(
        SidebarChromeLayoutSpec::stock(),
        screen_w,
        screen_h,
        active_tab,
        credits,
        power_produced,
        power_drained,
        tab_button_size,
        queue_items,
        build_options,
        ready_buildings,
        armed,
        producer_focus,
        scroll_rows,
        interner,
        &[],
        gadget_state,
        repair_button_size,
        sell_button_size,
        None,
        None,
    )
}

pub(crate) fn build_sidebar_view_with_spec(
    layout_spec: SidebarChromeLayoutSpec,
    screen_w: f32,
    screen_h: f32,
    active_tab: SidebarTab,
    credits: i32,
    power_produced: i32,
    power_drained: i32,
    tab_button_size: Option<[f32; 2]>,
    queue_items: &[QueueItemView],
    build_options: &[BuildOption],
    ready_buildings: &[ReadyBuildingView],
    armed: Option<&ArmedSidebarEntry>,
    producer_focus: &[ProducerFocusView],
    scroll_rows: usize,
    interner: Option<&crate::sim::intern::StringInterner>,
    sw_views: &[SuperWeaponView],
    gadget_state: &SidebarGadgetState,
    repair_button_size: Option<[f32; 2]>,
    sell_button_size: Option<[f32; 2]>,
    scroll_down_button_size: Option<[f32; 2]>,
    scroll_up_button_size: Option<[f32; 2]>,
) -> SidebarView {
    // Collect items first to know how many rows we need.
    let selected_category = active_tab.category();
    let mut all_entries = collect_build_entries(
        selected_category,
        queue_items,
        build_options,
        ready_buildings,
        armed,
        interner,
        sw_views,
    );
    let total_items = all_entries.len();
    let total_rows = (total_items + CAMEO_COLUMNS - 1) / CAMEO_COLUMNS;

    // Compute layout with actual item row count — sidebar height adapts to content.
    let layout = compute_layout_with_spec(layout_spec, screen_w, screen_h, total_rows);
    let panel_rect = Rect {
        x: layout.sidebar_x,
        y: 0.0,
        w: layout_spec.sidebar_width,
        h: screen_h,
    };
    let credits_frac = (credits.max(0) as f32 / 5000.0).clamp(0.0, 1.0);
    let power_frac = if power_drained <= 0 {
        1.0
    } else {
        (power_produced.max(0) as f32 / power_drained.max(1) as f32).clamp(0.0, 1.0)
    };
    let low_power = power_produced < power_drained;

    // Tab buttons bottom-align to the 16px strip so the extra button height
    // overhangs upward into side1 instead of downward into the cameo grid.
    let tab_count = SidebarTab::all().len();
    let tab_w = tab_button_size.map(|s| s[0]).unwrap_or(28.0);
    let tab_h = tab_button_size.map(|s| s[1]).unwrap_or(27.0);
    let tab_total = tab_w * tab_count as f32;
    let tab_start_x = layout.sidebar_x + (layout_spec.sidebar_width - tab_total) * 0.5;
    let tab_y = layout.cameo_grid_top - tab_h;
    let tabs: Vec<SidebarTabButton> = SidebarTab::all()
        .into_iter()
        .enumerate()
        .map(|(idx, tab)| {
            // Per-tab X nudges: tab00 shifted left 2px, tab03 shifted right 2px.
            let nudge = match idx {
                0 => -2.0,
                1 => -1.0,
                3 => 2.0,
                _ => 0.0,
            };
            SidebarTabButton {
                tab,
                rect: Rect {
                    x: tab_start_x + idx as f32 * tab_w + nudge,
                    y: tab_y,
                    w: tab_w,
                    h: tab_h,
                },
                active: tab == active_tab,
                disabled: gadget_state.tab_disabled[idx],
                frame_index: gadget_state.tab_frame(idx, tab == active_tab),
            }
        })
        .collect();

    // Repair / Sell SHP-driven toggle buttons. Position comes from
    // SidebarChromeLayoutSpec (already-scaled). Dimensions come from the
    // chrome atlas via repair_button_size / sell_button_size (already × ui_scale
    // at the call site) — matches the tab_button_size convention so hit-test
    // and render rects agree at every UI scale. When the atlas is unavailable
    // (callers passing None), rects collapse to zero size so hit-test never
    // matches. Frame index comes from SidebarGadgetState.
    let [repair_w, repair_h] = repair_button_size.unwrap_or([0.0, 0.0]);
    let [sell_w, sell_h] = sell_button_size.unwrap_or([0.0, 0.0]);
    let side1_y_local = layout.side1_y;
    let repair_rect = Rect {
        x: layout.sidebar_x + layout_spec.repair_x,
        y: side1_y_local + layout_spec.repair_y,
        w: repair_w,
        h: repair_h,
    };
    let sell_rect = Rect {
        x: layout.sidebar_x + layout_spec.sell_x,
        y: side1_y_local + layout_spec.sell_y,
        w: sell_w,
        h: sell_h,
    };
    let repair_button = SidebarToggleButton {
        rect: repair_rect,
        action: SidebarAction::ToggleRepairMode,
        active: gadget_state.repair_mode_on,
        disabled: gadget_state.repair_disabled,
        frame_index: gadget_state.repair_frame(),
    };
    let sell_button = SidebarToggleButton {
        rect: sell_rect,
        action: SidebarAction::ToggleSellMode,
        active: gadget_state.sell_mode_on,
        disabled: gadget_state.sell_disabled,
        frame_index: gadget_state.sell_frame(),
    };
    let (scroll_down_rect, scroll_up_rect) = scroll_button_rects(
        &layout,
        layout_spec.sidebar_width,
        scroll_down_button_size,
        scroll_up_button_size,
    );
    let scroll_down_button = SidebarScrollButton {
        rect: scroll_down_rect,
        disabled: false,
        frame_index: gadget_state.scroll_down_frame(),
    };
    let scroll_up_button = SidebarScrollButton {
        rect: scroll_up_rect,
        disabled: false,
        frame_index: gadget_state.scroll_up_frame(),
    };

    // Cameo grid positioning.
    let grid_top = layout.cameo_grid_top + layout_spec.cameo_inset_y;
    let row_height = layout_spec.cameo_row_height;
    let visible_rows = layout.side2_tile_count;
    let max_scroll_rows = total_rows.saturating_sub(visible_rows);
    let scroll_rows = scroll_rows.min(max_scroll_rows);

    let visible_items = scroll_rows * CAMEO_COLUMNS;
    let max_visible = visible_rows * CAMEO_COLUMNS;
    let items: Vec<SidebarItem> = all_entries
        .drain(..)
        .skip(visible_items)
        .take(max_visible)
        .enumerate()
        .map(|(idx, entry)| {
            let row = idx / CAMEO_COLUMNS;
            let col = idx % CAMEO_COLUMNS;
            let x = (layout.sidebar_x
                + layout_spec.cameo_inset_x
                + col as f32 * (layout_spec.cameo_width + layout_spec.cameo_gap_x))
                .round();
            let y = (grid_top + row as f32 * row_height).round();
            SidebarItem {
                rect: Rect {
                    x,
                    y,
                    w: layout_spec.cameo_width.round(),
                    h: layout_spec.cameo_height.round(),
                },
                type_id: entry.type_id,
                display_name: entry.display_name,
                cost: entry.cost,
                has_cameo_art: false,
                queue_category: entry.queue_category,
                enabled: entry.enabled,
                progress: entry.progress,
                queued_count: entry.queued_count,
                is_building_this_type: entry.is_building_this_type,
                is_ready: entry.is_ready,
                is_on_hold: entry.is_on_hold,
                is_armed: entry.is_armed,
                is_superweapon: entry.is_superweapon,
                super_weapon_section: entry.super_weapon_section,
            }
        })
        .collect();

    // Control buttons at bottom of sidebar (below side3).
    let btn_w = layout_spec.sidebar_width * 0.45;
    let btn_h = layout_spec.control_button_height;
    let btn_y = layout.side3_y + layout_spec.side3_height + layout_spec.control_block_top_pad;
    let btn_pad = 4.0 * (layout_spec.sidebar_width / 168.0); // scale padding proportionally
    let btn_x1 = layout.sidebar_x + btn_pad;
    let btn_x2 = layout.sidebar_x + layout_spec.sidebar_width - btn_w - btn_pad;

    // These two buttons are app-local controls, not native sidebar gadgets.
    // A native unit-tab cameo retains its own FactoryPtr/category, so the
    // combined presentation tab is not authority for a House factory slot.
    // When exactly one real queue category is present, use it. With multiple
    // independent categories there is no evidenced global selector; omitting
    // the ambiguous controls is safer than emitting a command for Vehicle.
    let active_queue_category = unique_category_for_tab(
        selected_category,
        queue_items.iter().map(|item| item.queue_category),
    );
    let active_queue_paused = active_queue_category.is_some_and(|category| {
        queue_items
            .iter()
            .find(|item| item.queue_category == category)
            .is_some_and(|item| item.state == BuildQueueState::Paused)
    });
    let producer_category = active_queue_category
        .filter(|category| {
            producer_focus
                .iter()
                .any(|focus| focus.category == *category)
        })
        .or_else(|| {
            if queue_items
                .iter()
                .any(|item| category_is_on_tab(selected_category, item.queue_category))
            {
                None
            } else {
                unique_category_for_tab(
                    selected_category,
                    producer_focus.iter().map(|focus| focus.category),
                )
            }
        });

    SidebarView {
        panel_rect,
        layout,
        credits,
        power_produced,
        power_drained,
        credits_frac,
        power_frac,
        low_power,
        scroll_rows,
        max_scroll_rows,
        tabs,
        items,
        repair_button,
        sell_button,
        scroll_down_button,
        scroll_up_button,
        pause_button: active_queue_category.map(|category| SidebarControlButton {
            rect: Rect {
                x: btn_x1,
                y: btn_y,
                w: btn_w,
                h: btn_h,
            },
            action: SidebarAction::TogglePauseQueue(category),
            label: if active_queue_paused {
                "Resume".to_string()
            } else {
                "Pause".to_string()
            },
        }),
        producer_button: producer_category.map(|category| SidebarControlButton {
            rect: Rect {
                x: btn_x2,
                y: btn_y,
                w: btn_w,
                h: btn_h,
            },
            action: SidebarAction::CycleProducer(category),
            label: "Factory".to_string(),
        }),
        cancel_button: SidebarControlButton {
            rect: Rect {
                x: btn_x1,
                y: btn_y + btn_h + layout_spec.control_button_gap,
                w: btn_w,
                h: btn_h,
            },
            action: SidebarAction::CancelLastBuild,
            label: "Cancel".to_string(),
        },
        cycle_owner_button: SidebarControlButton {
            rect: Rect {
                x: btn_x2,
                y: btn_y + btn_h + layout_spec.control_button_gap,
                w: btn_w,
                h: btn_h,
            },
            action: SidebarAction::CycleOwner,
            label: "Owner".to_string(),
        },
        starter_base_button: SidebarControlButton {
            rect: Rect {
                x: btn_x1,
                y: btn_y + (btn_h + layout_spec.control_button_gap) * 2.0,
                w: btn_w,
                h: btn_h,
            },
            action: SidebarAction::PlaceStarterBase,
            label: "Base".to_string(),
        },
        spawn_test_units_button: SidebarControlButton {
            rect: Rect {
                x: btn_x2,
                y: btn_y + (btn_h + layout_spec.control_button_gap) * 2.0,
                w: btn_w,
                h: btn_h,
            },
            action: SidebarAction::SpawnTestUnits,
            label: "Spawn".to_string(),
        },
    }
}

fn category_is_on_tab(
    selected_category: ProductionCategory,
    actual_category: ProductionCategory,
) -> bool {
    actual_category == selected_category
        || (selected_category == ProductionCategory::Vehicle
            && matches!(
                actual_category,
                ProductionCategory::Aircraft | ProductionCategory::Ship
            ))
}

fn unique_category_for_tab(
    selected_category: ProductionCategory,
    categories: impl IntoIterator<Item = ProductionCategory>,
) -> Option<ProductionCategory> {
    let mut unique = None;
    for category in categories {
        if !category_is_on_tab(selected_category, category) {
            continue;
        }
        match unique {
            None => unique = Some(category),
            Some(existing) if existing == category => {}
            Some(_) => return None,
        }
    }
    unique
}

struct BuildEntry {
    type_id: String,
    display_name: String,
    cost: Option<i32>,
    /// Exact House factory/queue slot represented by this cameo. The Vehicle
    /// tab is presentation-only grouping and must not replace Ship/Aircraft.
    queue_category: ProductionCategory,
    enabled: bool,
    progress: f32,
    queued_count: usize,
    /// True when this type is the one actively being produced in its category.
    is_building_this_type: bool,
    is_ready: bool,
    /// Production of this type is suspended (paused queue or out of funds).
    is_on_hold: bool,
    is_armed: bool,
    is_superweapon: bool,
    super_weapon_section: Option<String>,
}

fn collect_build_entries(
    category: ProductionCategory,
    queue_items: &[QueueItemView],
    build_options: &[BuildOption],
    ready_buildings: &[ReadyBuildingView],
    armed: Option<&ArmedSidebarEntry>,
    interner: Option<&crate::sim::intern::StringInterner>,
    sw_views: &[SuperWeaponView],
) -> Vec<BuildEntry> {
    // Building-placement is_armed: matched by interned type_id.
    let armed_building_id: Option<InternedId> = armed
        .and_then(ArmedSidebarEntry::as_building_placement)
        .and_then(|s| interner.and_then(|i| i.get(s)));
    // SW is_armed: matched by section name (string compare).
    let armed_sw_section: Option<&str> = armed.and_then(ArmedSidebarEntry::as_super_weapon);
    let resolve = |id: InternedId| -> String {
        interner.map_or(format!("#{}", id.index()), |i| i.resolve(id).to_string())
    };

    // Superweapon cameos go first on the Defense tab, sorted before regular items.
    let mut sw_entries: Vec<BuildEntry> = Vec::new();
    if category == ProductionCategory::Defense {
        for sw in sw_views {
            // Use sidebar_image (e.g. "INTICON") as the type_id for cameo atlas lookup.
            let type_id = sw
                .sidebar_image
                .as_deref()
                .unwrap_or(&sw.display_name)
                .to_string();
            sw_entries.push(BuildEntry {
                type_id,
                display_name: sw.display_name.clone(),
                cost: None,
                queue_category: ProductionCategory::Defense,
                enabled: sw.is_online,
                progress: sw.progress,
                queued_count: 0,
                is_building_this_type: !sw.is_ready && sw.is_online && sw.progress > 0.0,
                is_ready: sw.is_ready,
                is_on_hold: false,
                is_armed: armed_sw_section
                    .map_or(false, |s| s.eq_ignore_ascii_case(&sw.display_name)),
                is_superweapon: true,
                super_weapon_section: Some(sw.display_name.clone()),
            });
        }
    }

    // Collect build options, merging ready-building state into matching entries
    // so that a completed building shows "READY" on its existing cameo slot
    // instead of spawning a duplicate entry.
    let mut entries: Vec<BuildEntry> = build_options
        .iter()
        .filter(|opt| category_is_on_tab(category, opt.queue_category) && opt.visible_in_sidebar())
        .map(|opt| {
            // Check if this type has a completed building waiting for placement.
            let is_ready = ready_buildings.iter().any(|r| r.type_id == opt.type_id);
            let is_armed = is_ready && armed_building_id == Some(opt.type_id);

            if is_ready {
                // Building is done — show as ready for placement.
                BuildEntry {
                    type_id: resolve(opt.type_id),
                    display_name: opt.display_name.clone(),
                    cost: Some(opt.cost),
                    queue_category: opt.queue_category,
                    enabled: true,
                    progress: 1.0,
                    queued_count: 1,
                    is_building_this_type: false,
                    is_ready: true,
                    is_on_hold: false,
                    is_armed,
                    is_superweapon: false,
                    super_weapon_section: None,
                }
            } else {
                let queued_count = queue_items
                    .iter()
                    .filter(|item| {
                        item.type_id == opt.type_id && item.queue_category == opt.queue_category
                    })
                    .count();
                // Check if this type has an item in Building state (actively producing).
                let is_building_this_type = queue_items.iter().any(|item| {
                    item.type_id == opt.type_id
                        && item.queue_category == opt.queue_category
                        && item.state == crate::sim::production::BuildQueueState::Building
                });
                // Suspended production — the two ways a stock queue stalls:
                // the player paused it, or the house ran out of cash. gamemd
                // shows its `TXT_HOLD` status text for exactly this state.
                let is_on_hold = queue_items.iter().any(|item| {
                    item.type_id == opt.type_id
                        && item.queue_category == opt.queue_category
                        && matches!(
                            item.state,
                            BuildQueueState::Paused | BuildQueueState::NoFunds
                        )
                });
                let progress = queue_items
                    .iter()
                    .find(|item| {
                        item.type_id == opt.type_id && item.queue_category == opt.queue_category
                    })
                    .map(|item| {
                        let total = item.total_ms.max(1) as f32;
                        (total - item.remaining_ms as f32) / total
                    })
                    .unwrap_or(0.0)
                    .clamp(0.0, 1.0);
                BuildEntry {
                    type_id: resolve(opt.type_id),
                    display_name: opt.display_name.clone(),
                    cost: Some(opt.cost),
                    queue_category: opt.queue_category,
                    enabled: opt.enabled,
                    progress,
                    queued_count,
                    is_building_this_type,
                    is_ready: false,
                    is_on_hold,
                    is_armed: false,
                    is_superweapon: false,
                    super_weapon_section: None,
                }
            }
        })
        .collect();

    // Append any ready buildings that don't have a matching build option
    // (edge case: type was removed from buildable list but still in ready queue).
    for r in ready_buildings
        .iter()
        .filter(|r| r.queue_category == category)
    {
        let r_type_str = resolve(r.type_id);
        let already_listed = entries
            .iter()
            .any(|e| e.type_id.eq_ignore_ascii_case(&r_type_str));
        if !already_listed {
            let is_armed = armed_building_id == Some(r.type_id);
            entries.push(BuildEntry {
                type_id: r_type_str,
                display_name: r.display_name.clone(),
                cost: None,
                queue_category: r.queue_category,
                enabled: true,
                progress: 1.0,
                queued_count: 1,
                is_building_this_type: false,
                is_ready: true,
                is_on_hold: false,
                is_armed,
                is_superweapon: false,
                super_weapon_section: None,
            });
        }
    }

    // Prepend superweapon entries before regular defense items.
    if !sw_entries.is_empty() {
        sw_entries.append(&mut entries);
        return sw_entries;
    }

    entries
}

#[cfg(test)]
mod tests {
    use super::super::gadget_flash::SidebarGadgetState;
    use super::super::{SidebarAction, SidebarTab};
    use super::build_sidebar_view;
    use crate::rules::object_type::ObjectCategory;
    use crate::sim::intern::StringInterner;
    use crate::sim::production::{
        BuildDisabledReason, BuildOption, BuildQueueState, ProducerFocusView, ProductionCategory,
        QueueItemView,
    };

    fn approx_eq(a: f32, b: f32) {
        assert!(
            (a - b).abs() <= f32::EPSILON,
            "expected {a} ~= {b}, diff={}",
            (a - b).abs()
        );
    }

    #[test]
    fn tab_buttons_bottom_align_to_cameo_grid_top() {
        let view = build_sidebar_view(
            1280.0,
            960.0,
            SidebarTab::Building,
            0,
            0,
            0,
            Some([28.0, 27.0]),
            &[],
            &[],
            &[],
            None,
            &[],
            0,
            None,
            &SidebarGadgetState::new(),
            None,
            None,
        );

        for tab in &view.tabs {
            approx_eq(tab.rect.y + tab.rect.h, view.layout.cameo_grid_top);
        }
    }

    #[test]
    fn gadget_presentation_is_retained_in_sidebar_view() {
        let mut gadgets = SidebarGadgetState::new();
        gadgets.tab_disabled[1] = true;
        gadgets.repair_mode_on = true;
        gadgets.sell_disabled = true;
        gadgets.scroll_down_pressed = true;
        let view = build_sidebar_view(
            1280.0,
            960.0,
            SidebarTab::Building,
            0,
            0,
            0,
            Some([28.0, 27.0]),
            &[],
            &[],
            &[],
            None,
            &[],
            0,
            None,
            &gadgets,
            None,
            None,
        );

        assert!(view.tabs[1].disabled);
        assert!(view.repair_button.active);
        assert!(view.sell_button.disabled);
        assert_eq!(view.scroll_down_button.frame_index, 1);
        assert_eq!(view.scroll_up_button.frame_index, 0);
    }

    #[test]
    fn control_buttons_stay_inside_panel() {
        let view = build_sidebar_view(
            1280.0,
            960.0,
            SidebarTab::Building,
            1000,
            100,
            150,
            Some([28.0, 27.0]),
            &[],
            &[],
            &[],
            None,
            &[],
            0,
            None,
            &SidebarGadgetState::new(),
            None,
            None,
        );

        for button in [
            Some(&view.cancel_button),
            Some(&view.cycle_owner_button),
            Some(&view.starter_base_button),
            Some(&view.spawn_test_units_button),
            view.pause_button.as_ref(),
            view.producer_button.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            assert!(button.rect.x >= view.panel_rect.x);
            assert!(button.rect.y >= view.panel_rect.y);
            assert!(button.rect.x + button.rect.w <= view.panel_rect.x + view.panel_rect.w);
            assert!(button.rect.y + button.rect.h <= view.panel_rect.y + view.panel_rect.h);
        }
    }

    fn option(
        interner: &mut StringInterner,
        id: &str,
        enabled: bool,
        reason: Option<BuildDisabledReason>,
    ) -> BuildOption {
        BuildOption {
            type_id: interner.intern(id),
            display_name: id.to_string(),
            cost: 600,
            object_category: ObjectCategory::Building,
            queue_category: ProductionCategory::Building,
            enabled,
            reason,
        }
    }

    #[test]
    fn strict_gate_hides_blocked_items_and_greys_credit_shortfalls() {
        let mut interner = StringInterner::new();
        let build_options = vec![
            option(&mut interner, "GACNST", true, None),
            option(
                &mut interner,
                "GAPOWR",
                false,
                Some(BuildDisabledReason::MissingPrerequisite("GACNST".into())),
            ),
            option(
                &mut interner,
                "GAWEAP",
                false,
                Some(BuildDisabledReason::NoFactory),
            ),
            option(
                &mut interner,
                "GAAIRC",
                false,
                Some(BuildDisabledReason::WrongOwner),
            ),
            option(
                &mut interner,
                "GAREFN",
                false,
                Some(BuildDisabledReason::InsufficientCredits),
            ),
            option(
                &mut interner,
                "GADEPT",
                false,
                Some(BuildDisabledReason::AtBuildLimit),
            ),
        ];
        let view = build_sidebar_view(
            1280.0,
            960.0,
            SidebarTab::Building,
            0,
            0,
            0,
            Some([28.0, 27.0]),
            &[],
            &build_options,
            &[],
            None,
            &[],
            0,
            Some(&interner),
            &SidebarGadgetState::new(),
            None,
            None,
        );

        // Missing prereq / no factory / wrong faction are hidden entirely.
        let shown: Vec<&str> = view.items.iter().map(|i| i.type_id.as_str()).collect();
        assert_eq!(shown, ["GACNST", "GAREFN", "GADEPT"]);
        // Buildable item is enabled; credit shortfall and build limit are greyed.
        assert!(view.items[0].enabled);
        assert!(!view.items[1].enabled);
        assert!(!view.items[2].enabled);
    }

    #[test]
    fn strict_gate_empty_when_no_option_visible() {
        // Match start before conyard deploy: every option fails prereqs/factory.
        let mut interner = StringInterner::new();
        let build_options = vec![
            option(
                &mut interner,
                "GAPOWR",
                false,
                Some(BuildDisabledReason::MissingPrerequisite("GACNST".into())),
            ),
            option(
                &mut interner,
                "GAPILE",
                false,
                Some(BuildDisabledReason::NoFactory),
            ),
        ];
        let view = build_sidebar_view(
            1280.0,
            960.0,
            SidebarTab::Building,
            5000,
            0,
            0,
            Some([28.0, 27.0]),
            &[],
            &build_options,
            &[],
            None,
            &[],
            0,
            Some(&interner),
            &SidebarGadgetState::new(),
            None,
            None,
        );
        assert!(view.items.is_empty());
    }

    #[test]
    fn control_buttons_carry_their_actions() {
        // `sidebar::hit_test` was retired in A6 — the control/dev buttons moved
        // onto the gadget list. The driver (`gadget_input::apply_gadget_result`)
        // applies each button's own `SidebarAction`, so the wiring under test is
        // that the view builds those buttons with the right actions.
        let view = build_sidebar_view(
            1280.0,
            960.0,
            SidebarTab::Building,
            1000,
            100,
            150,
            Some([28.0, 27.0]),
            &[],
            &[],
            &[],
            None,
            &[],
            0,
            None,
            &SidebarGadgetState::new(),
            None,
            None,
        );

        assert_eq!(view.cancel_button.action, SidebarAction::CancelLastBuild);
        assert_eq!(view.cycle_owner_button.action, SidebarAction::CycleOwner);
        assert_eq!(
            view.starter_base_button.action,
            SidebarAction::PlaceStarterBase
        );
        assert_eq!(
            view.spawn_test_units_button.action,
            SidebarAction::SpawnTestUnits
        );
    }

    fn queue_item(
        interner: &mut StringInterner,
        id: &str,
        state: crate::sim::production::BuildQueueState,
    ) -> crate::sim::production::QueueItemView {
        crate::sim::production::QueueItemView {
            type_id: interner.intern(id),
            display_name: id.to_string(),
            queue_category: ProductionCategory::Building,
            state,
            remaining_ms: 5_000,
            total_ms: 10_000,
        }
    }

    fn unit_option(
        interner: &mut StringInterner,
        id: &str,
        category: ProductionCategory,
    ) -> BuildOption {
        BuildOption {
            type_id: interner.intern(id),
            display_name: id.to_string(),
            cost: 900,
            object_category: ObjectCategory::Vehicle,
            queue_category: category,
            enabled: true,
            reason: None,
        }
    }

    fn categorized_queue_item(
        interner: &mut StringInterner,
        id: &str,
        category: ProductionCategory,
        state: BuildQueueState,
        remaining_ms: u32,
    ) -> QueueItemView {
        QueueItemView {
            type_id: interner.intern(id),
            display_name: id.to_string(),
            queue_category: category,
            state,
            remaining_ms,
            total_ms: 10_000,
        }
    }

    fn producer(category: ProductionCategory, stable_id: u64) -> ProducerFocusView {
        ProducerFocusView {
            stable_id,
            display_name: format!("{category:?} factory"),
            category,
            rx: stable_id as u16,
            ry: stable_id as u16,
        }
    }

    #[test]
    fn ship_only_unit_tab_retains_ship_queue_state_and_control_actions() {
        let mut interner = StringInterner::new();
        let options = vec![unit_option(&mut interner, "DEST", ProductionCategory::Ship)];
        let queue = vec![categorized_queue_item(
            &mut interner,
            "DEST",
            ProductionCategory::Ship,
            BuildQueueState::Paused,
            5_000,
        )];
        // A land producer may coexist, but the one live Ship queue is the
        // unambiguous context for both app-local controls.
        let focus = vec![
            producer(ProductionCategory::Vehicle, 1),
            producer(ProductionCategory::Ship, 2),
        ];
        let view = build_sidebar_view(
            1280.0,
            960.0,
            SidebarTab::Vehicle,
            5_000,
            0,
            0,
            Some([28.0, 27.0]),
            &queue,
            &options,
            &[],
            None,
            &focus,
            0,
            Some(&interner),
            &SidebarGadgetState::new(),
            None,
            None,
        );

        assert_eq!(view.items.len(), 1);
        let dest = &view.items[0];
        assert_eq!(dest.type_id, "DEST");
        assert_eq!(dest.queue_category, ProductionCategory::Ship);
        assert_eq!(dest.queued_count, 1);
        assert!(dest.is_on_hold);
        approx_eq(dest.progress, 0.5);
        let pause = view.pause_button.as_ref().expect("Ship pause control");
        assert_eq!(pause.label, "Resume");
        assert_eq!(
            pause.action,
            SidebarAction::TogglePauseQueue(ProductionCategory::Ship)
        );
        assert_eq!(
            view.producer_button
                .as_ref()
                .expect("Ship producer control")
                .action,
            SidebarAction::CycleProducer(ProductionCategory::Ship)
        );
    }

    #[test]
    fn mixed_vehicle_and_ship_unit_tab_never_aliases_or_emits_ambiguous_controls() {
        let mut interner = StringInterner::new();
        let options = vec![
            unit_option(&mut interner, "MTNK", ProductionCategory::Vehicle),
            unit_option(&mut interner, "DEST", ProductionCategory::Ship),
        ];
        let queue = vec![
            categorized_queue_item(
                &mut interner,
                "MTNK",
                ProductionCategory::Vehicle,
                BuildQueueState::Building,
                9_000,
            ),
            categorized_queue_item(
                &mut interner,
                "DEST",
                ProductionCategory::Ship,
                BuildQueueState::Done,
                0,
            ),
        ];
        let focus = vec![
            producer(ProductionCategory::Vehicle, 1),
            producer(ProductionCategory::Ship, 2),
        ];
        let view = build_sidebar_view(
            1280.0,
            960.0,
            SidebarTab::Vehicle,
            5_000,
            0,
            0,
            Some([28.0, 27.0]),
            &queue,
            &options,
            &[],
            None,
            &focus,
            0,
            Some(&interner),
            &SidebarGadgetState::new(),
            None,
            None,
        );

        assert_eq!(view.items.len(), 2);
        let mtnk = view
            .items
            .iter()
            .find(|item| item.type_id == "MTNK")
            .unwrap();
        let dest = view
            .items
            .iter()
            .find(|item| item.type_id == "DEST")
            .unwrap();
        assert_eq!(mtnk.queue_category, ProductionCategory::Vehicle);
        assert_eq!(dest.queue_category, ProductionCategory::Ship);
        assert!(mtnk.is_building_this_type);
        assert!(!dest.is_building_this_type);
        approx_eq(mtnk.progress, 0.1);
        approx_eq(dest.progress, 1.0);
        assert_eq!(mtnk.queued_count, 1);
        assert_eq!(dest.queued_count, 1);
        assert!(view.pause_button.is_none());
        assert!(view.producer_button.is_none());
    }

    /// gamemd shows its `TXT_HOLD` status text while production is suspended.
    /// Our two suspended states are a player-paused queue and a house that ran
    /// out of cash; neither an actively-building nor a merely-queued item
    /// carries the flag.
    #[test]
    fn suspended_queue_items_mark_their_cameo_on_hold() {
        use crate::sim::production::BuildQueueState;

        let cases = [
            (BuildQueueState::Paused, true),
            (BuildQueueState::NoFunds, true),
            (BuildQueueState::Building, false),
            (BuildQueueState::Queued, false),
        ];
        for (state, expected) in cases {
            let mut interner = StringInterner::new();
            let build_options = vec![option(&mut interner, "GAPOWR", true, None)];
            let queue = vec![queue_item(&mut interner, "GAPOWR", state)];
            let view = build_sidebar_view(
                1280.0,
                960.0,
                SidebarTab::Building,
                5000,
                0,
                0,
                Some([28.0, 27.0]),
                &queue,
                &build_options,
                &[],
                None,
                &[],
                0,
                Some(&interner),
                &SidebarGadgetState::new(),
                None,
                None,
            );
            let item = view
                .items
                .iter()
                .find(|i| i.type_id.eq_ignore_ascii_case("GAPOWR"))
                .expect("GAPOWR cameo");
            assert_eq!(
                item.is_on_hold, expected,
                "state {state:?} should map is_on_hold = {expected}"
            );
        }
    }
}
