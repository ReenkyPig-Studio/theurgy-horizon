// ReformationPedestalGameTests insertion/rejection/extraction behaviours.
import {at, record, setblock, container, merge, wait, probe} from '../differential/harness.mjs';

async function interact(call, pos, item = null) {
  await call(['command', 'clear', '@s']);
  if (item) await call(['give', item, '1']);
  await call(['command', 'theurgy_interact', ...pos.map(String), 'up']);
  await wait(call, 2);
}

async function until(call, pos, predicate, description, timeout = 400) {
  for (let elapsed = 0; elapsed < timeout; elapsed += 10) {
    await wait(call, 10);
    if (predicate(await probe(call, pos))) return;
  }
  throw new Error('Timed out waiting for ' + description);
}

export async function runCustom(call) {
  const out = [];
  let index = 0;
  const origin = () => [(index++) * 16, 80, 0];
  {
    const o = origin(); await setblock(call, at(o), 'theurgy:reformation_source_pedestal');
    await container(call, at(o), 'theurgy:alchemical_sulfur_bone', 1);
    await record(out, 'theurgy:reformation_source_insert', call, o);
  }
  {
    const o = origin(); await setblock(call, at(o), 'theurgy:reformation_source_pedestal');
    await interact(call, at(o), 'theurgy:sulfuric_flux_emitter');
    await record(out, 'theurgy:reformation_source_rejects_invalid_held_item', call, o);
  }
  {
    const o = origin(); await setblock(call, at(o), 'theurgy:reformation_source_pedestal');
    await merge(call, at(o), '{Items:[{Slot:0b,id:"theurgy:alchemical_sulfur_bone",count:1}]}');
    await interact(call, at(o));
    await record(out, 'theurgy:reformation_source_extract', call, o);
  }
  {
    const o = origin(); await setblock(call, at(o, [3, 2, 2]), 'theurgy:reformation_target_pedestal');
    await container(call, at(o, [3, 2, 2]), 'theurgy:alchemical_sulfur_bone', 1);
    await record(out, 'theurgy:reformation_target_insert', call, o, [[3, 2, 2]]);
  }
  {
    const o = origin(); await setblock(call, at(o, [4, 2, 2]), 'theurgy:reformation_result_pedestal');
    await merge(call, at(o, [4, 2, 2]), '{Items:[{Slot:0b,id:"minecraft:cobblestone",count:1}]}');
    await interact(call, at(o, [4, 2, 2]));
    await record(out, 'theurgy:reformation_result_extract', call, o, [[4, 2, 2]]);
  }
  for (const mode of ['link', 'process', 'insufficient']) {
    const o = origin();
    await call(['fill', String(o[0] - 5), '78', '-5', String(o[0] + 5), '86', '5', 'minecraft:air']);
    const emitter = [2, 1, 2], target = [2, 1, 0], source = [0, 1, 0], result = [4, 1, 0];
    await setblock(call, at(o, emitter), 'theurgy:sulfuric_flux_emitter');
    await setblock(call, at(o, target), 'theurgy:reformation_target_pedestal');
    await setblock(call, at(o, source), 'theurgy:reformation_source_pedestal');
    await setblock(call, at(o, result), 'theurgy:reformation_result_pedestal');
    const sourcePos = at(o, source), targetPos = at(o, target), resultPos = at(o, result);
    await merge(call, at(o, emitter), `{sourcePedestals:[{blockPos:[I;${sourcePos.join(',')}],type:"SOURCE"}],targetPedestal:{blockPos:[I;${targetPos.join(',')}],type:"TARGET"},resultPedestal:{blockPos:[I;${resultPos.join(',')}],type:"RESULT"}}`);
    const locals = [emitter, target, source, result];
    if (mode === 'link') {
      await record(out, 'theurgy:sulfuric_flux_emitter_linking', call, o, locals);
      continue;
    }
    await container(call, at(o, source), 'theurgy:alchemical_niter_mobs_abundant', 1);
    await container(call, at(o, target), 'theurgy:alchemical_sulfur_bone', 1);
    if (mode === 'process') {
      await merge(call, at(o, emitter), '{mercuryFluxHandler:{energy:500}}');
      await until(call, at(o, result), state => state.items.some(stack => stack.id === 'theurgy:alchemical_sulfur_bone'), 'reformation result');
      await record(out, 'theurgy:sulfuric_flux_emitter_processing', call, o, locals);
    } else {
      await wait(call, 4);
      await record(out, 'theurgy:sulfuric_flux_emitter_insufficient_mercury_flux', call, o, locals);
    }
  }
  return out;
}

export default {name: 'differential-pedestals', runCustom};
