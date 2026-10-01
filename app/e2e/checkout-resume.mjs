/** Interruption test: start a card checkout, KILL the page mid-flight, reload,
 * and prove the checkout resumes from on-chain state and completes. */
import { mkdirSync } from "fs";
import { chromium } from "playwright";

const APP_URL = "http://localhost:5173/#/store";
const SHOTS = new URL("./shots-checkout/", import.meta.url).pathname;
mkdirSync(SHOTS, { recursive: true });

const main = async () => {
  const browser = await chromium.launch();
  const ctx = await browser.newContext({ viewport: { width: 1100, height: 950 } });
  const page = await ctx.newPage();

  await page.goto(APP_URL);
  await page.getByRole("button", { name: /pay with latch/i }).first().click();
  await page.locator(".consent input").check();
  await page.getByRole("button", { name: /pay with card/i }).click();
  await page.getByRole("button", { name: /^pay \$120/i }).click();

  // Interrupt brutally once the deal is being created on-chain.
  await page.getByText(/Creating the escrow deal/i).waitFor({ timeout: 120000 });
  console.log("💥 interrupting mid-checkout (page reload)…");
  await page.reload();

  await page.getByText(/Unfinished checkout/i).waitFor({ timeout: 20000 });
  console.log("resume banner shown ✓");
  await page.screenshot({ path: `${SHOTS}90-resume-banner.png`, fullPage: true });

  await page.getByRole("button", { name: /^resume$/i }).click();
  await page.getByRole("button", { name: /resume interrupted checkout/i }).click();
  await page.getByText(/Order placed/i).waitFor({ timeout: 180000 });
  console.log("resumed to completion ✓");
  await page.screenshot({ path: `${SHOTS}91-resumed-order-placed.png`, fullPage: true });

  console.log("\nRESUME E2E: PASS ✅");
  await browser.close();
};
main().catch((e) => {
  console.error("FAILED:", e.message);
  process.exit(1);
});
