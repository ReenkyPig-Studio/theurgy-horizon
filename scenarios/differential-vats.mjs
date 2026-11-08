// FermentationVatGameTests + DigestionVatGameTests storage/lid/drop behaviour.
import {at, record, setblock, container, merge, wait, probe} from '../differential/harness.mjs';

async function breakAt(call, pos) {
  await call(['tp', String(pos[0]), String(pos[1] + 2), String(pos[2] - 3)]);
  await call(['look-at', 'block', ...pos.map(String)]);
  const broken = await call(['break', ...pos.map(String)]);
  await wait(call, 4);
  return broken;
}
async function extract(call, pos) {
  await call(['command', 'clear', '@s']);
  await call(['tp', String(pos[0]), String(pos[1] + 2), String(pos[2] - 3)]);
  await call(['look-at', 'block', ...pos.map(String)]);
  await call(['interact', ...pos.map(String)]);
  await wait(call, 2);
}
async function closeLid(call, pos) {
  await call(['command', 'clear', '@s']);
  await call(['tp', String(pos[0]), String(pos[1] + 2), String(pos[2] - 3)]);
  await call(['look-at', 'block', ...pos.map(String)]);
  await call(['key', 'shift', 'down']);
  await call(['interact', ...pos.map(String)]);
  await call(['key', 'shift', 'up']);
}
async function until(call, pos, key, expected) {
  for (let i = 0; i < 30; i++) {
    await wait(call, 2);
    if ((await probe(call, pos)).properties?.[key] === expected) return;
  }
  throw new Error(`Timed out waiting for ${key}=${expected}`);
}

export async function runCustom(call) {
  const out = [];
  let index = 0;
  for (const kind of ['fermentation', 'digestion']) {
    const block = `theurgy:${kind}_vat`;
    const input = kind === 'fermentation' ? 'minecraft:oak_log' : 'theurgy:purified_gold';
    const prefix = `theurgy:${kind}_vat_`;
    const origin = () => [(index++) * 16, 80, 0];
    const clear = async o => call(['fill', String(o[0] - 3), '78', '-3', String(o[0] + 3), '85', '3', 'minecraft:air']);
    {
      const o = origin(); await clear(o); await setblock(call, at(o), block);
      await container(call, at(o), input, 1);
      await record(out, prefix + 'insert_input', call, o);
    }
    {
      const o = origin(); await clear(o); await setblock(call, at(o), block);
      await merge(call, at(o), '{fluids:[{Fluid:"theurgy:sal_ammoniac",Amount:1000}]}');
      await record(out, prefix + 'insert_fluid', call, o);
    }
    {
      const o = origin(); await clear(o); await setblock(call, at(o), block);
      await merge(call, at(o), '{Items:[{Slot:3b,id:"minecraft:cobblestone",count:1}]}');
      await extract(call, at(o));
      await record(out, prefix + 'extract_output', call, o);
    }
    {
      const o = origin(); await clear(o); await setblock(call, at(o), block);
      await merge(call, at(o), `{Items:[{Slot:0b,id:"${input}",count:1},{Slot:3b,id:"minecraft:cobblestone",count:1}]}`);
      await merge(call, at(o), '{fluids:[{Fluid:"theurgy:sal_ammoniac",Amount:1000}]}');
      await wait(call, 4);
      await record(out, prefix + 'blocked_output_does_not_consume_inputs', call, o);
    }
    {
      const o = origin(); await clear(o); await setblock(call, at(o), block);
      if (kind === 'fermentation') {
        await container(call, at(o), input, 1);
        await merge(call, at(o), '{fluids:[{Fluid:"theurgy:sal_ammoniac",Amount:1000}]}');
        await wait(call, 3);
      }
      await call(['command', 'theurgy_state', ...at(o).map(String), 'open', 'false']);
      await record(out, prefix + 'lid_closes', call, o);
    }
    {
      const o = origin(); await clear(o); await setblock(call, at(o), block);
      await merge(call, at(o), `{Items:[{Slot:0b,id:"${kind === 'fermentation' ? 'minecraft:oak_log' : 'minecraft:cobblestone'}",count:3}]}`);
      const dropped = await breakAt(call, at(o));
      await record(out, prefix + 'drops_items', call, o);
      out.at(-1).reply.drops = dropped.drops;
      out.at(-1).reply.entities = await call(['entities']);
    }
    if (kind === 'fermentation') {
      {
        const o = origin(); await clear(o); await setblock(call, at(o), block);
        await merge(call, at(o), '{Items:[{Slot:3b,id:"minecraft:cobblestone",count:1}]}');
        await until(call, at(o), 'has_output', 'true');
        await record(out, 'theurgy:fermentation_vat_has_output_true', call, o);
      }
      {
        const o = origin(); await clear(o); await setblock(call, at(o), block);
        await merge(call, at(o), '{Items:[{Slot:3b,id:"minecraft:cobblestone",count:1}]}');
        await wait(call, 5);
        await extract(call, at(o));
        await until(call, at(o), 'has_output', 'false');
        await record(out, 'theurgy:fermentation_vat_has_output_false', call, o);
      }
    }
  }
  return out;
}

export default {name: 'differential-vats', runCustom};
