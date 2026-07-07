import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { MiniWidget } from "./components/MiniWidget";
import { Sidebar } from "./components/Sidebar";
import { WindowTitleBar } from "./components/WindowTitleBar";
import { api } from "./lib/api";
import { providerFromWindowLabel } from "./lib/providers";
import { DashboardPage } from "./pages/Dashboard";
import { ProviderBreakdownPage } from "./pages/ProviderBreakdown";
import { ModelBreakdownPage } from "./pages/ModelBreakdown";
import { BudgetSettingsPage } from "./pages/BudgetSettings";
import { ApiKeySettingsPage } from "./pages/ApiKeySettings";
import { ModelPricingPage } from "./pages/ModelPricing";
import { UsageHistoryPage } from "./pages/UsageHistory";
import type { Page, ViewMode } from "./types";
import "./styles/global.css";

const PAGE_TITLES: Record<Exclude<Page, "widget">, string> = {
  dashboard: "Overview",
  providers: "Providers",
  models: "Models",
  budget: "Budget Settings",
  "api-keys": "API Keys",
  pricing: "Model Pricing",
  history: "Usage History",
};

function App() {
  const [mode, setMode] = useState<ViewMode>("widget");
  const [page, setPage] = useState<Page>("widget");
  const [pinned, setPinned] = useState(true);
  const [providerFilter, setProviderFilter] = useState<string | null>(null);
  const [isMainWindow, setIsMainWindow] = useState(true);

  const expand = useCallback(async () => {
    if (providerFilter) {
      await api.openMainDashboard();
      return;
    }
    setMode("expanded");
    setPage("dashboard");
    setPinned(false);
    await api.setAlwaysOnTop(false);
    await api.setWindowMode("expanded");
    await getCurrentWindow().show();
    await getCurrentWindow().setFocus();
  }, [providerFilter]);

  const collapse = useCallback(async () => {
    setMode("widget");
    setPage("widget");
    setPinned(true);
    await api.setAlwaysOnTop(true);
    await api.setWindowMode("widget");
    await getCurrentWindow().show();
    await getCurrentWindow().setFocus();
  }, []);

  const togglePin = useCallback(async () => {
    const next = !pinned;
    setPinned(next);
    await api.setAlwaysOnTop(next);
  }, [pinned]);

  const hide = useCallback(async () => {
    await getCurrentWindow().hide();
  }, []);

  useEffect(() => {
    (async () => {
      const label = getCurrentWindow().label;
      const provider = providerFromWindowLabel(label);
      setProviderFilter(provider);
      setIsMainWindow(label === "main");
      await api.setAlwaysOnTop(true);
      await api.setWindowMode("widget");
      if (label === "main") {
        api.refreshLiveData().catch(console.error);
      }
    })();
  }, []);

  useEffect(() => {
    const unlisten = listen<string>("navigate", async (event) => {
      if (event.payload === "dashboard") {
        setMode("expanded");
        setPage("dashboard");
        setPinned(false);
        await api.setAlwaysOnTop(false);
        await api.setWindowMode("expanded");
      }
    });
    return () => { unlisten.then((fn) => fn()); };
  }, []);

  if (mode === "widget") {
    return (
      <div className="widget-shell">
        <MiniWidget
          provider={providerFilter}
          pinned={pinned}
          onTogglePin={togglePin}
          onExpand={expand}
          onHide={hide}
        />
      </div>
    );
  }

  if (!isMainWindow) {
    return null;
  }

  const pageTitle = page === "widget" ? "Dashboard" : PAGE_TITLES[page];

  return (
    <div className="expanded-shell">
      <WindowTitleBar
        title="TOKENPULSE"
        subtitle={pageTitle}
        pinned={pinned}
        onTogglePin={togglePin}
        onCollapse={collapse}
      />
      <div className="expanded-body">
        <Sidebar current={page} onNavigate={setPage} />
        <main className="expanded-main">
          {page === "dashboard" && <DashboardPage />}
          {page === "providers" && <ProviderBreakdownPage />}
          {page === "models" && <ModelBreakdownPage />}
          {page === "budget" && <BudgetSettingsPage />}
          {page === "api-keys" && <ApiKeySettingsPage />}
          {page === "pricing" && <ModelPricingPage />}
          {page === "history" && <UsageHistoryPage />}
        </main>
      </div>
    </div>
  );
}

export default App;
