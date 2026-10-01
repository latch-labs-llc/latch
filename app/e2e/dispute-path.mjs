/** Dispute-path judge simulation: deal → Active → buyer disputes → joint 60/40
 * settlement → permissionless resolve → certificate. Screenshots in e2e/shots-dispute/. */
import { execSync } from "child_process";
import { mkdirSync } from "fs";
import { chromium } from "playwright";

const APP_URL = "http://localhost:5173";
const SHOTS = new URL("./shots-dispute/", import.meta.url).pathname;
mkdirSync(SHOTS, { recursive: true });
const SOL_PATH = `${process.env.HOME}/.local/share/solana/install/active_release/bin`;
const fundSol = (a, n) =>
  execSync(`${SOL_PATH}/solana transfer ${a} ${n} --allow-unfunded-recipient --url devnet --keypair ${process.env.HOME}/.config/solana/id.json`, { stdio: "pipe" });

let step = 0;
const shot = async (p, name) => {
  step += 1;
  await p.screenshot({ path: `${SHOTS}${String(step).padStart(2, "0")}-${name}.png`, fullPage: true });
  console.log(`  📸 ${step}-${name}`);
};
const connectBurner = async (page) => {
  await page.goto(APP_URL);
  await page.getByRole("button", { name: /select wallet/i }).click();
  await page.getByRole("button", { name: /burner/i }).click();
  return await page.locator("#my-address").getAttribute("title", { timeout: 20000 });
};

const main = async () => {
  const browser = await chromium.launch();
  const buyer = await (await browser.newContext({ viewport: { width: 1100, height: 900 } })).newPage();
  const seller = await (await browser.newContext({ viewport: { width: 1100, height: 900 } })).newPage();

  const buyerAddr = await connectBurner(buyer);
  const sellerAddr = await connectBurner(seller);
  console.log("buyer:", buyerAddr, "seller:", sellerAddr);
  fundSol(buyerAddr, 0.2);
  fundSol(sellerAddr, 0.1);

  await buyer.getByRole("button", { name: /get 1,000 ldd/i }).click();
  await buyer.getByText(/^1,000$/).waitFor({ timeout: 60000 });

  await buyer.getByPlaceholder("base58 address").first().fill(sellerAddr);
  const subj = buyer.getByLabel(/subject/i);
  await subj.fill("Sale of one vintage mahogany writing desk, shipped");
  await buyer.getByRole("button", { name: /create deal/i }).click();
  await buyer.waitForURL(/#\/deal\//, { timeout: 90000 });
  const dealUrl = buyer.url();
  console.log("deal:", dealUrl);

  await seller.goto(dealUrl);
  await seller.getByText(/matches the on-chain SHA-256/i).waitFor({ timeout: 30000 });
  await seller.locator(".consent input").check();
  await seller.getByRole("button", { name: /sign the agreement/i }).click();
  await seller.getByText(/You have signed/i).waitFor({ timeout: 60000 });

  await buyer.locator(".consent input").check();
  await buyer.getByRole("button", { name: /sign the agreement/i }).click();
  await buyer.getByText(/Fund the escrow/i).waitFor({ timeout: 60000 });
  await buyer.getByRole("button", { name: /deposit .* into escrow/i }).click();
  await buyer.getByText(/Confirm ready/i).waitFor({ timeout: 90000 });
  await buyer.getByRole("button", { name: /i'm ready/i }).click();
  await seller.getByRole("button", { name: /i'm ready/i }).click({ timeout: 30000 });
  await buyer.getByText(/Milestones/i).waitFor({ timeout: 90000 });
  console.log("active — raising dispute…");

  // The desk arrives scratched: the buyer disputes.
  await buyer.getByRole("button", { name: /raise a dispute/i }).click();
  await buyer.getByText(/Dispute open/i).waitFor({ timeout: 90000 });
  await shot(buyer, "dispute-open");

  // They negotiate 60 to buyer / 40 to seller. Both sign the same division.
  await buyer.getByPlaceholder("amount to seller").first().fill("40");
  await buyer.getByRole("button", { name: /sign settlement/i }).click();
  await buyer.getByText(/Current settlement proposal/i).waitFor({ timeout: 60000 });
  await shot(buyer, "buyer-proposed-settlement");

  await seller.getByText(/Dispute open/i).waitFor({ timeout: 30000 });
  await seller.getByPlaceholder("amount to seller").first().fill("40");
  await seller.getByRole("button", { name: /sign settlement/i }).click();
  await seller.getByText(/Signed by 2 of 2/i).waitFor({ timeout: 60000 });
  await shot(seller, "both-signed-settlement");

  // Anyone executes.
  await seller.getByRole("button", { name: /execute resolution/i }).click();
  await seller.getByText(/Deal completed/i).waitFor({ timeout: 90000 });
  await shot(seller, "settled-completed");

  await seller.getByRole("link", { name: /certificate/i }).first().click();
  await seller.getByText(/matches on-chain digest/i).waitFor({ timeout: 120000 });
  await shot(seller, "dispute-certificate");

  console.log("\nDISPUTE-PATH E2E: PASS ✅  deal:", dealUrl);
  await browser.close();
};
main().catch((e) => {
  console.error("FAILED:", e.message);
  process.exit(1);
});
