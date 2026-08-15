//! Opaque native RNG cursor handoff shared across the map/simulation boundary.
//!
//! The random-map generator produces this move-only receipt, the app transports
//! it, and the simulation consumes it. Keeping the DTO below both owners avoids
//! making live simulation depend on the pre-play generator implementation.

pub(crate) const MAPGEN_RNG_STATE_WORDS: usize = 250;

/// Move-only post-generation cursor handed to the live simulation.
///
/// Active YR seeds and advances `g_MapGenRng @ 0x00ABE890` while building a
/// random map (`FUN_00598960`), then bridge repair later draws from that same
/// object in `FUN_00598030`. The app may transport this value but cannot draw
/// from it or inspect its native state.
#[derive(Debug)]
pub(crate) struct MapGenRngContinuation {
    words: [u32; MAPGEN_RNG_STATE_WORDS],
    index_a: usize,
    index_b: usize,
}

impl MapGenRngContinuation {
    pub(crate) fn from_native_parts(
        words: [u32; MAPGEN_RNG_STATE_WORDS],
        index_a: usize,
        index_b: usize,
    ) -> Self {
        Self {
            words,
            index_a,
            index_b,
        }
    }

    pub(crate) fn into_native_parts(self) -> ([u32; MAPGEN_RNG_STATE_WORDS], usize, usize) {
        (self.words, self.index_a, self.index_b)
    }
}

#[cfg(test)]
mod tests {
    use super::MapGenRngContinuation;
    use crate::map::rmg::RmgRng;
    use crate::sim::rng::SimRng;

    #[test]
    fn post_generation_cursor_transfers_full_state_across_cursor_wraps() {
        for seed in [0, u16::MAX] {
            for prefix_draws in [0, 249, 250, 353] {
                let mut generated = RmgRng::new(seed);
                for _ in 0..prefix_draws {
                    let _ = generated.next_u32();
                }
                let mut expected = generated.clone();
                let continuation: MapGenRngContinuation = generated.into_continuation();
                let mut live = SimRng::from_mapgen_continuation(continuation);

                for draw in 0..500 {
                    assert_eq!(
                        live.next_u32(),
                        expected.next_u32(),
                        "seed {seed}, prefix {prefix_draws}, continuation draw {draw}"
                    );
                }
            }
        }
    }
}
