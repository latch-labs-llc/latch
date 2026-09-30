/**
 * Two-judge simulation: two independent browsers with zero-setup burner
 * wallets walk a complete deal against live devnet — exactly what a hackathon
 * judge would do. Screenshots land in e2e/shots/.
 *
 * Prereqs: `npm run dev` running on :5173; funder keypair with devnet SOL at
 * ~/.config/solana/id.json (used only to fund the burners' tx fees).
 */
import { execSync } from "child_process";
import { mkdirSync } from "fs";
import { chromium } from "playwright";

const APP_URL = "http://localhost:5173";
const SHOTS = new URL("./shots/", import.meta.url).pathname;
mkdirSync(SHOTS, { recursive: true });

const SOL_PATH = `${process.env.HOME}/.local/share/solana/install/active_release/bin`;
function fundSol(address, amount) {
  execSync(
    `${SOL_PATH}/solana transfer ${address} ${amount} --allow-unfunded-recipient --url devnet --keypair ${process.env.HOME}/.config/solana/id.json`,
    { stdio: "pipe" }
  );
}

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
  const buyerCtx = await browser.newContext({ viewport: { width: 1100, height: 900 } });
  const sellerCtx = await browser.newContext({ viewport: { width: 1100, height: 900 } });
  const buyer = await buyerCtx.newPage();
  const seller = await sellerCtx.newPage();
  buyer.on("pageerror", (e) => console.log("buyer pageerror:", e.message));
  seller.on("pageerror", (e) => console.log("seller pageerror:", e.message));

  console.log("connecting burner wallets…");
  const buyerAddr = await connectBurner(buyer);
  const sellerAddr = await connectBurner(seller);
  console.log("buyer:", buyerAddr);
  console.log("seller:", sellerAddr);
  await shot(buyer, "buyer-connected");

  console.log("funding burners with devnet SOL (fees)…");
  fundSol(buyerAddr, 0.2);
  fundSol(sellerAddr, 0.1);

  console.log("buyer: LDD faucet…");
  await buyer.getByRole("button", { name: /get 1,000 ldd/i }).click();
  await buyer.getByText(/^1,000$/).waitFor({ timeout: 60000 });
  await shot(buyer, "buyer-funded");

  console.log("buyer: creating deal…");
  await buyer.getByPlaceholder("base58 address").first().fill(sellerAddr);
  await buyer.getByRole("button", { name: /create deal/i }).click();
  await buyer.waitForURL(/#\/deal\//, { timeout: 90000 });
  const dealUrl = buyer.url();
  console.log("deal:", dealUrl);
  await buyer.getByText(/Draft/).first().waitFor({ timeout: 30000 });
  await shot(buyer, "deal-created-draft");

  console.log("seller: opening share link, verifying agreement, signing…");
  await seller.goto(dealUrl);
  await seller.getByText(/matches the on-chain SHA-256/i).waitFor({ timeout: 30000 });
  await shot(seller, "seller-sees-verified-agreement");
  await seller.locator(".consent input").check();
  await seller.getByRole("button", { name: /sign the agreement/i }).click();
  await seller.getByText(/You have signed/i).waitFor({ timeout: 60000 });
  await shot(seller, "seller-signed");

  console.log("buyer: signing…");
  await buyer.locator(".consent input").check();
  await buyer.getByRole("button", { name: /sign the agreement/i }).click();
  await buyer.getByText(/Fund the escrow/i).waitFor({ timeout: 60000 });
  await shot(buyer, "both-signed-now-fund");

  console.log("buyer: depositing…");
  await buyer.getByRole("button", { name: /deposit .* into escrow/i }).click();
  await buyer.getByText(/Confirm ready/i).waitFor({ timeout: 90000 });
  await shot(buyer, "funded");

  console.log("both: confirming ready…");
  await buyer.getByRole("button", { name: /i'm ready/i }).click();
  await seller.getByRole("button", { name: /i'm ready/i }).click({ timeout: 30000 });
  await buyer.getByText(/Milestones/i).waitFor({ timeout: 90000 });
  await shot(buyer, "active");

  console.log("both: approving milestone…");
  await buyer.getByRole("button", { name: /^approve$/i }).click();
  await seller.getByRole("button", { name: /^approve$/i }).click({ timeout: 60000 });

  console.log("release (permissionless crank, buyer clicks)…");
  await buyer.getByRole("button", { name: /release to seller/i }).click({ timeout: 60000 });
  await buyer.getByText(/Deal completed/i).waitFor({ timeout: 90000 });
  await shot(buyer, "completed");

  console.log("certificate…");
  await buyer.getByRole("link", { name: /certificate/i }).first().click();
  await buyer.getByText(/matches on-chain digest/i).waitFor({ timeout: 120000 });
  await shot(buyer, "certificate");

  console.log("\nTWO-JUDGE E2E: PASS ✅  deal:", dealUrl);
  await browser.close();
};

main().catch((e) => {
  console.error("E2E FAILED:", e.message);
  process.exit(1);
});
