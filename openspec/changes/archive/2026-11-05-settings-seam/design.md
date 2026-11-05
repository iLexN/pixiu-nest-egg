# Design

## Decisions

- **One home per concept.** Settings defaults belong with the `/api/settings` domain, not the month book: `goal_constants`, `semi_liquid_target`, and their `DEFAULT_*` literals move to `service::settings`, which already imports them — no new module needed, the seam already existed.
- Constants and fns move together so callers find one home for both the override and its fallback.
