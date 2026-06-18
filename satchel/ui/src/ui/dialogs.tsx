import { createContext, useContext } from "react";

// App-level dialog openers (merchant flows reachable from the header and the
// unlock gate). Screen-local dialogs (coin setup, board config) are owned by
// their screens instead, so they refresh their own data after a save.
export interface DialogOpeners {
  openMerchants: () => void;
  /** Open the new-merchant wizard for a chosen path (create vs import). */
  openNewMerchant: (mode: "create" | "import") => void;
}

export const DialogsCtx = createContext<DialogOpeners | null>(null);

export function useDialogs(): DialogOpeners {
  const c = useContext(DialogsCtx);
  if (!c) throw new Error("useDialogs outside provider");
  return c;
}
