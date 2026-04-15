# DocType Designer & Lifecycle Engine — Implementation Plan

> **Stack**: Rust backend · SurrealDB · Spotledger-UI (React + Zustand)  
> **Constraint**: No changes to existing save pipeline, controller, or DocType trait. Entirely additive.

---

## Architecture Constraints

- **SurrealDB is source of truth** — designer saves to `tabDocType` + `tabDocField`, then applies DDL via the existing `ensure_schema`.
- **No modification to existing save pipeline** — `controller.rs`, `save_proxy.rs`, `DocType` trait untouched.
- **Existing `frappe.client.*` methods NOT used for designer saves** — need atomic multi-table save + DDL apply → new dedicated handlers.
- **Tab system stays** — designer opens as a new tab type `"designer"` via the existing `openTab` Zustand action.
- **Dual-track meta system stays** — compiled Tier-0 doctypes are **not** editable in the designer (guard enforced at API layer).

---

## Phase 1 — Backend: `fn_source` table + designer API methods

### 1.1 New SurrealDB table: `fn_source`

Add to `generated/schema.surql`:

```surql
DEFINE TABLE IF NOT EXISTS fn_source SCHEMAFULL;
DEFINE FIELD fn_name     ON fn_source TYPE string;   -- e.g. "fn::sales_invoice::validate_totals"
DEFINE FIELD doctype     ON fn_source TYPE string;
DEFINE FIELD stage       ON fn_source TYPE string;   -- "validate" | "before_save" | etc.
DEFINE FIELD code        ON fn_source TYPE string;
DEFINE FIELD description ON fn_source TYPE option<string>;
DEFINE FIELD tags        ON fn_source TYPE option<array<string>>;
DEFINE FIELD modified_at ON fn_source TYPE datetime VALUE time::now();
DEFINE INDEX fn_source_doctype ON fn_source FIELDS doctype;
```

### 1.2 New Rust file: `crates/spotledger-http/src/methods/designer.rs`

Seven handlers registered via the existing `reg!()` macro pattern:

| Method | Handler | Purpose |
|---|---|---|
| `spotledger.designer.get_meta` | `handle_get_meta` | Load `tabDocType` + all `tabDocField` rows + child doctype list |
| `spotledger.designer.save` | `handle_save` | Upsert `tabDocType` + full-replace `tabDocField` + call `ensure_schema` |
| `spotledger.designer.generate_surql` | `handle_generate_surql` | Generate SurrealQL preview string — no write |
| `spotledger.designer.get_pipeline` | `handle_get_pipeline` | Load `pipeline_stage` + `has_node` + `pipeline_node` + `fn_source` for a doctype |
| `spotledger.designer.save_function` | `handle_save_function` | Upsert `fn_source` + `DEFINE FUNCTION` DDL + wire `pipeline_node` into stage |
| `spotledger.designer.delete_function` | `handle_delete_function` | Remove `has_node` edge + `pipeline_node` + `REMOVE FUNCTION` |
| `spotledger.designer.reorder_functions` | `handle_reorder_functions` | Bulk-update `ord` on `has_node` edges for a stage |

**`handle_save` logic (ordered):**
1. Guard: reject if doctype name is in the Tier-0 compiled list (`DocType`, `DocField`, `DocPerm`, `User`, `Role`, `UserPermission`, `Site`, `ModuleDef`) → return 400
2. Upsert `tabDocType` record
3. `DELETE FROM tabDocField WHERE parent = $doctype` — full replace (designer is authoritative for the field list)
4. Bulk-insert new `tabDocField` rows from request payload
5. Call `ensure_schema(adapter, doctype_name)` — existing Phase 1 DDL function in `spotledger-db/src/schema.rs`
6. Invalidate `meta_cache` entry for this doctype

**`handle_generate_surql` logic (pure compute, no writes):**
1. Fetch `tabDocType` + `tabDocField` for the requested doctype
2. Build and return SurrealQL string using the same DDL-building logic as `ensure_schema` but without executing it

### 1.3 Registration in `crates/spotledger-http/src/methods/mod.rs`

```rust
mod designer;
// inside build_method_registry():
reg!("spotledger.designer.get_meta",           designer::handle_get_meta);
reg!("spotledger.designer.save",               designer::handle_save);
reg!("spotledger.designer.generate_surql",     designer::handle_generate_surql);
reg!("spotledger.designer.get_pipeline",       designer::handle_get_pipeline);
reg!("spotledger.designer.save_function",      designer::handle_save_function);
reg!("spotledger.designer.delete_function",    designer::handle_delete_function);
reg!("spotledger.designer.reorder_functions",  designer::handle_reorder_functions);
```

---

## Phase 2 — Frontend Foundation

### 2.1 New types: `src/types/designer.ts`

```typescript
interface DesignerField {
  id: string;
  fieldname: string;
  label: string;
  fieldtype: FrappeFieldtype;
  options?: string;
  reqd: boolean;
  read_only: boolean;
  in_list_view: boolean;
  hidden: boolean;
  search_index: boolean;
  bold: boolean;
  assert_expr?: string;
  default_value?: string;
  compute_expr?: string;
  idx: number;
}

interface DesignerSection {
  id: string;
  label: string;
  columns: 1 | 2 | 3;
}

interface DesignerDoctype {
  name: string;
  module: string;
  is_child_table: boolean;
  issubmittable: boolean;
  sections: DesignerSection[];
  fields: DesignerField[];
}

type LifecycleStage =
  | "validate"
  | "before_save"
  | "on_save"
  | "on_submit"
  | "on_cancel"
  | "on_amend"
  | "scheduled";

interface PipelineFunction {
  id: string;
  fn_name: string;
  description?: string;
  tags: string[];
  code: string;
  stage: LifecycleStage;
  ord: number;
}

interface PipelineStageData {
  stage: LifecycleStage;
  functions: PipelineFunction[];
}
```

### 2.2 New API file: `src/lib/designerApi.ts`

Axios wrappers calling `POST /api/method/spotledger.designer.*`. Same pattern as `src/lib/frappeApi.ts`.

### 2.3 New store: `src/store/useDesignerStore.ts`

Separate Zustand + Immer store (keeps designer ephemeral state out of `useAppStore`):

```typescript
interface DesignerStore {
  doctype: string | null;
  activeTab: "design" | "logic" | "surql";
  selectedChildDoctype: string | null;
  selectedFieldId: string | null;
  selectedStage: LifecycleStage | null;
  selectedFunctionId: string | null;
  meta: DesignerDoctype | null;
  childMetas: Record<string, DesignerDoctype>;
  stages: PipelineStageData[];
  generatedSurql: string;
  isDirty: boolean;
  // actions: loadDoctype, setField, addField, removeField, moveField,
  //          addSection, removeSection, reorderFunctions, updateFunction, ...
}
```

### 2.4 Register `"designer"` tab type

- Add `type: "designer"` to `Tab` interface in `src/types/store.ts`
- Add `openDesignerTab(doctype: string)` action to `src/store/useAppStore.ts`
- Handle `"designer"` case in `src/app/panels/TabPanel.tsx` → render `<DesignerView doctype={tab.doctype} />`

### 2.5 Entry point: "Open in Designer" button

- In `src/app/panels/FormView/FormToolbar.tsx`: when `tab.doctype === "DocType"`, render an "Open Designer" icon-button that calls `openDesignerTab(formState.values.name)`

### 2.6 New dependency

Add to `Spotledger-ui/package.json`:

```
@dnd-kit/core
@dnd-kit/sortable
@dnd-kit/utilities
```

Chosen over HTML5 DnD for accessibility, touch support, and the per-column sortable context model.

---

## Phase 3 — Doc Design Tab

### Component tree: `src/app/panels/DesignerView/`

```
DesignerView/
  index.tsx                  — top-level: loads meta via React Query, populates store, 3-tab Radix Tabs
  DesignerTopbar.tsx         — doctype selector badge (Main=purple / Child=teal), dirty dot, Save button
  DocDesign/
    DesignCanvas.tsx         — DndContext root; section list (sortable); + Add Section at bottom
    SectionBlock.tsx         — inline rename, 1/2/3 col buttons, collapse toggle, per-column SortableContext
    FieldRow.tsx             — ⠿ grip, label, fieldtype badge, click-to-select; wrapped in @dnd-kit SortableItem
    FieldPalette.tsx         — left sidebar; field types grouped: Link/Relation, Text, Number, Date/Time, Select, Layout
    FieldPropertyPanel.tsx   — right panel (always visible); Radix Accordion: Identity / Validation / Flags
    ChildTableInline.tsx     — collapsible panel below Table-type field; compact field rows + "Open full editor"
  DocLogic/
    LifecycleSidebar.tsx     — 7 fixed stages; color dot + function count badge; 0-function stages shown dimmed
    FunctionList.tsx         — SortableContext list of FunctionCard for active stage; + Add function
    FunctionCard.tsx         — order badge, fn_name, description, tags, 3-line code preview, hover Edit/Remove
    CodeEditorPanel.tsx      — persistent right panel; monospace textarea + line-number gutter; Apply / Discard / Format
    FunctionPalette.tsx      — standard function library (Accounting, Stock, Common) draggable into stage
  SurrealQlView.tsx          — readOnly textarea; "View only" badge; Explain ↗ button
```

### 3.1 Drag-and-drop model

Two `DndContext` zones:

1. **Palette → Canvas**: drag a field type from `FieldPalette`, drop onto a column's drop zone → generates client UUID, adds `DesignerField` to store with default name, sets `selectedFieldId` (opens property panel immediately)
2. **Field reorder within / between columns**: each column is its own `<SortableContext items={columnFields}>` — `onDragEnd` updates field `idx` values in store

Section-level drag-to-reorder: outer `<SortableContext items={sectionIds}>` wrapping all `SectionBlock`s.

### 3.2 Section management

- `+ Add section` at canvas bottom → appends to `sections[]`, marks dirty
- Section label: inline `contentEditable` span (double-click to edit)
- Column count 1/2/3: buttons in section header; on reduce, excess fields move to last remaining column
- Collapse: local React state per `SectionBlock` (not persisted, UI only)
- Section drag-to-reorder via outer sortable context

### 3.3 Field property panel

Always mounted at right. Content driven by `selectedFieldId`. Default state: "Click a field to edit its properties."

Three Radix `<Accordion>` groups:
- **Identity**: fieldname (snake_case), label, fieldtype select, options/target
- **Validation**: ASSERT expression (monospace input), default value, VALUE computed expression
- **Flags**: Required, Read only, In list view, Hidden, Searchable (index), Bold — rendered as toggle pills

All changes write directly into `designerStore.meta.fields[id]` and mark `isDirty`.

### 3.4 Live SurrealQL generation

`generateSurql(meta, childMetas, stages): string` in `src/lib/surqlGenerator.ts` — pure TypeScript, no network call. Called via `useMemo` on every `meta` or `stages` change. Result stored in `designerStore.generatedSurql` for the SurrealQL tab.

Output mirrors what `handle_generate_surql` returns from the backend:

```
-- DocType: <Name>  (auto-generated — edit via UI only)
DEFINE TABLE <name> SCHEMAFULL;
DEFINE FIELD <field> ON <table> TYPE <type> [ASSERT ...] [DEFAULT ...] [VALUE ...];
...
DEFINE EVENT <stage> ON TABLE <name> WHEN ... THEN { fn::<doctype>::<fn>($this); };
...
```

### 3.5 Child table inline

`ChildTableInline` renders below any `Table`-type `FieldRow` when expanded. Shows the referenced child doctype's fields as compact rows (read from `childMetas[options]`). "+ Add field to child table" appends to the child meta. "Open full editor" sets `selectedChildDoctype` in store → `DesignCanvas` re-renders using child meta as its source.

---

## Phase 4 — Doc Logic Tab

### 4.1 Lifecycle sidebar (`LifecycleSidebar.tsx`)

Fixed 7 stages in semantic order:

| Stage | Group | Color |
|---|---|---|
| Validate | Before action (blocking) | blue |
| Before Save | Before action (blocking) | indigo |
| On Save | After action | green |
| On Submit | After action | emerald |
| On Cancel | After action | amber |
| On Amend | After action | orange |
| Scheduled | Scheduled | purple |

Each row: stage name + color dot + function count badge. Stages with 0 functions shown with dimmed badge (never hidden — users see the full lifecycle at a glance).

### 4.2 Function list (`FunctionList.tsx`)

`<SortableContext>` wrapping `<FunctionCard>` list. On drag end: update `ord` values in `useDesignerStore`, mark dirty. Does NOT fire API until Save.

Each `FunctionCard` shows:
- Sequential order badge (1, 2, 3…)
- Full fn name in monospace (`fn::sales_invoice::validate_totals`)
- Plain-English description (editable in code panel)
- Category tags
- 3–4 line code preview with bottom fade
- Hover: **Edit** and **Remove** buttons

### 4.3 Code editor panel (`CodeEditorPanel.tsx`)

Persistent right panel (not a modal). Mounted when `selectedFunctionId !== null`, list remains scrollable.

- **Header**: fn name (monospace) + stage label + color dot + description
- **Toolbar**: Format | Review ↗ | Snippet ↗ | context vars (`$this · $event · $auth · $session`)
- **Editor**: monospace `<textarea>` + `<pre>` line-number gutter (synced on scroll); Tab key → 2-space indent (keydown handler)
- **Footer**: live line count | **Discard** (reverts to last applied) | **Apply** (writes to store, updates card preview, marks dirty)

Changes not written to SurrealDB until topbar Save.

### 4.4 Adding / removing functions

- **Add**: `+ Add function` button in stage pane → create `PipelineFunction` with placeholder name + body, append to `stages[stage].functions`, set `selectedFunctionId`, open code editor, focus fn name
- **Remove**: hover Remove on card → confirmation prompt → splice from array, clear editor if open, mark dirty

### 4.5 Naming convention (enforced by UI)

All functions follow `fn::<doctype>::<action>`. The fn name input validates this pattern client-side.

---

## Phase 5 — SurrealQL Tab + Save + Keyboard Shortcuts

### 5.1 SurrealQL tab (`SurrealQlView.tsx`)

Renders `designerStore.generatedSurql` in `<textarea readOnly>` (monospace). Prominent "View only — edit via Doc Design and Doc Logic" badge. "Explain ↗" button (opens AI panel pre-filled with content, or placeholder toast if AI panel not ready).

### 5.2 Save orchestration (`DesignerView/index.tsx`)

Triggered by topbar Save button or `Ctrl/Cmd+S`:

1. `designerApi.save(meta, childMetas)` → backend does field replace + DDL apply
2. For each function with `_dirty: true`: `designerApi.saveFunction(fn)` → backend upserts `fn_source` + `DEFINE FUNCTION` + wires `pipeline_node` into stage
3. For each function in `removedFunctionIds`: `designerApi.deleteFunction(fn_name)`
4. If any stage's function order changed: `designerApi.reorderFunctions(stage, orderedIds)`
5. On all settled: clear `isDirty`, push success notification to `useAppStore`

### 5.3 Keyboard shortcuts (registered in `DesignerView/index.tsx` useEffect)

| Shortcut | Action |
|---|---|
| `Ctrl/Cmd + S` | Save |
| `Escape` | Discard active code edit |
| `Ctrl/Cmd + Enter` | Apply code edit |
| `Tab` | 2-space indent (in code editor textarea) |
| `Ctrl/Cmd + /` | Toggle line comment (in code editor textarea) |

---

## Phase 6 — Future (out of scope for this delivery)

Items from `surreal-erp-feature-plan.md` §11 Phases 3/4:

- Submittable DocType toggle → auto-inject locked `status` (Draft/Submitted/Cancelled) + `amended_from` fields; activate On Submit / On Cancel / On Amend stages
- Permissions configuration UI → per-role CRUD + submit/cancel matrix → generate `DEFINE TABLE ... PERMISSIONS` block
- Standard function library palette (drag `fn::accounting::create_gl_entries` etc. from sidebar into stage)
- Function version history with diff view + Restore (`fn_version` table, clock icon on card)
- Syntax tokenizer for basic SurrealQL keyword highlighting in code editor
- Scheduled functions with cron expression input field on function card

---

## Relevant Files

### Backend — modified (minimal surgical changes)

| File | Change |
|---|---|
| `crates/spotledger-http/src/methods/mod.rs` | Add `mod designer;` + 7 `reg!()` calls |
| `generated/schema.surql` | Append `fn_source` table + index DDL |

### Backend — new

| File | Purpose |
|---|---|
| `crates/spotledger-http/src/methods/designer.rs` | All 7 designer API handlers |

### Backend — reused unchanged

| File | Used for |
|---|---|
| `crates/spotledger-db/src/schema.rs` | `ensure_schema()` called from `handle_save` |
| `crates/spotledger-core/src/meta.rs` | `DocTypeMeta`, `DocField` shapes for DDL building |

### Frontend — modified (minimal)

| File | Change |
|---|---|
| `src/types/store.ts` | Add `"designer"` to `Tab.type` union |
| `src/store/useAppStore.ts` | Add `openDesignerTab(doctype: string)` action |
| `src/app/panels/TabPanel.tsx` | Add `case "designer":` branch |
| `src/app/panels/FormView/FormToolbar.tsx` | "Open Designer" button when viewing a DocType doc |
| `Spotledger-ui/package.json` | Add `@dnd-kit/core`, `@dnd-kit/sortable`, `@dnd-kit/utilities` |

### Frontend — new

| File / Folder | Purpose |
|---|---|
| `src/types/designer.ts` | All designer-specific TypeScript types |
| `src/lib/designerApi.ts` | Axios wrappers for designer methods |
| `src/lib/surqlGenerator.ts` | Client-side SurrealQL generation (pure TS, no network) |
| `src/store/useDesignerStore.ts` | Designer Zustand + Immer store |
| `src/app/panels/DesignerView/` | Full component tree (14 components — see Phase 3 tree) |

---

## Verification Checklist

1. Open a `Customer` DocType record → FormToolbar shows "Open Designer" → click opens `designer` tab
2. Doc Design: drag `Data` type from palette → field appears in canvas column, property panel activates
3. Add section → set 2 columns → drag field between columns → layout reflows correctly
4. `Table`-type field → `ChildTableInline` expands below it with child doctype's field rows
5. "Open full editor" on child table → doctype selector badge switches, canvas re-renders child meta
6. SurrealQL tab reflects every field change live (no Save needed)
7. Save → verify in Surrealist: `tabDocField` rows replaced; `DEFINE FIELD` applied for the table
8. Doc Logic → Validate stage → Add function → code editor panel opens
9. Edit code body → Apply → card preview shows updated code snippet
10. Drag-reorder two functions → after Save, verify `has_node.ord` values in SurrealDB
11. `Ctrl+S` triggers save; `Escape` discards open code edit; `Ctrl+Enter` applies it
12. API guard: attempt designer save for `"DocType"` (Tier-0 type) → backend returns 400

---

## Key Decisions

| Decision | Rationale |
|---|---|
| Full-replace `tabDocField` on save | Designer is authoritative; avoids field-level diff complexity for v1 |
| SurrealQL generation is client-side | Pure TS, instant feedback, no server round-trip |
| Separate `useDesignerStore` | Keeps large ephemeral state out of `useAppStore`; designer state doesn't need persistence |
| `@dnd-kit` over HTML5 DnD API | Accessibility, touch, per-column sortable contexts |
| Tier-0 guard at API layer | Prevents accidental corruption of compiled system types |
| Designer entry via FormToolbar on DocType form | Minimal change to shell; natural discovery path for users |
| Phase 6 items out of scope | Submittable/permissions/versioning are additive on top of Phase 1–5 foundation |
