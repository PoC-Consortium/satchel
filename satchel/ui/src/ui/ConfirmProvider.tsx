import { createContext, useCallback, useContext, useRef, useState, type ReactNode } from "react";
import { Button, Dialog, DialogActions, DialogContent, DialogContentText, DialogTitle } from "@mui/material";
import { useT } from "../i18n";

// Promise-based confirm dialog — a typed MUI replacement for window.confirm,
// used for the irreversible/destructive actions (send, take, abort, remove).

export interface ConfirmOpts {
  title: string;
  body: ReactNode;
  confirmLabel?: string;
  cancelLabel?: string;
  danger?: boolean;
  /** Widen the dialog for richer bodies (e.g. the take summary with fees). */
  wide?: boolean;
  /** Disable the confirm button (e.g. insufficient funds) — the user can still
   *  read the summary and cancel, but can't proceed. Fixed at call time; a
   *  component inside `body` can override it reactively via useConfirmDisable. */
  confirmDisabled?: boolean;
}

type ConfirmFn = (opts: ConfirmOpts) => Promise<boolean>;

const Ctx = createContext<ConfirmFn | null>(null);

// Body-driven override of `confirmDisabled`: lets a component rendered inside
// the confirm body flip the confirm button as its own async work resolves (e.g.
// LockFundsGate streaming the funds pre-check), instead of the caller awaiting
// chain-touching calls BEFORE opening the dialog. Reset on every confirm().
const DisableCtx = createContext<((disabled: boolean) => void) | null>(null);

export function useConfirm(): ConfirmFn {
  const c = useContext(Ctx);
  if (!c) throw new Error("useConfirm outside ConfirmProvider");
  return c;
}

/** For components rendered INSIDE a confirm body: reactively enable/disable the
 *  dialog's confirm button (overrides the caller's static `confirmDisabled`). */
export function useConfirmDisable(): (disabled: boolean) => void {
  const c = useContext(DisableCtx);
  if (!c) throw new Error("useConfirmDisable outside ConfirmProvider");
  return c;
}

export function ConfirmProvider({ children }: { children: ReactNode }) {
  const t = useT();
  const [open, setOpen] = useState(false);
  const [opts, setOpts] = useState<ConfirmOpts | null>(null);
  // null = no body override; the caller's static confirmDisabled applies.
  const [bodyDisabled, setBodyDisabled] = useState<boolean | null>(null);
  const resolver = useRef<(v: boolean) => void>();

  const confirm = useCallback<ConfirmFn>((o) => {
    setOpts(o);
    setBodyDisabled(null); // each dialog starts from its caller's static flag
    setOpen(true);
    return new Promise<boolean>((resolve) => {
      resolver.current = resolve;
    });
  }, []);

  const close = (result: boolean) => {
    setOpen(false);
    resolver.current?.(result);
  };

  return (
    <Ctx.Provider value={confirm}>
      {children}
      <DisableCtx.Provider value={setBodyDisabled}>
        <Dialog open={open} onClose={() => close(false)} maxWidth={opts?.wide ? "sm" : "xs"} fullWidth>
          {opts && (
            <>
              <DialogTitle>{opts.title}</DialogTitle>
              <DialogContent>
                <DialogContentText component="div" sx={{ whiteSpace: "pre-line" }}>
                  {opts.body}
                </DialogContentText>
              </DialogContent>
              <DialogActions sx={{ px: 3, pb: 2 }}>
                <Button onClick={() => close(false)} color="inherit">
                  {opts.cancelLabel ?? t("common.cancel")}
                </Button>
                <Button
                  onClick={() => close(true)}
                  variant="contained"
                  color={opts.danger ? "error" : "primary"}
                  disabled={bodyDisabled ?? opts.confirmDisabled}
                >
                  {opts.confirmLabel ?? t("common.confirm")}
                </Button>
              </DialogActions>
            </>
          )}
        </Dialog>
      </DisableCtx.Provider>
    </Ctx.Provider>
  );
}
