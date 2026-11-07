// SPDX-License-Identifier: MIT
// Theurgy sal-ammoniac ore generation.
//
// Java paths: `datagen/worldgen/ConfiguredFeatures.bootstrap`,
// `datagen/worldgen/PlacedFeatures.bootstrap`, vanilla `OreFeature.place`,
// `WorldgenRandom.setDecorationSeed/setFeatureSeed`, `CountPlacement`,
// `InSquarePlacement` and `HeightRangePlacement.triangle`.

use horizon_core::random::{LegacyRandom, RandomSource, WorldgenRandom};

const UNDERGROUND_ORES_STEP: i32 = 6; // GenerationStep.Decoration ordinal.
// Real FeatureSorter oracle: runs/java-oracle-2026-11-07T05-40-09.710Z,
// game-test-server.log marker THEURGY_FEATURE_INDEX=34;STEP=6.
const SAL_AMMONIAC_FEATURE_INDEX: i32 = 34;
const ORE_SIZE: usize = 9;
const ORE_COUNT: usize = 20;

type Rng = WorldgenRandom<LegacyRandom>;

fn chunk_rng(world: i64, chunk: [i32; 2]) -> Rng {
    let mut rng = WorldgenRandom::new(LegacyRandom::new(0));
    let decoration = rng.set_decoration_seed(world, chunk[0], chunk[1]);
    rng.set_feature_seed(decoration, SAL_AMMONIAC_FEATURE_INDEX, UNDERGROUND_ORES_STEP);
    rng
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

fn triangle_height(rng: &mut Rng) -> i32 {
    let first = rng.next_int_between_inclusive(-64, 200).expect("fixed valid height range");
    let second = rng.next_int_between_inclusive(-64, 200).expect("fixed valid height range");
    (first + second) / 2
}

/// OCEAN_FLOOR_WG is one above the highest block in this tag. The 16x16
/// cache is computed once per chunk instead of rescanning for all 20 veins.
fn ocean_floor_heights(game: &mut crate::game::Game<'_>, chunk: [i32; 2]) -> Vec<i32> {
    let mut heights = vec![-64; 256];
    for local_x in 0..16 {
        for local_z in 0..16 {
            let x = chunk[0].wrapping_mul(16) + local_x;
            let z = chunk[1].wrapping_mul(16) + local_z;
            for y in (-64..=319).rev() {
                let block = game.native_block([x, y, z]);
                if !block.is_empty()
                    && game.res.items().block_in_tag("minecraft:blocks_motion_in_heightmap", &block)
                {
                    heights[(local_x + local_z * 16) as usize] = y + 1;
                    break;
                }
            }
        }
    }
    heights
}

/// OreFeature rejects a vein before generating cells if the entire search
/// square is below `y_start`. Out-of-chunk columns are not writable and are
/// therefore not part of the current chunk's heightmap.
fn any_ocean_floor_at_least(
    heights: &[i32],
    [x_start, y_start, z_start]: [i32; 3],
    size_xz: i32,
    chunk: [i32; 2],
) -> bool {
    for x in x_start..=x_start.saturating_add(size_xz) {
        for z in z_start..=z_start.saturating_add(size_xz) {
            if x.div_euclid(16) != chunk[0] || z.div_euclid(16) != chunk[1] {
                continue;
            }
            let local_x = x.rem_euclid(16) as usize;
            let local_z = z.rem_euclid(16) as usize;
            if heights.get(local_x + local_z * 16).copied().unwrap_or(-64) >= y_start {
                return true;
            }
        }
    }
    false
}

/// Vanilla `OreFeature.place` for size=9 and no air-exposure discard. The
/// current chunk acts as vanilla's BulkSectionAccess write boundary.
fn place_ore_feature(
    game: &mut crate::game::Game<'_>,
    dimension: &str,
    rng: &mut Rng,
    origin: [i32; 3],
    chunk: [i32; 2],
    heights: &mut [i32],
) -> usize {
    let direction = rng.next_float() * std::f32::consts::PI;
    let spread_xy = ORE_SIZE as f32 / 8.0;
    let max_radius = ((ORE_SIZE as f32 / 16.0 * 2.0 + 1.0) / 2.0).ceil() as i32;
    let (ox, oz) = (f64::from(origin[0]), f64::from(origin[2]));
    let sin = horizon_core::jmath::sin(f64::from(direction));
    let cos = horizon_core::jmath::cos(f64::from(direction));
    let spread = f64::from(spread_xy);
    let x0 = ox + sin * spread;
    let x1 = ox - sin * spread;
    let z0 = oz + cos * spread;
    let z1 = oz - cos * spread;
    let y0 = f64::from(origin[1] + rng.next_int_bound(3).expect("bound 3") - 2);
    let y1 = f64::from(origin[1] + rng.next_int_bound(3).expect("bound 3") - 2);
    let spread_ceil = spread.ceil() as i32;
    let x_start = origin[0].wrapping_sub(spread_ceil).wrapping_sub(max_radius);
    let y_start = origin[1].wrapping_sub(2).wrapping_sub(max_radius);
    let z_start = origin[2].wrapping_sub(spread_ceil).wrapping_sub(max_radius);
    let size_xz = 2 * (spread_ceil + max_radius);
    let size_y = 2 * (2 + max_radius);
    if !any_ocean_floor_at_least(heights, [x_start, y_start, z_start], size_xz, chunk) {
        return 0;
    }

    let mut cells = vec![[0.0f64; 4]; ORE_SIZE];
    for (index, cell) in cells.iter_mut().enumerate() {
        // Both divisions and the sine argument are f32 in vanilla/Mth.
        let step = index as f32 / ORE_SIZE as f32;
        let step64 = f64::from(step);
        let spread = rng.next_double() * ORE_SIZE as f64 / 16.0;
        let sine = horizon_core::geometry::trig::sin(f64::from(std::f32::consts::PI * step));
        let radius = (f64::from(sine + 1.0) * spread + 1.0) / 2.0;
        *cell = [
            x0 + step64 * (x1 - x0),
            y0 + step64 * (y1 - y0),
            z0 + step64 * (z1 - z0),
            radius,
        ];
    }
    for first in 0..ORE_SIZE - 1 {
        if cells[first][3] <= 0.0 {
            continue;
        }
        for second in first + 1..ORE_SIZE {
            if cells[second][3] <= 0.0 {
                continue;
            }
            let dx = cells[first][0] - cells[second][0];
            let dy = cells[first][1] - cells[second][1];
            let dz = cells[first][2] - cells[second][2];
            let dr = cells[first][3] - cells[second][3];
            if dr * dr > dx * dx + dy * dy + dz * dz {
                if dr > 0.0 {
                    cells[second][3] = -1.0;
                } else {
                    cells[first][3] = -1.0;
                }
            }
        }
    }

    let tested_len = (size_xz as usize) * (size_y as usize) * (size_xz as usize);
    let mut tested = vec![0u64; tested_len.div_ceil(64)];
    let mut written = 0usize;
    for [xx, yy, zz, radius] in cells {
        if radius < 0.0 {
            continue;
        }
        let x_min = ((xx - radius).floor() as i32).max(x_start);
        let y_min = ((yy - radius).floor() as i32).max(y_start);
        let z_min = ((zz - radius).floor() as i32).max(z_start);
        let x_max = ((xx + radius).floor() as i32).max(x_min);
        let y_max = ((yy + radius).floor() as i32).max(y_min);
        let z_max = ((zz + radius).floor() as i32).max(z_min);
        for x in x_min..=x_max {
            let xd = (f64::from(x) + 0.5 - xx) / radius;
            if xd * xd >= 1.0 {
                continue;
            }
            for y in y_min..=y_max {
                if !(-64..=319).contains(&y) {
                    continue;
                }
                let yd = (f64::from(y) + 0.5 - yy) / radius;
                if xd * xd + yd * yd >= 1.0 {
                    continue;
                }
                for z in z_min..=z_max {
                    let zd = (f64::from(z) + 0.5 - zz) / radius;
                    if xd * xd + yd * yd + zd * zd >= 1.0 {
                        continue;
                    }
                    let index = i64::from(x.wrapping_sub(x_start))
                        + i64::from(y.wrapping_sub(y_start)) * i64::from(size_xz)
                        + i64::from(z.wrapping_sub(z_start))
                            * i64::from(size_xz)
                            * i64::from(size_y);
                    let Ok(index) = usize::try_from(index) else {
                        continue;
                    };
                    if index >= tested_len {
                        continue;
                    }
                    let (word, bit) = (&mut tested[index / 64], 1u64 << (index % 64));
                    if *word & bit != 0 {
                        continue;
                    }
                    *word |= bit;
                    // BulkSectionAccess never writes into neighbouring chunks.
                    if x.div_euclid(16) != chunk[0] || z.div_euclid(16) != chunk[1] {
                        continue;
                    }
                    let current = game.native_block([x, y, z]);
                    if current.is_empty() {
                        continue;
                    }
                    let deepslate = game
                        .res
                        .items()
                        .block_in_tag("minecraft:deepslate_ore_replaceables", &current);
                    let stone = !deepslate
                        && game
                            .res
                            .items()
                            .block_in_tag("minecraft:stone_ore_replaceables", &current);
                    let Some(ore) = replacement_for_tags(stone, deepslate) else {
                        continue;
                    };
                    let Some(state) = game.pack.assets.registry.default_state(ore) else {
                        continue;
                    };
                    if game.native_set_block(dimension, [x, y, z], state, 2) {
                        written += 1;
                        let index = x.rem_euclid(16) as usize + z.rem_euclid(16) as usize * 16;
                        if y + 1 > heights[index] {
                            heights[index] = y + 1;
                        }
                    }
                }
            }
        }
    }
    written
}

/// Theurgy's count=20, size=9, in-square, triangle(-64,200) feature.
pub fn chunk_load(
    game: &mut crate::game::Game<'_>,
    dimension: &str,
    chunk: [i32; 2],
    new: bool,
) {
    if !new || dimension != "minecraft:overworld" {
        return;
    }
    let mut heights = ocean_floor_heights(game, chunk);
    let mut rng = chunk_rng(game.level.seed, chunk);
    for _ in 0..ORE_COUNT {
        let x = chunk[0].wrapping_mul(16) + rng.next_int_bound(16).expect("bound 16");
        let z = chunk[1].wrapping_mul(16) + rng.next_int_bound(16).expect("bound 16");
        let y = triangle_height(&mut rng);
        place_ore_feature(game, dimension, &mut rng, [x, y, z], chunk, &mut heights);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feature_seed_uses_real_sorter_index() {
        assert_eq!(SAL_AMMONIAC_FEATURE_INDEX, 34);
        assert_eq!(UNDERGROUND_ORES_STEP, 6);
    }

    #[test]
    fn worldgen_rng_matches_horizon_java_reference() {
        let mut reference = WorldgenRandom::new(LegacyRandom::new(0));
        let decoration = reference.set_decoration_seed(42, 3, -2);
        reference.set_feature_seed(decoration, 34, 6);
        let mut actual = chunk_rng(42, [3, -2]);
        for _ in 0..128 {
            assert_eq!(actual.next_long(), reference.next_long());
        }
    }

    #[test]
    fn triangle_and_replacements_stay_in_java_domain() {
        let mut rng = WorldgenRandom::new(LegacyRandom::new(7));
        for _ in 0..512 {
            assert!((-64..=200).contains(&triangle_height(&mut rng)));
        }
        assert_eq!(replacement_for_tags(true, false), Some("theurgy:sal_ammoniac_ore"));
        assert_eq!(replacement_for_tags(false, true), Some("theurgy:deepslate_sal_ammoniac_ore"));
        assert_eq!(replacement_for_tags(false, false), None);
    }

    #[test]
    fn different_chunks_have_different_streams() {
        let mut a = chunk_rng(42, [0, 0]);
        let mut b = chunk_rng(42, [1, 0]);
        let mut c = chunk_rng(42, [0, 1]);
        assert_ne!(a.next_long(), b.next_long());
        let mut a = chunk_rng(42, [0, 0]);
        assert_ne!(a.next_long(), c.next_long());
    }
}
