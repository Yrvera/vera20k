//! Command-bar gadgets reuse the ordinary retained mouse driver and actions.

use super::*;

pub(super) fn sync(state: &mut AppState) {
    let layout = crate::app::presentation::sidebar_build::command_bar::layout_and_slots(state);
    let closed = state
        .match_state
        .match_presentation
        .sidebar_gadget_state
        .command_bar_closed;
    let gadgets = &mut state.match_state.match_presentation.in_game_gadgets;
    sync_prepared(gadgets, layout.as_ref(), closed);
}

fn sync_prepared(
    gadgets: &mut InGameGadgets,
    layout: Option<&(
        crate::sidebar::command_bar::CommandBarLayout,
        Vec<Option<usize>>,
    )>,
    closed: bool,
) {
    let handles =
        *gadgets.command_bar.get_or_insert_with(|| {
            std::array::from_fn(|i| {
                let id = if i == 11 { 0xF0 } else { 0xD6 + i as u16 };
                let kind = if i == 9 {
                    ToggleKind::Flip
                } else {
                    ToggleKind::Plain
                };
                gadgets.list.add_tail(
                    GadgetSpec::button(GadgetRect::new(0, 0, 0, 0), id, kind)
                        .with_flags(if i < 3 { 0x55 } else { 5 }),
                )
            })
        });
    for (command, handle) in handles.into_iter().enumerate() {
        let rect = layout.and_then(|(layout, slots)| {
            if command == 11 {
                Some(layout.left_cap)
            } else {
                slots
                    .iter()
                    .position(|id| *id == Some(command))
                    .and_then(|slot| layout.slot(slot))
            }
        });
        if let Some(gadget) = gadgets.list.get_mut(handle) {
            gadget.is_disabled = rect.is_none();
            gadget.rect = rect.map(rect_px).unwrap_or(GadgetRect::new(0, 0, 0, 0));
            if command == 11 {
                gadget.id = if closed { 0xF1 } else { 0xF0 };
            }
        }
    }
}

pub(super) fn apply(state: &mut AppState, result: u16) -> bool {
    let id = result & !(RESULT_BUTTON | RESULT_RIGHT);
    if id == 0xF0 || id == 0xF1 {
        state
            .match_state
            .match_presentation
            .sidebar_gadget_state
            .command_bar_closed = id == 0xF0;
        return true;
    }
    if !(0xD6..0xE1).contains(&id) {
        return false;
    }
    // Original6D0660 routes these IDs by command identity. Existing action
    // owners retain gameplay authority; unsupported Planning/Beacon/Cheer
    // mechanics remain the same explicit residual as their keyboard commands.
    crate::app::input::dispatch::dispatch_command_bar(
        state,
        (id - 0xD6) as usize,
        result & RESULT_RIGHT != 0,
    );
    true
}

pub(super) fn publish(state: &mut AppState) {
    let gadgets = &state.match_state.match_presentation.in_game_gadgets;
    let Some(handles) = gadgets.command_bar else {
        return;
    };
    let pressed = std::array::from_fn::<_, 12, _>(|i| {
        gadgets.list.get(handles[i]).is_some_and(|g| {
        matches!(g.behavior,GadgetBehavior::Button(b) if if i == 9 { b.is_on } else { b.is_pressed })
    })
    });
    let out = &mut state.match_state.match_presentation.sidebar_gadget_state;
    out.command_pressed.copy_from_slice(&pressed[..11]);
    out.command_thumb_pressed = pressed[11];
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sidebar::command_bar::{CommandBarLayout, parse_button_list};

    #[test]
    fn command_bar_open_closed_and_pressed_use_the_real_gadget_driver() {
        let layout = |open| {
            CommandBarLayout::new([800, 600], 168, [28, 32], [52, 32], [28, 32], open).unwrap()
        };
        let slots = parse_button_list("Team01,Team02,TypeSelect,Deploy,Guard,PlanningMode");
        let mut gadgets = InGameGadgets::new();
        sync_prepared(&mut gadgets, Some(&(layout(true), slots.clone())), false);
        let handles = gadgets.command_bar.unwrap();
        assert_eq!(
            gadgets.list.get(handles[0]).unwrap().rect,
            GadgetRect::new(32, 568, 52, 32)
        );
        assert!(gadgets.list.get(handles[2]).unwrap().is_disabled);
        let input = |key, x, left| GadgetInput {
            queued_key: key,
            event_x: x,
            event_y: 570,
            mouse_x: x,
            mouse_y: 570,
            left_held: left,
            ..Default::default()
        };
        tick(
            &mut gadgets.list,
            &mut gadgets.focus,
            &GadgetInput::default(),
            &mut gadgets.out,
        );
        tick(
            &mut gadgets.list,
            &mut gadgets.focus,
            &input(KEY_LMB_DOWN, 40, true),
            &mut gadgets.out,
        );
        assert!(
            matches!(gadgets.list.get(handles[0]).unwrap().behavior,GadgetBehavior::Button(b) if b.is_pressed)
        );
        assert_eq!(
            tick(
                &mut gadgets.list,
                &mut gadgets.focus,
                &input(KEY_LMB_UP, 40, false),
                &mut gadgets.out
            ),
            0x80D6
        );
        assert!(
            matches!(gadgets.list.get(handles[0]).unwrap().behavior,GadgetBehavior::Button(b) if !b.is_pressed)
        );
        tick(
            &mut gadgets.list,
            &mut gadgets.focus,
            &input(KEY_LMB_DOWN, 8, true),
            &mut gadgets.out,
        );
        assert_eq!(
            tick(
                &mut gadgets.list,
                &mut gadgets.focus,
                &input(KEY_LMB_UP, 8, false),
                &mut gadgets.out
            ),
            0x80F0
        );
        sync_prepared(&mut gadgets, Some(&(layout(false), slots)), true);
        assert!(
            handles[..11]
                .iter()
                .all(|h| gadgets.list.get(*h).unwrap().is_disabled)
        );
        assert_eq!(
            gadgets.list.get(handles[11]).unwrap().rect,
            GadgetRect::new(576, 568, 28, 32)
        );
        tick(
            &mut gadgets.list,
            &mut gadgets.focus,
            &input(KEY_LMB_DOWN, 580, true),
            &mut gadgets.out,
        );
        assert_eq!(
            tick(
                &mut gadgets.list,
                &mut gadgets.focus,
                &input(KEY_LMB_UP, 580, false),
                &mut gadgets.out
            ),
            0x80F1
        );
    }
}
