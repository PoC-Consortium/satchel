"""The Satchel GUI seam + the playground teardown machinery
(TEST_FRAMEWORK_PLAN §2.4) — everything the retired tools/playground-*.ps1
skeleton did, once:

  * satchel.json writing (UTF-8 WITHOUT BOM — a BOM breaks serde_json in
    pactd) + the coin-entry shapes (core-rpc vs pact-seed/Electrum);
  * Tauri sidecar staging for the host triple;
  * config-dir semantics (regtest wipe-pactd-state vs viewer persist);
  * launching Satchel (cargo tauri dev, or the BUILT exe for two-window
    setups) with SATCHEL_NETWORK / SATCHEL_DATA_DIR / WebView2 isolation;
  * teardown: pidfile tree-kills, orphan-driver hunt by CMDLINE (never a bare
    process name), the port sweep off framework.util's mainnet-safe registry,
    and the wait-until-ports-free loop.

HARD INVARIANT (memory `no-kill-nodes-by-name`): every kill here is by
recorded PID or by REGISTRY PORT. Nothing is ever killed by process name, and
the registry structurally excludes the live mainnet/testnet pactd (9737/9738).
"""

import json
import os
import shutil
import subprocess
import sys
import time

from framework import binaries
from framework.util import all_teardown_ports

LOG_DIR = os.path.join(binaries.REPO_DIR, ".playground")
PID_FILE = os.path.join(LOG_DIR, "pids.txt")

MANAGED_PORT = 9739          # Satchel's regtest pactd offset (mainnet 9737!)
OBSERVER_PORT = 9740
VIEWER_PORT = 9747
VITE_PORT = 5173

IS_WINDOWS = sys.platform == "win32"


# ---------------------------------------------------------------------------
# Config dirs + satchel.json

def config_base(name="org.pocx.satchel"):
    """Satchel's config root for a given app-dir base name, per OS (the Tauri
    app-config convention)."""
    local = os.environ.get("LOCALAPPDATA")
    if local:
        return os.path.join(local, name)
    if os.path.isdir(os.path.expanduser("~/Library/Application Support")):
        return os.path.expanduser(f"~/Library/Application Support/{name}")
    return os.path.expanduser(f"~/.config/{name}")


def core_rpc_coin(coin_id, rpc_port, wallet, confirmations):
    """A node-backed coin entry (user/pass auth, NOT cookie)."""
    return {
        "coin_id": coin_id,
        "chain_data": f"http://pactharness:pactharness@127.0.0.1:{rpc_port}"
                      f"/wallet/{wallet}",
        "funding_wallet": "core-rpc",
        "confirmations": confirmations,
    }


def electrum_coin(coin_id, servers, confirmations):
    """A nodeless (pact-seed) coin entry. IMPORTANT: for a nodeless coin,
    pactd/Satchel consume `chain_data` VERBATIM (compose_chain_data() is
    skipped when auth_method is None), so the FULL server set must live in
    chain_data (home first, then views/standbys); extra_backends mirrors it —
    exactly how the coin-setup UI persists a nodeless coin."""
    return {
        "coin_id": coin_id,
        "chain_data": ",".join(servers),
        "funding_wallet": "pact-seed",
        "extra_backends": list(servers),
        "confirmations": confirmations,
    }


def write_satchel_json(net_dir, coins, board_urls, nostr_relays, listen_port,
                       tick_secs, auto_fund=True, ui_extra=None,
                       omit_relays_key=False):
    """Write <net_dir>/satchel.json, UTF-8 WITHOUT BOM. omit_relays_key: leave
    `nostr_relays` out entirely so the container-level serde default fills the
    six RECOMMENDED_NOSTR_RELAYS (the mainnet viewer)."""
    ui = {"theme": "system", "language": "en", "nav_open": True}
    ui.update(ui_extra or {})
    cfg = {
        "pactd_path": binaries.pactd().replace(os.sep, "/"),
        "coins": coins,
        "board_urls": board_urls,
        "nostr_relays": nostr_relays,
        "listen": f"127.0.0.1:{listen_port}",
        "auto_fund": auto_fund,
        "tick_secs": tick_secs,
        "ui": ui,
    }
    if omit_relays_key:
        del cfg["nostr_relays"]
    if not auto_fund:
        del cfg["auto_fund"]
    os.makedirs(net_dir, exist_ok=True)
    path = os.path.join(net_dir, "satchel.json")
    with open(path, "w", encoding="utf-8", newline="\n") as fh:  # no BOM
        json.dump(cfg, fh, indent=2)
        fh.write("\n")
    return path


def refresh_pactd_path(net_dir):
    """Viewer --persist: keep the user's config, but make pactd_path track
    this checkout."""
    path = os.path.join(net_dir, "satchel.json")
    with open(path, encoding="utf-8") as fh:
        cfg = json.load(fh)
    cfg["pactd_path"] = binaries.pactd().replace(os.sep, "/")
    with open(path, "w", encoding="utf-8", newline="\n") as fh:
        json.dump(cfg, fh, indent=2)
        fh.write("\n")


def wipe_pactd_state(net_dir):
    """Factory-new managed pactd (seed/db/relay cursor) for a reproducible
    regtest run; the rest of the config dir stays."""
    state = os.path.join(net_dir, "pactd")
    if os.path.exists(state):
        shutil.rmtree(state)
    os.makedirs(net_dir, exist_ok=True)


def copy_coin_templates(net_dir):
    """coins.toml (consensus params for file-added coins like ltc) + the LTC
    icon next to satchel.json, so Satchel AND its managed pactd resolve the
    `ltc` template. Harmless when ltc is unused."""
    src = os.path.join(binaries.REPO_DIR, "satchel")
    shutil.copy2(os.path.join(src, "coins.toml"), os.path.join(net_dir, "coins.toml"))
    shutil.copy2(os.path.join(src, "ltc.svg"), os.path.join(net_dir, "ltc.svg"))


# ---------------------------------------------------------------------------
# Builds + sidecars

def host_triple():
    out = subprocess.run(["rustc", "-vV"], capture_output=True, text=True,
                         check=True).stdout
    for line in out.splitlines():
        if line.startswith("host:"):
            return line.split(":", 1)[1].strip()
    raise RuntimeError("rustc -vV did not report a host triple")


def stage_sidecars():
    """tauri.conf declares pactd + pact-cli as externalBin; a binary must
    exist for the host triple or `cargo tauri dev`/`build` refuses to start.
    (satchel.json still points pactd_path at the absolute debug pactd — the
    copy only satisfies the build.)"""
    triple = host_triple()
    dest = os.path.join(binaries.REPO_DIR, "satchel", "binaries")
    os.makedirs(dest, exist_ok=True)
    ext = binaries.EXE
    shutil.copy2(binaries.pactd(), os.path.join(dest, f"pactd-{triple}{ext}"))
    shutil.copy2(binaries.pact_cli(), os.path.join(dest, f"pact-cli-{triple}{ext}"))


def build_satchel_exe():
    """A STANDALONE satchel binary (frontendDist = ui/dist, no vite dev
    server) for two-window setups: `cargo tauri build --debug --no-bundle`."""
    subprocess.run(["cargo", "tauri", "build", "--debug", "--no-bundle"],
                   cwd=os.path.join(binaries.REPO_DIR, "satchel"), check=True)
    exe = os.path.join(binaries.REPO_DIR, "satchel", "target", "debug",
                       "satchel" + binaries.EXE)
    if not os.path.exists(exe):
        raise RuntimeError(f"satchel binary not found at {exe}")
    return exe


# ---------------------------------------------------------------------------
# Launching

def _logf(name):
    os.makedirs(LOG_DIR, exist_ok=True)
    return open(os.path.join(LOG_DIR, name), "w", encoding="utf-8")


def launch_tauri_dev(network, data_dir=None, log_prefix="satchel"):
    """cargo tauri dev (Vite + the Tauri window + managed pactd). Returns the
    Popen. SATCHEL_NETWORK selects the network subdir; SATCHEL_DATA_DIR (when
    given) isolates the whole config root."""
    env = dict(os.environ)
    env["SATCHEL_NETWORK"] = network
    if data_dir:
        env["SATCHEL_DATA_DIR"] = data_dir
    proc = subprocess.Popen(
        ["cargo", "tauri", "dev"],
        cwd=os.path.join(binaries.REPO_DIR, "satchel"),
        stdout=_logf(f"{log_prefix}.out.log"), stderr=_logf(f"{log_prefix}.err.log"),
        env=env, **({"start_new_session": True} if not IS_WINDOWS else {}))
    record_pid(proc.pid)
    return proc


def launch_built(exe, network, data_dir=None, webview_dir=None,
                 log_prefix="satchel"):
    """Run the built satchel exe. Two instances of the same exe share one
    WebView2 user-data dir by default and the SECOND window fails to create
    its webview — hand each its own playground-local dir."""
    env = dict(os.environ)
    env["SATCHEL_NETWORK"] = network
    if data_dir:
        env["SATCHEL_DATA_DIR"] = data_dir
    if webview_dir:
        env["WEBVIEW2_USER_DATA_FOLDER"] = webview_dir
    proc = subprocess.Popen(
        [exe],
        stdout=_logf(f"{log_prefix}.out.log"), stderr=_logf(f"{log_prefix}.err.log"),
        env=env, **({"start_new_session": True} if not IS_WINDOWS else {}))
    record_pid(proc.pid)
    return proc


def wait_health(port, secs=45):
    """Poll a managed pactd's /health (no auth needed)."""
    import urllib.request
    deadline = time.time() + secs
    while time.time() < deadline:
        try:
            with urllib.request.urlopen(f"http://127.0.0.1:{port}/health", timeout=2):
                return True
        except Exception:  # noqa: BLE001
            time.sleep(0.5)
    return False


# ---------------------------------------------------------------------------
# Teardown (PID/port-only — see the module docstring invariant)

def record_pid(pid):
    os.makedirs(LOG_DIR, exist_ok=True)
    with open(PID_FILE, "a", encoding="utf-8") as fh:
        fh.write(f"{pid}\n")


def kill_pid_tree(pid):
    """Force-kill a recorded PID with its whole tree (cargo -> vite + satchel
    -> managed pactd)."""
    if not pid or pid <= 0:
        return
    if IS_WINDOWS:
        subprocess.run(["taskkill", "/T", "/F", "/PID", str(pid)],
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    else:
        import signal
        try:
            os.killpg(pid, signal.SIGKILL)   # launched with start_new_session
        except (ProcessLookupError, PermissionError):
            try:
                os.kill(pid, signal.SIGKILL)
            except ProcessLookupError:
                pass


def _pids_on_port(port):
    pids = set()
    if IS_WINDOWS:
        out = subprocess.run(["netstat", "-ano", "-p", "TCP"],
                             capture_output=True, text=True).stdout
        for line in out.splitlines():
            parts = line.split()
            if (len(parts) >= 5 and parts[0] == "TCP"
                    and parts[1].endswith(f":{port}") and parts[3] == "LISTENING"):
                try:
                    pids.add(int(parts[4]))
                except ValueError:
                    pass
    else:
        out = subprocess.run(["lsof", "-ti", f"tcp:{port}", "-sTCP:LISTEN"],
                             capture_output=True, text=True).stdout
        for line in out.split():
            try:
                pids.add(int(line))
            except ValueError:
                pass
    return pids


def kill_by_ports(ports):
    for port in ports:
        for pid in _pids_on_port(port):
            kill_pid_tree(pid)


def wait_ports_free(ports, timeout=20):
    """A force-killed node holds its socket briefly; starting a fresh node
    before the port frees races the bind."""
    deadline = time.time() + timeout
    while time.time() < deadline:
        if not any(_pids_on_port(p) for p in ports):
            return True
        time.sleep(0.5)
    return False


def kill_orphan_drivers(cmdline_marker=r"-m play(\s|$)"):
    """A half-dead prior play driver holds no listening port, yet its Harness
    cleanup would `stop` the FRESH nodes we are about to start. Hunt it by
    CMDLINE REGEX (`python -m play …`) — never by a bare process name (the
    mainnet daemons are unreachable by this match)."""
    me = os.getpid()
    if IS_WINDOWS:
        ps = ("Get-CimInstance Win32_Process -Filter \"Name = 'python.exe'\" | "
              "Where-Object { $_.CommandLine -and $_.CommandLine -match '%s' } | "
              "Select-Object -ExpandProperty ProcessId" % cmdline_marker)
        out = subprocess.run(["powershell", "-NoProfile", "-Command", ps],
                             capture_output=True, text=True).stdout
        for line in out.split():
            try:
                pid = int(line)
            except ValueError:
                continue
            if pid != me:
                kill_pid_tree(pid)
    else:
        out = subprocess.run(["pgrep", "-f", cmdline_marker],
                             capture_output=True, text=True).stdout
        for line in out.split():
            try:
                pid = int(line)
            except ValueError:
                continue
            if pid != me:
                kill_pid_tree(pid)


def teardown(ports=None, pid_file=PID_FILE, hunt_orphans=True, tag="play"):
    """Full playground teardown: recorded PID trees -> orphan drivers ->
    the port sweep (defaults to the ENTIRE mainnet-safe registry, which
    closes the old knockdown.ps1 gaps) -> wait for the ports to free."""
    ports = list(ports) if ports is not None else all_teardown_ports()
    if os.path.exists(pid_file):
        with open(pid_file, encoding="utf-8") as fh:
            for line in fh:
                try:
                    kill_pid_tree(int(line.strip()))
                except ValueError:
                    pass
        os.remove(pid_file)
    if hunt_orphans:
        kill_orphan_drivers()
    kill_by_ports(ports)
    if not wait_ports_free(ports):
        print(f"[{tag}] warning: some playground ports still busy after 20s")
