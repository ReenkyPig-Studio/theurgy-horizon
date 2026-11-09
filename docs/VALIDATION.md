# Проверки

## Финальный принятый порт (2026-11-09)
- Единый прогон `runs/acceptance-2026-11-09T11-37-36.767Z/result.json` завершился
  **ACCEPTANCE_PASS**: 68/68 шагов с exit code 0, включая `npm test`, `npm run fast`,
  native plan/check/build, 24 headless-режима, Java `--trace`, GUI, LAN, differential,
  packaging и установку распакованного пакета.
- Java baseline: `runs/java-oracle-2026-11-09T12-20-59.621Z/result.json`,
  **JAVA_STATE_ORACLE_PASS**, 153 traced tests; SHA256 traces
  `b8b256f6f9f4260bd470675b67099decc43101887a233980b3baf628fa362005`.
  `artifacts/differential.json`: **152/152 MATCH, 0 MISSING_RUST, 0 MISMATCH, complete=true**.
- `docs/FEATURES.md` и `artifacts/feature-status.json`: **135/135 VERIFIED**, без
  PORTED-UNVERIFIED/PARTIAL/UNPORTED. Связи проверяются `tools/feature-evidence.mjs`;
  отсутствующий run/log/ID не даёт статуса VERIFIED.
- Все 24 headless-режима успешны: основные аппараты, chaos, interactions, item/flux/logistics,
  survival tools/divination, filters/content rules, incubation, real persistence/reload,
  reformation, fluids, wire graph, advanced logistics, все list/attribute rules и vat
  interactions. Run ID каждого режима сохранены в `artifacts/gui-evidence.json`.
- Worldgen `runs/game-1794227872201`: 44 observations, 17/17 ore positions против
  изолированной Java fixture, обе ore forms, persistence и запрет regeneration старых чанков.
- GUI визуально проверен после прогона: renderer `runs/game-1794228087282` (87 observations)
  показывает модели и reformation trails; particles/wires/fit/HUD `runs/game-1794228220578`;
  wand wheel/modes `runs/game-1794228183851`; localized tooltip `runs/game-1794228260776`;
  Hermetica, recipe catalog, filters и derivative HUD также успешны.
- LAN `runs/multiplayer-1794228295294`: **THEURGY_MULTIPLAYER_PASS**, 17 observations,
  authoritative guest edits, invalid-rule rejection и operator boundary.
- Пакет `artifacts/theurgy-26.3-1.120.1-rust-horizon.hmod`: 3524 entries, 5 326 944 bytes,
  SHA256 `853ec9661d42f797f99580ba20c78026dec208a1f1e5f49f3df6bc8d7eee687d`.
  Два последовательных `tools/package.mjs --no-build` дали идентичный архив.
- `artifacts/package-smoke.json`: **PACKAGE_SMOKE_PASS**. Пакет распакован, Rust native source
  из него прошёл check/build и запустил `custom:filters` в `.local/package-smoke/game`;
  evidence `runs/game-1794229001292`, log `runs/package-smoke-2026-11-09T12-46-20.870Z`.
- Публикация в registry остаётся отдельным ограничением Horizon (`native` registry support),
  но локальный установленный пакет и чистый запуск подтверждены. Финальный пункт `todo.md`
  отмечен только после этих результатов.

Разделы ниже — история промежуточных прогонов. Они не заменяют и не отменяют этот финальный
acceptance; старые «ещё не проверено» относятся только к своим датированным checkpoints.

## Исторический контрольный срез (2026-11-08, полная differential матрица)
- Последний полный Java/NeoForge oracle: `runs/java-oracle-2026-11-07T21-43-52.670Z/result.json`,
  `JAVA_STATE_ORACLE_PASS`, 153 traced tests (152 обязательных поведения + auxiliary worldgen).
  `LegacyRandomSource(1).nextInt(16) = 11,1,6` подтверждён отдельным Java probe.
- `artifacts/differential.json`: **152/152 MATCH, 0 MISSING_RUST, 0 MISMATCH**.
  Отчёт содержит пути ко всем свежим isolated-game evidence и SHA Java traces. Подтверждены
  calcination, liquefaction, distiller, vats, brazier, flux/capacitor/catalyst/emitters,
  incubator, logistics/nexus/proxy/probe, reformation/pedestals, placements и все три
  sulfur-attuned divination recipes. Это состояние матрицы на указанных evidence, а не
  автоматическое доказательство чистой финальной установки.
- Worldgen: `runs/game-1794119650821`, 44 observations; все **17/17** рудных позиций
  совпали с isolated Java fixture `runs/java-oracle-2026-11-07T21-43-52.670Z/java/worldgen.json`.
  Совпали seed, chunk RNG, origins, replacement targets и обе ore forms. Natural ore persistence
  и запрет повторной генерации старых чанков также пройдены.
- Divination crafting: `runs/game-1794119934894`, три recipe probes, **3/3 MATCH**.
  Авторитетные per-sulfur shaped recipes сохраняют `divination_linked_block/tag`, включая
  `minecraft:dirt` и `c:ores/copper`; локальный client callback больше не требуется.
- Runtime config override: `runs/game-1794119599463`, 10 observations; isolated
  `scenarios/client-features-config.json` отображает sal ammoniac ore в `minecraft:stone`.
  Conditioned content rules: `runs/game-1794119606372`, 16 observations.
- Изолированный LAN host/guest: `runs/multiplayer-1794119236742`,
  `THEURGY_MULTIPLAYER_PASS`, 17 observations. Проверены authoritative filter edits,
  отклонение invalid rule и operator-permission boundary.
- Остались fresh GUI/regression/package acceptance и финальная clean native сборка после
  удаления исследовательского attunement-hook. Пункт «Мод на 100% перенесён на раст»
  по-прежнему не отмечен.

## Последний чистый кандидат (2026-11-08)
- Свежая GUI-сборка без диагностических `TOOLTIP_*` маркеров: `runs/game-1794131810522`,
  `custom:tooltips`, 16 observations. Survival inventory screenshot действительно показывает
  `Glass Divination Rod` и `The divination rod is attuned to Sal Ammoniac Ore`; hover начинается
  с пустой области, чтобы не поднимать stack в курсор. Чтение linked component использует
  authoritative stack NBT, а block id локализуется через `block.<namespace>.<path>`.
- `npm test`: 37/37; `npm run fast`: PASS; `npm run test:modules`: 40/40. В
  `tools/module-tests.mjs` исправлен поиск Windows link-search paths при использовании
  `target/native-agent-cache`; это tooling fix, не ослабление assertions.
- Content: 480 accepted / 788 conditionally disabled recipes, 108 attunement recipes,
  32 blocks, 393 items, 29 block entities, 389 NeoForge tag files, Hermetica 3/96/392.
- Два подряд `tools/package.mjs --no-build` дали одинаковый архив: 3524 entries,
  SHA256 `39275a3e1f883fbb80c0021e807045ae14968deb7b1968a972ef6ce8854cadaf`.
  Это package checkpoint до завершения полного acceptance и clean-package smoke.
- Wand GUI `runs/game-1794131750933` (45 observations): настоящий wheel input,
  server-authoritative direction/enabled/frequency/capacitor side modes. `custom:client-features`
  `runs/game-1794131761286` (37 observations) дополнительно подтверждает видимые particles,
  wires, fit outlines, HUD и source display.
- Divination timed use: linked rod больше не заявляет click до `start_mod_use`, поэтому
  `native_living_entity_use_item_finish` действительно вызывается. Изолированный nearest-ore
  fixture `runs/game-1794130123158` (23 observations) нашёл `[40,88,40]` с 2D bucket 0;
  естественные руды в fixture airspace очищены, ожидаемая Java nearest-result семантика не изменена.
- Incubator completion regression: `runs/game-1794138818034` подтверждает output вместе с
  `isProcessing=1`; после исправления `artifacts/differential.json` снова 152/152 MATCH.
  `tools/acceptance.mjs` теперь вызывает Java oracle с `--trace`, поэтому fresh state baseline
  обязателен и не подменяется 148 smoke tests.
- Renderer re-review: исправлены texel-координаты, повторное деление размеров кубов и
  fallback для `draw.light == 0`; `runs/game-1794130299084` (65 observations) содержит
  реальные цветные GeckoLib/native screenshots. Старые невалидные screenshots не используются
  как evidence. Полный acceptance и clean-package smoke ещё обязательны.
## Актуальный native/headless прогон (Horizon 2944480)
- Native source/executable/content используют один pin `294448014fba585f8d5d09dd8886f482493b6417`.
- `node tools/native.mjs build-headless`: реальный fast build с `--no-default-features`, без запуска GUI.
- Все `tools/game.mjs` режимы по умолчанию добавляют `--headless`: нет окна/GPU/renderer,
  но выполняются настоящий integrated server, isolated world, content registry и native hooks.
- `node tools/headless.mjs all --no-build`: `THEURGY_ALL_MACHINES_PASS`, 65 observations
  (calcination, liquefaction, fermentation, digestion, accumulation, reformation, incubation),
  evidence `runs/game-1793843230536/evidence.json`.
- `node tools/headless.mjs machines --no-build`: `THEURGY_MACHINE_SCENARIOS_PASS`, 49 steps,
  включая heat/cold/invalid input/output/save/reload/break conservation,
  evidence `runs/game-1793753303519/evidence.json` (до последующего harness-only улучшения break evidence).
- `node tools/headless.mjs chaos-machines --no-build`: `THEURGY_MACHINE_CHAOS_PASS`, seed 9732, 24 steps,
  evidence `runs/game-1793752886632/evidence.json`.
- `node tools/headless.mjs interactions --no-build`: `THEURGY_INTERACTIONS_PASS`, 35 observations,
  evidence `runs/game-1793843393444/evidence.json`: настоящие give/place/break, обе половины
  double-high, spill содержимого ровно один раз, запрет occupied-upper без потери предмета.
- После интеграции mercurial wand на чистых isolated worlds: items 16 observations
  (`runs/game-1793843074505`), copper-wire endpoints 30 (`runs/game-1793843117638`),
  divination scanning 15 (`runs/game-1793843182004`).
- `runs/secure-wand`: candidate SHA256 `024621bd3e4d1f6af0c731c79efd4352a0c4b22904974646b12e595830c5d428`,
  scenario SHA256 `045951a81412f79de8b96ec8f375f6f1279c202de3783b4d1716af63ea781424`,
  STATIC_PASS → AI_ALLOW → LOCAL_APPROVED → VERIFIED. Изолированный real-game сценарий:
  `THEURGY_CUSTOM_PASS mode=wand observations=40`, evidence
  `.agents/secure-secure-wand/wand/runs/game-1793842089878/evidence.json`.
  Проверены Java string codec режимов, Shift+wheel, direction/FACING, enable/disable,
  selected frequency и capacitor UP OUTPUT→BOTH. `scroll-at` требует окно, поэтому только
  этот trusted harness запускается с GUI; вызов того же сценария в headless не является PASS.
- Настоящий Java/NeoForge `runGameTestServer` без GUI: `JAVA_GAMETESTS_PASS`, 148/148 upstream tests,
  pinned source 33b959c…, evidence `runs/java-oracle-2026-11-04T01-41-51.953Z/result.json`.
  Это Java baseline, не Rust differential PASS. Запуск: `npm run java:oracle`.
- Runtime matrix `runs/runtime-main`: 8 coding agents в изолированных worktrees, owned files,
  обязательный locked native build + headless game; готовые patches появляются в `runs/runtime-main/patches`.
  Параллельные checkpoint writes сериализованы после обнаруженного Windows rename race.
- `runs/secure-wire-graph`: candidate SHA256 `bea4e1b0f3005775f5c73be65cfe563e1e8da26ee445efb3ba9f6b90326fd415`,
  VERIFIED; Java wire `{from:Long,to:Long}` graph, connect/disconnect, save/reload и frequency
  isolation без `nexusId` shortcut, 56 observations, evidence
  `.agents/secure-secure-wire-graph/wire-graph/runs/game-1793846117479/evidence.json`.
- `runs/secure-filters`: candidate SHA256 `a34d5146899c553fb6dbfde31bf6cd906ba033039b2a3c29aae4b2c9d2306264`,
  VERIFIED; filter count-one attachment/removal, preserved custom name, `filter` NBT, reload и
  `has_filter`, 23 observations, evidence
  `.agents/secure-secure-filters/filters/runs/game-1793848533588/evidence.json`.
  Это не доказательство filter GUI, attribute/list matching или fluid filtering.
- `runs/secure-filter-kabuai`: provider `kabuai/deepseek-v4.1-flash-1017`, candidate SHA256
  `08690facc7e1eacbdf3b06b6451ed594d8116bb3f44dd12e839a768bb72a6551`, VERIFIED.
  CodeDefinedGUI 1.14 list filters: allow/deny на extractor и inserter, exact components,
  water fluids; 128 observations, evidence
  `.agents/secure-secure-filter-kabuai/filter-matching/runs/game-1793902171950/evidence.json`.
  Main повтор: `runs/game-1793903267826`. Attribute filters и их GUI отдельно не проверены.
- `runs/secure-attribute-filters`: provider `kabuai/deepseek-v4.1-flash-1017`, candidate SHA256
  `4ca41ffe1c5ccc871442a2cc4f485cd7260590c1fc0f1ec61de8f28434ec4314`, VERIFIED.
  Attribute `match_any/match_all/deny`, inverted rules, `in_tag` и standard `renamed`:
  82 observations, evidence `.agents/secure-secure-attribute-filters/attribute-filters/runs/game-1793907212768/evidence.json`;
  main `runs/game-1793908228458`. Остальные attribute kinds и GUI не заявлены.
- `runs/secure-standard-rules`: provider `kabuai/deepseek-v4.1-flash-1017`, candidate SHA256
  `1aaac85dedb8c86dccb7cb6941b836fc01517f13411118781163a4a418afe6ec`, VERIFIED.
  Все standard attribute kinds: placeable/consumable/fluid/enchant/name/damage/stack/equip/fuel/
  smelting/smoking/blasting/compost, 276 observations, evidence
  `.agents/secure-secure-standard-rules/standard-attributes/runs/game-1793917122188/evidence.json`;
  main `runs/game-1793918311191`. Отдельно остаются added_by/enchantment/fluid_contents/item_name
  и GUI. Rustc потребовал `RUST_MIN_STACK=33554432` для большого injected crate.
- `runs/secure-special-gemini`: candidate SHA256 `6eb31ef122a51591c76ce91681a9a76c391765ab790795d833769fd209173904`,
  VERIFIED. `added_by`, `enchantment`, `fluid_contents`, `item_name`: 86 observations,
  evidence `.agents/secure-secure-special-gemini/special-attributes/runs/game-1793922825787/evidence.json`;
  main `runs/game-1793923962257`. Standard 276 и list/attribute regression также PASS.
- `npm run test:modules`: 40/40 pure tests через `tools/module-tests.mjs` (logistics 21,
  worldgen 2, selection 3, interactions 2, items 5, registry 7); wrapper сохраняет production
  NBT/algorithm helpers/tests, опуская только требующие реального Game функции. Это не
  замена game evidence. Java/Horizon pack и wire regression после интеграции: filters 23
  (`runs/game-1793849048665`), wire graph 56 (`runs/game-1793849049528`), tools 30
  (`runs/game-1793849146637`), nexus logistics 32 (`runs/game-1793849143107`), machines 65
  (`runs/game-1793849210474`), interactions 35 (`runs/game-1793849349928`).
- После wand/provider изменений: `npm test` 34/34; `npm run fast` 28/28 Rust tests + doc-tests;
  `node tools/native.mjs check-headless` и `build-headless` PASS.
- Ранее recipes с `neoforge:conditions` молча отбрасывались Horizon до загрузки mod tags;
  `tools/build-content.mjs` теперь удаляет только conditions, сохраняя ingredients/results/times.
- Это подтверждённый игровой vertical slice всех основных аппаратов, но не автоматическое
  доказательство 100% parity всех item interactions, logistics, rendering и Java GameTests.

## Review-исправления Java 26.3 (Horizon 2944480)

- Выдуманные apparatus-меню удалены полностью: `native/theurgy/screens.rs`, оба screen-hook
  и JSON `data/theurgy/horizon/menu/*`. Вместо них `interactions.rs` переносит
  `TwoSlotItemHandlerBehaviour`, `DynamicOneOutputSlotItemHandlerBehaviour` и
  `OneSlotItemHandlerBehaviour`: ПКМ вставляет допустимый ресурс, пустая рука забирает
  output, затем input, без открытия экрана и с сохранением компонентов.
- `registry.rs` теперь повторяет `CapabilityRegistry`: у calcination/distillation/
  liquefaction/fermentation/digestion UP=только входы, DOWN=только output, горизонтальные
  стороны=combined inventory. Обычные input inventories доступны на извлечение, но нижний
  hopper физически не видит их. Живой сценарий подтвердил, что под дистиллятором забирается
  только `mercury_shard`, а `bread` во входе остаётся.
- `selection.rs` — общий перенос `SelectionBehaviour`: ПКМ/ЛКМ, сброс при смене предмета,
  диапазоны 8/16, замена единственной target/result точки, Java `blockPos` NBT и запись
  выбора при установке. Pending state изолирован по UUID игрока. `MercuryFluxEmitter`
  отправляет ровно 10 flux каждые 20 тиков только выбранной точке; `Reformation` читает
  только `sourcePedestals`/`targetPedestal`/`resultPedestal`, без поиска по миру.
- `theurgy:tag` в incubation разрешается через первый зарегистрированный предмет в порядке
  тега (`TagUtil`/`TagRecipeResult`), включая count/components; пустой тег даёт Java barrier,
  а не молча пустой результат. Проверено производство `theurgy:sal_ammoniac_crystal`.
- Нативный worldgen подключён через `chunk_load(new=true)` только в Overworld. Сломанная
  ссылка dev-mod `theurgy:sal_ammoniac_ore` удалена из generator-facing datapack; вклад
  выполняет `worldgen.rs` с ограниченными 20×9 попытками, world-seeded deterministic RNG,
  tag-match replacement и записью только новых чанков. Обычная руда найдена в живом мире
  и пережила reload без повторной генерации старых чанков.
- `DivinationRodItem`/`Scanner`: attunement проверяет vanilla
  `ToolMaterial.incorrectBlocksForDrops`; linked block раскрывается через первый непустой
  `c:ores/...` (two-word name имеет приоритет) либо дополняется deepslate-вариантом. Скан
  ограничен сферой Java range 96, выбирает ближайший результат по 3D, но записывает display
  distance по 2D; `divination_pos` сохраняется как `BlockPos.CODEC` IntArray, промах даёт 7.0.

Фактические проверки этой волны:

- `node tools/native.mjs build-headless` — PASS, 18 modules / 15 injections, отдельный
  `target/horizon-headless-cargo/fast/horizon-client.exe`; `node tools/native.mjs plan` — PASS.
- `custom:review-fixes`: `THEURGY_CUSTOM_PASS`, 86 observations,
  `runs/game-1794011604590/evidence.json`. Реальные ПКМ-инвентари без меню, top/bottom
  hopper, tag-result, выбор/установка mercury и sulfuric emitters, запрет unselected target.
- `custom:worldgen`: 15 observations, `runs/game-1794012874723/evidence.json`.
  В pinned test terrain контрольный `find-block minecraft:deepslate` = 0, поэтому live
  deepslate-target честно помечен `skipped`; подменять stone на deepslate-руду запрещено.
  Правило обоих Java targets отдельно проверено unit-тестом worldgen.
- Финальный binary: reformation 129 (`runs/game-1794011829744`), flux 33
  (`runs/game-1794011781043`), all machines 66 (`runs/game-1794012127773`).
- Дополнительно: machine scenarios 49 (`runs/game-1794012330177`), interactions 35
  (`runs/game-1794011719881`), incubation chaos 7 (`runs/game-1794012408063`),
  items 16 (`runs/game-1794011664163`). Divination 17 (`runs/game-1794011439287`)
  отдельно доказывает поиск только deepslate-варианта после удаления исходной руды
  и нулевой 2D-distance bucket при вертикальном смещении.
- `npm test` 34/34; `npm run fast` 28/28 + policy tests; `npm run test:modules`
  40/40 (logistics 21, worldgen 2, selection 3, interactions 2, items 5, registry 7).

Это доказывает исправление перечисленных дивергенций в реальном Horizon, но не превращает
их в полный Java differential всего мода. Полный parity остальных рендереров, внешних
модов и всех item interactions по-прежнему требует отдельных сценариев/oracle.

## История matrix-кампании
- Все 56 ролей завершены и имеют per-role journal: 24 builders, 24 chaos-testers, 8 reviewers.
- Сохранены 48 предложений Rust (24 реализации + 24 adversarial test modules).
- Это НЕ 48 принятых модулей: candidate code не компилировался, не исполнялся и не подключался к игре.
- Реальный pinned Java source: release/v26.3-1.120.1 @ 33b959c9ed0da1de2aff649e9728c1a03470b1bd.
- Повторное resume восстановило 56 ролей и сделало 0 API requests; см. runs/port-26.3-main/resume-noop.log.
- При разработке resume были найдены и исправлены ошибки порядка skip и ссылки на ledger.
  Добавлены проверка готовой роли ДО running, общий mutable ledger при resume, .previous snapshots,
  история retry и recovery evidence. Первые отладочные resume журналы/stdout сохранены в run;
  recovery/controller-bug.json документирует ограничение старого учёта стоимости.
- Текущие 19/19 инфраструктурных тестов и 6/6 принятых Rust unit tests passed.
- Машинный manifest: artifacts/port-campaign.json. Его статус GENERATED_AND_SAVED_ONLY.
- Следующий этап: ревью/сборка candidates, differential Java 26.3 tests, native build, отдельный игровой мир.
  Текущая полная волна сама по себе не доказывает gameplay parity.


## Обновление: исходник 26.3, chaos-тестеры, 32 одновременно
- Подтверждены remote-ветка version/26.3 и GitHub release/v26.3-1.120.1 с настоящим JAR.
- upstream/theurgy закреплён на 33b959c9ed0da1de2aff649e9728c1a03470b1bd.
- doctor: commit/clean source/Minecraft 26.3/NeoForge 26.3.0.76 — OK.
- 56 ролей: 24 builders + 24 paired chaos-testers + 8 reviewers, concurrency=32.
- npm test: 14/14 passed, включая фактический peak=32 в offline pool test,
  ровно одного тестера на builder, own-output/target SHA и source-version gate.
- npm run fast: 6/6 Rust-тестов passed после переключения исходника.
  CraftingBehaviour.java между старым pin и этим релизом не изменился (git diff пуст).
- npm run game:chaos: VANILLA_CHAOS_PASS seed=9731, 32 шага.
  Реальная игра, отдельный .local/chaos; evidence: runs/game-1793738133548/evidence.json.
- Live smoke builder + tester + reviewer: runs/2026-11-03T20-37-11.483Z/checkpoint.json,
  все 3 роли завершились, 3 API requests, 2 unaccepted proposals; SDK estimate $0.009652632.
  Предыдущий smoke тестера был отвергнут placeholder-gate, не интегрирован.
- Это не запуск 32 платных сессий: предел проверен offline, live smoke — три роли.
- Agent-generated adversarial tests не компилировались/не исполнялись и не считаются PASS.
  Автоматически исполнялся только trusted vanilla chaos plan, не тест машин Theurgy.

## Историческая первоначальная проверка (default-ветка 26.1.2)
Эти live checkpoints относятся к старому source pin, не подтверждают порт версии 26.3.

- Rust toolchain: rustc 1.99.1 GNU из horizon-launcher.
- pi SDK 1.4.0, официальный deepseek-flash, тот же scoped credential, без печати ключа.
- npm test: 7/7 passed (board ownership, quotas, immutable snapshot, placeholder rejection).
- npm run fast: 6/6 Rust regression tests passed.
- npm run check: pure Rust engine passed.
- horizon-mixins plan: OK, 1 crate + 1 injection.
- horizon-mixins apply: OK, isolated target/horizon-stage.
- Cargo check --locked --offline -p theurgy_engine внутри настоящего staged Horizon workspace: OK.
- npm run game: VANILLA_HORIZON_SMOKE_PASS.
  Evidence: runs/game-1793737213630/evidence.json; actual Horizon 26.3,
  isolated TheurgyLab, block write/read, save, quit.
- Последний live worker/reviewer smoke: runs/2026-11-03T20-29-49.129Z/checkpoint.json,
  10 API requests, 1 unaccepted Rust candidate, independent text review.
  SDK-priced estimate: $0.015093648 для этого прогона, НЕ сумма всей отладки.
- При отладке были также неудачные bounded прогоны (исследователь исчерпывал квоту,
  один ранний proposal был PLACEHOLDER и непригоден). Не интегрированы.
  Теперь delivery — отдельный tool-free one-request сеанс после bounded research;
  placeholders отвергаются структурным gate. Research quota exhaustion сохраняется отдельно.

## Текущие проверенные результаты (pin 26.3)

- `runs/java-oracle-2026-11-07T12-52-30.730Z`: **JAVA_STATE_ORACLE_PASS**, 153/153 traced tests (152 behaviour + worldgen index), 0 failed. Включает block/BE/item traces, recipe outputs, UUID и capability capacities.
- `runs/multiplayer-1794068703456`: **THEURGY_MULTIPLAYER_PASS**, 17 observations. Isolated LAN host + guest; guest filter edit is authoritative on host, invalid rule rejected, operator probe remains unavailable with cheats disabled.
- `runs/game-1794065368573`: **THEURGY_CUSTOM_PASS `custom:filter-ui`**, 30 observations. List/attribute filter screens, ghost item add/remove, mode/component changes, server-authoritative SelectedItem NBT.
- `runs/game-1794064883182`: **THEURGY_CUSTOM_PASS `custom:differential-distiller`**, 11 observations; все 11 distiller behaviours отмечены `MATCH` в `artifacts/differential.json`.
- `runs/game-1794061635832`: **THEURGY_CUSTOM_PASS `custom:differential-liquefaction`**, 15 observations; все 15 behaviours отмечены `MATCH`.
- `runs/game-1794060708456`: **THEURGY_CUSTOM_PASS `custom:differential-calcination`**, 13 observations; все 13 behaviours отмечены `MATCH`.
- `runs/game-1794060926105`: **THEURGY_CUSTOM_PASS `custom:differential-pedestals`**, 8 observations; reformation pedestal/emitter behaviours отмечены `MATCH`.

Остальные группы ещё проходят проверку. Эти записи не означают завершение `todo.md` целиком и не отмечают последний пункт.

## Историческая начальная неготовность (снята поздними прогонами)
- Полная компиляция horizon-client с glue и игровой bootstrap: не завершена этим сетапом.
- Весь рой 32 ролей: конфигурация готова, в smoke запускалась пара, не все 32.
- Candidate code не был принят, скомпилирован или исполнен.
- Java 26.3 differential oracle ещё не реализован.
- theurgy:distiller не зарегистрирован. game:theurgy ожидаемо FAIL до интеграции контента.
- Никакие машины, inventories, GUI, ассеты и полный gameplay Theurgy не объявляются готовыми.
