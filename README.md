# backend — Rust platform (флотационный контракт)

Три крейта чистой архитектуры:

- `crates/contracts` — единственный источник структур контракта (см. `docs/CONTRACTS.md`).
- `crates/engine` — Discovery Engine: чистый, без I/O и доменных слов. Вся доменная
  семантика приходит из данных (pack, fixtures, factories).
- `crates/platform` — axum-платформа (порт 8080), HTTP-шов web ↔ platform.

## Данные

Единый источник правды — каталог `docs/` (`fixtures/`, `packs/flotation-v1.yaml`,
`factories/*.yaml`, `golden/`). Указывается через `NORNIKEL_ROOT`.

## Запуск

```sh
NORNIKEL_ROOT=../docs cargo run -p platform
# platform listening on http://127.0.0.1:8080

# с живым сайдкаром (extract/diagnose по HTTP, файловый fallback):
SIDECAR_URL=http://127.0.0.1:8765 NORNIKEL_ROOT=../docs cargo run -p platform
```

Эндпоинты:
- `POST /run {factory_id, pack_id?, kpi_contract?}` → `{run_id, board}`
- `GET /board?run_id=`, `POST /rerun {run_id, action}`, `GET /hypothesis/:id`
- `GET /extract`, `GET /expert_hypotheses`
- `GET /benchmark?run_id=` — покрытие эталонных гипотез экспертов (обычно 100% на KGMK)
- `GET /data_readiness?run_id=` — качество исходного xlsx (ref_error и т.п.)
- `GET /trace/:hyp_id?run_id=` — трассировка гипотезы до claims (страницы PDF) и ячеек xlsx
- `GET /roadmap?run_id=&max_capex=` — план действий с честной де-дубликацией стоимости
- `GET /factories` — мультифабричная карта денег по всем 4 фабрикам
- `GET /export/board.{json,csv}`

Фронт ходит через vite/nginx proxy `/api` → один origin (CORS в platform нет).

## Тесты

```sh
cargo test
UPDATE_GOLDEN=1 cargo test -p engine golden_board_flotation_v1   # перегенерить golden
```
