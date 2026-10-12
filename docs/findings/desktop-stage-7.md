# Desktop stage 7: manual edits, organization and persistent Undo

Stage 7 connects the persisted [manual action log](../spec/ui-metadata.md) to typed desktop commands and the first-version UI. Provider evidence, source transactions and source labels are never overwritten; every write is a dated manual period in `ui_local_state` plus a compensating-capable action in `ui_manual_actions`.

## What the user can now do

- **Rates and borrowing costs.** Leaf holding rows in allocation tables carry an inline APY editor with a pin icon and an effective date defaulting to today; the details panel repeats it with the rate provenance (`selected_rate`). Liability holdings use their `BorrowingPosition` scope; a holding with no subscription overrides the aggregate `AccountAsset` scope. Entering a blank value records an explicit _unknown_ rate period, and `Reset to source` removes the dated manual period so source evidence applies again.
- **Prices.** Holdings whose asset is not the reporting asset carry an inline price editor (reporting currency) with pin and effective date. The reporting asset itself has no quote and therefore no price override.
- **Dust and spam.** Dust classification offers Dust, Significant and Reset to auto per provider/asset. Spam exclusion requires a reason and carries a restore control; restoring clears the portfolio-wide decision while account reviews stay.
- **Local names and organization.** Subscription names are editable with a `Use provider name` reset. Account nodes offer a local name, a `Move to…` selector listing only the cycle-free valid parents inside the same provider, a reset to the provider default, and group creation under the current node.
- **Undo.** The `Change history` section (moved from the navbar into Settings in the later annotation follow-up) lists every manual action newest-first with its prior and resulting state, Undo links, and `Undone`/`Compensating change` badges. Undo is offered only while the target's current revision still matches the action's resulting revision; a newer manual change on the same target disables the older Undo and the UI says so.

## Commands

- `apply_manual_edit(edit)` in [libs/lib-app/src/manual.rs](../../libs/lib-app/src/manual.rs) translates a typed `ManualEdit` into the exact scope, revision and value expected by [`Store::apply_manual_change`](../../libs/lib-core/src/ui_metadata.rs). It rejects future effective dates ("it would never apply"), rejects pinning when no manual period exists, and preserves earlier/later manual periods when a correction is backdated.
- The desktop host exposes `manual_edit` and `undo_action` Tauri commands ([apps/desktop/src-tauri/src/main.rs](../../apps/desktop/src-tauri/src/main.rs)). Both rebuild the snapshot, so the monotonic snapshot ID invalidates in-flight view requests instead of mixing cutoffs.
- Rate/price are stored from exact decimal strings. The frontend converts between percent and fraction with string math ([percentToFraction](../../apps/desktop/src/App.tsx)); `JS numbers` never reach a stored rate.

## Wire additions

`Startup` now carries `edits` ([`EditContext`](../../apps/desktop/src-tauri/src/lib.rs): accounts, `local_state`, and the ordered action log). Allocation trees attach `edits` to leaf holding nodes, and `details` returns a per-holding `HoldingEdits` block (`rate`, `price`, `dust`, `spam`, subscription `name`) plus `NodeAccount` grouping controls. Contract version stays 1; host and frontend ship together.

The details pane follows the snapshot: an effect re-reads the currently open allocation when a new snapshot arrives, so a committed edit refreshes the panel in place rather than going stale.

## Review mode

- `review-api.ts` rejects `manual_edit`/`undo_action` like every other desktop-only command, and the edit affordances are disabled in review mode. The review export therefore never writes.
- The review/fixture generator exposes the same shapes, so the browser fixture and desktop tests stay in sync.

## Decisions and deviations

- **Manual rates are periods, not replacements.** A backdated correction keeps subsequent applicable periods, and pinning applies to the latest manual period, matching `selected_rate`'s "latest applicable, pinned wins" rule.
- **Dust `auto` is stored explicitly.** Clearing the local record and storing `Dust(auto)` behave identically today; the explicit value keeps the action log readable.
- **Group creation is not undoable.** It creates an account/structure row rather than local state, so it has no action-log entry. Reparenting and every other organization change is undoable. An unused group disappears from the allocation tree because the tree is holdings-driven.
- **Price editing is unavailable in Hidden privacy mode.** Hidden mode masks monetary figures everywhere, and an editable quote is a monetary figure, so the price editor reports that it is hidden instead of revealing a value. Rate editing stays available: the design system intentionally preserves percentage rates in hidden and scaled modes.
- **Row editing needs width.** Dated inline editors stay on one line; below 900px the table shows the derived rate only and manual editing happens in the details panel, matching how the details column already behaves.
- **Aggregate liability holdings without a single position have no rate editor.** There is no aggregate borrowing scope in `RateScope`; such holdings keep their displayed rate and provenance.
- **`Editable.Preview` was hidden from AT.** Ark renders the preview as a `span` with `aria-label` and `tabIndex=0`, which axe flags twice (`aria-prohibited-attr`, then `aria-hidden-focus`). The preview is now `aria-hidden` and `tabIndex=-1`; label, input and edit trigger already carry the semantics. This also fixes a pre-existing violation that the desktop axe scan exposed.

## Validation

- `cargo test --locked --workspace --exclude try-polars --exclude money-tracker-desktop -- --skip adapters::binance::tests::cached_binance_account_reconciles` (binance's legacy live test still needs credentials and is skipped by CI).
- `cargo test -p money-tracker-desktop --test portfolio`: `manual_rate_edit_then_undo_restores_source_earnings` (edit → summary 50→60 → Undo → 50, close, reopen, action log retains the compensating action) and `dust_spam_and_grouping_controls_change_the_projection` (dust → spam → group → move, verified through the projected tree).
- `cargo clippy --all` is clean for the new code; `cargo fmt --all --check` passes.
- `pnpm typecheck`, `pnpm build`, design-system browser suite (10 passed) and `pnpm test:desktop` (6 passed, including `manual rate, price, dust and spam edits commit typed requests` which asserts the exact typed request payloads, the undo link and its `Undone` state). `npx prettier --write` was applied to the changed frontend files; the wider repository still has pre-existing formatting debt in untouched files.
