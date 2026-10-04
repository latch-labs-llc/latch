/**
 * Try-to-Cheat: a standing, fully funded, Active deal on devnet that judges
 * attack with REAL transactions. The demo party secrets are DELIBERATELY
 * public (devnet play money; same policy as the demo merchants) — the whole
 * point is that even the parties' own keys cannot break the rules, and no
 * operator key exists that can.
 *
 * Deal: TrueDeadlock (can never time out), 500 LDD vaulted, Active.
 */
import { Keypair, PublicKey } from "@solana/web3.js";

export const ATTACK_DEAL = new PublicKey("BoN26mW439q1jU3YXtpsftvEiMygbcEN4BuiHWKyjsVd");
export const ATTACK_BUYER_ATA = new PublicKey("Guek1YbUGphZTiL5NRXzrs6ijVNL25szvbaTsWw9dRrM");
export const ATTACK_SELLER_ATA = new PublicKey("EXnkvMuRJx9t2Q6iAgtHaWZnqFRbcmCKT3izDTkxXFZd");

const BUYER_SECRET = new Uint8Array([89,201,22,96,246,36,155,235,38,162,156,63,105,203,110,22,9,233,239,236,242,175,16,229,12,130,68,127,176,109,19,149,183,148,171,231,186,46,228,132,164,132,215,38,202,129,19,21,21,95,156,45,232,20,169,55,1,104,105,92,47,68,50,130]);
const SELLER_SECRET = new Uint8Array([145,49,45,79,145,45,251,193,202,171,172,113,204,101,27,71,127,112,76,67,175,54,164,120,126,47,91,19,194,255,49,229,236,139,202,95,165,106,50,87,134,87,213,89,201,7,188,180,64,186,87,195,50,76,26,66,149,84,255,40,36,172,45,102]);
const OPERATOR_SECRET = new Uint8Array([199,168,192,229,182,131,36,211,15,240,47,183,249,190,59,10,175,194,78,77,59,187,197,36,104,113,198,251,6,14,171,109,54,173,251,15,2,94,103,25,236,215,100,18,10,10,157,137,109,70,120,242,169,157,216,64,44,99,49,93,29,220,66,214]);

export const attackBuyer = () => Keypair.fromSecretKey(BUYER_SECRET);
export const attackSeller = () => Keypair.fromSecretKey(SELLER_SECRET);
export const attackOperator = () => Keypair.fromSecretKey(OPERATOR_SECRET);
