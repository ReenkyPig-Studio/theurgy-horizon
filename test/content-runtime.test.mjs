import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';

test('NeoForge ingredient tags are present as runtime content', () => {
  const normal = JSON.parse(fs.readFileSync('content/theurgy/data/c/tags/item/cobblestones/normal.json', 'utf8'));
  assert.ok(normal.values.includes('minecraft:cobblestone'));
  const cobblestones = JSON.parse(fs.readFileSync('content/theurgy/data/c/tags/item/cobblestones.json', 'utf8'));
  assert.ok(cobblestones.values.includes('#c:cobblestones/normal'));
  const copper = JSON.parse(fs.readFileSync('content/theurgy/data/c/tags/item/ores/copper.json', 'utf8'));
  assert.ok(copper.values.includes('#minecraft:copper_ores'));
});

test('filter UI edits require server validation', () => {
  const client = fs.readFileSync('native/theurgy/filter_client.rs', 'utf8');
  const server = fs.readFileSync('native/theurgy/filter.rs', 'utf8');
  assert.ok(client.includes('click_command'));
  assert.equal(client.includes('set_other'), false);
  assert.ok(server.includes('native_attribute_filter'));
  assert.ok(server.includes('is_valid'));
  assert.ok(server.includes('the original filter is no longer held'));
});

test('content pack does not emit forbidden foreign namespaces', () => {
  assert.equal(fs.existsSync('content/theurgy/data/neoforge/tags'), false);
  assert.equal(fs.existsSync('content/theurgy/data/theurgy/horizon/hermetica.json'), false);
  const book = JSON.parse(fs.readFileSync('content/theurgy/data/theurgy/hermetica.json', 'utf8'));
  assert.equal(book.categories.length, 3);
  assert.equal(book.entries.length, 96);
  assert.equal(book.entries.reduce((count, entry) => count + entry.pages.length, 0), 392);
});
