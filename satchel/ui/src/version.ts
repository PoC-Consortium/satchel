// App version shown in the sidenav header + Settings → About.
//
// Static placeholder for now (C7): the real value will come from Tauri's
// app metadata and an update badge from polling the GitHub releases API. Kept
// in lockstep with `tauri.conf.json`'s `version` by hand until then. The display
// string carries the pre-release suffix (the rc release tag); the bundle/manifest
// versions stay numeric `0.1.0` (Windows installers reject non-numeric versions).
export const APP_VERSION = "0.1.0-rc1";

/** Update-check state — hard-wired "up to date" until C7 wires GitHub polling. */
export const UPDATE_AVAILABLE = false;
