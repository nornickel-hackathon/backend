# crates/platform — Платформа (Роль 1, Rust Track)
ХРЕБЕТ. axum-API + Tauri + sqlite + Claim-стор + snapshot/hash.
- Владелец `crates/contracts` и генерации TS-типов (ts-rs → web/src/contracts.ts).
- Валидирует ВЕСЬ JSON от сайдкара на входе. Спавнит python как процесс (без PyO3).
- Зовёт engine как библиотеку. Tauri: file dialog, скан папки, десктоп-сборка.

## Чистая архитектура (зависимости направлены внутрь)
- `domain/`         — бизнес-правила: `validation`, `snapshot`, `rerun`. Чистые, без I/O и axum.
- `application/`    — use cases (`run`/`board`/`hypothesis`/`rerun`) + порты (`ports.rs`) + `UseCaseError`.
- `infrastructure/` — адаптеры портов: файловые источники + in-memory run-стор.
- `web/`            — axum-хендлеры/DTO/маппинг ошибок/middleware (HTTP delivery).
- `state.rs`        — composition root (DI): собирает адаптеры за портами.

Правило: `domain` ни от кого не зависит; `application` зависит только от `domain`+`engine`+портов;
`infrastructure`/`web` реализуют/вызывают порты. HTTP-статусы и файловый I/O не протекают в use cases.
