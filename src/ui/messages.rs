//! Chat/system message list model (study §3.1 / contract §4): a bounded list
//! of text-label rows over the tactical viewport. Pure: text measurement is
//! injected (`measure: &dyn Fn(&str) -> i32`, pixels) so the model never
//! touches the renderer; deadlines are injected `now_ms` values. The app
//! driver owns the clock and MUST feed pause-adjusted time (`PauseAwareClock`
//! below): the native composite timer FREEZES during pause (contract §4.2
//! step 8 / §4.3), so a row's remaining lifetime survives a pause intact.
//!
//! ## Dependency rules
//! - ui/ module: std only — no render/, assets/, sidebar/, audio/, net/.

/// Slot-pool cap (native list walks up to 0xE label slots).
pub const MESSAGE_SLOTS: usize = 14;
/// Per-slot text cap (162 wide chars incl. the terminator).
pub const MESSAGE_SLOT_CHARS: usize = 161;
/// Per-line char cap handed to the fitter (native cap 0x6F — contract §4.2 step 4).
pub const MESSAGE_LINE_FIT_CHARS: usize = 111;
/// Pixel pad subtracted from the width budget (native `budget = MaxWidth −
/// prefix_width − 8`).
pub const MESSAGE_WIDTH_PAD_PX: i32 = 8;
/// Row stride (native Init hardcodes 0x13 = 19 px — NOT the font height).
pub const MESSAGE_LINE_HEIGHT_PX: i32 = 19;
/// Prefix separator (native wide ":" literal).
pub const MESSAGE_PREFIX_SEPARATOR: &str = ":";
/// Retail visible-message cap (Init maxMsg=6; clamped ≤ MESSAGE_SLOTS).
pub const MESSAGE_MAX_VISIBLE_RETAIL: usize = 6;

/// One live message row.
#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    pub text: String,
    pub rgb: [f32; 3],
    /// None = never expires (native timeout −1 → deadline 0).
    pub deadline_ms: Option<u64>,
    /// Screen Y, restacked after every insert/expiry.
    pub y: i32,
}

/// Outcome of an `add_message` call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AddOutcome {
    /// Number of rows added (≥1 when the text wrapped).
    pub added: usize,
    /// True when the caller should play the insert sound (top-level
    /// non-silent adds only; wrapped continuation rows are always silent).
    pub play_sound: bool,
}

#[derive(Debug)]
pub struct MessageList {
    x: i32,
    y: i32,
    max_visible: usize,
    max_width_px: i32,
    messages: Vec<Message>,
}

impl MessageList {
    pub fn new(x: i32, y: i32, max_visible: usize, max_width_px: i32) -> Self {
        Self {
            x,
            y,
            max_visible: max_visible.min(MESSAGE_SLOTS),
            max_width_px,
            messages: Vec::with_capacity(MESSAGE_SLOTS),
        }
    }

    /// Re-anchor on viewport/layout change (the native viewport re-init,
    /// contract §4.1): x = tactical_x + 3, y = tactical_y, width = tactical_w − 14.
    pub fn set_view(&mut self, x: i32, y: i32, max_width_px: i32) {
        self.x = x;
        self.y = y;
        self.max_width_px = max_width_px;
        self.restack();
    }

    pub fn x(&self) -> i32 {
        self.x
    }

    /// Rows in walk order = insertion order = top-to-bottom (G20 analogue on
    /// the list's own walk).
    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    /// Contract §4.2: compose prefix+":"+fitted text, evict the head when
    /// full, tail-insert, restack, recurse silently on the remainder.
    pub fn add_message(
        &mut self,
        prefix: Option<&str>,
        text: &str,
        rgb: [f32; 3],
        timeout_ms: Option<u64>,
        silent: bool,
        now_ms: u64,
        measure: &dyn Fn(&str) -> i32,
    ) -> AddOutcome {
        let mut outcome = AddOutcome {
            added: 0,
            play_sound: false,
        };
        self.add_inner(prefix, text, rgb, timeout_ms, silent, now_ms, measure, &mut outcome);
        outcome
    }

    #[allow(clippy::too_many_arguments)]
    fn add_inner(
        &mut self,
        prefix: Option<&str>,
        text: &str,
        rgb: [f32; 3],
        timeout_ms: Option<u64>,
        silent: bool,
        now_ms: u64,
        measure: &dyn Fn(&str) -> i32,
        outcome: &mut AddOutcome,
    ) {
        if text.is_empty() {
            return;
        }
        // Step 2 — compose the prefix.
        let compose_prefix = match prefix {
            Some(p) => format!("{p}{MESSAGE_PREFIX_SEPARATOR}"),
            None => String::new(),
        };
        // Step 3 — pixel budget for the text part.
        let budget = self.max_width_px - measure(&compose_prefix) - MESSAGE_WIDTH_PAD_PX;
        if budget <= 0 {
            return;
        }
        // Step 4 — fit (char cap + px budget, word break).
        let fit_bytes = fit_prefix_bytes(text, budget, MESSAGE_LINE_FIT_CHARS, measure);
        if fit_bytes == 0 {
            return;
        }
        let mut line = compose_prefix;
        line.push_str(&text[..fit_bytes]);
        let line: String = line.chars().take(MESSAGE_SLOT_CHARS).collect();
        // Step 5 — evict the oldest while at the visible cap.
        while self.messages.len() >= self.max_visible {
            self.messages.remove(0);
        }
        // Slot-pool guard (unreachable with retail max_visible ≤ 14).
        if self.messages.len() >= MESSAGE_SLOTS {
            return;
        }
        // Steps 6-12 — tail insert + restack.
        self.messages.push(Message {
            text: line,
            rgb,
            deadline_ms: timeout_ms.map(|t| now_ms + t),
            y: 0,
        });
        self.restack();
        outcome.added += 1;
        if !silent {
            outcome.play_sound = true;
        }
        // Step 13 — wrap recursion: skip control chars after the break, then
        // re-add the remainder with the SAME prefix, always silent.
        if fit_bytes < text.len() {
            let remainder = text[fit_bytes..].trim_start_matches(|c: char| (c as u32) < 0x20);
            if !remainder.is_empty() {
                self.add_inner(prefix, remainder, rgb, timeout_ms, true, now_ms, measure, outcome);
            }
        }
    }

    /// Contract §4.3 — expiry: remove rows whose deadline passed strictly
    /// (`now > deadline`), then restack. Returns true when anything expired.
    /// `now_ms` must come from the pause-adjusted clock (`PauseAwareClock`):
    /// the native composite timer freezes during pause, so deadlines resume
    /// with their remaining lifetime intact. The driver additionally skips
    /// this call entirely while paused.
    pub fn manage(&mut self, now_ms: u64) -> bool {
        let before = self.messages.len();
        self.messages
            .retain(|m| m.deadline_ms.is_none_or(|d| now_ms <= d));
        let expired = self.messages.len() != before;
        if expired {
            self.restack();
        }
        expired
    }

    fn restack(&mut self) {
        let base = self.y;
        for (i, m) in self.messages.iter_mut().enumerate() {
            m.y = base + i as i32 * MESSAGE_LINE_HEIGHT_PX;
        }
    }
}

/// Greedy px-budget fitter with word break (contract lane §4.2 step-4 fitter
/// shape, doc fidelity — exact boundary semantics are a plan deferred item):
/// fits chars while BOTH the char cap and the pixel budget hold; on overflow
/// backs up to just-after the last space when one exists. Returns the fitted
/// BYTE length.
fn fit_prefix_bytes(
    text: &str,
    budget_px: i32,
    max_chars: usize,
    measure: &dyn Fn(&str) -> i32,
) -> usize {
    let mut fitted = 0usize;
    let mut chars = 0usize;
    let mut last_break: Option<usize> = None;
    for (idx, ch) in text.char_indices() {
        let end = idx + ch.len_utf8();
        if chars + 1 > max_chars || measure(&text[..end]) > budget_px {
            return last_break.unwrap_or(idx);
        }
        if ch == ' ' {
            last_break = Some(end);
        }
        fitted = end;
        chars += 1;
    }
    fitted
}

/// Pause-aware message clock (contract §4.2 step 8 / §4.3): the native
/// composite timer FREEZES during pause, so message `now` must stop too —
/// otherwise a deadline that elapses on the wall clock during a pause would
/// expire its row the instant the game unpauses, instead of resuming the
/// remaining lifetime. Pure: wall-clock ms are injected; the app driver feeds
/// pause edges via `set_paused`.
#[derive(Debug, Default)]
pub struct PauseAwareClock {
    /// Sum of all completed pause spans, in wall ms.
    paused_total_ms: u64,
    /// Wall timestamp when the current pause began (None = not paused).
    pause_started_wall_ms: Option<u64>,
}

impl PauseAwareClock {
    /// Feed the live pause flag once per frame; edges are detected here.
    pub fn set_paused(&mut self, paused: bool, wall_ms: u64) {
        match (paused, self.pause_started_wall_ms) {
            (true, None) => self.pause_started_wall_ms = Some(wall_ms),
            (false, Some(started)) => {
                self.paused_total_ms += wall_ms.saturating_sub(started);
                self.pause_started_wall_ms = None;
            }
            _ => {}
        }
    }

    /// Pause-adjusted now: wall ms minus every paused span. Constant (frozen)
    /// for the duration of a pause.
    pub fn now(&self, wall_ms: u64) -> u64 {
        let in_pause = self
            .pause_started_wall_ms
            .map(|s| wall_ms.saturating_sub(s))
            .unwrap_or(0);
        wall_ms.saturating_sub(self.paused_total_ms + in_pause)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 10 px per char monospace fake measure.
    fn mono(s: &str) -> i32 {
        s.chars().count() as i32 * 10
    }

    const WHITE: [f32; 3] = [1.0, 1.0, 1.0];

    fn list(max_visible: usize, width: i32) -> MessageList {
        MessageList::new(3, 0, max_visible, width)
    }

    #[test]
    fn add_composes_prefix_and_restacks_19px() {
        let mut l = list(6, 500);
        let o = l.add_message(Some("Boris"), "attack", WHITE, None, false, 0, &mono);
        assert_eq!(o.added, 1);
        assert!(o.play_sound);
        let o = l.add_message(None, "second", WHITE, None, true, 0, &mono);
        assert!(!o.play_sound, "silent add suppresses the sound");
        let rows = l.messages();
        assert_eq!(rows[0].text, "Boris:attack");
        assert_eq!(rows[1].text, "second");
        assert_eq!(rows[0].y, 0);
        assert_eq!(rows[1].y, MESSAGE_LINE_HEIGHT_PX, "19 px stride");
    }

    #[test]
    fn eviction_drops_oldest_at_cap() {
        let mut l = list(3, 500);
        for i in 0..4 {
            l.add_message(None, &format!("m{i}"), WHITE, None, true, 0, &mono);
        }
        let texts: Vec<&str> = l.messages().iter().map(|m| m.text.as_str()).collect();
        assert_eq!(texts, vec!["m1", "m2", "m3"], "head (oldest) evicted");
        assert_eq!(l.messages()[0].y, 0, "restacked from the top");
    }

    #[test]
    fn wrap_recursion_reincludes_prefix_and_is_silent() {
        // Budget: width 200 − prefix "P:" 20 − pad 8 = 172 px → 17 chars/line.
        let mut l = list(6, 200);
        let text = "aaaa bbbb cccc dddd eeee"; // 24 chars → must wrap
        let o = l.add_message(Some("P"), text, WHITE, None, false, 0, &mono);
        assert!(o.added >= 2, "wrapped into multiple rows");
        assert!(o.play_sound, "ONE sound for the top-level add");
        for row in l.messages() {
            assert!(row.text.starts_with("P:"), "prefix re-included on wraps: {}", row.text);
            assert!(mono(&row.text) <= 200 - MESSAGE_WIDTH_PAD_PX + 20);
        }
        // All input chars survive across the rows (minus break spaces).
        let joined: String = l
            .messages()
            .iter()
            .map(|m| m.text.trim_start_matches("P:"))
            .collect::<Vec<_>>()
            .join(" ");
        let normalize = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
        assert_eq!(normalize(&joined), normalize(text));
    }

    #[test]
    fn word_break_backs_up_to_last_space() {
        // budget 100 px = 10 chars; "hello worlds" breaks after "hello ".
        let fit = fit_prefix_bytes("hello worlds", 100, 111, &mono);
        assert_eq!(&"hello worlds"[..fit], "hello ");
    }

    #[test]
    fn zero_budget_or_unfittable_adds_nothing() {
        let mut l = list(6, 5); // budget 5−0−8 < 0
        let o = l.add_message(None, "text", WHITE, None, false, 0, &mono);
        assert_eq!(o.added, 0);
        assert!(!o.play_sound);
        // First char wider than the budget → fit 0 → nothing.
        let mut l2 = list(6, 17); // budget 17−0−8 = 9 < 10
        let o = l2.add_message(None, "x", WHITE, None, false, 0, &mono);
        assert_eq!(o.added, 0);
    }

    #[test]
    fn manage_expires_strictly_after_deadline() {
        let mut l = list(6, 500);
        l.add_message(None, "temp", WHITE, Some(1000), true, 0, &mono);
        l.add_message(None, "forever", WHITE, None, true, 0, &mono);
        assert!(!l.manage(1000), "now == deadline: kept (strict >)");
        assert!(l.manage(1001), "now > deadline: expired");
        let texts: Vec<&str> = l.messages().iter().map(|m| m.text.as_str()).collect();
        assert_eq!(texts, vec!["forever"], "0/None = never expires");
        assert_eq!(l.messages()[0].y, 0, "restacked after expiry");
    }

    #[test]
    fn pause_freezes_deadline_arithmetic() {
        // Contract §4.2 step 8 / §4.3: the native composite timer FREEZES
        // during pause. Post(timeout 4000) at t=0, pause wall 1000..11000:
        // the row must survive until pause-ADJUSTED now exceeds 4000 (i.e.
        // wall > 14000), NOT expire on unpause.
        let mut l = list(6, 500);
        let mut clock = PauseAwareClock::default();
        l.add_message(None, "temp", WHITE, Some(4000), true, clock.now(0), &mono);
        clock.set_paused(true, 1000);
        assert_eq!(clock.now(5000), 1000, "clock frozen mid-pause");
        clock.set_paused(false, 11000);
        assert_eq!(clock.now(11000), 1000, "10 s pause span subtracted");
        assert!(!l.manage(clock.now(12000)), "wall 12000 → adjusted 2000: kept");
        assert!(!l.manage(clock.now(14000)), "adjusted 4000 == deadline: kept (strict >)");
        assert!(l.manage(clock.now(14001)), "adjusted 4001 > deadline: expired");
    }

    #[test]
    fn set_view_reanchors_rows() {
        let mut l = list(6, 500);
        l.add_message(None, "a", WHITE, None, true, 0, &mono);
        l.add_message(None, "b", WHITE, None, true, 0, &mono);
        l.set_view(10, 100, 400);
        assert_eq!(l.messages()[0].y, 100);
        assert_eq!(l.messages()[1].y, 100 + MESSAGE_LINE_HEIGHT_PX);
        assert_eq!(l.x(), 10);
    }

    #[test]
    fn line_char_cap_111_applies() {
        let mut l = list(14, 100_000);
        let long = "y".repeat(300);
        let o = l.add_message(None, &long, WHITE, None, true, 0, &mono);
        assert!(o.added >= 2);
        assert_eq!(
            l.messages()[0].text.chars().count(),
            MESSAGE_LINE_FIT_CHARS,
            "first row capped at 111 chars"
        );
    }
}
