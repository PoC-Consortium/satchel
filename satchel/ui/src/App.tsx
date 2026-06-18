import { useCallback, useState } from "react";
import { Box } from "@mui/material";
import { useApp } from "./AppContext";
import { usePrefs } from "./prefs";
import { useT } from "./i18n";
import { DenomProvider } from "./denom";
import { UpdateProvider } from "./update";
import UpdateDialog from "./components/UpdateDialog";
import { ConfirmProvider } from "./ui/ConfirmProvider";
import { DialogsCtx, type DialogOpeners } from "./ui/dialogs";
import { NavCtx } from "./ui/nav";
import Header from "./components/Header";
import Sidebar, { type Route } from "./components/Sidebar";
import ActiveSwaps from "./components/ActiveSwaps";
import LogPanel from "./components/LogPanel";
import Toasts from "./components/Toasts";
import { Disconnected, NoTauri } from "./components/StatusViews";
import SwapsScreen from "./screens/SwapsScreen";
import CorkboardScreen from "./screens/CorkboardScreen";
import PostOfferScreen from "./screens/PostOfferScreen";
import PrivateCreateScreen from "./screens/PrivateCreateScreen";
import PrivateSlipsScreen from "./screens/PrivateSlipsScreen";
import PrivateReceiveScreen from "./screens/PrivateReceiveScreen";
import WalletScreen from "./screens/WalletScreen";
import SettingsScreen from "./screens/SettingsScreen";
import Wizard from "./dialogs/Wizard";
import SeedProvision from "./dialogs/SeedProvision";
import Unlock from "./dialogs/Unlock";
import MerchantManager from "./dialogs/MerchantManager";
import ExitGate from "./components/ExitGate";

type Modal = { kind: "merchants" } | { kind: "wizard"; mode: "create" | "import" } | null;

// Routes that show the active-swaps dock above the activity log. Trading views
// only (the Corkboard today; add the next trading view here when it lands).
const SWAP_DOCK_ROUTES: Route[] = ["board"];

export default function App() {
  const app = useApp();
  const t = useT();
  const { prefs, update } = usePrefs();
  const [route, setRoute] = useState<Route>("board");
  const [modal, setModal] = useState<Modal>(null);
  // Nav open/closed is a persisted UI pref (UI-1, in satchel.json) — the source
  // of truth is `prefs`, toggled through the prefs updater.
  const navOpen = prefs.nav_open;
  // UI-4: the docked log's collapsed state is local UI state (persisting it
  // would need an out-of-scope prefs field). Defaults to expanded so the
  // narration log — load-bearing for following swap progress — is visible.
  const [logCollapsed, setLogCollapsed] = useState(false);
  const toggleLog = useCallback(() => setLogCollapsed((c) => !c), []);
  const closeModal = () => setModal(null);

  const toggleNav = useCallback(() => update({ nav_open: !navOpen }), [update, navOpen]);

  const openers: DialogOpeners = {
    openMerchants: () => setModal({ kind: "merchants" }),
    openNewMerchant: (mode) => setModal({ kind: "wizard", mode }),
  };

  // Phase-driven gates render only when no user dialog is stacked over them.
  // First run (no merchant) opens straight into the merchant manager's empty
  // welcome — create/import there, then the wizard names + provisions.
  const showFirstRun = app.phase === "wizard" && modal === null;
  const showSeedGate = app.phase === "seed" && modal === null;
  const showUnlockGate = app.phase === "unlock" && modal === null;

  function screen() {
    if (app.phase === "no-tauri") return <NoTauri />;
    if (app.phase === "disconnected") return <Disconnected />;
    switch (route) {
      case "board":
        return <CorkboardScreen />;
      case "post-offer":
        return <PostOfferScreen />;
      case "private-create":
        return <PrivateCreateScreen />;
      case "private-slips":
        return <PrivateSlipsScreen />;
      case "private-receive":
        return <PrivateReceiveScreen />;
      case "swaps":
        return <SwapsScreen />;
      case "wallets":
        return <WalletScreen />;
      case "settings":
        return <SettingsScreen />;
    }
  }

  return (
    <DenomProvider>
    <UpdateProvider>
    <ConfirmProvider>
      <NavCtx.Provider value={setRoute}>
        <DialogsCtx.Provider value={openers}>
          <Box sx={{ display: "flex", height: "100vh", overflow: "hidden" }}>
            <Sidebar route={route} onNavigate={setRoute} open={navOpen} onToggle={toggleNav} />
            <Box
              component="main"
              sx={{ flexGrow: 1, minWidth: 0, display: "flex", flexDirection: "column" }}
            >
              <Header
                navOpen={navOpen}
                onMenuToggle={toggleNav}
                onSettings={() => setRoute("settings")}
                onLiveSwaps={() => setRoute("board")}
              />
              {/* Scrolling content column — the log lives OUTSIDE this so it
                  stays docked while long pages scroll (UI-4). */}
              <Box sx={{ flexGrow: 1, minHeight: 0, overflowY: "auto", display: "flex", flexDirection: "column" }}>
                <Box sx={{ p: 3, maxWidth: 1180, width: "100%", mx: "auto", flexGrow: 1 }}>
                  {screen()}
                </Box>
              </Box>
              {/* Fixed bottom docks — always visible regardless of page scroll.
                  Active swaps sit directly above the activity log on trading
                  views, and only when a swap is in flight (else it collapses). */}
              {SWAP_DOCK_ROUTES.includes(route) && <ActiveSwaps />}
              <LogPanel collapsed={logCollapsed} onToggle={toggleLog} />
            </Box>
          </Box>

          {/* Gating flows (boot-driven). First run → the empty merchant manager
              (welcome + create/import); choosing one opens the wizard. */}
          {showFirstRun && (
            <MerchantManager firstRun onNewMerchant={openers.openNewMerchant} />
          )}
          {showSeedGate && (
            <SeedProvision
              label={app.activeMerchant?.label ?? t("merchants.thisMerchant")}
              onDone={app.boot}
              onLater={closeModal}
            />
          )}
          {showUnlockGate && <Unlock onDone={app.boot} onSwitch={openers.openMerchants} />}

          {/* User-triggered dialogs. */}
          {modal?.kind === "merchants" && (
            <MerchantManager onClose={closeModal} onNewMerchant={openers.openNewMerchant} />
          )}
          {modal?.kind === "wizard" && (
            <Wizard
              mode={modal.mode}
              firstRun={app.phase === "wizard"}
              onClose={closeModal}
              onDone={async () => {
                // Refresh, then clear the modal ourselves (boot won't unmount a
                // modal-driven dialog) — else "Done" leaves it stuck open.
                await app.boot();
                closeModal();
              }}
            />
          )}

          <ExitGate />
          <UpdateDialog />
          <Toasts />
        </DialogsCtx.Provider>
      </NavCtx.Provider>
    </ConfirmProvider>
    </UpdateProvider>
    </DenomProvider>
  );
}
