import test from 'node:test';
import assert from 'node:assert/strict';
import {readPin,writePin,missingHooks,mapToMod,internals} from '../tools/horizon-port.mjs';

const toml='[compatibility]\nhorizon_revision = "'+'a'.repeat(40)+'"\n\n[[injection]]\nid = "x"\nhook = "frame"\nhook_version = 1\n\n[[injection]]\nhook = "gone"\nhook_version = 2\n';

test('pin is read and rewritten in place',()=>{
  assert.equal(readPin(toml),'a'.repeat(40));
  assert.equal(readPin(writePin(toml,'b'.repeat(40))),'b'.repeat(40));
});
test('hooks missing by name or version are all listed',()=>{
  assert.deepEqual(missingHooks(toml,{hooks:[{name:'frame',version:1},{name:'gone',version:1}]}),['gone@2']);
});
test('staged lines map back to the mod source, Horizon lines do not',()=>{
  const staged='fn horizon() {\n}\npub(crate) mod theurgy_host {\n    fn a() {}\n    fn b() {}\n}\nfn after() {}\n';
  assert.deepEqual(mapToMod(staged,5),{file:'native/theurgy/host.rs',line:2});
  assert.equal(mapToMod(staged,5,'name = "theurgy_host"\nsource = "other.rs"\n').file,'native/theurgy/other.rs');
  assert.equal(mapToMod(staged,1),null);
  assert.equal(mapToMod(staged,7),null);
});
test('only non-hook targets and modules outside game.rs count as internals',()=>{
  assert.deepEqual(internals('file = "crates/horizon-client/src/game.rs"\nfile = "crates/x/src/y.rs"\nhook = "frame"\ntarget = "a::b"\n'),['crates/x/src/y.rs','a::b']);
});
