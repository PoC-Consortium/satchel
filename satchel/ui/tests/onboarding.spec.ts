import { expect, test, type Page } from "@playwright/test";

// Exercise the production bundle with an in-memory Tauri boundary. No daemon,
// real seed, wallet, network service, or user configuration is touched.
async function boot(page: Page, locked = false) {
  await page.addInitScript(({ locked }) => {
    const w = window as unknown as Record<string, unknown>;
    const calls: string[] = [];
    w.testRpcCalls = calls;
    w.__TAURI_INTERNALS__ = {
      metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
      transformCallback: () => 1,
      unregisterCallback: () => {},
      invoke: async (command: string, args: { method?: string } = {}) => {
        if (command === "get_ui_prefs") return { language: "en", theme: "light", onboarded: true };
        if (command === "pactd_rpc") {
          const method = args.method!;
          calls.push(method);
          if (method === "listmerchants") return locked
            ? { active: "test", merchants: [{ id: "test", label: "Test merchant" }] }
            : { active: null, merchants: [] };
          if (method === "getinfo") return { seed_exists: true, locked: true, version: "test" };
          throw new Error(`Unexpected RPC in onboarding smoke test: ${method}`);
        }
        return null;
      },
    };
  }, { locked });
  await page.goto("/");
}

test("first-run welcome cannot be dismissed with Escape", async ({ page }) => {
  const errors: string[] = [];
  page.on("pageerror", error => errors.push(error.message));
  await boot(page);
  const dialog = page.getByRole("dialog");
  await expect(dialog).toContainText("Welcome to Satchel");
  await page.keyboard.press("Escape");
  await expect(dialog).toBeVisible();
  expect(errors).toEqual([]);
});

test("seed autocomplete preserves keyboard input and checksum gating", async ({ page }) => {
  const errors: string[] = [];
  page.on("pageerror", error => errors.push(error.message));
  await boot(page);
  await page.getByRole("button", { name: "Import", exact: true }).click();
  await page.getByLabel("Merchant name").fill("Import test");
  await page.getByRole("button", { name: "Continue", exact: true }).click();
  const next = page.getByRole("button", { name: "Continue", exact: true });
  await expect(next).toBeDisabled();
  const first = page.getByRole("combobox", { name: "Word 1", exact: true });
  await first.fill("aban");
  await page.getByRole("option", { name: "abandon", exact: true }).click();
  await expect(first).toHaveValue("abandon");
  for (let n = 2; n <= 12; n++) {
    await page.getByRole("combobox", { name: `Word ${n}`, exact: true }).fill("abandon");
  }
  await expect(next).toBeDisabled(); // Twelve valid words, invalid checksum.
  await page.getByRole("combobox", { name: "Word 12", exact: true }).fill("about");
  await page.keyboard.press("Tab");
  await expect(next).toBeEnabled();
  await next.click();
  await expect(page.getByRole("dialog")).toContainText("Protect the seed");
  const calls = await page.evaluate(() => (window as unknown as { testRpcCalls: string[] }).testRpcCalls);
  expect(calls).not.toContain("createmerchant");
  expect(calls).not.toContain("importseed");
  expect(errors).toEqual([]);
});

test("locked wallet gate remains visible after Escape", async ({ page }) => {
  await boot(page, true);
  const dialog = page.getByRole("dialog");
  await expect(dialog).toBeVisible();
  await expect(dialog.locator('input[type="password"]')).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(dialog).toBeVisible();
});
