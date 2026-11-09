// SPDX-License-Identifier: MIT
package com.klikli_dev.theurgy.gametest;

import net.minecraft.core.registries.Registries;
import net.minecraft.world.level.levelgen.LegacyRandomSource;
import net.minecraft.core.BlockPos;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.gametest.framework.GameTestHelper;
import net.minecraft.resources.ResourceKey;
import net.minecraft.world.level.biome.BiomeGenerationSettings;
import net.minecraft.world.level.biome.FeatureSorter;
import net.minecraft.world.level.levelgen.placement.PlacedFeature;
import com.klikli_dev.theurgy.Theurgy;
import com.klikli_dev.theurgy.registry.BlockRegistry;
import net.minecraft.core.registries.BuiltInRegistries;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import com.google.gson.GsonBuilder;
import com.google.gson.JsonArray;
import com.google.gson.JsonObject;

/** Auxiliary Java-only oracle for the exact FeatureSorter index. */
public final class WorldgenTraceGameTests {
    private WorldgenTraceGameTests() {}

    public static void featureIndex(GameTestHelper helper) {
        var access = helper.getLevel().getServer().registryAccess();
        var biomes = access.lookupOrThrow(Registries.BIOME).listElements().map(holder -> holder.value().getGenerationSettings()).toList();
        var steps = FeatureSorter.buildFeaturesPerStep(biomes, BiomeGenerationSettings::features, true);
        var key = ResourceKey.create(Registries.PLACED_FEATURE, Theurgy.loc("sal_ammoniac_ore"));
        var placed = access.lookupOrThrow(Registries.PLACED_FEATURE).getOrThrow(key);
        for (int step = 0; step < steps.size(); step++) {
            var data = steps.get(step);
            int index = data.indexMapping().applyAsInt(placed.value());
            if (index >= 0 && data.features().contains(placed.value())) {
                final int foundStep = step;
                final int foundIndex = index;
                helper.runAfterDelay(1, () -> {
                    System.out.println("THEURGY_FEATURE_INDEX=" + foundIndex + ";STEP=" + foundStep);
                    writeOreTrace(helper);
                    helper.succeed();
                });
                return;
            }
        }
        helper.fail("sal_ammoniac_ore was not found in FeatureSorter");
    }

    private static void writeOreTrace(GameTestHelper helper) {
        try {
            var rngProbe = new LegacyRandomSource(1L);
            System.out.println("THEURGY_LEGACY_PROBE=" + rngProbe.nextInt(16) + "," + rngProbe.nextInt(16) + "," + rngProbe.nextInt(16));
            var level = helper.getLevel();
            var origin = helper.testInfo.getTestOrigin();
            var key = ResourceKey.create(Registries.PLACED_FEATURE, Theurgy.loc("sal_ammoniac_ore"));
            var placed = level.getServer().registryAccess().lookupOrThrow(Registries.PLACED_FEATURE).getOrThrow(key);
            var ores = new JsonArray();
            for (int attempt = 0; attempt < 16 && ores.isEmpty(); attempt++) {
                origin = origin.east(attempt == 0 ? 0 : 16);
            for (int x = origin.getX() - 24; x <= origin.getX() + 24; x++) {
                for (int z = origin.getZ() - 24; z <= origin.getZ() + 24; z++) {
                    for (int y = -64; y <= 200; y++) {
                        var pos = new BlockPos(x, y, z);
                        var state = level.getBlockState(pos);
                        if (state.is(BlockRegistry.SAL_AMMONIAC_ORE.get()) || state.is(BlockRegistry.DEEPSLATE_SAL_AMMONIAC_ORE.get())) {
                            level.setBlock(pos, (y < 8 ? Blocks.DEEPSLATE : Blocks.STONE).defaultBlockState(), 3);
                        }
                    }
                }
            }
            for (int x = origin.getX() - 24; x <= origin.getX() + 24; x++) {
                for (int z = origin.getZ() - 24; z <= origin.getZ() + 24; z++) {
                    for (int y = 16; y <= 319; y++) {
                        level.setBlock(new BlockPos(x, y, z), Blocks.AIR.defaultBlockState(), 3);
                    }
                }
            }
            for (int x = origin.getX() - 8; x <= origin.getX() + 8; x++) {
                for (int z = origin.getZ() - 8; z <= origin.getZ() + 8; z++) {
                    for (int y = 0; y <= 15; y++) {
                        level.setBlock(new BlockPos(x, y, z), (y < 8 ? Blocks.DEEPSLATE : Blocks.STONE).defaultBlockState(), 3);
                    }
                }
            }
            var seed = level.getSeed();
            var decorationRandom = new LegacyRandomSource(seed);
            long xScale = decorationRandom.nextLong() | 1;
            long zScale = decorationRandom.nextLong() | 1;
            long decorationSeed = ((long) (origin.getX() >> 4) * xScale + (long) (origin.getZ() >> 4) * zScale) ^ seed;
            var random = new LegacyRandomSource(decorationSeed + 34 + 60000);
            var feature = level.getServer().registryAccess().lookupOrThrow(Registries.FEATURE)
                    .getOrThrow(ResourceKey.create(Registries.FEATURE, Theurgy.loc("sal_ammoniac_ore")));
            for (int i = 0; i < 20; i++) {
                int x = (origin.getX() >> 4 << 4) + random.nextInt(16);
                int z = (origin.getZ() >> 4 << 4) + random.nextInt(16);
                int y = (random.nextInt(265) - 64 + random.nextInt(265) - 64) / 2;
                feature.value().place(level, level.getChunkSource().getGenerator(), random, new BlockPos(x, y, z));
            }
            for (int x = origin.getX() - 24; x <= origin.getX() + 24; x++) {
                for (int z = origin.getZ() - 24; z <= origin.getZ() + 24; z++) {
                    for (int y = -64; y <= 200; y++) {
                        var pos = new BlockPos(x, y, z);
                        var state = level.getBlockState(pos);
                        if (!state.is(BlockRegistry.DEEPSLATE_SAL_AMMONIAC_ORE.get()) && !state.is(BlockRegistry.SAL_AMMONIAC_ORE.get())) continue;
                        var ore = new JsonObject();
                        ore.addProperty("id", BuiltInRegistries.BLOCK.getKey(state.getBlock()).toString());
                        var position = new JsonArray();
                        position.add(x);
                        position.add(y);
                        position.add(z);
                        ore.add("pos", position);
                        ores.add(ore);
                    }
                }
            }
            }
            var root = new JsonObject();
            root.addProperty("seed", level.getSeed());
            root.addProperty("origin", origin.toShortString());
            root.add("ores", ores);
            Path dir = Path.of(System.getProperty("theurgy.trace.dir", "run/theurgy-trace"));
            Files.createDirectories(dir);
            Path file = dir.resolve("worldgen.json");
            Files.writeString(file, new GsonBuilder().disableHtmlEscaping().create().toJson(root), StandardCharsets.UTF_8);
            System.out.println("THEURGY_WORLDGEN_TRACE=" + file + ";SEED=" + level.getSeed() + ";ORIGIN=" + origin.toShortString() + ";COUNT=" + ores.size());
        } catch (Exception error) {
            throw new IllegalStateException(error);
        }
    }

    private static int[] intArray(int x, int y, int z) {
        return new int[] {x, y, z};
    }
}
