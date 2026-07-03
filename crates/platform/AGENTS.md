# crates/platform — Платформа (Роль 1, Rust Track)
ХРЕБЕТ. axum-API + Tauri + sqlite + Claim-стор + snapshot/hash.
- Владелец `crates/contracts` и генерации TS-типов (ts-rs → web/src/contracts.ts).
- Валидирует ВЕСЬ JSON от сайдкара на входе. Спавнит python как процесс (без PyO3).
- Зовёт engine как библиотеку. Tauri: file dialog, скан папки, десктоп-сборка.
