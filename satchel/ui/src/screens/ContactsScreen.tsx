import { useMemo, useState } from "react";
import {
  Box,
  Chip,
  IconButton,
  TextField,
  ToggleButton,
  ToggleButtonGroup,
  Tooltip,
  Typography,
} from "@mui/material";
import LockOutlinedIcon from "@mui/icons-material/LockOutlined";
import EditOutlinedIcon from "@mui/icons-material/EditOutlined";
import DeleteOutlineIcon from "@mui/icons-material/DeleteOutline";
import SearchIcon from "@mui/icons-material/Search";
import { useContacts } from "../contacts";
import { useConfirm } from "../ui/ConfirmProvider";
import { useT } from "../i18n";
import { C } from "../theme";
import { shortId } from "../identity";
import CounterpartyTag from "../components/CounterpartyTag";
import ContactEditDialog from "../components/ContactEditDialog";
import type { Contact, ContactStatus } from "../api/types";

type Filter = "all" | "trusted" | "blocked";

// The Contacts tab: manage the local-only address book. A private, single-device
// list — nothing here is published or shared (see contacts.tsx). Annotate the
// people you trade with so an opaque pubkey becomes "Alice from the meetup".
export default function ContactsScreen() {
  const t = useT();
  const { book, remove } = useContacts();
  const confirm = useConfirm();
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<Filter>("all");
  const [editId, setEditId] = useState<string | null>(null);

  const all = useMemo(
    () => Object.values(book).sort((a, b) => (b.added || 0) - (a.added || 0)),
    [book],
  );

  const rows = useMemo(() => {
    const q = query.trim().toLowerCase();
    return all.filter((c) => {
      if (filter === "trusted" && c.status !== "trusted") return false;
      if (filter === "blocked" && c.status !== "blocked") return false;
      if (!q) return true;
      return (
        c.nick.toLowerCase().includes(q) ||
        (c.note ?? "").toLowerCase().includes(q) ||
        c.id.toLowerCase().includes(q)
      );
    });
  }, [all, query, filter]);

  const askRemove = async (c: Contact) => {
    const who = c.nick.trim() || shortId(c.id);
    const ok = await confirm({
      title: t("contacts.removeConfirmTitle"),
      body: t("contacts.removeConfirmBody", { who }),
      confirmLabel: t("contacts.remove"),
      danger: true,
    });
    if (ok) remove(c.id);
  };

  return (
    <Box sx={{ display: "flex", flexDirection: "column", gap: 2 }}>
      <Box>
        <Typography variant="h1" sx={{ fontSize: 18, mb: 0.5 }}>
          {t("contacts.title")}
        </Typography>
        <Typography sx={{ color: "text.secondary", fontSize: 13 }}>
          {t("contacts.subtitle")}
        </Typography>
      </Box>

      {/* Local-only privacy reminder. */}
      <Box
        sx={{
          display: "flex",
          alignItems: "flex-start",
          gap: 1,
          p: 1.25,
          borderRadius: 1.5,
          border: `1px solid ${C.line}`,
          bgcolor: "action.hover",
        }}
      >
        <LockOutlinedIcon sx={{ fontSize: 18, color: "text.secondary", mt: 0.2, flex: "none" }} />
        <Typography sx={{ fontSize: 12.5, color: "text.secondary" }}>
          {t("contacts.privacyNote")}
        </Typography>
      </Box>

      {/* Controls: search + standing filter. */}
      <Box sx={{ display: "flex", gap: 1.5, flexWrap: "wrap", alignItems: "center" }}>
        <TextField
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder={t("contacts.searchPlaceholder")}
          size="small"
          sx={{ flex: 1, minWidth: 220 }}
          slotProps={{
            input: {
              startAdornment: <SearchIcon sx={{ fontSize: 18, color: "text.disabled", mr: 0.75 }} />,
            },
          }}
        />
        <ToggleButtonGroup
          value={filter}
          exclusive
          size="small"
          onChange={(_, v: Filter | null) => v && setFilter(v)}
        >
          <ToggleButton value="all">{t("contacts.filterAll")}</ToggleButton>
          <ToggleButton value="trusted">{t("contacts.filterTrusted")}</ToggleButton>
          <ToggleButton value="blocked">{t("contacts.filterBlocked")}</ToggleButton>
        </ToggleButtonGroup>
      </Box>

      {rows.length === 0 ? (
        <Typography sx={{ color: "text.secondary", fontSize: 13, py: 4, textAlign: "center" }}>
          {all.length === 0 ? t("contacts.empty") : t("contacts.emptyFiltered")}
        </Typography>
      ) : (
        <Box sx={{ display: "flex", flexDirection: "column" }}>
          {rows.map((c) => (
            <ContactRow
              key={c.id}
              c={c}
              onEdit={() => setEditId(c.id)}
              onRemove={() => askRemove(c)}
            />
          ))}
          <Typography sx={{ color: "text.disabled", fontSize: 11.5, mt: 1 }}>
            {t("contacts.count", { n: rows.length })}
          </Typography>
        </Box>
      )}

      {editId && (
        <ContactEditDialog id={editId} open={!!editId} onClose={() => setEditId(null)} />
      )}
    </Box>
  );
}

const STATUS_CHIP: Record<ContactStatus, { labelKey: string; color: string | null }> = {
  trusted: { labelKey: "contacts.statusTrusted", color: C.good },
  blocked: { labelKey: "contacts.statusBlocked", color: C.bad },
  neutral: { labelKey: "contacts.statusNeutral", color: null },
};

function ContactRow({
  c,
  onEdit,
  onRemove,
}: {
  c: Contact;
  onEdit: () => void;
  onRemove: () => void;
}) {
  const t = useT();
  const chip = STATUS_CHIP[c.status];
  const added = c.added ? new Date(c.added).toLocaleDateString() : "—";

  return (
    <Box
      sx={{
        display: "grid",
        gridTemplateColumns: "minmax(160px, 1.2fr) minmax(0, 2fr) auto auto auto",
        alignItems: "center",
        gap: 1.5,
        py: 1,
        px: 0.5,
        borderBottom: `1px solid ${C.line}`,
      }}
    >
      <CounterpartyTag id={c.id} />

      <Typography
        sx={{ fontSize: 12.5, color: c.note ? "text.secondary" : "text.disabled" }}
        noWrap
        title={c.note || ""}
      >
        {c.note || "—"}
      </Typography>

      {chip.color ? (
        <Chip
          size="small"
          variant="outlined"
          label={t(chip.labelKey)}
          sx={{ height: 22, color: chip.color, borderColor: chip.color }}
        />
      ) : (
        <Chip
          size="small"
          variant="outlined"
          label={t(chip.labelKey)}
          sx={{ height: 22, color: "text.secondary", borderColor: "divider" }}
        />
      )}

      <Typography sx={{ fontSize: 12, color: "text.disabled", fontFamily: C.mono }}>
        {added}
      </Typography>

      <Box sx={{ display: "flex", gap: 0.25 }}>
        <Tooltip title={t("contacts.menuEdit")}>
          <IconButton size="small" onClick={onEdit} aria-label={t("contacts.menuEdit")}>
            <EditOutlinedIcon sx={{ fontSize: 17 }} />
          </IconButton>
        </Tooltip>
        <Tooltip title={t("contacts.remove")}>
          <IconButton size="small" onClick={onRemove} aria-label={t("contacts.remove")}>
            <DeleteOutlineIcon sx={{ fontSize: 17 }} />
          </IconButton>
        </Tooltip>
      </Box>
    </Box>
  );
}
