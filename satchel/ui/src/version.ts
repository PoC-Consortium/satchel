// App version shown in the sidenav header + Settings → About.
//
// Static placeholder for now (C7): the real value will come from Tauri's
// app metadata and an update badge from polling the GitHub releases API. Kept
// in lockstep with `tauri.conf.json`'s `version` by hand until then.
export const APP_VERSION = "0.1.0";

/** Update-check state — hard-wired "up to date" until C7 wires GitHub polling. */
export const UPDATE_AVAILABLE = false;
