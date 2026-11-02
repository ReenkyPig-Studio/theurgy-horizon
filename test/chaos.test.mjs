import {test} from 'node:test';
import assert from 'node:assert/strict';
import {chaosPlan} from '../tools/chaos-plan.mjs';
test('chaos replay is deterministic and seed-sensitive',()=>{
 assert.deepEqual(chaosPlan(42),chaosPlan(42));
 assert.notDeepEqual(chaosPlan(42),chaosPlan(43));
});
test('chaos is bounded, no arbitrary commands or access to player saves',()=>{
 const plan=chaosPlan();
 assert.ok(plan.actions.length<=100);
 for(const a of plan.actions) {
   assert.ok(['setblock','block','save'].includes(a.args[0]));
   if(a.args[0]!=='save') {
     assert.ok(Number(a.args[1])>=0&&Number(a.args[1])<4);
     assert.equal(a.args[2],'80');
     assert.ok(Number(a.args[3])>=0&&Number(a.args[3])<4);
   }
 }
 assert.throws(()=>chaosPlan(42,65));
});
