// Real player-item interaction checks, designed for Horizon --headless.
// No screenshot, pixel click or client-only mock is involved.
const s=p=>p.map(String);
const name=r=>String(r?.state??r?.block??'');

export async function runInteractions(call,assert) {
  const out=[];
  const step=async(name,args,check)=>{
    const reply=await call(args);
    if(check)check(reply);
    out.push({name,reply});
    if(args[0]==='setblock'||args[0]==='place'||args[0]==='break')await call(['wait','2']);
    return reply;
  };
  const tp=async p=>step('approach '+p.join(','),['tp',...p.map(String)]);
  const block=async(p,expected)=>step('block '+p.join(','),['block',...s(p)],r=>{
    assert.ok(name(r).startsWith(expected),`expected ${expected}, got ${name(r)}`);
  });
  const place=async(p)=>{
    await step('clear lower '+p.join(','),['setblock',...s(p),'minecraft:air']);
    await step('clear upper '+p.join(','),['setblock',p[0],p[1]+1,p[2],'minecraft:air'].map(String));
    await tp([p[0]+0.5,p[1]+1,p[2]+2.5]);
    return step('place distiller '+p.join(','),['place',...s(p),'--item','theurgy:distiller']);
  };
  const put=async p=>step('seed '+p.join(','),['container',...s(p),'minecraft:bread','1']);
  const mine=async(p,upper)=>{
    await tp([p[0]+0.5,p[1]+(upper?2:1),p[2]+2.5]);
    await step('break '+p.join(','),['break',...s(p)],r=>{
      assert.equal(r.ok,true);
      assert.equal(r.drops?.['minecraft:bread'],1,'machine contents must spill exactly once');
      assert.equal(r.drops?.['theurgy:distiller']??0,0,'creative break must not drop the apparatus');
    });
  };

  await step('interaction lab',['tp','62','80','8']);
  await step('interaction chunks',['wait','40']);
  await step('distiller item',['give','theurgy:distiller','1']);

  // Upper half is created by the real BlockItem placement hook.
  await step('support A',['setblock','60','79','0','minecraft:stone']);
  await place([60,80,0]);
  await block([60,80,0],'theurgy:distiller');
  await block([60,81,0],'theurgy:distiller');
  await put([60,80,0]);
  await mine([60,81,0],true);
  await block([60,80,0],'minecraft:air');
  await block([60,81,0],'minecraft:air');

  // Breaking the lower half must remove the upper without a second drop.
  await step('support B',['setblock','64','79','0','minecraft:stone']);
  await place([64,80,0]);
  await block([64,80,0],'theurgy:distiller');
  await block([64,81,0],'theurgy:distiller');
  await put([64,80,0]);
  await mine([64,80,0],false);
  await block([64,80,0],'minecraft:air');
  await block([64,81,0],'minecraft:air');

  // Java getStateForPlacement rejects a non-replaceable upper position before
  // the lower half or item count is committed.
  await step('support blocked',['setblock','68','79','0','minecraft:stone']);
  await step('occupied upper',['setblock','68','81','0','minecraft:stone']);
  await step('clear blocked lower',['setblock','68','80','0','minecraft:air']);
  await tp([68.5,81,2.5]);
  let refused=false;
  try { await call(['place','68','80','0','--item','theurgy:distiller']); }
  catch { refused=true; }
  assert.ok(refused,'placement with an occupied upper half must fail');
  out.push({name:'occupied upper refused',reply:{refused}});
  await call(['wait','2']);
  await block([68,80,0],'minecraft:air');
  await block([68,81,0],'minecraft:stone');
  await step('item conserved',['inventory'],r=>{
    assert.ok((r.counts?.['theurgy:distiller']??0)>=1,'failed placement consumed the last item');
  });
  return out;
}
