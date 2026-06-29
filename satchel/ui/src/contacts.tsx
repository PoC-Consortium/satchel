import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { getContacts, inTauri, setContacts } from "./api/tauri";
import type { Contact, ContactBook, ContactStatus } from "./api/types";

// The local-only contact book: a private, single-machine address book mapping a
// counterparty's BIP340 hex pubkey to a user-chosen nick, freeform note, and a
// trusted/blocked standing. It lives in satchel.json (same "Satchel owns
// persisted state" principle as prefs/merchants) — NOT on a relay, never signed,
// never published. It's purely how THIS user annotates who they've traded with;
// the engine never sees it and it carries no protocol weight (a `blocked` flag
// only changes local display + warnings, it cannot stop a trade — atomicity
// does that).
//
// The nick is an ALIAS shown alongside the spoof-proof identicon/fingerprint,
// never a replacement (a chosen name must never be able to impersonate a key).
//
// Flow mirrors prefs.tsx: load once from `get_contacts` on mount, and write the
// whole book through `set_contacts` on every change (it's small + single-user,
// so a full-map write is simpler than per-entry patching and never races).

interface ContactsCtx {
  book: ContactBook;
  /** True once the persisted book has been read. */
  loaded: boolean;
  get: (id: string | null | undefined) => Contact | undefined;
  /** Create-or-update an entry (merges the patch); stamps `added` on first sight. */
  upsert: (id: string, patch: Partial<Omit<Contact, "id">>) => void;
  /** Convenience: set just the standing. */
  setStatus: (id: string, status: ContactStatus) => void;
  remove: (id: string) => void;
}

const Ctx = createContext<ContactsCtx | null>(null);

/** Normalize whatever `get_contacts` returned (null / Null / partial rows) into
 *  a well-formed book, so consumers never have to defend against missing fields. */
function normalize(raw: ContactBook | null | undefined): ContactBook {
  const out: ContactBook = {};
  if (!raw || typeof raw !== "object") return out;
  for (const [id, c] of Object.entries(raw)) {
    if (!c || typeof c !== "object") continue;
    out[id] = {
      id,
      nick: typeof c.nick === "string" ? c.nick : "",
      note: typeof c.note === "string" ? c.note : undefined,
      status: c.status === "trusted" || c.status === "blocked" ? c.status : "neutral",
      added: typeof c.added === "number" ? c.added : 0,
    };
  }
  return out;
}

export function ContactsProvider({ children }: { children: ReactNode }) {
  const [book, setBook] = useState<ContactBook>({});
  const [loaded, setLoaded] = useState(false);

  useEffect(() => {
    let alive = true;
    (async () => {
      if (!inTauri()) {
        if (alive) setLoaded(true);
        return;
      }
      try {
        const raw = await getContacts();
        if (alive) setBook(normalize(raw));
      } catch {
        /* keep empty */
      } finally {
        if (alive) setLoaded(true);
      }
    })();
    return () => {
      alive = false;
    };
  }, []);

  // Both writers update in-memory immediately (inside the state updater, so they
  // see the latest book) then best-effort persist the whole map. A failed write
  // leaves the UI correct; the next change retries the full book.
  const upsert = useCallback(
    (id: string, patch: Partial<Omit<Contact, "id">>) => {
      setBook((prev) => {
        const existing = prev[id];
        const merged: Contact = {
          id,
          nick: patch.nick ?? existing?.nick ?? "",
          note: "note" in patch ? patch.note : existing?.note,
          status: patch.status ?? existing?.status ?? "neutral",
          added: existing?.added ?? Date.now(),
        };
        const next = { ...prev, [id]: merged };
        if (inTauri()) void setContacts(next).catch(() => {});
        return next;
      });
    },
    [],
  );

  const remove = useCallback((id: string) => {
    setBook((prev) => {
      if (!(id in prev)) return prev;
      const next = { ...prev };
      delete next[id];
      if (inTauri()) void setContacts(next).catch(() => {});
      return next;
    });
  }, []);

  const setStatus = useCallback(
    (id: string, status: ContactStatus) => upsert(id, { status }),
    [upsert],
  );

  const get = useCallback(
    (id: string | null | undefined) => (id ? book[id] : undefined),
    [book],
  );

  const value = useMemo(
    () => ({ book, loaded, get, upsert, setStatus, remove }),
    [book, loaded, get, upsert, setStatus, remove],
  );
  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}

export function useContacts(): ContactsCtx {
  const c = useContext(Ctx);
  if (!c) throw new Error("useContacts outside ContactsProvider");
  return c;
}
