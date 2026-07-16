"""Coordination services: the Corkboard noticeboard + the local Nostr relay.

Single home (Phase 1) — this kills the second, diverged NostrRelay copy that
lived in satchel_playground_nostr.py. The one deliberate unification: the
fail-loud port-in-use probe (a leaked relay's stale event DB poisons any run
that reuses its fixed port) now applies to playgrounds too, not just the e2e
suite.
"""

import os
import shlex
import socket
import subprocess
import time
import urllib.request

from framework import binaries

CORKBOARD_PORT = 19790
NOSTR_RELAY_PORT = 19791             # the e2e suites' relay
PLAYGROUND_NOSTR_RELAY_PORT = 19788  # the playgrounds' relay (ps1-pinned in
                                     # Alice's satchel.json as ws://…:19788)


class Corkboard:
    """The noticeboard server."""

    def __init__(self, workdir, port=CORKBOARD_PORT):
        self.port = port
        self.db = os.path.join(workdir, "corkboard.sqlite")
        self.url = f"http://127.0.0.1:{port}"
        self.proc = None

    def start(self):
        self.proc = subprocess.Popen(
            [binaries.corkboard(), "--listen", f"127.0.0.1:{self.port}", "--db", self.db],
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        deadline = time.time() + 30
        while time.time() < deadline:
            if self.proc.poll() is not None:
                raise RuntimeError(f"corkboard exited early: {self.proc.returncode}")
            try:
                urllib.request.urlopen(f"{self.url}/health", timeout=5)
                print(f"[e2e] corkboard up on :{self.port}")
                return
            except Exception:
                time.sleep(0.2)
        raise TimeoutError("corkboard did not come up")

    def stop(self):
        if self.proc:
            self.proc.terminate()
            try:
                self.proc.wait(timeout=15)
            except subprocess.TimeoutExpired:
                self.proc.kill()
            self.proc = None

    def reset(self):
        """Bring the board back up on the same URL/port but backed by a FRESH,
        empty DB — equivalent to an operator wipe / redeploy, while clients keep
        their (now ahead-of-fresh-board) relay cursors. We switch to a new file
        rather than unlink the old one, which Windows may still hold briefly."""
        self.stop()
        self._gen = getattr(self, "_gen", 0) + 1
        self.db = os.path.join(os.path.dirname(self.db), f"corkboard-reset-{self._gen}.sqlite")
        self.start()


class NostrRelay:
    """A local Nostr relay (bundled nostr-rs-relay). Ephemeral: config + db
    live under the temp workspace, wiped on teardown. Override the binary with
    PACT_NOSTR_RELAY_BIN or the whole command with PACT_NOSTR_RELAY_CMD
    ({port}/{dir} substituted). `name` is the relay's self-reported info name
    (cosmetic; playgrounds pass "pact-playground")."""

    def __init__(self, workdir, port=NOSTR_RELAY_PORT, name="pact-e2e"):
        self.port = port
        self.host = "127.0.0.1"
        self.ws_url = f"ws://{self.host}:{port}"
        self.dir = os.path.join(workdir, "nostr-relay")
        self.name = name
        os.makedirs(self.dir, exist_ok=True)
        self.proc = None

    def _build_cmd(self):
        tmpl = os.environ.get("PACT_NOSTR_RELAY_CMD")
        if tmpl:
            return shlex.split(
                tmpl.replace("{port}", str(self.port)).replace("{dir}", self.dir))
        relay_bin = binaries.nostr_relay_default()
        if not os.path.exists(relay_bin):
            raise RuntimeError(
                f"nostr-rs-relay not found at {relay_bin}. Set PACT_NOSTR_RELAY_BIN "
                "or PACT_NOSTR_RELAY_CMD.")
        cfg = os.path.join(self.dir, "config.toml")
        db = self.dir.replace(os.sep, "/")
        with open(cfg, "w", encoding="utf-8") as fh:
            fh.write(
                f'[info]\nrelay_url = "{self.ws_url}/"\nname = "{self.name}"\n\n'
                f'[network]\naddress = "{self.host}"\nport = {self.port}\n\n'
                f'[database]\ndata_directory = "{db}"\n')
        return [relay_bin, "--config", cfg, "--db", self.dir]

    def start(self):
        # The port is fixed: a relay leaked by an earlier (crashed) run would
        # keep listening, this start would "succeed" against it, and its STALE
        # event DB would poison the scenario (same test npubs). Fail loudly.
        with socket.socket() as probe:
            if probe.connect_ex((self.host, self.port)) == 0:
                raise RuntimeError(
                    f"port {self.port} already in use — leaked relay from a "
                    "previous run? Kill it (by port, never by name) first.")
        cmd = self._build_cmd()
        self.proc = subprocess.Popen(
            cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        deadline = time.time() + 30
        while time.time() < deadline:
            if self.proc.poll() is not None:
                raise RuntimeError(f"nostr relay exited early: {self.proc.returncode}")
            try:
                with socket.create_connection((self.host, self.port), timeout=2):
                    print(f"[relay] nostr relay up on :{self.port} ({self.ws_url})")
                    return self
            except OSError:
                time.sleep(0.2)
        raise TimeoutError("nostr relay did not come up")

    def stop(self):
        if self.proc:
            self.proc.terminate()
            try:
                self.proc.wait(timeout=15)
            except subprocess.TimeoutExpired:
                self.proc.kill()
            self.proc = None
