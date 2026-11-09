# Контракты и цикл портирования

## Версии
Java upstream: klikli-dev/theurgy, ветка version/26.3, release/v26.3-1.120.1,
commit 33b959c9ed0da1de2aff649e9728c1a03470b1bd; NeoForge 26.3.0.76 / Minecraft 26.3.
Есть настоящий release asset theurgy-26.3-neoforge-1.120.1.jar.
Нюанс upstream: gradle.properties в теге ещё содержит mod_version=1.120.0; тег/JAR — 1.120.1.
См. upstream.lock.json: doctor и live runner проверяют commit, clean source и версии до API.
Ранее ошибочно использовалась default-ветка 26.1.2; старые checkpoints не evidence нового source pin.
Цель: Horizon / Minecraft 26.3. Rust — установленный launcher toolchain 1.99.1 GNU.
Pristine Horizon для native stage и game: `294448014fba585f8d5d09dd8886f482493b6417`
(nightly-2026-11-03-2944480). Исходник, executable и prepared resources должны совпадать;
старый c9d474d вызывал mismatch registry state order и больше не используется для игры.
Mixins: ddd9700b21997c95d95ac6ee5ecafe3830cf622a, CLI schema 6.
config.json — локальные пути, без секретов.

## Архитектура
56 ролей = 24 builders + 24 независимых chaos-testers (один на builder) + 8 reviewers.
Максимум 32 API-сессии одновременно. Первая волна — 24 builders,
вторая — 24 testers + 8 reviewers в одном ограниченном пуле, достигающем 32 parallel jobs.
Ревьюеры проверяют builders независимо и не выдают параллельно создаваемые tester proposals за проверенные.
Общий неизменяемый RAM snapshot Java/Horizon/native, RAM proposals, mailboxes 32 x 2000 символов.
Агенты не получают FS/network/shell. Каждый builder владеет одним кандидатом в engine/src/candidates/.
Два bounded этапа: до четырёх research-запросов с Matrix tools, затем один tool-free delivery-запрос.
Исчерпание research-квоты не запускает ретрай; сохранённые прочитанные источники передаются delivery.
Rust-кандидат проходит только текстовый структурный gate; это НЕ Rust parser или security sandbox.
Testers и reviewers стартуют после builder-волны; одно ревью покрывает три подсистемы.
Тестер читает исходный Java и candidate, генерирует adversarial Rust tests только в своём candidates/chaos/<id>.rs.
Проверяет дюпы/сохранение ресурсов, нули/переполнения, полный output, повторные tick/start/stop,
обрыв heat, замену входа, неправильные components, частичные transfer/rollback, save/reload и destruction.
Не меняет реализацию и не подгоняет assertions под баг. Неперенесённые host-сценарии — NOT EXECUTED.
Proposal привязан к SHA256 target; отсутствующий target означает BLOCKED без платного запроса тестера.
После trusted integration несовпадение target hash требует пересмотра тестов.
Длинные источники/кандидаты могут быть урезаны под quota; такое ревью обязано отметить BLOCKED,
а не выдавать неполный просмотр за одобрение.
Нет бесконечных ретраев, автоспавна или автоматического выполнения полученного Rust.

Snapshot читается страницами; каждая запись содержит SHA256. Proposal immutable и хранит baseHash.
Каждая завершённая роль сохраняется атомарно в runs/<campaign>/tasks/<id>.json,
затем обновляется checkpoint.json; перед заменой сохраняется <file>.previous.
При resume новые результаты используют тот же mutable ledger, а не его копию. Manifest фиксирует selection, source pin, task graph и request quota.
После падения: npm run swarm -- --resume latest (или runs/<campaign>).
Завершённые роли не повторяются; проверяются hash кода, ownership и target tester hash.
Неоконченные роли повторяются: неизвестно, успел ли провайдер принять последний запрос при внезапном сбое.
Failed/blocked сохраняются и не повторяются без явного --retry-failed (это новые платные попытки).
Resume нельзя смешивать с новыми --only/--agents/--requests или другим upstream pin.
Ctrl+C/SIGTERM сохраняет interrupted checkpoint. Изменение scheduler при том же task graph возможно;
изменение задач требует новой кампании. Журнал — не sandbox, код всё ещё untrusted.
Явные checkpoint содержат proposals, reviews, токены и ошибки; сессии pi на диск не пишутся.
Потребление provider-reported tokens и SDK-priced cost estimate — в ledger.
Это не гарантированный долларовый budget: верхний предел — число запросов и output tokens;
входной контекст тоже оплачивается. Для короткого запуска используйте --only и --requests.
У тестов и LLM разная параллельность: агенты не запускают компилятор.

## Слои проверки
1. npm test — ограничения/ownership/почта, без API.
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

## Интеграция proposals
node tools/proposal.mjs runs/<run>/checkpoint.json crafting
Проверить код и независимое review. Затем:
node tools/proposal.mjs runs/<run>/checkpoint.json crafting --approve-reviewed-code
Команда пишет только назначенный candidate и проверяет SHA/baseHash. Она не добавляет его в lib.rs.
Доверенный интегратор согласует общие типы, переносит нужный код в принятые модули,
подключает модуль явно, запускает Rust/native тесты. build.rs/dependencies/unsafe запрещены
для pure core. Candidate tests не исполняются просто из-за появления checkpoint.

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
