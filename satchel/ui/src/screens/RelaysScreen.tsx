import { useCallback, useEffect, useRef, useState } from "react";
import { Box, Button, Card, CardContent, IconButton, Tab, Tabs, Tooltip, Typography } from "@mui/material";
import RefreshIcon from "@mui/icons-material/Refresh";
import { rpc } from "../api/tauri";
import { useApp } from "../AppContext";
import { useNavigate } from "../ui/nav";
import { useT } from "../i18n";
import { EmptyState } from "../components/StatusViews";
import { formatBytes, uptimeSince } from "../format";
import { C } from "../theme";
import type { RelayStatus, ServerStatus } from "../api/types";

// The Network monitor: one tab for the Nostr relay pool (Phoenix
// "peers"-equivalent) plus one tab per configured coin with Electrum servers
// (issue #100). Strictly read-only AND display-only — both tabs poll cheap
// in-memory backend reads (`boardstatus` / `serverstatus`); opening this page
// never dials a relay or a server. Relays are added/removed in Settings →
// Network; Electrum servers in the coin's setup.
const POLL_MS = 5000;

/** Status token → dot colour. Connected = green, connecting/pending = amber,
 *  hard-stopped (terminated/banned) = red, otherwise grey. */
function dotColor(status: string | undefined, connected: boolean): string {
  if (connected || status === "connected") return "success.main";
  if (status === "connecting" || status === "pending") return "warning.main";
  if (status === "terminated" || status === "banned") return "error.main";
  return "text.disabled";
}

/** Server health state → dot colour. Healthy = green, in-backoff = red,
 *  untested standby = grey (it was simply never needed — the honest answer). */
function serverDotColor(state: string): string {
  if (state === "healthy") return "success.main";
  if (state === "down") return "error.main";
  return "text.disabled";
}

export default function RelaysScreen() {
  const t = useT();
  const { coins } = useApp();
  const [tab, setTab] = useState<string>("nostr");

  // One tab per configured coin that actually has Electrum servers.
  const serverCoins = coins.filter((c) => c.configured && (c.servers_total ?? 0) > 0);
  const active = tab !== "nostr" && !serverCoins.some((c) => c.id === tab) ? "nostr" : tab;

  return (
    <Box sx={{ display: "flex", flexDirection: "column", gap: 2 }}>
      <Box>
        <Typography variant="h1" sx={{ fontSize: 18, mb: 0.5 }}>
          {t("relays.title")}
        </Typography>
        <Tabs
          value={active}
          onChange={(_, v: string) => setTab(v)}
          sx={{ minHeight: 36, "& .MuiTab-root": { minHeight: 36, py: 0.5 } }}
        >
          <Tab value="nostr" label={t("relays.tabNostr")} />
          {serverCoins.map((c) => (
            <Tab key={c.id} value={c.id} label={c.display_name} />
          ))}
        </Tabs>
      </Box>
      {active === "nostr" ? (
        <NostrTab />
      ) : (
        <ServersTab
          coinId={active}
          coinName={serverCoins.find((c) => c.id === active)?.display_name ?? active}
        />
      )}
    </Box>
  );
}

// ---- Nostr relays (the original Relays monitor, unchanged behavior) --------

function NostrTab() {
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
        <Typography sx={{ flex: 1, minWidth: 0, color: "text.secondary", fontSize: 13 }}>
          {t("relays.subtitle")}
        </Typography>
        {total > 0 && (
          <Typography sx={{ fontSize: 13, color: up > 0 ? "success.main" : "warning.main", fontWeight: 600 }}>
            {t("relays.connectedCount", { up, total })}
          </Typography>
        )}
        <RefreshButton loading={loading} onClick={() => void load()} />
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

function RefreshButton({ loading, onClick }: { loading: boolean; onClick: () => void }) {
  const t = useT();
  return (
    <Tooltip title={t("relays.refresh")}>
      <span>
        <IconButton size="small" onClick={onClick} disabled={loading} aria-label={t("relays.refresh")}>
          <RefreshIcon
            fontSize="small"
            sx={loading ? { animation: "spin 1s linear infinite", "@keyframes spin": { to: { transform: "rotate(360deg)" } } } : undefined}
          />
        </IconButton>
      </span>
    </Tooltip>
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

// ---- Electrum servers per coin (issue #100) ---------------------------------

function ServersTab({ coinId, coinName }: { coinId: string; coinName: string }) {
  const t = useT();
  const [rows, setRows] = useState<ServerStatus[] | null>(null);
  const [loading, setLoading] = useState(false);
  const [notConnected, setNotConnected] = useState(false);
  const busy = useRef(false);

  const load = useCallback(async () => {
    if (busy.current) return;
    busy.current = true;
    setLoading(true);
    try {
      const r = await rpc<{ servers: ServerStatus[] }>("serverstatus", [coinId]);
      setRows(r.servers || []);
      setNotConnected(false);
    } catch {
      setNotConnected(true);
    } finally {
      busy.current = false;
      setLoading(false);
    }
  }, [coinId]);

  useEffect(() => {
    setRows(null); // switching coins: don't show the previous coin's rows
    void load();
    const id = setInterval(() => void load(), POLL_MS);
    return () => clearInterval(id);
  }, [load]);

  const healthy = (rows ?? []).filter((r) => r.state === "healthy").length;
  const total = rows?.length ?? 0;

  return (
    <Box sx={{ display: "flex", flexDirection: "column", gap: 2 }}>
      <Box sx={{ display: "flex", alignItems: "center", gap: 1.5 }}>
        <Typography sx={{ flex: 1, minWidth: 0, color: "text.secondary", fontSize: 13 }}>
          {t("relays.serversSubtitle", { coin: coinName })}
        </Typography>
        {total > 0 && (
          <Typography sx={{ fontSize: 13, color: healthy > 0 ? "success.main" : "warning.main", fontWeight: 600 }}>
            {t("relays.healthyCount", { healthy, total })}
          </Typography>
        )}
        <RefreshButton loading={loading} onClick={() => void load()} />
      </Box>

      {notConnected ? (
        <EmptyState title={t("relays.notConnected")}>{t("relays.notConnectedBody")}</EmptyState>
      ) : rows && rows.length === 0 ? (
        <EmptyState title={t("relays.noServers")} />
      ) : (
        <Card variant="outlined">
          <CardContent sx={{ p: 0, "&:last-child": { pb: 0 } }}>
            {(rows ?? []).map((r, i) => (
              <ServerRow key={r.url} r={r} last={i === (rows?.length ?? 0) - 1} />
            ))}
          </CardContent>
        </Card>
      )}
    </Box>
  );
}

/** Role token → i18n label + colour. The elected wallet home leads, active
 *  views follow, cold standbys trail greyed. */
function roleBits(role: string | undefined): { key: string; color: string } | null {
  if (role === "wallet") return { key: "relays.roleWallet", color: "primary.main" };
  if (role === "view") return { key: "relays.roleView", color: "success.main" };
  if (role === "standby") return { key: "relays.roleStandby", color: "text.disabled" };
  return null;
}

function ServerRow({ r, last }: { r: ServerStatus; last: boolean }) {
  const t = useT();
  const role = roleBits(r.role);
  const stateLabel =
    r.state === "healthy"
      ? t("relays.stateHealthy")
      : r.state === "down"
        ? r.retry_in_secs != null && r.retry_in_secs > 0
          ? t("relays.retryIn", { secs: r.retry_in_secs })
          : t("relays.stateDown")
        : t("relays.stateUntested");
  const tip =
    t("relays.serverTip", { requests: r.requests ?? 0, failures: r.failures ?? 0 }) +
    (r.last_error ? ` · ${t("relays.serverErr", { error: r.last_error })}` : "");
  return (
    <Tooltip title={tip} placement="top-start">
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
        {/* health dot */}
        <Box
          sx={{
            width: 9,
            height: 9,
            borderRadius: "50%",
            flex: "none",
            bgcolor: serverDotColor(r.state),
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
        {/* role badge */}
        <Typography
          sx={{
            flex: "none",
            width: 64,
            textAlign: "right",
            fontSize: 11,
            letterSpacing: "0.05em",
            textTransform: "uppercase",
            color: role?.color ?? "text.disabled",
          }}
        >
          {role ? t(role.key) : "—"}
        </Typography>
        {/* health state */}
        <Typography
          sx={{
            flex: "none",
            width: 92,
            textAlign: "right",
            fontSize: 12,
            color: r.state === "healthy" ? "success.main" : r.state === "down" ? "error.main" : "text.secondary",
          }}
        >
          {stateLabel}
        </Typography>
        {/* latency */}
        <Typography sx={{ flex: "none", width: 64, textAlign: "right", fontSize: 12, color: "text.secondary", fontVariantNumeric: "tabular-nums" }}>
          {r.latency_ms != null ? t("relays.ms", { ms: r.latency_ms }) : "—"}
        </Typography>
      </Box>
    </Tooltip>
  );
}
