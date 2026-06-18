import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { checkAppUpdate, getAppVersion, inTauri, type UpdateInfo } from "./api/tauri";
import { APP_VERSION } from "./version";

// In-app update notifications — the Phoenix pattern (phoenix-pocx
// AppUpdateService). On startup and every 6h we ask the Rust backend to check
// GitHub releases (PoC-Consortium/satchel); when a newer one exists the sidebar
// shows a badge that opens a dialog. A dismissed version is remembered so the
// badge stays quiet until the NEXT release. Everything degrades silently: no
// Tauri, offline, or no releases yet → no badge, no error.

const CHECK_INTERVAL_MS = 6 * 60 * 60 * 1000; // 6 hours
const DISMISSED_KEY = "satchel-dismissed-update-version";

interface UpdateCtx {
  /** Current app version (from the backend; falls back to the build constant). */
  version: string;
  /** Latest update info, or null until/unless a check succeeds. */
  info: UpdateInfo | null;
  /** Whether to show the badge: an update is available and not dismissed. */
  showBadge: boolean;
  /** Re-run the check now (also opens the dialog — used by the badge click). */
  openDialog: () => void;
  closeDialog: () => void;
  dialogOpen: boolean;
  /** Silence the badge for this version until a newer one ships. */
  dismiss: () => void;
}

const Ctx = createContext<UpdateCtx | null>(null);

export function useUpdate(): UpdateCtx {
  const c = useContext(Ctx);
  if (!c) throw new Error("useUpdate outside UpdateProvider");
  return c;
}

export function UpdateProvider({ children }: { children: ReactNode }) {
  const [version, setVersion] = useState<string>(APP_VERSION);
  const [info, setInfo] = useState<UpdateInfo | null>(null);
  const [dialogOpen, setDialogOpen] = useState(false);
  // Dismissed version, persisted in localStorage (mirrors Phoenix). Held in
  // state too so showBadge stays reactive after a dismiss.
  const [dismissed, setDismissed] = useState<string | null>(() => localStorage.getItem(DISMISSED_KEY));

  const check = useCallback(async () => {
    try {
      setInfo(await checkAppUpdate());
    } catch {
      /* no releases yet / offline / rate-limited — leave info as-is (no badge) */
    }
  }, []);

  // Keep `check` reachable from the interval without it being a dep.
  const checkRef = useRef(check);
  checkRef.current = check;

  useEffect(() => {
    if (!inTauri()) return;
    void getAppVersion()
      .then(setVersion)
      .catch(() => {
        /* keep the build constant */
      });
    void check();
    const id = setInterval(() => void checkRef.current(), CHECK_INTERVAL_MS);
    return () => clearInterval(id);
  }, [check]);

  const showBadge = useMemo(() => {
    if (!info?.available || !info.latestVersion) return false;
    return dismissed !== info.latestVersion;
  }, [info, dismissed]);

  const dismiss = useCallback(() => {
    if (info?.latestVersion) {
      localStorage.setItem(DISMISSED_KEY, info.latestVersion);
      setDismissed(info.latestVersion);
    }
  }, [info]);

  const openDialog = useCallback(() => {
    void check();
    setDialogOpen(true);
  }, [check]);
  const closeDialog = useCallback(() => setDialogOpen(false), []);

  const value: UpdateCtx = {
    version,
    info,
    showBadge,
    openDialog,
    closeDialog,
    dialogOpen,
    dismiss,
  };
  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}
