# Проверки

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
