import { useEffect, useState } from "react";
import { AppMenubar } from "./shell/menubar/AppMenubar";
import { Sidebar } from "./shell/sidebar/Sidebar";
import { WorkbenchLayout } from "./shell/tabbar/WorkbenchLayout";
import { CommandPalette } from "./shell/palette/CommandPalette";
import { StatusBar } from "./shell/statusbar/StatusBar";
import { UIOverlays } from "./components/UIOverlays";
import { LoginPage } from "./pages/LoginPage";
import { useShell } from "./store/shell-store";
import { call } from "./naja/call";
import { boot } from "./naja/boot";
import "./naja/index"; // side-effect: registers naja on window

export default function App() {
  const user = useShell((s) => s.user);
  const setUser = useShell((s) => s.setUser);
  const [checking, setChecking] = useState(true);

  // On mount check if a session cookie is already valid
  useEffect(() => {
    call<string>("frappe.auth.get_logged_user", {}, { background: true })
      .then(async (u) => {
        if (u && u !== "Guest") {
          setUser(u, u);
          await boot.load();
        }
      })
      .catch(() => {})
      .finally(() => setChecking(false));
  }, [setUser]);

  if (checking) return null;
  if (!user) return <LoginPage />;

  return (
    <div className="app-shell">
      <AppMenubar />
      <div className="app-body">
        <Sidebar />
        <WorkbenchLayout />
      </div>
      <StatusBar />
      <CommandPalette />
      <UIOverlays />
    </div>
  );
}
