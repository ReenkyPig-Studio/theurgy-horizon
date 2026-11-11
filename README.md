# Theurgy for Horizon

A Rust port of [Theurgy](https://github.com/klikli-dev/theurgy) by klikli-dev to
**Horizon 26.3**. This repository is a fork of the original mod: the Java/NeoForge
sources were removed in the first commit of this branch and replaced by a native
Horizon implementation that reuses Theurgy's original assets and data.

- Ported from: [`release/v26.3-1.120.1`](https://github.com/klikli-dev/theurgy/releases/tag/release/v26.3-1.120.1),
  commit `33b959c9ed0da1de2aff649e9728c1a03470b1bd` (Minecraft 26.3, NeoForge 26.3.0.76), pinned in `upstream.lock.json`.
- Target: Horizon nightly-2026-11-03 (`2944480`), pinned in `native/theurgy/horizon-mixin.toml`.
- Not affiliated with or endorsed by klikli-dev. Please do not report issues with this port upstream.

## Installing

The mod is two files with the id `theurgy`; both are required:

| File | Contents |
| --- | --- |
| `theurgy-26.3-1.120.1-content.hmod` | resources: `horizon-mod.toml`, `assets/`, `data/` |
| `theurgy-26.3-1.120.1-native.hnmod` | native code: `horizon-native.json`, `mixins/theurgy/` (built by the launcher) |

They are produced by `npm run package` into `artifacts/` (not committed; attach them to a
release). Their SHA256 is recorded in `artifacts/package.json`.

## What is covered

Machines, items, logistics, recipes, world generation and the Hermetica guide book are ported.
JEI/EMI, Jade and KubeJS integrations are out of scope because those mods do not exist on Horizon.
The feature matrix is in [docs/FEATURES.md](docs/FEATURES.md); what was actually verified, and
how, is in [docs/VALIDATION.md](docs/VALIDATION.md) and `artifacts/*.json`
(the docs are written in Russian).

## Layout

| Path | Purpose |
| --- | --- |
| `content/theurgy/` | content addon (generated from upstream resources by `tools/build-content.mjs`) |
| `native/theurgy/` | Horizon glue (`*.rs`, `horizon-mixin.toml`) |
| `native/theurgy/engine/` | pure Rust core, testable without the game |
| `scenarios/`, `differential/`, `oracle/` | in-game scenarios and the Java differential oracle |
| `tools/`, `test/` | build, packaging and test tooling (Node.js, no npm dependencies) |
| `docs/` | porting notes, feature matrix, validation log |

## Building and testing

Requires Node.js, the Horizon launcher (it provides the Rust toolchain and `horizon-mixins`)
and a pristine Horizon source checkout.

```powershell
copy config.example.json config.json   # then set your local paths
git worktree add --detach upstream/theurgy release/v26.3-1.120.1   # read-only Java reference

npm run doctor          # check toolchain, Horizon and the upstream pin
npm test                # offline tool/content tests
npm run fast            # pure Rust engine tests, no game
npm run test:modules    # module tests of the native glue
npm run native:build    # real Horizon build with the mod
npm run game:headless   # all machines in an isolated headless world
npm run acceptance      # full acceptance: core + native + Java differential + GUI + LAN + package
npm run package         # write the .hmod / .hnmod into artifacts/
```

Game tests only use isolated `.local/<mode>` homes and worlds; they never touch player saves.
More detail: [docs/PORTING.md](docs/PORTING.md).

## Git remotes

`upstream` is `https://github.com/klikli-dev/theurgy.git`. The port lives on the
`horizon/26.3` branch, which starts at the upstream tag `release/v26.3-1.120.1`.

## Licensing

Same as upstream Theurgy: code is **MIT**, assets and data are **CC-BY-4.0**
(© klikli-dev); see `LICENSES/`, `REUSE.toml` and
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) for Modonomicon and NeoForge material.
The upstream changelog is kept as `CHANGELOG.md`.
