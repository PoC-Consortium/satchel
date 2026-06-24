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
import LanguageMenu from "./components/LanguageMenu";
import Sidebar, { type Route } from "./components/Sidebar";
import ActiveSwaps from "./components/ActiveSwaps";
import LogPanel from "./components/LogPanel";
import Toasts from "./components/Toasts";
import { Disconnected, NoTauri } from "./components/StatusViews";
import SwapsScreen from "./screens/SwapsScreen";
import RelaysScreen from "./screens/RelaysScreen";
import CorkboardScreen from "./screens/CorkboardScreen";
import PostOfferScreen from "./screens/PostOfferScreen";
import PrivateCreateScreen from "./screens/PrivateCreateScreen";
import PrivateSlipsScreen from "./screens/PrivateSlipsScreen";
import PrivateReceiveScreen from "./screens/PrivateReceiveScreen";
import WalletScreen from "./screens/WalletScreen";
import SettingsScreen from "./screens/SettingsScreen";
import Wizard from "./dialogs/Wizard";
import SeedProvision from "./dialogs/SeedProvision";
import CoinWizard from "./dialogs/CoinWizard";
import Unlock from "./dialogs/Unlock";
import MerchantManager from "./dialogs/MerchantManager";
import ExitGate from "./components/ExitGate";

type Modal = { kind: "merchants" } | { kind: "wizard"; mode: "create" | "import" } | null;

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
  const showCoinGate = app.phase === "coins" && modal === null;

  // The header (and its language picker) sits behind these onboarding dialogs'
  // backdrops, so it can't be reached until setup finishes. Float a language
  // switcher above the backdrop during every pre-ready gate — phoenix lets you
  // set your language before onboarding; this gives the same freedom rather
  // than forcing a user through setup in a language they may not read.
  const onboardingGate =
    app.phase === "wizard" ||
    app.phase === "seed" ||
    app.phase === "unlock" ||
    app.phase === "coins";

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
      case "relays":
        return <RelaysScreen />;
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
              {/* Fixed bottom docks — always visible regardless of page scroll
                  AND on every page (RC2), so the fund button + funding alert are
                  never hidden behind a tab. Collapses when no swap is in flight. */}
              <ActiveSwaps />
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
          {showCoinGate && <CoinWizard onDone={app.boot} />}

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

          {/* First-run language escape hatch: lifted above the dialog backdrop
              (MUI modal = 1300) so it stays clickable while a gate is open. */}
          {onboardingGate && (
            <Box
              sx={{
                position: "fixed",
                top: 8,
                right: 12,
                zIndex: (th) => th.zIndex.modal + 100,
              }}
            >
              <LanguageMenu menuZIndex={1500} />
            </Box>
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
