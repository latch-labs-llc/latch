/** Protection-dial send: burner connects, grabs faucet funds, creates a
 * protected payment (deal + own signature + share link), then flips the
 * dial and sends an instant transfer. */
import { chromium } from "playwright";
import { Keypair } from "@solana/web3.js";

const main = async () => {
  const browser = await chromium.launch();
  const page = await (await browser.newContext({ viewport: { width: 1100, height: 950 } })).newPage();

  await page.goto("http://localhost:5173/");
  await page.getByRole("button", { name: /select wallet/i }).click();
  await page.getByText(/burner wallet/i).click();
  await page.getByText(/your devnet balances/i).waitFor({ timeout: 30000 });

  // faucets
  await page.getByRole("button", { name: /get .* sol/i }).click();
  await page.waitForTimeout(4000);
  await page.getByRole("button", { name: /get 1,000 ldd/i }).click();
  await page.getByText(/^1,000$|1,000 LDD|1,000/).first().waitFor({ timeout: 60000 });
  console.log("burner funded ✓");

  await page.goto("http://localhost:5173/#/send");
  const recipient = Keypair.generate().publicKey.toBase58();
  await page.getByPlaceholder(/base58 address/i).fill(recipient);
  await page.getByRole("button", { name: /create protected payment/i }).click();
  await page.getByText(/Protected payment created/i).waitFor({ timeout: 120000 });
  console.log("protected send: deal created + signed, share link shown ✓");

  await page.getByRole("button", { name: /instant/i }).click();
  await page.getByRole("button", { name: /send .* instantly/i }).click();
  await page.getByText(/Sent — final/i).waitFor({ timeout: 90000 });
  console.log("instant send: plain transfer confirmed ✓");

  console.log("\nDIAL-SEND E2E: PASS ✅");
  await browser.close();
};
main().catch((e) => {
  console.error("FAILED:", e.message);
  process.exit(1);
});
