# Проверки

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
- После wand/provider изменений: `npm test` 34/34; `npm run fast` 28/28 Rust tests + doc-tests;
  `node tools/native.mjs check-headless` и `build-headless` PASS.
- Ранее recipes с `neoforge:conditions` молча отбрасывались Horizon до загрузки mod tags;
  `tools/build-content.mjs` теперь удаляет только conditions, сохраняя ingredients/results/times.
- Это подтверждённый игровой vertical slice всех основных аппаратов, но не автоматическое
  доказательство 100% parity всех item interactions, logistics, rendering и Java GameTests.

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

## Не проверено / не сделано
- Полная компиляция horizon-client с glue и игровой bootstrap: не завершена этим сетапом.
- Весь рой 32 ролей: конфигурация готова, в smoke запускалась пара, не все 32.
- Candidate code не был принят, скомпилирован или исполнен.
- Java 26.3 differential oracle ещё не реализован.
- theurgy:distiller не зарегистрирован. game:theurgy ожидаемо FAIL до интеграции контента.
- Никакие машины, inventories, GUI, ассеты и полный gameplay Theurgy не объявляются готовыми.
