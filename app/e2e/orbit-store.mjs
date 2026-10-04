/** Second-merchant proof: Orbit Supply Co. runs on @latch-labs/checkout with
 * ~15 lines of integration. Full guest purchase, zero wallets. */
import { mkdirSync } from "fs";
import { chromium } from "playwright";

const APP_URL = "http://localhost:5173/#/orbit";
const SHOTS = new URL("./shots-checkout/", import.meta.url).pathname;
mkdirSync(SHOTS, { recursive: true });

const main = async () => {
  const browser = await chromium.launch();
  const ctx = await browser.newContext({ viewport: { width: 1100, height: 950 } });
  const page = await ctx.newPage();

  await page.goto(APP_URL);
  await page.getByRole("button", { name: /buy protected/i }).click();
  await page.getByText(/Order placed/i).waitFor({ timeout: 240000 });
  console.log("orbit order placed ✓");
  await page.screenshot({ path: `${SHOTS}95-orbit-order-placed.png`, fullPage: true });

  // The deal is real: open its page read-only.
  await page.getByRole("link", { name: /view the deal/i }).click();
  await page.getByText(/Active/i).first().waitFor({ timeout: 60000 });
  console.log("orbit deal Active on-chain ✓");
  await page.screenshot({ path: `${SHOTS}96-orbit-deal.png`, fullPage: true });

  console.log("\nORBIT (2nd merchant) E2E: PASS ✅");
  await browser.close();
};
main().catch((e) => {
  console.error("FAILED:", e.message);
  process.exit(1);
});
