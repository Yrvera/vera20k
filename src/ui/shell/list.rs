//! Shared native list and scrollbar geometry from 0x00618D40 / 0x0061C690.
//! Native list rows use GAME.FNT height 17 + 2; inner rectangles exclude borders.

use super::geom::RectPx;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListScrollPart {
    Up,
    Down,
    Thumb,
    Track,
}

pub const ROW_HEIGHT: i32 = 19;

#[derive(Debug, Clone, Copy)]
pub struct ShellListGeometry {
    pub outer: RectPx,
    pub content: RectPx,
    pub scrollbar: Option<RectPx>,
    pub thumb: Option<RectPx>,
    pub visible_rows: usize,
    pub max_top: usize,
}
impl ShellListGeometry {
    pub fn new(outer: RectPx, count: usize, top: usize) -> Self {
        let inner_height = (outer.h - 2).max(0);
        let visible_rows = (inner_height / ROW_HEIGHT) as usize;
        let max_top = count.saturating_sub(visible_rows);
        let scrollbar =
            (max_top > 0).then(|| RectPx::new(outer.x + outer.w - 21, outer.y, 20, outer.h));
        let content = RectPx::new(
            outer.x + 1,
            outer.y + 1,
            (outer.w - 2 - if scrollbar.is_some() { 20 } else { 0 }).max(0),
            inner_height,
        );
        let thumb = scrollbar.map(|bar| {
            let height = thumb_height(bar.h, max_top);
            let span = (bar.h - 2 - 44 - height).max(1);
            let offset = (span as i64 * top.min(max_top) as i64 / max_top as i64) as i32;
            RectPx::new(bar.x + 1, bar.y + 1 + 22 + offset, 18, height)
        });
        Self {
            outer,
            content,
            scrollbar,
            thumb,
            visible_rows,
            max_top,
        }
    }
    pub fn row(&self, visible: usize) -> RectPx {
        RectPx::new(
            self.content.x,
            self.content.y + visible as i32 * ROW_HEIGHT,
            self.content.w,
            ROW_HEIGHT
                .min(self.content.h - visible as i32 * ROW_HEIGHT)
                .max(0),
        )
    }
    pub fn row_at(&self, count: usize, top: usize, x: i32, y: i32) -> Option<usize> {
        if !self.content.contains(x, y) {
            return None;
        }
        let index = top + ((y - self.content.y) / ROW_HEIGHT) as usize;
        (index < count).then_some(index)
    }
    pub fn scroll_part_at(&self, x: i32, y: i32) -> Option<ListScrollPart> {
        let bar = self.scrollbar?;
        if !bar.contains(x, y) {
            return None;
        }
        let local_y = y - bar.y;
        if x <= bar.x {
            return None;
        }
        if local_y < 22 {
            return Some(ListScrollPart::Up);
        }
        if local_y > bar.h - 2 - 22 {
            return Some(ListScrollPart::Down);
        }
        let thumb = self.thumb?;
        let native_top = thumb.y - bar.y - 1;
        if local_y >= native_top && local_y < native_top + thumb.h {
            Some(ListScrollPart::Thumb)
        } else {
            Some(ListScrollPart::Track)
        }
    }
    pub fn top_at_pointer(&self, y: i32) -> usize {
        let (Some(bar), Some(thumb)) = (self.scrollbar, self.thumb) else {
            return 0;
        };
        let span = (bar.h - 2 - 44 - thumb.h).max(1);
        let numerator = (y - bar.y - 22 - thumb.h / 2).clamp(0, span);
        (numerator as i64 * self.max_top as i64 / span as i64) as usize
    }
}

/// 0x0061C818 uses natural log with the stored binary64 0.2; 0x007C5F00
/// truncates the result to integer, then the caller enforces the 14 px minimum.
/// f64 arithmetic preserves ordinary control geometry; x87 rounding-edge cases
/// remain outside the sampled native comparison coverage.
pub fn thumb_height(height: i32, range: usize) -> i32 {
    let track = f64::from(height - 2 - 44);
    (track - ((range + 1) as f64).ln() * track * 0.2)
        .trunc()
        .max(14.0) as i32
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn thumb_height_matches_native_sampled_geometry() {
        let vectors: serde_json::Value = serde_json::from_str(include_str!(
            "../../../tools/storage_oracle/saved_scrollbar.json"
        ))
        .unwrap();
        let cases = vectors["cases"].as_array().unwrap();
        assert_eq!(cases.len(), 450);
        for case in cases {
            assert_eq!(
                thumb_height(
                    case["height"].as_i64().unwrap() as i32,
                    case["range"].as_u64().unwrap() as usize
                ),
                case["thumb"].as_i64().unwrap() as i32
            );
        }
    }
    #[test]
    fn border_and_scroll_range_share_one_geometry_for_hit_and_paint() {
        let list = ShellListGeometry::new(RectPx::new(10, 20, 400, 255), 23, 10);
        assert_eq!(list.visible_rows, 13);
        assert_eq!(list.max_top, 10);
        assert_eq!(list.scrollbar, Some(RectPx::new(389, 20, 20, 255)));
        assert_eq!(list.row_at(23, 10, 13, 22), Some(10));
        assert_eq!(list.top_at_pointer(2000), 10);
        assert_eq!(list.top_at_pointer(-1), 0);
    }
}
