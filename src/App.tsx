import { useCallback, useEffect, useState, startTransition } from "react";
import { flushSync } from "react-dom";
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
  const windowLabel = getCurrentWindow().label;
  const isMainWindow = windowLabel === "main";
  const providerFilter = providerFromWindowLabel(windowLabel);
  const [mode, setMode] = useState<ViewMode>("widget");
  const [page, setPage] = useState<Page>("widget");
  const [pinned, setPinned] = useState(true);

  const [visited, setVisited] = useState<Set<Page>>(() => new Set(["dashboard"]));

  useEffect(() => {
    if (page !== "widget") {
      setVisited((prev) => {
        if (prev.has(page)) return prev;
        const next = new Set(prev);
        next.add(page);
        return next;
      });
    }
  }, [page]);

  const navigate = useCallback((next: Page) => {
    startTransition(() => setPage(next));
  }, []);

  const applyExpandedView = useCallback(() => {
    setMode("expanded");
    setPage("dashboard");
    setPinned(false);
    setVisited((prev) => {
      const next = new Set(prev);
      next.add("dashboard");
      next.add("api-keys");
      return next;
    });
  }, []);

  const applyWidgetView = useCallback(() => {
    setMode("widget");
    setPage("widget");
    setPinned(true);
  }, []);

  const syncMainViewState = useCallback(async () => {
    const expanded = await api.getMainViewExpanded();
    if (expanded) {
      flushSync(() => applyExpandedView());
    } else {
      flushSync(() => applyWidgetView());
    }
  }, [applyExpandedView, applyWidgetView]);

  const expand = useCallback(async () => {
    await api.openMainDashboard();
    if (getCurrentWindow().label === "main") {
      await syncMainViewState();
    }
  }, [syncMainViewState]);

  const collapse = useCallback(async () => {
    flushSync(() => applyWidgetView());
    await api.collapseToWidgets();
  }, [applyWidgetView]);

  const hideExpanded = useCallback(async () => {
    await collapse();
    await getCurrentWindow().hide();
  }, [collapse]);

  const togglePin = useCallback(async () => {
    const next = !pinned;
    setPinned(next);
    await api.setAlwaysOnTop(next);
  }, [pinned]);

  const hide = useCallback(async () => {
    await getCurrentWindow().hide();
  }, []);

  useEffect(() => {
    if (!isMainWindow) {
      void api.setAlwaysOnTop(true);
      return;
    }
    void syncMainViewState();
  }, [isMainWindow, syncMainViewState]);

  useEffect(() => {
    if (!isMainWindow) return;

    const unlisten = listen("view-state-changed", () => {
      void syncMainViewState();
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [isMainWindow, syncMainViewState]);

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

  const pageTitle = PAGE_TITLES[page as Exclude<Page, "widget">];

  return (
    <div className="expanded-shell">
      <WindowTitleBar
        title="TOKENPULSE"
        subtitle={pageTitle}
        pinned={pinned}
        onTogglePin={togglePin}
        onCollapse={collapse}
        onHide={hideExpanded}
      />
      <div className="expanded-body">
        <Sidebar current={page} onNavigate={navigate} />
        <main className="expanded-main">
          {visited.has("dashboard") && (
            <div style={{ display: page === "dashboard" ? "block" : "none", height: "100%" }}>
              <DashboardPage />
            </div>
          )}
          {visited.has("providers") && (
            <div style={{ display: page === "providers" ? "block" : "none", height: "100%" }}>
              <ProviderBreakdownPage />
            </div>
          )}
          {visited.has("models") && (
            <div style={{ display: page === "models" ? "block" : "none", height: "100%" }}>
              <ModelBreakdownPage />
            </div>
          )}
          {visited.has("budget") && (
            <div style={{ display: page === "budget" ? "block" : "none", height: "100%" }}>
              <BudgetSettingsPage />
            </div>
          )}
          {visited.has("api-keys") && (
            <div style={{ display: page === "api-keys" ? "block" : "none", height: "100%" }}>
              <ApiKeySettingsPage />
            </div>
          )}
          {visited.has("pricing") && (
            <div style={{ display: page === "pricing" ? "block" : "none", height: "100%" }}>
              <ModelPricingPage />
            </div>
          )}
          {visited.has("history") && (
            <div style={{ display: page === "history" ? "block" : "none", height: "100%" }}>
              <UsageHistoryPage />
            </div>
          )}
        </main>
      </div>
    </div>
  );
}

export default App;
