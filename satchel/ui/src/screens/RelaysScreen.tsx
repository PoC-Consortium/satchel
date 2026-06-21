import { useCallback, useEffect, useRef, useState } from "react";
import { Box, Button, Card, CardContent, IconButton, Tooltip, Typography } from "@mui/material";
import RefreshIcon from "@mui/icons-material/Refresh";
import { rpc } from "../api/tauri";
import { useNavigate } from "../ui/nav";
import { useT } from "../i18n";
import { EmptyState } from "../components/StatusViews";
import { formatBytes, uptimeSince } from "../format";
import { C } from "../theme";
import type { RelayStatus } from "../api/types";

// The Relays monitor (Phoenix "peers"-equivalent): one row per configured Nostr
// relay with its live connection status, latency, uptime and traffic. Read-only;
// relays are added/removed in Settings → Network. Polls pactd `boardstatus`
// (a cheap in-memory read of the relay pool) while mounted.
const POLL_MS = 5000;

/** Status token → dot colour. Connected = green, connecting/pending = amber,
 *  hard-stopped (terminated/banned) = red, otherwise grey. */
function dotColor(status: string | undefined, connected: boolean): string {
  if (connected || status === "connected") return "success.main";
  if (status === "connecting" || status === "pending") return "warning.main";
  if (status === "terminated" || status === "banned") return "error.main";
  return "text.disabled";
}

export default function RelaysScreen() {
  const t = useT();
  const navigate = useNavigate();
  const [rows, setRows] = useState<RelayStatus[] | null>(null);
  const [loading, setLoading] = useState(false);
  const [notConnected, setNotConnected] = useState(false);
  const busy = useRef(false);

  const load = useCallback(async () => {
    if (busy.current) return;
    busy.current = true;
    setLoading(true);
    try {
      const r = await rpc<{ relays: RelayStatus[] }>("boardstatus");
      setRows(r.relays || []);
      setNotConnected(false);
    } catch {
      setNotConnected(true);
    } finally {
      busy.current = false;
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
    const id = setInterval(() => void load(), POLL_MS);
    return () => clearInterval(id);
  }, [load]);

  const up = (rows ?? []).filter((r) => r.connected).length;
  const total = rows?.length ?? 0;

  return (
    <Box sx={{ display: "flex", flexDirection: "column", gap: 2 }}>
      <Box sx={{ display: "flex", alignItems: "center", gap: 1.5 }}>
        <Box sx={{ flex: 1, minWidth: 0 }}>
          <Typography variant="h1" sx={{ fontSize: 18, mb: 0.5 }}>
            {t("relays.title")}
          </Typography>
          <Typography sx={{ color: "text.secondary", fontSize: 13 }}>
            {t("relays.subtitle")}
          </Typography>
        </Box>
        {total > 0 && (
          <Typography sx={{ fontSize: 13, color: up > 0 ? "success.main" : "warning.main", fontWeight: 600 }}>
            {t("relays.connectedCount", { up, total })}
          </Typography>
        )}
        <Tooltip title={t("relays.refresh")}>
          <span>
            <IconButton size="small" onClick={() => void load()} disabled={loading} aria-label={t("relays.refresh")}>
              <RefreshIcon
                fontSize="small"
                sx={loading ? { animation: "spin 1s linear infinite", "@keyframes spin": { to: { transform: "rotate(360deg)" } } } : undefined}
              />
            </IconButton>
          </span>
        </Tooltip>
      </Box>

      {notConnected ? (
        <EmptyState title={t("relays.notConnected")}>{t("relays.notConnectedBody")}</EmptyState>
      ) : rows && rows.length === 0 ? (
        <EmptyState
          title={t("relays.none")}
          action={
            <Button variant="contained" onClick={() => navigate("settings")}>
              {t("relays.goToNetwork")}
            </Button>
          }
        >
          {t("relays.noneBody")}
        </EmptyState>
      ) : (
        <Card variant="outlined">
          <CardContent sx={{ p: 0, "&:last-child": { pb: 0 } }}>
            {(rows ?? []).map((r, i) => (
              <RelayRow key={r.url} r={r} last={i === (rows?.length ?? 0) - 1} />
            ))}
          </CardContent>
        </Card>
      )}
    </Box>
  );
}

function RelayRow({ r, last }: { r: RelayStatus; last: boolean }) {
  const t = useT();
  const statsTip = t("relays.statsTip", {
    success: r.success ?? 0,
    attempts: r.attempts ?? 0,
    down: formatBytes(r.bytes_received),
    up: formatBytes(r.bytes_sent),
  });
  return (
    <Tooltip title={statsTip} placement="top-start">
      <Box
        sx={{
          display: "flex",
          alignItems: "center",
          gap: 1.25,
          px: 2,
          py: 1.25,
          borderBottom: last ? "none" : `1px solid ${C.line}`,
        }}
      >
        {/* status dot */}
        <Box
          sx={{
            width: 9,
            height: 9,
            borderRadius: "50%",
            flex: "none",
            bgcolor: dotColor(r.status, r.connected),
          }}
        />
        {/* url (mono, truncates) */}
        <Typography
          sx={{
            flex: 1,
            minWidth: 0,
            fontFamily: C.mono,
            fontSize: 13,
            overflow: "hidden",
            textOverflow: "ellipsis",
            whiteSpace: "nowrap",
          }}
        >
          {r.url}
        </Typography>
        {/* status label */}
        <Typography
          sx={{
            flex: "none",
            width: 92,
            textAlign: "right",
            fontSize: 12,
            textTransform: "capitalize",
            color: r.connected ? "success.main" : "text.secondary",
          }}
        >
          {r.status ?? (r.connected ? t("relays.up") : t("relays.down"))}
        </Typography>
        {/* latency */}
        <Typography sx={{ flex: "none", width: 64, textAlign: "right", fontSize: 12, color: "text.secondary", fontVariantNumeric: "tabular-nums" }}>
          {r.latency_ms != null ? t("relays.ms", { ms: r.latency_ms }) : "—"}
        </Typography>
        {/* uptime */}
        <Typography sx={{ flex: "none", width: 72, textAlign: "right", fontSize: 12, color: "text.secondary", fontVariantNumeric: "tabular-nums" }}>
          {r.connected ? uptimeSince(r.connected_since) : "—"}
        </Typography>
      </Box>
    </Tooltip>
  );
}
