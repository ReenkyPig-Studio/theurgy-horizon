// SPDX-License-Identifier: MIT
// Generated into an isolated Java-oracle worktree by tools/java-oracle.mjs.
package com.klikli_dev.theurgy.gametest;

import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import com.google.gson.JsonArray;
import com.google.gson.JsonObject;
import net.minecraft.core.BlockPos;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.gametest.framework.GameTestHelper;
import net.minecraft.gametest.framework.GameTestInfo;
import net.minecraft.gametest.framework.GameTestListener;
import net.minecraft.gametest.framework.GameTestRunner;
import net.minecraft.nbt.NbtOps;
import net.minecraft.world.entity.item.ItemEntity;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.phys.AABB;

import java.io.BufferedWriter;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardOpenOption;
import java.util.Comparator;
import java.util.function.Consumer;

/** Real Java GameTest state oracle. Never included in the pinned read-only source. */
public final class TraceGameTests {
    private static final Gson GSON = new GsonBuilder().disableHtmlEscaping().create();
    private static BufferedWriter writer;

    private TraceGameTests() {}

    public static Consumer<GameTestHelper> instrument(Consumer<GameTestHelper> body) {
        return helper -> {
            GameTestInfo info = helper.testInfo;
            info.addListener(new Listener());
            snapshot(info, "body_start");
            helper.onEachTick(() -> snapshot(info, "tick"));
            body.accept(helper);
            snapshot(info, "body_end");
        };
    }

    private static final class Listener implements GameTestListener {
        @Override public void testStructureLoaded(GameTestInfo info) { snapshot(info, "structure_loaded"); }
        @Override public void testPassed(GameTestInfo info, GameTestRunner runner) { snapshot(info, "passed"); }
        @Override public void testFailed(GameTestInfo info, GameTestRunner runner) { snapshot(info, "failed"); }
        @Override public void testAddedForRerun(GameTestInfo oldInfo, GameTestInfo newInfo, GameTestRunner runner) {}
    }

    private static synchronized void snapshot(GameTestInfo info, String phase) {
        try {
            if (writer == null) {
                Path dir = Path.of(System.getProperty("theurgy.trace.dir", "run/theurgy-trace"));
                Files.createDirectories(dir);
                writer = Files.newBufferedWriter(dir.resolve("traces.jsonl"), StandardCharsets.UTF_8,
                        StandardOpenOption.CREATE, StandardOpenOption.TRUNCATE_EXISTING, StandardOpenOption.WRITE);
                Runtime.getRuntime().addShutdownHook(new Thread(TraceGameTests::close));
            }
            JsonObject root = new JsonObject();
            root.addProperty("id", info.id().toString());
            root.addProperty("phase", phase);
            root.addProperty("tick", info.getTick());
            root.addProperty("gameTime", info.getLevel().getGameTime());
            root.addProperty("origin", info.getTestOrigin().toShortString());
            if (info.getError() != null) root.addProperty("error", info.getError().getMessage());
            AABB bounds = info.getStructureBounds();
            JsonArray blocks = new JsonArray();
            BlockPos origin = info.getTestOrigin();
            BlockPos min = BlockPos.containing(bounds.minX, bounds.minY, bounds.minZ);
            BlockPos max = BlockPos.containing(Math.ceil(bounds.maxX) - 1, Math.ceil(bounds.maxY) - 1, Math.ceil(bounds.maxZ) - 1);
            for (BlockPos pos : BlockPos.betweenClosed(min, max)) {
                BlockState state = info.getLevel().getBlockState(pos);
                var entity = info.getLevel().getBlockEntity(pos);
                if (state.isAir() && entity == null) continue;
                JsonObject block = new JsonObject();
                BlockPos relative = pos.subtract(origin);
                block.add("pos", intArray(relative.getX(), relative.getY(), relative.getZ()));
                block.addProperty("id", BuiltInRegistries.BLOCK.getKey(state.getBlock()).toString());
                JsonObject properties = new JsonObject();
                state.getValues().sorted(Comparator.comparing(value -> value.property().getName())).forEach(value ->
                        properties.addProperty(value.property().getName(), String.valueOf(value.value())));
                block.add("properties", properties);
                if (entity != null) block.addProperty("nbt", entity.saveWithFullMetadata(info.getLevel().registryAccess()).toString());
                blocks.add(block);
            }
            root.add("blocks", blocks);
            JsonArray items = new JsonArray();
            var entities = info.getLevel().getEntitiesOfClass(ItemEntity.class, bounds);
            entities.sort(Comparator.comparingDouble(e -> e.getX() * 1_000_000 + e.getY() * 1_000 + e.getZ()));
            for (ItemEntity entity : entities) {
                JsonObject item = new JsonObject();
                item.add("pos", doubleArray(entity.getX() - origin.getX(), entity.getY() - origin.getY(), entity.getZ() - origin.getZ()));
                item.addProperty("stack", ItemStack.OPTIONAL_CODEC.encodeStart(NbtOps.INSTANCE, entity.getItem()).getOrThrow().toString());
                items.add(item);
            }
            root.add("items", items);
            writer.write(GSON.toJson(root));
            writer.newLine();
            writer.flush();
        } catch (Throwable error) {
            throw new IllegalStateException("Theurgy Java trace failed for " + info.id() + "/" + phase, error);
        }
    }

    private static JsonArray intArray(int... values) {
        JsonArray out = new JsonArray(); for (int value : values) out.add(value); return out;
    }
    private static JsonArray doubleArray(double... values) {
        JsonArray out = new JsonArray(); for (double value : values) out.add(value); return out;
    }
    /** Additional semantic state for recipe-only tests with no block/entity snapshot. */
    public static synchronized void recordStack(ItemStack stack) {
        try {
            if (writer == null) {
                Path dir = Path.of(System.getProperty("theurgy.trace.dir", "run/theurgy-trace"));
                Files.createDirectories(dir);
                writer = Files.newBufferedWriter(dir.resolve("traces.jsonl"), StandardCharsets.UTF_8,
                        StandardOpenOption.CREATE, StandardOpenOption.TRUNCATE_EXISTING, StandardOpenOption.WRITE);
                Runtime.getRuntime().addShutdownHook(new Thread(TraceGameTests::close));
            }
            JsonObject root = new JsonObject();
            root.addProperty("phase", "stack");
            root.addProperty("method", Thread.currentThread().getStackTrace()[2].getMethodName());
            root.addProperty("stack", ItemStack.OPTIONAL_CODEC.encodeStart(NbtOps.INSTANCE, stack).getOrThrow().toString());
            writer.write(GSON.toJson(root));
            writer.newLine();
            writer.flush();
        } catch (Throwable error) {
            throw new IllegalStateException("Theurgy Java stack trace failed", error);
        }
    }

    /** Numeric semantic state (capability capacities) absent from persisted NBT. */
    public static synchronized void recordValue(int value) {
        try {
            if (writer == null) {
                Path dir = Path.of(System.getProperty("theurgy.trace.dir", "run/theurgy-trace"));
                Files.createDirectories(dir);
                writer = Files.newBufferedWriter(dir.resolve("traces.jsonl"), StandardCharsets.UTF_8,
                        StandardOpenOption.CREATE, StandardOpenOption.TRUNCATE_EXISTING, StandardOpenOption.WRITE);
                Runtime.getRuntime().addShutdownHook(new Thread(TraceGameTests::close));
            }
            JsonObject root = new JsonObject();
            var caller = Thread.currentThread().getStackTrace()[2];
            root.addProperty("phase", "value");
            root.addProperty("class", caller.getClassName());
            root.addProperty("method", caller.getMethodName());
            root.addProperty("value", value);
            writer.write(GSON.toJson(root));
            writer.newLine();
            writer.flush();
        } catch (Throwable error) {
            throw new IllegalStateException("Theurgy Java value trace failed", error);
        }
    }

    /** String semantic state (UUID/network values) absent from persisted NBT. */
    public static synchronized void recordText(String value) {
        try {
            if (writer == null) {
                Path dir = Path.of(System.getProperty("theurgy.trace.dir", "run/theurgy-trace"));
                Files.createDirectories(dir);
                writer = Files.newBufferedWriter(dir.resolve("traces.jsonl"), StandardCharsets.UTF_8,
                        StandardOpenOption.CREATE, StandardOpenOption.TRUNCATE_EXISTING, StandardOpenOption.WRITE);
                Runtime.getRuntime().addShutdownHook(new Thread(TraceGameTests::close));
            }
            JsonObject root = new JsonObject();
            var caller = Thread.currentThread().getStackTrace()[2];
            root.addProperty("phase", "text");
            root.addProperty("class", caller.getClassName());
            root.addProperty("method", caller.getMethodName());
            root.addProperty("value", value);
            writer.write(GSON.toJson(root));
            writer.newLine();
            writer.flush();
        } catch (Throwable error) {
            throw new IllegalStateException("Theurgy Java text trace failed", error);
        }
    }

    private static synchronized void close() {
        if (writer != null) try { writer.close(); } catch (IOException ignored) {} finally { writer = null; }
    }

}
