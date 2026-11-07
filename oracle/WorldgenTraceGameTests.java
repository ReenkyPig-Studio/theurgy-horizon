// SPDX-License-Identifier: MIT
package com.klikli_dev.theurgy.gametest;

import net.minecraft.core.registries.Registries;
import net.minecraft.gametest.framework.GameTestHelper;
import net.minecraft.resources.ResourceKey;
import net.minecraft.world.level.biome.BiomeGenerationSettings;
import net.minecraft.world.level.biome.FeatureSorter;
import net.minecraft.world.level.levelgen.placement.PlacedFeature;
import com.klikli_dev.theurgy.Theurgy;

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
                    helper.succeed();
                });
                return;
            }
        }
        helper.fail("sal_ammoniac_ore was not found in FeatureSorter");
    }
}
