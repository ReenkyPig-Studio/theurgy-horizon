// SPDX-License-Identifier: MIT
// Native contribution for `theurgy:sal_ammoniac_ore` (Java 26.3).
//
// Reference: `datagen/worldgen/ConfiguredFeatures.bootstrap`,
// `PlacedFeatures.bootstrap` and `BiomeModifiers.bootstrap`. Horizon's dev-mod
// loader does not add content-mod configured features to its generator catalog,
// so the registered feature is applied here when an Overworld chunk is first
// loaded. Existing chunks are never regenerated.

#[derive(Clone, Copy)]
struct ChunkRng {
    state: u64,
}

impl ChunkRng {
    fn new(seed: u64) -> Self {
        Self {
            state: seed ^ 0x9E37_79B9_7F4A_7C15,
        }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn next_i32(&mut self, min: i32, max: i32) -> i32 {
        debug_assert!(min <= max);
        let span = (i64::from(max) - i64::from(min) + 1) as u64;
        min + (self.next_u64() % span) as i32
    }
}

fn chunk_seed(world: i64, cx: i32, cz: i32) -> u64 {
    let x = cx as i64 as u64;
    let z = cz as i64 as u64;
    let mut hash = world as u64;
    hash ^= x.wrapping_mul(0xD6E8_FEB8_6659_FD93);
    hash ^= z.wrapping_mul(0xA24B_AED4_963E_E407);
    hash ^ x.rotate_left(31) ^ z.rotate_left(17)
}

fn replacement_for_tags(stone: bool, deepslate: bool) -> Option<&'static str> {
    if deepslate {
        Some("theurgy:deepslate_sal_ammoniac_ore")
    } else if stone {
        Some("theurgy:sal_ammoniac_ore")
    } else {
        None
    }
}

/// Applies the configured `OreFeature(size=9)` twenty times with the placed
/// feature's in-square spread and trapezoid height distribution (-64..200).
/// Every loop is bounded; at most 180 block writes are attempted per chunk.
pub fn chunk_load(
    game: &mut crate::game::Game<'_>,
    dimension: &str,
    chunk: [i32; 2],
    new: bool,
) {
    if !new || dimension != "minecraft:overworld" {
        return;
    }

    let chunk_x = chunk[0].saturating_mul(16);
    let chunk_z = chunk[1].saturating_mul(16);
    let mut rng = ChunkRng::new(chunk_seed(game.level.seed, chunk[0], chunk[1]));

    for _ in 0..20 {
        let base_x = chunk_x + rng.next_i32(0, 15);
        let base_z = chunk_z + rng.next_i32(0, 15);
        let first = rng.next_i32(-64, 200);
        let second = rng.next_i32(-64, 200);
        let base_y = (first + second) / 2;

        for index in 0..9 {
            let (dx, dy, dz) = if index == 0 {
                (0, 0, 0)
            } else {
                (
                    rng.next_i32(-1, 1),
                    rng.next_i32(-1, 1),
                    rng.next_i32(-1, 1),
                )
            };
            // A post-generation hook may only modify the loaded chunk. This is
            // the native write-radius equivalent of FeatureRegion's boundary.
            let pos = [
                (base_x + dx).clamp(chunk_x, chunk_x + 15),
                (base_y + dy).clamp(-64, 200),
                (base_z + dz).clamp(chunk_z, chunk_z + 15),
            ];
            let current = game.native_block(pos);
            let deepslate = game
                .res
                .items()
                .block_in_tag("minecraft:deepslate_ore_replaceables", &current);
            let stone = !deepslate
                && game
                    .res
                    .items()
                    .block_in_tag("minecraft:stone_ore_replaceables", &current);
            if !deepslate && !stone {
                continue;
            }
            let Some(ore) = replacement_for_tags(stone, deepslate) else {
                continue;
            };
            let Some(state) = game.pack.assets.registry.default_state(ore) else {
                continue;
            };
            game.native_set_block(dimension, pos, state, 3);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rng_is_seeded_and_bounded() {
        let mut a = ChunkRng::new(chunk_seed(42, -3, 7));
        let mut b = ChunkRng::new(chunk_seed(42, -3, 7));
        for _ in 0..256 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
        let mut other = ChunkRng::new(chunk_seed(43, -3, 7));
        assert_ne!(a.next_u64(), other.next_u64());
        for _ in 0..256 {
            let y = a.next_i32(-64, 200);
            assert!((-64..=200).contains(&y));
        }
    }

    #[test]
    fn replacement_rules_match_both_java_targets() {
        assert_eq!(
            replacement_for_tags(true, false),
            Some("theurgy:sal_ammoniac_ore")
        );
        assert_eq!(
            replacement_for_tags(false, true),
            Some("theurgy:deepslate_sal_ammoniac_ore")
        );
        assert_eq!(replacement_for_tags(false, false), None);
    }
}
