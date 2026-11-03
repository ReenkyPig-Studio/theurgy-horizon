// SPDX-License-Identifier: MIT
// Bounded performance harness for the pure Theurgy engine.
//
// Two clearly separated kinds of evidence:
//   1. `check`  runs the engine's Rust performance tests. Those assert
//               deterministic *work* identities (recipe lookups, recipe scans,
//               matcher calls, plan builds, block updates, commits, plan-buffer
//               growth) and never look at a clock, so they are safe in CI.
//   2. `report` runs the same bounded measurements N times and prints the
//               observed median/min/max wall time per tick. Timing is REPORTED,
//               never asserted: no threshold here can fail a build.
//
// Nothing outside native/theurgy/engine is written; Cargo output goes to
// THEURGY_CARGO_TARGET (shared) or target/perf, never to the launcher.
import fs from 'node:fs/promises';
import path from 'node:path';
import { spawn } from 'node:child_process';
import { root, config, rustEnv } from './common.mjs';

const cargo = path.join(config.rust, 'bin/cargo.exe');
const manifest = path.join(root, 'native/theurgy/engine/Cargo.toml');
const targetDir = process.env.THEURGY_CARGO_TARGET ?? path.join(root, 'target/perf');

// Each entry mirrors a measurement in engine/src/performance.rs.
//
// The `bounds` below are structural facts taken from the Java reference
// (CraftingBehaviour.tickServer), not timings:
//   * tickServer calls getRecipe() exactly once per tick, and only when the
//     machine has input (`if (hasInput) {...} else stopProcessing()`), so an idle
//     machine performs zero lookups;
//   * canCraft(recipe) reuses that resolved holder and does no lookup;
//   * getTotalTime() — the only second lookup — runs only on a craft tick
//     (inside tryFinishProcessing) and inside onInputItemChanged;
//   * each lookup proposes every row of the recipe type, so
//     scans == lookups * rows;
//   * a machine builds at most one host plan per craft tick, so
//     plan_calls <= crafts <= ticks <= lookups;
//   * block updates are throttled to real state changes, so
//     block_updates <= 2 * (1 + crafts) + 1.
// Anything outside those is a real behavioural defect, not machine noise.
export const BOUNDS = Object.freeze({
  maxRuns: 32,
  defaultRuns: 5,
  maxTicks: 20_000,
  fixtures: Object.freeze([
    { id: 'distiller', kind: 'theurgy:distillation', rows: 2, ticks: 400 },
    { id: 'liquefaction', kind: 'theurgy:liquefaction', rows: 2, ticks: 400 },
    { id: 'vat', kind: 'theurgy:fermentation', rows: 1, ticks: 400 },
    { id: 'linear_scan_64', kind: 'theurgy:distillation', rows: 64, ticks: 64 },
    { id: 'idle', kind: 'theurgy:distillation', rows: 2, ticks: 64 },
  ]),
});

/** Clamps a user-supplied run count into the harness's bounded range. */
export function clampRuns(value) {
  const n = Number(value);
  if (!Number.isFinite(n)) return BOUNDS.defaultRuns;
  return Math.max(1, Math.min(BOUNDS.maxRuns, Math.trunc(n)));
}

/**
 * Validates one measurement line emitted by the engine report against the
 * structural bounds above. Returns a list of violations; empty means the
 * observed work is consistent with the Java semantics. No timing is examined.
 */
export function checkWorkBounds(name, work) {
  const problems = [];
  const ints = [
    'ticks', 'scans', 'matcher_calls', 'plan_calls', 'block_updates', 'commits',
    'crafts', 'lookups', 'total_time_lookups', 'item_slots', 'parts', 'rows',
  ];
  for (const key of ints) {
    if (!Number.isInteger(work[key]) || work[key] < 0) {
      problems.push(`${name}: ${key} must be a non-negative integer, got ${work[key]}`);
    }
  }
  if (problems.length) return problems;

  const { ticks, rows, lookups, total_time_lookups, crafts, scans } = work;
  const allLookups = lookups + total_time_lookups;

  // Lookup identity: one getRecipe per tick (only with input), plus exactly one
  // getTotalTime per craft tick and per input content-type change.
  if (lookups > ticks) {
    problems.push(`${name}: lookups ${lookups} > ticks ${ticks} (getRecipe is per-tick)`);
  }
  if (total_time_lookups > crafts + 1) {
    problems.push(
      `${name}: total_time_lookups ${total_time_lookups} > crafts ${crafts} + 1 input change`,
    );
  }
  // Every lookup walks the whole table: scans is exactly lookups * rows.
  if (scans !== allLookups * rows) {
    problems.push(`${name}: scans ${scans} != lookups ${allLookups} * rows ${rows}`);
  }
  // Idle machines never reach getRecipe() at all.
  if (name === 'idle' && allLookups !== 0) {
    problems.push(`${name}: an idle machine must not perform recipe lookups, got ${allLookups}`);
  }
  if (name === 'idle' && (scans !== 0 || work.matcher_calls !== 0 || work.plan_calls !== 0)) {
    problems.push(`${name}: idle machine reported match/plan work`);
  }

  // The matcher closure runs once per (row, ingredient part, candidate slot):
  // find_recipe tests each ingredient part against each non-empty slot until it
  // hits or exhausts the slots. `parts` is the largest per-row part count.
  const matcherBudget = allLookups * rows * Math.max(1, work.parts) * Math.max(1, work.item_slots);
  if (work.matcher_calls > matcherBudget) {
    problems.push(`${name}: matcher_calls ${work.matcher_calls} exceeds row*part*slot budget ${matcherBudget}`);
  }
  // The matcher may legitimately be skipped: find_recipe short-circuits on the
  // ingredient COUNT before ever calling matches(). So there is no meaningful
  // lower bound here beyond "a tick with input that selects a recipe ran at
  // least one matcher", which the Rust identity tests assert directly.
  if (work.plan_calls > crafts) {
    problems.push(`${name}: plan_calls ${work.plan_calls} > crafts ${crafts} (one plan per craft)`);
  }
  if (work.commits > crafts) {
    problems.push(`${name}: commits ${work.commits} > crafts ${crafts}`);
  }
  if (crafts > ticks) {
    problems.push(`${name}: crafts ${crafts} > ticks ${ticks}`);
  }
  // start (1) + craft-sendBlockUpdated (1) per craft + the up-front input change.
  const updateBudget = 2 * (1 + crafts) + 1;
  if (work.block_updates > updateBudget) {
    problems.push(`${name}: block_updates ${work.block_updates} > budget ${updateBudget}`);
  }
  return problems;
}

function exec(args, { quiet = false } = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(cargo, args, {
      cwd: root,
      env: { ...rustEnv, CARGO_TARGET_DIR: targetDir },
      stdio: quiet ? ['ignore', 'pipe', 'pipe'] : 'inherit',
    });
    let out = '';
    child.stdout?.on('data', d => (out += d));
    child.stderr?.on('data', d => (out += d));
    child.on('error', reject);
    child.on('exit', code => (code === 0 ? resolve(out) : reject(new Error(`cargo ${args.join(' ')} exited ${code}`))));
  });
}

// Cargo globals (--locked, --offline, --manifest-path) belong after the subcommand.
const common = ['--locked', '--offline', '--manifest-path', manifest, '-p', 'theurgy_engine'];

/** Runs the deterministic work-identity tests. This is the pass/fail gate. */
async function check() {
  await exec(['test', ...common, '--lib', 'performance::', '--', '--test-threads=1']);
  console.log(`OK engine performance work-identity tests (${BOUNDS.fixtures.length} fixtures bounded)`);
}

/**
 * Parses the `fixture=... key=value ...` lines printed by
 * `performance::tests::prints_measurement_report`.
 */
export function parseReport(text) {
  const rows = [];
  for (const raw of text.split(/\r?\n/)) {
    const line = raw.trim();
    const m = /^fixture=(\S+)\s+(.*)$/.exec(line);
    if (!m) continue;
    const fields = {};
    for (const x of m[2].matchAll(/(\w+)=(0x[0-9a-f]+|[->\d.]+)/g)) {
      fields[x[1]] = x[2];
    }
    rows.push({
      name: m[1],
      ticks: Number(fields.ticks),
      scans: Number(fields.scans),
      matcher_calls: Number(fields.matcher_calls),
      plan_calls: Number(fields.plan_calls),
      block_updates: Number(fields.block_updates),
      commits: Number(fields.commits),
      crafts: Number(fields.crafts),
      lookups: Number(fields.lookups),
      total_time_lookups: Number(fields.total_time_lookups),
      item_slots: Number(fields.item_slots ?? 0),
      parts: Number(fields.parts ?? 0),
      rows: Number(fields.rows ?? 0),
      ns_per_tick: Number(fields.ns_per_tick),
      work_per_tick: Number(fields.work_per_tick),
      checksum: fields.checksum,
    });
  }
  return rows;
}

/**
 * Runs the engine's own `performance::report()` (a lib unit test that prints one
 * line per fixture) `runs` times and collects the rows. Each sample is a full
 * bounded measurement inside Rust, so timing includes the work but not Node.
 */
async function measure(runs) {
  const byName = new Map();
  for (let i = 0; i < runs; i++) {
    const text = await exec(
      ['test', ...common, '--release', '--lib', 'performance::tests::prints_measurement_report', '--', '--nocapture', '--exact'],
      { quiet: true },
    );
    for (const sample of parseReport(text)) {
      if (!byName.has(sample.name)) byName.set(sample.name, { name: sample.name, samples: [] });
      byName.get(sample.name).samples.push(sample);
    }
  }
  return [...byName.values()];
}

function median(values) {
  const s = [...values].sort((a, b) => a - b);
  if (!s.length) return 0;
  const mid = s.length >> 1;
  return s.length % 2 ? s[mid] : (s[mid - 1] + s[mid]) / 2;
}

/**
 * Summarizes samples. `checksum` must be identical across runs: a differing
 * checksum means the *behaviour* differed, which is a real failure, whereas a
 * differing ns_per_tick is just machine noise and is only reported.
 */
export function summarize(row) {
  const tickValues = row.samples.map(s => s.ns_per_tick);
  const workValues = row.samples.map(s => s.work_per_tick);
  const checksums = new Set(row.samples.map(s => s.checksum));
  const first = row.samples[0] ?? {};
  return {
    name: row.name,
    runs: row.samples.length,
    ticks: first.ticks ?? 0,
    rows: first.rows ?? 0,
    lookups: first.lookups ?? 0,
    crafts: first.crafts ?? 0,
    work_per_tick_median: Number(median(workValues).toFixed(3)),
    ns_per_tick_median: Number(median(tickValues).toFixed(1)),
    ns_per_tick_min: Number(Math.min(...tickValues).toFixed(1)),
    ns_per_tick_max: Number(Math.max(...tickValues).toFixed(1)),
    stable_behaviour: checksums.size === 1,
    checksums: [...checksums],
  };
}

async function report(runs) {
  const rows = await measure(runs);
  if (!rows.length) throw new Error('engine produced no measurements');
  const summaries = rows.map(summarize);
  const problems = [];
  for (const row of rows) {
    for (const sample of row.samples) {
      problems.push(...checkWorkBounds(row.name, sample));
    }
    if (!summarize(row).stable_behaviour) problems.push(`${row.name}: behaviour checksum changed across runs`);
  }
  // Every declared fixture must actually have been measured.
  for (const fixture of BOUNDS.fixtures) {
    if (!rows.some(r => r.name === fixture.id)) problems.push(`missing measurement for fixture ${fixture.id}`);
  }
  const outDir = path.join(root, 'target/perf-report');
  await fs.mkdir(outDir, { recursive: true });
  await fs.writeFile(
    path.join(outDir, 'report.json'),
    JSON.stringify({ generatedAt: new Date().toISOString(), runs, timingIsNotAGate: true, summaries, problems }, null, 2),
  );
  console.table(summaries);
  console.log('Timing is informational only; work bounds above are the enforced contract.');
  if (problems.length) {
    for (const p of problems) console.error('FAIL', p);
    process.exitCode = 1;
  }
}

const isMain = process.argv[1] && path.resolve(process.argv[1]) === path.resolve(new URL(import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, '$1'));
if (isMain) {
  const cmd = process.argv[2] ?? 'check';
  if (cmd === 'check') await check();
  else if (cmd === 'report') await report(clampRuns(process.argv[3]));
  else throw new Error('usage: node tools/bench-port.mjs check|report [runs]');
}
