import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Satchel's frontend build. Output goes to `ui/dist`, which tauri.conf.json
// points `frontendDist` at (and which `tauri::generate_context!()` embeds at
// compile time, so `cargo build` needs `dist` to exist — run `npm run build`
// first, or use `cargo tauri dev` which runs `beforeDevCommand`).
//
// Fixed dev port 5173 so tauri.conf.json's `devUrl` matches; `strictPort`
// makes a port clash fail loudly instead of silently moving the HMR server
// out from under Tauri.
export default defineConfig({
  plugins: [react()],
  // Relative base so the embedded assets resolve under Tauri's custom
  // protocol (tauri://localhost) rather than an absolute host root.
  base: "./",
  clearScreen: false,
  build: {
    outDir: "dist",
    emptyOutDir: true,
    target: "esnext",
  },
  server: {
    // Bind explicitly to IPv4. Vite's default `localhost` binds to IPv6 `::1`
    // only on Windows, while Tauri's dev-server health probe hits IPv4
    // `127.0.0.1` — the mismatch leaves Tauri stuck "Waiting for your frontend
    // dev server to start". Pin both sides to 127.0.0.1 (see tauri.conf.json
    // devUrl) so they agree.
    host: "127.0.0.1",
    port: 5173,
    strictPort: true,
  },
});
