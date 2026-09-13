//! Active saved-game children B7/2B4/2B5, owned by original 0x00558DD0.
//! Resource bytes and original 60B7A0 comparisons are preserved in
//! tools/storage_oracle/saved_game_layout.{py,json}. See the pause-save-shells
//! evidence dated 2026-09-12 for callers, text keys and active paint policy.

use super::geom::{self, RectPx};
use super::in_game_shell::InGameShellLayout;
use crate::ui::skirmish_shell::{SavedSeedLayout, SavedSeedMode};

#[derive(Clone, Copy)]
struct Resource {
    list: RectPx,
    prompt: RectPx,
}

fn resource(mode: SavedSeedMode) -> Resource {
    match mode {
        SavedSeedMode::Load => Resource {
            list: RectPx::new(79, 78, 266, 187),
            prompt: RectPx::new(79, 44, 266, 24),
        },
        SavedSeedMode::Save => Resource {
            list: RectPx::new(79, 80, 266, 157),
            prompt: RectPx::new(79, 46, 266, 24),
        },
        SavedSeedMode::Delete => Resource {
            list: RectPx::new(78, 76, 266, 187),
            prompt: RectPx::new(78, 42, 266, 24),
        },
    }
}

const EDIT_DLU: RectPx = RectPx::new(80, 253, 266, 14);
const ACTION_DLU: RectPx = RectPx::new(425, 122, 108, 22);

/// Original saved-game vtable getters 55A050/55A070/55A090. The common
/// SavedSeedMode action/prompt labels also apply to these retail resources.
pub const fn saved_game_title_label(mode: SavedSeedMode) -> (&'static str, &'static str) {
    match mode {
        SavedSeedMode::Load => ("GUI:LoadMissionMenu", "Load Mission"),
        SavedSeedMode::Save => ("GUI:SaveMissionMenu", "Save Mission"),
        SavedSeedMode::Delete => ("GUI:DeleteMissionMenu", "Delete Mission"),
    }
}

fn ordinary_rect(dlu: RectPx, width: i32, height: i32) -> RectPx {
    let raw = geom::dlu_rect(dlu.x, dlu.y, dlu.w, dlu.h);
    // Original60B7A0 clamps final coordinates, not the signed half-delta.
    RectPx::new(
        (raw.x + (width - 800) / 2).max(0),
        (raw.y + (height - 600) / 2).max(0),
        raw.w,
        raw.h,
    )
}

/// Physical active-session layout. Static40C remains visible: 558F8A hides
/// it only for the inactive pre-match browser. Caller supplies loaded SHP sizes.
pub fn saved_game_layout(
    mode: SavedSeedMode,
    width: i32,
    height: i32,
    shell: InGameShellLayout,
    button_size: [i32; 2],
) -> SavedSeedLayout {
    let controls = resource(mode);
    let screen = RectPx::new(0, 0, width, height);
    let action = ACTION_DLU;
    SavedSeedLayout {
        screen,
        dialog: screen,
        // 608CD0 routes694 through active60B1D0; no launcher +7 adjustment.
        title: RectPx::new(width - 165, 2, 162, 16),
        prompt: ordinary_rect(controls.prompt, width, height),
        list: ordinary_rect(controls.list, width, height),
        name_edit: (mode == SavedSeedMode::Save).then(|| ordinary_rect(EDIT_DLU, width, height)),
        action: shell.button_rect(
            geom::dlu_rect(action.x, action.y, action.w, action.h),
            button_size,
        ),
        // 609730 ->60B350: bottom button follows the SIDE3 upper edge.
        back: RectPx::new(
            width - 147,
            shell.side3.y - button_size[1],
            button_size[0],
            button_size[1],
        ),
        // Active60B550 uses parent left+10 and bottom-existingheight-1.
        blank: RectPx::new(10, height - 21, 455, 20),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(values: &serde_json::Value) -> RectPx {
        let n = |i: usize| values[i].as_i64().unwrap() as i32;
        RectPx::new(n(0), n(1), n(2), n(3))
    }

    fn mode(id: u64) -> SavedSeedMode {
        match id {
            0xb7 => SavedSeedMode::Load,
            0x2b4 => SavedSeedMode::Save,
            0x2b5 => SavedSeedMode::Delete,
            _ => panic!("unexpected native resource"),
        }
    }

    #[test]
    fn ordinary_controls_match_original_resource_bytes_and_native_placement() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../tools/storage_oracle/saved_game_layout.json"
        ))
        .unwrap();
        for case in fixture["cases"].as_array().unwrap() {
            let mode = mode(case["dialog_id"].as_u64().unwrap());
            let control = case["control_id"].as_u64().unwrap();
            let source = match control {
                0x40c => resource(mode).prompt,
                0x525 | 0x527 | 0x528 => resource(mode).list,
                0x526 => EDIT_DLU,
                _ => panic!("unexpected ordinary control"),
            };
            assert_eq!(source, rect(&case["dlu_rect"]));
            assert_eq!(
                ordinary_rect(
                    source,
                    case["width"].as_i64().unwrap() as i32,
                    case["height"].as_i64().unwrap() as i32,
                ),
                rect(&case["rect"]),
                "dialog {mode:?}, control {control:x}"
            );
        }
    }
}
