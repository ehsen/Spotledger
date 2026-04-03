import * as Menubar from "@radix-ui/react-menubar";
import * as AlertDialog from "@radix-ui/react-alert-dialog";
import { useShell } from "../../store/shell-store";
import { call } from "../../naja/call";

// Menu config — extend here to add new menus and items
const MENUS = [
  {
    label: "File",
    items: [
      { label: "New…", shortcut: "Ctrl+N", action: "new" },
      { label: "Open…", shortcut: "Ctrl+K", action: "palette" },
      { separator: true },
      { label: "Save", shortcut: "Ctrl+S", action: "save" },
      { separator: true },
      { label: "Sign out", action: "logout" },
    ],
  },
  {
    label: "Edit",
    items: [
      { label: "Reload", shortcut: "Ctrl+R", action: "reload" },
    ],
  },
  {
    label: "View",
    items: [
      { label: "Command Palette", shortcut: "Ctrl+K", action: "palette" },
      { label: "AI Assistant", action: "ai" },
      { separator: true },
      { label: "List View", action: "list" },
    ],
  },
  {
    label: "Go",
    items: [
      { label: "Next Tab", shortcut: "Ctrl+Tab", action: "next_tab" },
      { label: "Close Tab", shortcut: "Ctrl+W", action: "close_tab" },
    ],
  },
  {
    label: "Help",
    items: [
      { label: "Welcome", action: "welcome" },
    ],
  },
];

type MenuItem =
  | { label: string; shortcut?: string; action: string; separator?: never }
  | { separator: true; label?: never; action?: never; shortcut?: never };

interface MenuDef { label: string; items: MenuItem[] }

export function AppMenubar() {
  const fullName = useShell((s) => s.fullName);
  const clearUser = useShell((s) => s.clearUser);
  const setPaletteOpen = useShell((s) => s.setPaletteOpen);
  const openTab = useShell((s) => s.openTab);

  async function dispatch(action: string) {
    switch (action) {
      case "palette": setPaletteOpen(true); break;
      case "logout":
        try { await call("logout"); } catch { /* ignore */ }
        clearUser();
        break;
      case "welcome": openTab({ kind: "welcome", title: "Welcome" }); break;
      case "ai": openTab({ kind: "ai", title: "AI Assistant" }); break;
      default: break;
    }
  }

  return (
    <Menubar.Root className="menubar">
      <div className="menubar-logo">⚡ Spotledger</div>

      {(MENUS as MenuDef[]).map((menu) => (
        <Menubar.Menu key={menu.label}>
          <Menubar.Trigger className="menubar-trigger">{menu.label}</Menubar.Trigger>
          <Menubar.Portal>
            <Menubar.Content className="menubar-content" align="start" sideOffset={4}>
              {menu.items.map((item, idx) =>
                "separator" in item && item.separator ? (
                  <Menubar.Separator key={`sep-${idx}`} className="menubar-separator" />
                ) : (
                  <Menubar.Item
                    key={item.label}
                    className="menubar-item"
                    onSelect={() => dispatch(item.action!)}
                  >
                    <span>{item.label}</span>
                    {item.shortcut && (
                      <span className="menubar-shortcut">{item.shortcut}</span>
                    )}
                  </Menubar.Item>
                ),
              )}
            </Menubar.Content>
          </Menubar.Portal>
        </Menubar.Menu>
      ))}

      <div className="menubar-spacer" />
      <div className="menubar-user">{fullName}</div>
    </Menubar.Root>
  );
}
