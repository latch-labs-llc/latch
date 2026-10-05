/**
 * Payee refund: two burner wallets set up a funded deal; the SELLER clicks
 * "Refund the buyer" and the whole deposit returns to the buyer — no dispute,
 * no buyer signature. Screenshots land in e2e/shots-refund/.
 *
 * Prereqs: `npm run dev` on :5173; funder keypair with devnet SOL at
 * ~/.config/solana/id.json (used only to fund the burners' tx fees).
 */
import { execSync } from "child_process";
import { mkdirSync } from "fs";
import { chromium } from "playwright";

const APP_URL = "http://localhost:5173";
const SHOTS = new URL("./shots-refund/", import.meta.url).pathname;
mkdirSync(SHOTS, { recursive: true });

const SOL_PATH = `${process.env.HOME}/.local/share/solana/install/active_release/bin`;
const fundSol = (address, amount) =>
  execSync(
    `${SOL_PATH}/solana transfer ${address} ${amount} --allow-unfunded-recipient --url devnet --keypair ${process.env.HOME}/.config/solana/id.json`,
    { stdio: "pipe" }
  );

let step = 0;
async function shot(page, name) {
  step += 1;
  await page.screenshot({ path: `${SHOTS}${String(step).padStart(2, "0")}-${name}.png`, fullPage: true });
  console.log(`  📸 ${step}-${name}`);
}

async function connectBurner(page) {
  await page.goto(APP_URL);
  await page.getByRole("button", { name: /select wallet/i }).click();
  await page.getByRole("button", { name: /burner/i }).click();
  const addr = await page.locator("#my-address").getAttribute("title", { timeout: 20000 });
  if (!addr) throw new Error("no burner address");
  return addr;
}

const main = async () => {
  const browser = await chromium.launch();
  const buyer = await (await browser.newContext({ viewport: { width: 1100, height: 900 } })).newPage();
  const seller = await (await browser.newContext({ viewport: { width: 1100, height: 900 } })).newPage();
  seller.on("dialog", (d) => d.accept()); // the refund asks for confirmation

  const buyerAddr = await connectBurner(buyer);
  const sellerAddr = await connectBurner(seller);
  fundSol(buyerAddr, 0.2);
  fundSol(sellerAddr, 0.1);
  await buyer.getByRole("button", { name: /get 1,000 ldd/i }).click();
  await buyer.getByText(/^1,000$/).waitFor({ timeout: 60000 });

  await buyer.getByPlaceholder("base58 address").first().fill(sellerAddr);
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
  console.log("funded ✓ — buyer's 1,000 LDD is in the vault");

  // No reload: a burner wallet gets a fresh key on reload. The deal page
  // polls on-chain state, so the seller's view updates on its own.
  await seller.getByText(/Can't deliver\?/i).waitFor({ timeout: 90000 });
  await shot(seller, "seller-sees-refund-option");
  await seller.getByRole("button", { name: /refund the buyer/i }).click();
  await seller.getByText(/Deal cancelled/i).waitFor({ timeout: 90000 });
  await shot(seller, "refunded-cancelled");
  console.log("seller refunded → deal cancelled ✓");

  // In-app navigation (hash change) keeps the same burner wallet.
  await buyer.evaluate(() => { window.location.hash = "#/"; });
  await buyer.getByText(/^1,000$/).first().waitFor({ timeout: 60000 });
  console.log("buyer balance back to 1,000 LDD ✓");

  await browser.close();
  console.log("\nPAYEE-REFUND E2E: PASS ✅  deal:", dealUrl);
};

main().catch((e) => {
  console.error("PAYEE-REFUND E2E: FAIL ❌", e);
  process.exit(1);
});
