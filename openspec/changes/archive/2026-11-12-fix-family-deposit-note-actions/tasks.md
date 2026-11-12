# Tasks

## 1. Note Actions Spacing

- [x] 1.1 In `frontend/src/views/FamilyDepositsView.vue`, wrap the holder card's note toggle button (`隱藏備註`/`顯示備註`/`備註`) and the `新增備註`/`編輯備註` link in a `<div class="note-actions">`, carrying the edit link's condition as `v-if="shownNotes.has(holder.holder) && editingNote !== holder.holder"`; keep the editing textarea and `.note-text` paragraph rendering below via the existing `shownNotes` template
- [x] 1.2 Add `class="note-actions"` to the `<div>` wrapping the note editor's 儲存/取消 buttons, and add a scoped rule `.note-actions { display: flex; align-items: baseline; gap: 0.5rem; }`; keep `.note-block .link { padding: 0 }`
- [x] 1.3 Verify with `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit`, then restart `wealth-backend` per AGENTS.md (`lsof -ti :8787 | xargs kill`, relaunch detached with `nohup`) and confirm a holder card with no note shows `隱藏備註` and `新增備註` clearly separated on http://127.0.0.1:8787
