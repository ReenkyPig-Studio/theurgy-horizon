# Third-party notices

## Theurgy

- Upstream: `klikli-dev/theurgy`, `release/v26.3-1.120.1`, commit `33b959c9ed0da1de2aff649e9728c1a03470b1bd`.
- Code and assets retain the upstream `LICENSES/` directory and `REUSE.toml` in the generated content addon.
- GeckoLib models, animations, book pages, translations and other assets retain their original attribution; CC-BY-4.0 assets are not relicensed.

## NeoForge 26.3.0.76

- The generated addon includes functional JSON item/block tag definitions from the pinned `neoforge-26.3.0.76-universal.jar` so recipes using `#c:*` resolve at runtime.
- Pinned artifact metadata declares **LGPL 2.1** (`<name>LGPL 2.1</name>`), with the license URL recorded in the POM: <https://github.com/NeoForged/NeoForge/blob/1.13-pre/LICENSE.txt>.
- Source/project: <https://github.com/NeoForged/NeoForge>. The local POM is `.local/gradle/caches/modules-2/files-2.1/net.neoforged/neoforge/26.3.0.76/`.
- Only data tag JSON is redistributed; NeoForge code is not linked into the pure Rust engine or presented as original work. The build records the copied file count and the local deterministic build also performs its own content-policy check.

No provider credentials, player saves or launcher instance data are included in the addon or these notices.
