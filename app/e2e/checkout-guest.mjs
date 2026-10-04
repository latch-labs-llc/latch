/** Guest-checkout judge simulation: the "never touch crypto" path.
 * NO wallet is ever connected — store → Pay with Latch → simulated card →
 * order placed → confirm delivery → Completed → certificate.
 * Screenshots in e2e/shots-checkout/. */
import { mkdirSync } from "fs";
import { chromium } from "playwright";

const APP_URL = "http://localhost:5173/#/store";
const SHOTS = new URL("./shots-checkout/", import.meta.url).pathname;
mkdirSync(SHOTS, { recursive: true });

let step = 0;
const shot = async (p, name) => {
  step += 1;
  await p.screenshot({ path: `${SHOTS}${String(step).padStart(2, "0")}-${name}.png`, fullPage: true });
  console.log(`  📸 ${step}-${name}`);
};

const main = async () => {
  const browser = await chromium.launch();
  const page = await (await browser.newContext({ viewport: { width: 1100, height: 950 } })).newPage();
  page.on("pageerror", (e) => console.log("pageerror:", e.message));

  await page.goto(APP_URL);
  await page.getByText(/Walnut & Oak/).first().waitFor({ timeout: 20000 });
  await shot(page, "storefront");

  console.log("opening checkout for the desk…");
  await page.getByRole("button", { name: /pay with latch/i }).first().click();
  await page.getByText(/Protected payment\./i).waitFor({ timeout: 10000 });
  await shot(page, "pay-sheet");

  await page.locator(".consent input").check();
  await page.getByRole("button", { name: /pay with card/i }).click();
  await page.getByText(/SIMULATED/).waitFor({ timeout: 10000 });
  await shot(page, "card-form");

  console.log("paying with simulated card (guest wallet under the hood)…");
  await page.getByRole("button", { name: /^pay \$120/i }).click();
  await page.getByText(/Order placed/i).waitFor({ timeout: 180000 });
  await shot(page, "order-placed");

  await page.getByRole("button", { name: /back to the store/i }).click();
  await page.getByText(/Your orders/i).waitFor({ timeout: 20000 });
  await page.getByText(/Active/).first().waitFor({ timeout: 30000 });
  await shot(page, "orders-active");

  console.log("confirming delivery…");
  await page.getByRole("button", { name: /confirm delivery/i }).click();
  await page.getByText(/Completed/).first().waitFor({ timeout: 120000 });
  await shot(page, "orders-completed");

  console.log("certificate (still no wallet connected)…");
  await page.getByRole("link", { name: /^certificate$/i }).first().click();
  await page.getByText(/matches on-chain digest/i).waitFor({ timeout: 120000 });
  await shot(page, "certificate-no-wallet");

  console.log("\nGUEST CHECKOUT E2E: PASS ✅ (zero wallets connected)");
  await browser.close();
};
main().catch((e) => {
  console.error("FAILED:", e.message);
  process.exit(1);
});
