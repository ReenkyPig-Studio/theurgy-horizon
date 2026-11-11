# Контракты и цикл портирования

## Версии
Java upstream: klikli-dev/theurgy, ветка version/26.3, release/v26.3-1.120.1,
commit 33b959c9ed0da1de2aff649e9728c1a03470b1bd; NeoForge 26.3.0.76 / Minecraft 26.3.
Есть настоящий release asset theurgy-26.3-neoforge-1.120.1.jar.
Нюанс upstream: gradle.properties в теге ещё содержит mod_version=1.120.0; тег/JAR — 1.120.1.
См. upstream.lock.json: doctor проверяет commit, clean source и версии.
Ранее ошибочно использовалась default-ветка 26.1.2; старые checkpoints не evidence нового source pin.
Цель: Horizon / Minecraft 26.3. Rust — установленный launcher toolchain 1.99.1 GNU.
Pristine Horizon для native stage и game: `294448014fba585f8d5d09dd8886f482493b6417`
(nightly-2026-11-03-2944480). Исходник, executable и prepared resources должны совпадать;
старый c9d474d вызывал mismatch registry state order и больше не используется для игры.
Mixins: ddd9700b21997c95d95ac6ee5ecafe3830cf622a, CLI schema 6.
config.json — локальные пути (gitignored); шаблон — config.example.json.

## Перенос на другую ревизию Horizon
`npm run port -- <каталог исходников Horizon> [--mixins <exe>] [--apply]` (tools/horizon-port.mjs).
Копирует native/ в target/port/<rev> с переписанным pin, сверяет хуки с native-hooks.json цели,
запускает настоящие mixins plan/apply и cargo check, пишет runs/port-<rev>/report.json с ошибками,
привязанными к файлам native/theurgy. Оригинальные native/, pinned source и launcher не меняются.
Почти весь мод сидит на версионированных хуках и переносится без правок; ломается только код,
привязанный к внутренностям Horizon (`target =` и модули вне game.rs) — отчёт перечисляет их в `internals`.
`--apply` при чистой проверке переписывает pin в horizon-mixin.toml (package.mjs читает его оттуда);
config.json переключается вручную, после чего нужен новый acceptance: PORT_OK — это только "компилируется".

## Слои проверки
1. npm test — офлайн-тесты инструментов и контента.
2. npm run fast — только принятый pure Rust engine, без игры; 6 начальных регрессий.
3. Java differential oracle — следующий этап: воспроизвести одинаковые input/tick/reload
   traces на настоящем Theurgy и Rust, сравнить состояние, inventory, fluids и события.
   Только реальный Java процесс даёт oracle evidence. Переписанный на JS алгоритм им не является.
   Локальный Horizon tools/neoforge_oracle.py рассчитан на 1.21.1 / NeoForge 21.1.259:
   его нельзя без адаптации выдать за oracle для текущего Java 26.3.
4. node tools/native.mjs plan|stage|check|build — настоящий horizon-mixins + Cargo.
   Создаёт отдельный target/horizon-stages/<pid>, исходники launcher не меняет. Отдельный инкрементальный target/horizon-cargo.
   Сначала проверять core; тяжёлую игру пересобирать только после интеграции glue.
5. npm run game — настоящий vanilla Horizon, isolated world/socket/save; не проверяет Theurgy.
6. npm run game:theurgy — нативный executable после build; проверяет наличие блока theurgy:distiller.
   Пока контент/registry не портированы, этот тест должен FAIL, а не подменять блок vanilla.
   Даже PASS регистрации блока ещё не доказывает работу машины.
7. npm run game:chaos — seed 9731, 32 bounded шага в .local/chaos: повторные setblock,
   air/stone/glass/chest/hopper, чтение состояния после каждой записи и сохранение.
   Это настоящий vanilla game/control stability test, не проверка машин Theurgy и не автоисполнение agent code.
   Одновременно запускается один игровой процесс; 32 — лимит LLM, не 32 копии Minecraft.

## Дополнительная приёмочная проверка

Полный последовательный прогон (без candidate proposals, с общим native lock):

```powershell
npm run acceptance
```

Он сохраняет `runs/acceptance-<timestamp>/result.json` и завершается FAIL при
любом обязательном сбое или неполной differential matrix. Внутри одного прогона
свежие headless/GUI binaries собираются один раз и переиспользуются только для
этой же проверки; внешний `THEURGY_CACHE_BINARY` не подставляется. `--skip-java` и
`--skip-gui` предназначены только для диагностики и выдают `ACCEPTANCE_PARTIAL`,
а не финальный PASS. Для worldgen нужен свежий ненулевой Java trace:

```powershell
$env:THEURGY_WORLDGEN_ORACLE="runs/java-oracle-2026-11-07T21-43-52.670Z/java/worldgen.json"
$env:THEURGY_WORLD_SEED="0"
node tools/agent-harness.mjs custom:worldgen
```

Отдельные команды для отладки:

```powershell
node tools/java-oracle.mjs --trace
node tools/differential.mjs --all
node tools/multiplayer.mjs
node tools/agent-harness.mjs custom:filter-ui
node tools/agent-harness.mjs custom:recipes
node tools/agent-harness.mjs custom:render
node tools/package.mjs
node tools/package-smoke.mjs       # native + content именно из распакованного .hmod, isolated world
```

`tools/differential.mjs` читает реальные Java traces и только `diff:`-probes из изолированных Horizon миров. `--allow-partial` предназначен исключительно для разработки и не является приёмочным критерием. `tools/multiplayer.mjs` создаёт отдельные LAN host/guest homes и не использует реальные сохранения игрока.

## Следующие вертикальные срезы
- Сначала distiller: регистрации блока/item + рецепты + inventory transactions + heat + BE tick/save/load.
- Сценарии из Java DistillerGameTests: рецепт, нехватка ингредиента, полный output, потеря heat,
  save/reload посередине, hopper/sided transfer, block break drops, multiplayer sync.
- После каждого среза фиксировать Java trace и Horizon state evidence отдельно от review.
- Затем calcination/liquefaction, mercury/sulfur, fermentation/incubation/digestion/reformation,
  логистика, menus/rendering/guidebook/dependencies.
- Внешние GeckoLib/Modonomicon/JEI API не существуют автоматически в Horizon. Нужна замена/явный gap.

## Текущая граница
Интегрированы native block entities, контейнеры/танки, рецепты, все основные аппараты,
предметы, логистика, worldgen и клиентские native replacements. Последний Java/Horizon
контрольный срез содержит 152/152 MATCH и точный worldgen 17/17; GUI имеет отдельные
реальные screenshots для renderer, книги, фильтров, wand/HUD и localized tooltips.
Это ещё не финальное заявление об установке: полный fresh acceptance, LAN/package-smoke
и обновление evidence выполняются только на binaries, собранных из текущего дерева.
