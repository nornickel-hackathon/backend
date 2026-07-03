# crates/engine — Discovery Engine (Роль 1)
ЯДРО. Чистый Rust, БЕЗ I/O, БЕЗ HTTP, БЕЗ доменных слов (AGENT_RULES.md §1).

## Типы графа
```rust
use contracts::{GraphNode, GraphEdge};
pub type Graph = petgraph::DiGraph<GraphNode, GraphEdge>;
```
Platform строит граф из `ExtractResponse` и передаёт `&Graph` в `discover()`. Engine граф не строит.

## Операторы (P0 — первые два; gap — P0 если успеваем, иначе P1)
- `mechanism_path`: incoming рёбра типа `mechanism`/`proxy` от KPI до tag `controllable`.
- `substitution`: рёбра типа `substitution` — альтернативный factor с тем же путём до KPI.
- `gap`: отсутствующие комбинации → `needs_expert_review`.
- `contradiction`: два claim с противоположным polarity на одном edge → гипотеза граничного условия.
- `analogy_transfer`: перенос подтверждённого пути на аналогичный узел по тегам.

## Скоринг
Формулы — в `docs/SCORING.md`. Веса и правила — из `DomainPack`, не захардкожены.

## Детерминизм
Обязателен. Случайность только с seed. Каждый PR обновляет golden-тест на fixtures.

## Зависимости
`petgraph`. НЕ зависит от platform/sidecar.
