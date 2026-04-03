import { useEffect, useRef, useCallback, useState, type ReactNode } from "react";
import {
  Layout,
  Model,
  Actions,
  Action,
  TabNode,
  TabSetNode,
  IJsonTabNode,
  DockLocation,
} from "flexlayout-react";
import "flexlayout-react/style/light.css";
import { create_initial_model } from "./tab-types";
import { useShell } from "../../store/shell-store";
import { FormEngine } from "../../form-engine/FormEngine";
import { ListView } from "../../list-view/ListView";
import { AIPanel } from "../../ai-panel/AIPanel";
import { events } from "../../naja/events";

let _layoutRef: Layout | null = null;
export function get_layout_ref(): Layout | null { return _layoutRef; }

const STORAGE_KEY = "naja:layout:v1";

function load_model(): Model {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved) return Model.fromJson(JSON.parse(saved));
  } catch { /* ignore */ }
  return create_initial_model();
}

function save_model(model: Model): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(model.toJson()));
  } catch { /* ignore */ }
}

export function WorkbenchLayout() {
  const [model] = useState<Model>(load_model);
  const layoutRef = useRef<Layout>(null);
  const open = useShell((s) => s.openTab);
  const close = useShell((s) => s.closeTab);
  const tabs = useShell((s) => s.tabs);

  // Keep layout ref available globally for open_tab calls
  useEffect(() => {
    if (layoutRef.current) _layoutRef = layoutRef.current;
  });

  // Sync shell store tab opens → FlexLayout
  useEffect(() => {
    const lastTab = tabs[tabs.length - 1];
    if (!lastTab) return;
    const existing = model.getNodeById(lastTab.id);
    if (existing) {
      model.doAction(Actions.selectTab(lastTab.id));
      return;
    }
    const tabNode: IJsonTabNode = {
      type: "tab",
      id: lastTab.id,
      name: lastTab.title + (lastTab.dirty ? " ●" : ""),
      component: lastTab.kind,
      config: {
        doctype: lastTab.doctype,
        docname: lastTab.docname,
      },
    };
    // Add to the main tabset (first tabset found)
    const firstTabset = _find_first_tabset(model);
    if (firstTabset) {
      model.doAction(Actions.addNode(tabNode, firstTabset.getId(), DockLocation.CENTER, -1));
    }
  }, [tabs, model]);

  const factory = useCallback((node: TabNode): ReactNode => {
    const component = node.getComponent();
    const cfg = node.getConfig() as { doctype?: string; docname?: string } | undefined;
    switch (component) {
      case "welcome":
        return <WelcomePane />;
      case "list":
        return cfg?.doctype ? <ListView doctype={cfg.doctype} /> : null;
      case "form":
        return cfg?.doctype ? (
          <FormEngine
            doctype={cfg.doctype}
            docname={cfg.docname}
            tabId={node.getId()}
          />
        ) : null;
      case "ai":
        return <AIPanel />;
      default:
        return <div className="pane-empty">Unknown tab type: {component}</div>;
    }
  }, []);

  function handleModelChange() {
    save_model(model);
  }

  function handleAction(action: Action): Action | undefined {
    if (action.type === Actions.DELETE_TAB) {
      const id = action.data?.node as string | undefined;
      if (id) {
        close(id);
        events.emit("tab:closed", { tabId: id });
      }
    }
    return action;
  }

  return (
    <div className="workbench-layout">
      <Layout
        ref={layoutRef}
        model={model}
        factory={factory}
        onModelChange={handleModelChange}
        onAction={handleAction}
        classNameMapper={(cls) => cls}
      />
    </div>
  );
}

function WelcomePane() {
  const open = useShell((s) => s.openTab);
  const setPaletteOpen = useShell((s) => s.setPaletteOpen);
  return (
    <div className="welcome-pane">
      <div className="welcome-content">
        <h1>Spotledger ⚡</h1>
        <p>Press <kbd>Ctrl+K</kbd> to open any DocType.</p>
        <div className="welcome-actions">
          <button className="btn btn-primary" onClick={() => setPaletteOpen(true)}>
            Open DocType…
          </button>
          <button className="btn btn-ghost" onClick={() => open({ kind: "ai", title: "AI Assistant" })}>
            AI Assistant
          </button>
        </div>
      </div>
    </div>
  );
}

function _find_first_tabset(model: Model): TabSetNode | null {
  let found: TabSetNode | null = null;
  model.visitNodes((node) => {
    if (!found && node.getType() === "tabset") {
      found = node as TabSetNode;
    }
  });
  return found;
}
