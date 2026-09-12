//! Retained Drive/Ship track progress and one resumable Process_Track call.
//!
//! Native Drive4B150D/4B1596/4B1F48 and Ship6A0BD5/6A0C52/6A158B
//! pay the current cursor, run owner callbacks, then increment the retained
//! cursor. The call-local budget must survive those callbacks independently
//! of the residual stored on the locomotor. Executable evidence:
//! tools/spatial_oracle/locomotor_track_cursor.{py,json,meta.json}.

use super::drive_track::{self, TrackPoint, TurnTrack};
use crate::sim::components::{DriveCoord, TrackProgress};
use crate::util::native_x87::{NativeF32Bits, NativeF64Bits, X87Chop53};

const POINT_COST: i32 = 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TrackFamily {
    Drive,
    Ship,
}

impl TrackFamily {
    fn turn(self, index: i32) -> Option<&'static TurnTrack> {
        let count = match self {
            Self::Drive => 72,
            Self::Ship => 64,
        };
        (0..count)
            .contains(&index)
            .then(|| drive_track::turn_track_at(index as usize))
            .flatten()
    }
}

impl TrackProgress {
    /// Selector publication precedes fresh acceptance's later cursor-zero
    /// write (Drive4B4016..4034 then4B4659; Ship6A3642..3660 then6A3C88).
    pub(crate) fn select_fresh(&mut self, family: TrackFamily, first: u8, second: u8) -> bool {
        if first >= 8 || second >= 8 {
            return false;
        }
        let mut index = i32::from(first) * 8 + i32::from(second);
        if family.turn(index).is_none_or(|turn| turn.normal_track == 0) {
            index = i32::from(first) * 9;
        }
        self.turn_index = index;
        self.reversed = false;
        true
    }

    pub(crate) fn accept_fresh(&mut self) {
        self.cursor = 0;
    }

    /// Force_Track4B0C53/4B0C56 writes these BEFORE its owner guards,
    /// including caller selector -1. It preserves short selection/residual.
    pub(crate) fn select_forced(&mut self, turn_index: i32) {
        self.turn_index = turn_index;
        self.cursor = 0;
    }

    /// Candidate selection is observable during the following owner callback.
    /// Only the surviving common tail advances entry-1 to entry.
    pub(crate) fn accept_chain(&mut self, family: TrackFamily, turn_index: i32) -> bool {
        let Some(turn) = family.turn(turn_index) else {
            return false;
        };
        // Drive4B1B83/4B1B85/4B1B87 rejects the null raw selector before inspecting
        // entry metadata; RawTrack[0] is an inert record, not a usable curve.
        if turn.normal_track == 0 {
            return false;
        }
        let Some(raw) = drive_track::raw_track_meta(turn.normal_track) else {
            return false;
        };
        if raw.entry_index == 0 {
            return false;
        }
        self.turn_index = turn_index;
        self.cursor = i32::from(raw.entry_index) - 1;
        self.reversed = false;
        true
    }

    /// Drive4B210E/Ship6A1751: completion preserves short and residual.
    pub(crate) fn clear_selector(&mut self) {
        self.turn_index = -1;
        self.cursor = 0;
    }

    fn selected_raw(&self, family: TrackFamily) -> Option<(u8, &'static TurnTrack)> {
        let turn = family.turn(self.turn_index)?;
        // Original read selects the short byte directly, without a fallback
        // to normal when that byte is zero. The process host owns that guard.
        let raw = if self.reversed {
            turn.short_track
        } else {
            turn.normal_track
        };
        (raw != 0).then_some((raw, turn))
    }

    /// Drive4B2312..23FE / Ship6A1951..1A45. With no paid point the start
    /// remains the live owner coordinate, including the previous residual step.
    /// This does not change facing, cursor, or retained budget.
    pub(crate) fn residual_step(
        &self,
        family: TrackFamily,
        current: DriveCoord,
        head: DriveCoord,
    ) -> Option<ResidualStep> {
        if self.residual < 1 {
            return None;
        }
        let (raw, turn) = self.selected_raw(family)?;
        let point = raw_point(raw, self.cursor)?;
        if self.cursor != 0 && point.x == 0 && point.y == 0 {
            return None;
        }
        let (dx, dy, _) =
            drive_track::transform_track_point(point.x, point.y, point.facing, turn.flags);
        let full = DriveCoord {
            x: head.x.wrapping_add(i32::from(dx)),
            y: head.y.wrapping_add(i32::from(dy)),
            z: current.z,
        };
        let (_, delta) = residual_delta(
            [
                full.x.wrapping_sub(current.x),
                full.y.wrapping_sub(current.y),
                0,
            ],
            self.residual,
        );
        Some(ResidualStep {
            current,
            full,
            interpolated: DriveCoord {
                x: current.x.wrapping_add(delta[0]),
                y: current.y.wrapping_add(delta[1]),
                z: current.z,
            },
        })
    }
}

/// Original helper75F540 receives an actual chopped f32 argument. Replacing
/// its computation with integer delta*residual/7 changes even delta7,budget1.
fn residual_delta(delta: [i32; 3], residual: i32) -> (NativeF32Bits, [i32; 3]) {
    let reciprocal = X87Chop53::load_f64(NativeF64Bits::from_bits(0x3fc2_4924_9249_2492))
        .expect("original finite reciprocal");
    let factor_bits =
        X87Chop53::store_f32(X87Chop53::mul(X87Chop53::load_i32(residual), reciprocal))
            .expect("signed residual produces a finite f32 factor");
    let factor = X87Chop53::load_f32(factor_bits).expect("stored finite factor");
    let other = X87Chop53::sub(X87Chop53::load_i32(1), factor);
    let zero_term = X87Chop53::mul(X87Chop53::load_i32(0), other);
    let scaled = delta.map(|component| {
        X87Chop53::ftol_i64(X87Chop53::add(
            X87Chop53::mul(X87Chop53::load_i32(component), factor),
            zero_term,
        ))
        .expect("signed component and residual product fits i64") as i32
    });
    (factor_bits, scaled)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ResidualStep {
    pub current: DriveCoord,
    pub full: DriveCoord,
    pub interpolated: DriveCoord,
}

impl ResidualStep {
    /// Drive4B2452..24AD / Ship6A1AA3..1AFE compare actual Cell identities.
    /// The host supplies those comparisons, preserving shared dummy-cell
    /// identity. Signed coordinate-cell comparison is a later, separate gate.
    pub fn choose(
        self,
        matches_current_cell: bool,
        matches_full_cell: bool,
        residual: i32,
    ) -> DriveCoord {
        if matches_current_cell || matches_full_cell {
            self.interpolated
        } else if residual > 3 {
            self.full
        } else {
            self.current
        }
    }
}

/// Original arrays have a separately paid zero-XY terminator. The existing
/// geometry catalog omits it; retain its actual facing too (raw15 ends at192,
/// while the last real point faces188). All25 family/raw arrays are compared
/// with original retail bytes by the accompanying corpus test.
fn raw_point(raw: u8, cursor: i32) -> Option<TrackPoint> {
    let points = drive_track::raw_track_points(raw);
    let index = usize::try_from(cursor).ok()?;
    if let Some(point) = points.get(index) {
        return Some(*point);
    }
    const TERMINAL_FACING: [u8; 16] = [
        0, 0, 32, 32, 64, 96, 64, 32, 64, 64, 96, 160, 160, 64, 32, 192,
    ];
    if points.is_empty() || index != points.len() {
        return None;
    }
    Some(TrackPoint {
        x: 0,
        y: 0,
        facing: *TERMINAL_FACING.get(usize::from(raw))?,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PaidSample {
    pub raw_index: u8,
    pub cursor: i32,
    pub xy: [i32; 2],
    pub facing: u8,
    pub flags: u8,
    pub terminal: bool,
    pub chain: bool,
    pub occupation_handoff: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TrackPayment {
    Exhausted,
    Sample(PaidSample),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProcessPhase {
    Ready,
    PointCallbacks,
    TerminalCallbacks,
}

/// A call-local continuation, never serialized and never holding an entity
/// borrow. After a callback the host must refetch the same active instance;
/// callback death/limbo/off-map exits drop this call without residual writeback.
#[derive(Debug)]
pub(crate) struct TrackProcess {
    budget: i32,
    phase: ProcessPhase,
}

impl TrackProcess {
    pub fn begin(progress: &TrackProgress, fresh_budget: i32) -> Self {
        Self {
            budget: progress.residual.wrapping_add(fresh_budget),
            phase: ProcessPhase::Ready,
        }
    }

    pub fn budget(&self) -> i32 {
        self.budget
    }

    /// None means the supplied selector/cursor has no readable catalog point;
    /// the host must establish native active-track admission before calling.
    /// A payment exposes a callback barrier, not an automatic cursor advance.
    pub fn pay_current(
        &mut self,
        family: TrackFamily,
        progress: &TrackProgress,
    ) -> Option<TrackPayment> {
        assert_eq!(
            self.phase,
            ProcessPhase::Ready,
            "complete the paid point's callbacks before another payment"
        );
        if self.budget <= POINT_COST {
            return Some(TrackPayment::Exhausted);
        }
        let (raw_index, turn) = progress.selected_raw(family)?;
        let point = raw_point(raw_index, progress.cursor)?;
        let raw = drive_track::raw_track_meta(raw_index)?;
        self.budget = self.budget.wrapping_sub(POINT_COST);
        let terminal = progress.cursor != 0 && point.x == 0 && point.y == 0;
        self.phase = if terminal {
            ProcessPhase::TerminalCallbacks
        } else {
            ProcessPhase::PointCallbacks
        };
        Some(TrackPayment::Sample(PaidSample {
            raw_index,
            cursor: progress.cursor,
            xy: [i32::from(point.x), i32::from(point.y)],
            facing: point.facing,
            flags: turn.flags,
            terminal,
            chain: progress.cursor == i32::from(raw.chain_index),
            occupation_handoff: progress.cursor == i32::from(raw.occupation_handoff_point_index),
        }))
    }

    /// Increment the REFETCHED retained cursor. A chain may have replaced it
    /// with entry-1 while the paid point's callback was running.
    pub fn finish_surviving_point(&mut self, progress: &mut TrackProgress) -> bool {
        assert_eq!(self.phase, ProcessPhase::PointCallbacks);
        progress.cursor = progress.cursor.wrapping_add(1);
        self.phase = ProcessPhase::Ready;
        self.budget > POINT_COST
    }

    /// Drive4B1F97..2006 / Ship6A15DA..1649, with original Math::ftol.
    /// The input is the actual Foot coordinate at the paid sentinel, not a
    /// reconstructed last-point coordinate. Head Z does not enter this refund.
    pub fn adjust_terminal_budget(&mut self, current: DriveCoord, head: DriveCoord) {
        assert_eq!(self.phase, ProcessPhase::TerminalCallbacks);
        let distance = head
            .x
            .wrapping_sub(current.x)
            .wrapping_abs()
            .wrapping_add(head.y.wrapping_sub(current.y).wrapping_abs());
        let reciprocal = X87Chop53::load_f64(NativeF64Bits::from_bits(0x3fb7_45d1_745d_1746))
            .expect("original finite reciprocal");
        let fraction = X87Chop53::mul(X87Chop53::load_i32(distance), reciprocal);
        let adjustment = X87Chop53::mul(
            X87Chop53::sub(X87Chop53::load_i32(1), fraction),
            X87Chop53::load_i32(POINT_COST),
        );
        let amount =
            X87Chop53::ftol_i64(adjustment).expect("signed XY distance refund fits i64") as i32;
        self.budget = self.budget.wrapping_add(amount);
    }

    /// Explicit native writeback; do not put this in Drop or a callback finally
    /// path because early lifecycle exits deliberately retain the old residual.
    pub fn store_residual(&self, progress: &mut TrackProgress) {
        progress.residual = self.budget;
    }
}

#[cfg(test)]
#[path = "track_process_tests.rs"]
mod tests;
