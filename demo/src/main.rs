//! Reference demo server: walks a shipped marketplace sale through the
//! latch program with REAL devnet transactions, a signing ceremony
//! over a real agreement (hashed on-chain), and a printable signature
//! certificate reconstructed from on-chain data.
//!
//! Run: cargo run -p escrow-demo   → open http://localhost:7878

use {
    anchor_lang::{
        prelude::Pubkey,
        solana_program::{instruction::Instruction, system_instruction, system_program},
        AccountDeserialize, InstructionData, ToAccountMetas,
    },
    anchor_spl::{
        associated_token::get_associated_token_address_with_program_id,
        token_2022::spl_token_2022,
    },
    serde_json::{json, Value},
    sha2::{Digest, Sha256},
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_rpc_client::rpc_client::RpcClient,
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
    latch::{state::*, CreateDealParams},
    std::{
        io::Read as _,
        sync::Mutex,
        time::{SystemTime, UNIX_EPOCH},
    },
    tiny_http::{Header, Method, Response, Server},
};

const PORT: u16 = 7878;
const URL: &str = "https://api.devnet.solana.com";
const MINT_LEN: usize = 82;
const TOKEN_ACCOUNT_LEN: usize = 165;
const PRICE: u64 = 1_200_000_000; // 1,200.000000 dUSD (6 decimals)
const SETTLE_TO_SELLER: u64 = 600_000_000; // 50/50 settlement in the dispute path
const DISPUTE_TIMEOUT_SECS: i64 = 120;
const RECOVERY_DELAY_SECS: i64 = 86_400;

// ---------------------------------------------------------------- state

struct Env {
    funder: Keypair,
    buyer: Keypair,
    seller: Keypair,
    recovery_agent: Pubkey,
    mint: Pubkey,
    buyer_ata: Pubkey,
    seller_ata: Pubkey,
    setup_log: Vec<(String, String)>,
}

struct Sess {
    deal_id: u64,
    deal: Pubkey,
    vault: Pubkey,
    agreement: String,
    terms_hash: [u8; 32],
    path: Option<&'static str>, // "complete" | "dispute"
    next_step: usize,           // 1..=8; 9 = done
    log: Vec<StepLog>,
}

struct StepLog {
    step: usize,
    label: String,
    txs: Vec<(String, String)>, // (tx label, signature)
}

struct App {
    env: Option<Env>,
    sess: Option<Sess>,
}

// ---------------------------------------------------------------- tx plumbing

fn send(
    client: &RpcClient,
    ixs: &[Instruction],
    payer: &Pubkey,
    signers: &[&Keypair],
) -> Result<String, String> {
    let blockhash = client.get_latest_blockhash().map_err(|e| e.to_string())?;
    let msg = Message::new_with_blockhash(ixs, Some(payer), &blockhash);
    let mut unique: Vec<&Keypair> = Vec::new();
    for s in signers {
        if !unique.iter().any(|u| u.pubkey() == s.pubkey()) {
            unique.push(s);
        }
    }
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &unique)
        .map_err(|e| e.to_string())?;
    client
        .send_and_confirm_transaction(&tx)
        .map(|s| s.to_string())
        .map_err(|e| e.to_string())
}

fn event_authority() -> Pubkey {
    Pubkey::find_program_address(&[b"__event_authority"], &latch::id()).0
}

fn tp() -> Pubkey {
    anchor_spl::token::ID
}

fn program_ix(data: Vec<u8>, metas: Vec<anchor_lang::solana_program::instruction::AccountMeta>) -> Instruction {
    Instruction::new_with_bytes(latch::id(), &data, metas)
}

fn ix_sign_terms(party: &Pubkey, deal: &Pubkey) -> Instruction {
    program_ix(
        latch::instruction::SignTerms { consent_true_deadlock: true }.data(),
        latch::accounts::SignTerms {
            party: *party,
            deal: *deal,
            event_authority: event_authority(),
            program: latch::id(),
        }
        .to_account_metas(None),
    )
}

fn ix_confirm_ready(party: &Pubkey, deal: &Pubkey) -> Instruction {
    program_ix(
        latch::instruction::ConfirmReady {}.data(),
        latch::accounts::ConfirmReady {
            party: *party,
            deal: *deal,
            event_authority: event_authority(),
            program: latch::id(),
        }
        .to_account_metas(None),
    )
}

fn ix_approve(party: &Pubkey, deal: &Pubkey) -> Instruction {
    program_ix(
        latch::instruction::ApproveMilestone { index: 0 }.data(),
        latch::accounts::ApproveMilestone {
            party: *party,
            deal: *deal,
            event_authority: event_authority(),
            program: latch::id(),
        }
        .to_account_metas(None),
    )
}

fn ix_raise(party: &Pubkey, deal: &Pubkey) -> Instruction {
    program_ix(
        latch::instruction::RaiseDeadlock {}.data(),
        latch::accounts::RaiseDeadlock {
            party: *party,
            deal: *deal,
            event_authority: event_authority(),
            program: latch::id(),
        }
        .to_account_metas(None),
    )
}

fn ix_res_sign(signer: &Pubkey, deal: &Pubkey, amount: u64) -> Instruction {
    program_ix(
        latch::instruction::ResolutionSign { amount_to_payee: amount }.data(),
        latch::accounts::ResolutionSign {
            signer: *signer,
            deal: *deal,
            event_authority: event_authority(),
            program: latch::id(),
        }
        .to_account_metas(None),
    )
}

// ---------------------------------------------------------------- agreement

fn build_agreement(env: &Env, deal: &Pubkey, deal_id: u64) -> String {
    format!(
        r#"ESCROW SALE AGREEMENT (DEMONSTRATION)

This is a demonstration document on Solana devnet. It is not a real
agreement, creates no legal obligations, and involves no real funds.

1. PARTIES.
   Buyer:  the holder of Solana wallet {buyer}
   Seller: the holder of Solana wallet {seller}

2. SALE. Seller agrees to sell and ship to Buyer, and Buyer agrees to
   purchase: one (1) vintage mahogany writing desk, serial no. VMD-1912-044,
   in the condition described in the listing incorporated by reference
   (the "Item").

3. PRICE AND ESCROW. The purchase price is 1,200.000000 Demo USD (token
   mint {mint}). Upon mutual execution, Buyer shall deposit the full
   price into the split-control escrow vault governed by on-chain deal
   account {deal} (deal id {deal_id}) under program
   {program} on Solana devnet. No party, and no third party including
   the platform operator, can move escrowed funds unilaterally.

4. RELEASE. The price is released to Seller upon confirmation of
   delivery by both parties (2-of-2 approval of Milestone 1: "Item
   delivered as described"). Once approvals are recorded on-chain,
   release may be executed by anyone; neither party can then block it.

5. DISPUTES. Either party may raise a dispute on-chain. The parties may
   settle a dispute at any time by jointly signing a payout division.
   If a dispute remains unresolved {timeout} seconds after being raised
   (a demonstration-compressed period; production agreements would
   specify days), the escrowed funds return to Buyer in full.

6. RECOVERY. The parties designate the holder of wallet
   {recovery} as Recovery Agent (1-of-1) for key loss, incapacity, or
   compliance with a court order. Any recovery payout may only be
   directed to the parties, and becomes executable only after a
   {rec_delay}-hour on-chain notice period.

7. ELECTRONIC EXECUTION AND CONSENT. The parties consent to transact
   electronically. Each party executes this agreement by submitting a
   signed Solana transaction from their wallet recording their approval
   of the SHA-256 digest of this document on the deal account. The
   parties intend such signatures to constitute electronic signatures
   under the E-SIGN Act and UETA, and intend the on-chain record of the
   digest, signatures, and timestamps to be the authoritative record of
   execution.

8. GOVERNING LAW. [Demonstration only — a production agreement would
   specify governing law, venue, notices, warranties, and remedies.]

EXECUTION: by on-chain signature of each party's wallet, recorded on
deal account {deal}.
"#,
        buyer = env.buyer.pubkey(),
        seller = env.seller.pubkey(),
        mint = env.mint,
        deal = deal,
        deal_id = deal_id,
        program = latch::id(),
        timeout = DISPUTE_TIMEOUT_SECS,
        recovery = env.recovery_agent,
        rec_delay = RECOVERY_DELAY_SECS / 3600,
    )
}

// ---------------------------------------------------------------- steps

fn setup_env(client: &RpcClient) -> Result<Env, String> {
    let home = std::env::var("HOME").map_err(|e| e.to_string())?;
    let funder = solana_keypair::read_keypair_file(format!("{home}/.config/solana/id.json"))
        .map_err(|e| format!("no keypair at ~/.config/solana/id.json: {e}"))?;
    let balance = client.get_balance(&funder.pubkey()).map_err(|e| e.to_string())?;
    if balance < 300_000_000 {
        return Err("funder needs ≥0.3 devnet SOL (faucet.solana.com)".into());
    }
    let buyer = Keypair::new();
    let seller = Keypair::new();
    let mut log = Vec::new();

    let sig = send(
        client,
        &[
            system_instruction::transfer(&funder.pubkey(), &buyer.pubkey(), 80_000_000),
            system_instruction::transfer(&funder.pubkey(), &seller.pubkey(), 50_000_000),
        ],
        &funder.pubkey(),
        &[&funder],
    )?;
    log.push(("Fund demo wallets (Buyer, Seller) with devnet SOL for fees".into(), sig));

    let mint_kp = Keypair::new();
    let rent = client
        .get_minimum_balance_for_rent_exemption(MINT_LEN)
        .map_err(|e| e.to_string())?;
    let sig = send(
        client,
        &[
            system_instruction::create_account(
                &funder.pubkey(),
                &mint_kp.pubkey(),
                rent,
                MINT_LEN as u64,
                &tp(),
            ),
            spl_token_2022::instruction::initialize_mint2(
                &tp(),
                &mint_kp.pubkey(),
                &funder.pubkey(),
                None,
                6,
            )
            .map_err(|e| e.to_string())?,
        ],
        &funder.pubkey(),
        &[&funder, &mint_kp],
    )?;
    log.push(("Create Demo USD test mint (stand-in stablecoin)".into(), sig));
    let mint = mint_kp.pubkey();

    let mut make_ta = |owner: &Pubkey, label: &str| -> Result<Pubkey, String> {
        let kp = Keypair::new();
        let rent = client
            .get_minimum_balance_for_rent_exemption(TOKEN_ACCOUNT_LEN)
            .map_err(|e| e.to_string())?;
        let sig = send(
            client,
            &[
                system_instruction::create_account(
                    &funder.pubkey(),
                    &kp.pubkey(),
                    rent,
                    TOKEN_ACCOUNT_LEN as u64,
                    &tp(),
                ),
                spl_token_2022::instruction::initialize_account3(&tp(), &kp.pubkey(), &mint, owner)
                    .map_err(|e| e.to_string())?,
            ],
            &funder.pubkey(),
            &[&funder, &kp],
        )?;
        log.push((format!("Create {label} token account"), sig));
        Ok(kp.pubkey())
    };
    let buyer_ata = make_ta(&buyer.pubkey(), "Buyer")?;
    let seller_ata = make_ta(&seller.pubkey(), "Seller")?;

    let sig = send(
        client,
        &[spl_token_2022::instruction::mint_to(
            &tp(),
            &mint,
            &buyer_ata,
            &funder.pubkey(),
            &[],
            2_000_000_000,
        )
        .map_err(|e| e.to_string())?],
        &funder.pubkey(),
        &[&funder],
    )?;
    log.push(("Mint 2,000 Demo USD to Buyer".into(), sig));

    Ok(Env {
        funder,
        buyer,
        seller,
        recovery_agent: Pubkey::new_unique(),
        mint,
        buyer_ata,
        seller_ata,
        setup_log: log,
    })
}

fn new_session(env: &Env, client: &RpcClient) -> Result<Sess, String> {
    // Top up the Buyer if a previous demo deal spent their balance.
    let raw: u64 = client
        .get_token_account_balance(&env.buyer_ata)
        .map_err(|e| e.to_string())?
        .amount
        .parse()
        .unwrap_or(0);
    let mut pre_txs: Vec<(String, String)> = Vec::new();
    if raw < PRICE {
        let sig = send(
            client,
            &[spl_token_2022::instruction::mint_to(
                &tp(),
                &env.mint,
                &env.buyer_ata,
                &env.funder.pubkey(),
                &[],
                2_000_000_000 - raw,
            )
            .map_err(|e| e.to_string())?],
            &env.funder.pubkey(),
            &[&env.funder],
        )?;
        pre_txs.push(("Top up Buyer to 2,000 dUSD".into(), sig));
    }
    let deal_id = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let deal = Pubkey::find_program_address(
        &[b"deal", env.buyer.pubkey().as_ref(), &deal_id.to_le_bytes()],
        &latch::id(),
    )
    .0;
    let vault = get_associated_token_address_with_program_id(&deal, &env.mint, &tp());
    let agreement = build_agreement(env, &deal, deal_id);
    let terms_hash: [u8; 32] = Sha256::digest(agreement.as_bytes()).into();

    let params = CreateDealParams {
        deal_id,
        parties: vec![env.buyer.pubkey(), env.seller.pubkey()],
        payer_idx: 0,
        payee_idx: 1,
        approval_threshold: 2,
        terms_hash,
        price_risk_term: 0,
        total_amount: PRICE,
        milestone_amounts: vec![PRICE],
        deadlock_rule: DeadlockRule::TimeoutRefund,
        timer_mode: TimerMode::FromDeadlock,
        timeout_secs: DISPUTE_TIMEOUT_SECS,
        split_bps: 0,
        tie_breaker: Pubkey::default(),
        recovery_signers: vec![env.recovery_agent],
        recovery_threshold: 1,
        recovery_delay_secs: RECOVERY_DELAY_SECS,
        accepted_risk_flags: u16::MAX,
    };
    let ix = program_ix(
        latch::instruction::CreateDeal { params }.data(),
        latch::accounts::CreateDeal {
            creator: env.buyer.pubkey(),
            deal,
            mint: env.mint,
            vault,
            token_program: tp(),
            associated_token_program: anchor_spl::associated_token::ID,
            system_program: system_program::ID,
            event_authority: event_authority(),
            program: latch::id(),
        }
        .to_account_metas(None),
    );
    let sig = send(client, &[ix], &env.buyer.pubkey(), &[&env.buyer])?;

    Ok(Sess {
        deal_id,
        deal,
        vault,
        agreement,
        terms_hash,
        path: None,
        next_step: 2,
        log: vec![StepLog {
            step: 1,
            label: "Agreement drafted; escrow deal created on-chain with its SHA-256 digest".into(),
            txs: {
                let mut t = pre_txs;
                t.push(("create_deal".into(), sig));
                t
            },
        }],
    })
}

fn run_step(app: &mut App, client: &RpcClient) -> Result<Value, Value> {
    if app.env.is_none() {
        let env = setup_env(client).map_err(|e| json!({"error": e}))?;
        app.env = Some(env);
        return Ok(json!({"ok": true, "step": 0}));
    }
    let env = app.env.as_ref().unwrap();
    if app.sess.is_none() {
        let sess = new_session(env, client).map_err(|e| json!({"error": e}))?;
        app.sess = Some(sess);
        return Ok(json!({"ok": true, "step": 1}));
    }
    let sess = app.sess.as_mut().unwrap();
    let step = sess.next_step;
    let mut txs: Vec<(String, String)> = Vec::new();
    let label: String = match step {
        2 => {
            let sig = send(client, &[ix_sign_terms(&env.buyer.pubkey(), &sess.deal)], &env.buyer.pubkey(), &[&env.buyer])
                .map_err(|e| json!({"error": e}))?;
            txs.push(("sign_terms (Buyer)".into(), sig));
            "Buyer executed the agreement: wallet signature over the document digest, recorded on-chain".into()
        }
        3 => {
            let sig = send(client, &[ix_sign_terms(&env.seller.pubkey(), &sess.deal)], &env.seller.pubkey(), &[&env.seller])
                .map_err(|e| json!({"error": e}))?;
            txs.push(("sign_terms (Seller)".into(), sig));
            "Seller executed the agreement — all parties signed; terms are now frozen".into()
        }
        4 => {
            let ix = program_ix(
                latch::instruction::Deposit { amount: PRICE }.data(),
                latch::accounts::Deposit {
                    payer: env.buyer.pubkey(),
                    deal: sess.deal,
                    mint: env.mint,
                    vault: sess.vault,
                    payer_token_account: env.buyer_ata,
                    token_program: tp(),
                    event_authority: event_authority(),
                    program: latch::id(),
                }
                .to_account_metas(None),
            );
            let sig = send(client, &[ix], &env.buyer.pubkey(), &[&env.buyer])
                .map_err(|e| json!({"error": e}))?;
            txs.push(("deposit 1,200 dUSD".into(), sig));
            "Buyer funded the escrow vault — neither party can move these funds alone".into()
        }
        5 => {
            for (kp, who) in [(&env.buyer, "Buyer"), (&env.seller, "Seller")] {
                let sig = send(client, &[ix_confirm_ready(&kp.pubkey(), &sess.deal)], &kp.pubkey(), &[kp])
                    .map_err(|e| json!({"error": e}))?;
                txs.push((format!("confirm_ready ({who})"), sig));
            }
            "Both parties confirmed ready — deal is Active; Seller ships the item".into()
        }
        6..=8 => {
            let path = sess.path.ok_or_else(|| json!({"need_path": true}))?;
            match (path, step) {
                ("complete", 6) => {
                    let sig = send(client, &[ix_approve(&env.buyer.pubkey(), &sess.deal)], &env.buyer.pubkey(), &[&env.buyer])
                        .map_err(|e| json!({"error": e}))?;
                    txs.push(("approve_milestone (Buyer)".into(), sig));
                    "Item arrived — Buyer confirmed delivery on-chain".into()
                }
                ("complete", 7) => {
                    let sig = send(client, &[ix_approve(&env.seller.pubkey(), &sess.deal)], &env.seller.pubkey(), &[&env.seller])
                        .map_err(|e| json!({"error": e}))?;
                    txs.push(("approve_milestone (Seller)".into(), sig));
                    "Seller co-confirmed — 2-of-2 release approvals recorded".into()
                }
                ("complete", 8) => {
                    let ix = program_ix(
                        latch::instruction::ReleaseMilestone { index: 0 }.data(),
                        latch::accounts::ReleaseMilestone {
                            deal: sess.deal,
                            mint: env.mint,
                            vault: sess.vault,
                            payee_token_account: env.seller_ata,
                            token_program: tp(),
                            event_authority: event_authority(),
                            program: latch::id(),
                        }
                        .to_account_metas(None),
                    );
                    let sig = send(client, &[ix], &env.seller.pubkey(), &[&env.seller])
                        .map_err(|e| json!({"error": e}))?;
                    txs.push(("release_milestone (permissionless crank)".into(), sig));
                    "Payment released to Seller — final settlement, no chargebacks. Deal complete".into()
                }
                ("dispute", 6) => {
                    let sig = send(client, &[ix_raise(&env.buyer.pubkey(), &sess.deal)], &env.buyer.pubkey(), &[&env.buyer])
                        .map_err(|e| json!({"error": e}))?;
                    txs.push(("raise_deadlock (Buyer)".into(), sig));
                    "Buyer raised a dispute — funds stay locked; the pre-agreed rules now govern".into()
                }
                ("dispute", 7) => {
                    for (kp, who) in [(&env.buyer, "Buyer"), (&env.seller, "Seller")] {
                        let sig = send(
                            client,
                            &[ix_res_sign(&kp.pubkey(), &sess.deal, SETTLE_TO_SELLER)],
                            &kp.pubkey(),
                            &[kp],
                        )
                        .map_err(|e| json!({"error": e}))?;
                        txs.push((format!("resolution_sign 50/50 ({who})"), sig));
                    }
                    "Parties negotiated and both signed a 50/50 settlement".into()
                }
                ("dispute", 8) => {
                    let ix = program_ix(
                        latch::instruction::Resolve {}.data(),
                        latch::accounts::Resolve {
                            deal: sess.deal,
                            mint: env.mint,
                            vault: sess.vault,
                            payer_token_account: env.buyer_ata,
                            payee_token_account: env.seller_ata,
                            token_program: tp(),
                            event_authority: event_authority(),
                            program: latch::id(),
                        }
                        .to_account_metas(None),
                    );
                    let sig = send(client, &[ix], &env.buyer.pubkey(), &[&env.buyer])
                        .map_err(|e| json!({"error": e}))?;
                    txs.push(("resolve (permissionless crank)".into(), sig));
                    "Settlement executed: 600 dUSD to Seller, 600 dUSD back to Buyer. Deal complete".into()
                }
                _ => return Err(json!({"error": "demo finished — start a new deal"})),
            }
        }
        _ => return Err(json!({"error": "demo finished — start a new deal"})),
    };
    sess.log.push(StepLog { step, label, txs });
    sess.next_step += 1;
    Ok(json!({"ok": true, "step": step}))
}

// ---------------------------------------------------------------- views

fn fetch_deal(client: &RpcClient, deal: &Pubkey) -> Option<Deal> {
    let data = client.get_account_data(deal).ok()?;
    Deal::try_deserialize(&mut data.as_slice()).ok()
}

fn ui_balance(client: &RpcClient, ata: &Pubkey) -> String {
    client
        .get_token_account_balance(ata)
        .map(|b| b.ui_amount_string)
        .unwrap_or_else(|_| "0".into())
}

fn state_json(app: &App, client: &RpcClient) -> Value {
    let base_steps = [
        ("Prepare demo environment", "Fund two demo wallets (Buyer & Seller), create the Demo USD test mint, give the Buyer 2,000 dUSD. Devnet only — no real funds."),
        ("Draft agreement & create escrow deal", "The sale agreement is generated, its SHA-256 digest computed, and the on-chain deal account created carrying that digest, the milestone, the dispute rule, and the party-chosen Recovery Agent."),
        ("Buyer signs the agreement", "Signing ceremony: the Buyer reviews the document and consents to electronic execution; their wallet signs a transaction recording approval of the digest."),
        ("Seller signs the agreement", "Same ceremony for the Seller. With all parties signed, every fund-flow term freezes permanently."),
        ("Buyer funds the escrow", "1,200 dUSD moves into the program-owned vault. No person — including the platform — holds a key to it."),
        ("Both parties confirm ready", "Performance begins: the Seller ships the desk."),
    ];
    let complete_steps = [
        ("Buyer confirms delivery", "The desk arrives. The Buyer approves Milestone 1 on-chain."),
        ("Seller co-confirms", "2-of-2 approvals recorded — release conditions met."),
        ("Payment releases to Seller", "Anyone can execute the release now (permissionless). Final settlement — no chargeback risk."),
    ];
    let dispute_steps = [
        ("Buyer raises a dispute", "Suppose the desk arrives scratched. The Buyer disputes on-chain; funds stay locked under the pre-agreed rules."),
        ("Parties settle 50/50", "They negotiate a partial refund; both wallets sign the same payout division."),
        ("Settlement executes", "The escrow pays 600 dUSD to each side per the joint signature. (Had they not settled, the agreed timeout-refund rule would apply.)"),
    ];

    let env_done = app.env.is_some();
    let mut steps = vec![];
    let next = if !env_done { 0 } else if app.sess.is_none() { 1 } else { app.sess.as_ref().unwrap().next_step };
    let path = app.sess.as_ref().and_then(|s| s.path);

    let mut push = |idx: usize, label: &str, desc: &str, txs: Vec<Value>| {
        let status = if idx < next { "done" } else if idx == next { "next" } else { "todo" };
        steps.push(json!({"idx": idx, "label": label, "desc": desc, "status": status, "txs": txs}));
    };
    let env_txs: Vec<Value> = app.env.as_ref().map(|e| e.setup_log.iter().map(|(l, s)| json!({"label": l, "sig": s})).collect()).unwrap_or_default();
    push(0, base_steps[0].0, base_steps[0].1, env_txs);
    for i in 1..6 {
        let txs = app
            .sess
            .as_ref()
            .map(|s| {
                s.log
                    .iter()
                    .filter(|l| l.step == i)
                    .flat_map(|l| l.txs.iter().map(|(tl, sig)| json!({"label": tl, "sig": sig})))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        push(i, base_steps[i].0, base_steps[i].1, txs);
    }
    let branch = match path {
        Some("dispute") => Some(&dispute_steps),
        Some("complete") => Some(&complete_steps),
        _ => None,
    };
    if let Some(branch) = branch {
        for (j, (label, desc)) in branch.iter().enumerate() {
            let i = 6 + j;
            let txs = app
                .sess
                .as_ref()
                .map(|s| {
                    s.log
                        .iter()
                        .filter(|l| l.step == i)
                        .flat_map(|l| l.txs.iter().map(|(tl, sig)| json!({"label": tl, "sig": sig})))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            push(i, label, desc, txs);
        }
    }

    let mut deal_info = json!(null);
    if let (Some(env), Some(sess)) = (&app.env, &app.sess) {
        let onchain = fetch_deal(client, &sess.deal);
        deal_info = json!({
            "address": sess.deal.to_string(),
            "state": onchain.as_ref().map(|d| format!("{:?}", d.state)).unwrap_or("—".into()),
            "vault_balance": ui_balance(client, &sess.vault),
            "buyer_balance": ui_balance(client, &env.buyer_ata),
            "seller_balance": ui_balance(client, &env.seller_ata),
            "terms_hash": hex(&sess.terms_hash),
            "buyer": env.buyer.pubkey().to_string(),
            "seller": env.seller.pubkey().to_string(),
        });
    }

    json!({
        "steps": steps,
        "next": next,
        "path": path,
        "need_path": next == 6 && path.is_none(),
        "done": next > 8,
        "agreement": app.sess.as_ref().map(|s| s.agreement.clone()),
        "deal": deal_info,
        "program": latch::id().to_string(),
    })
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn ts(t: i64) -> String {
    chrono::DateTime::from_timestamp(t, 0)
        .map(|d| d.format("%B %-d, %Y %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| "—".into())
}

fn certificate_html(app: &App, client: &RpcClient) -> String {
    let (env, sess) = match (&app.env, &app.sess) {
        (Some(e), Some(s)) => (e, s),
        _ => return "<h1>No deal yet</h1>".into(),
    };
    let deal = fetch_deal(client, &sess.deal);
    let (state, signed_rows, milestone_row) = match &deal {
        Some(d) => {
            let rows = [("Buyer", 0usize), ("Seller", 1usize)]
                .iter()
                .map(|(who, i)| {
                    let t = d.parties_signed_at[*i];
                    let when = if t > 0 { ts(t) } else { "not signed".into() };
                    format!(
                        "<tr><td>{who}</td><td class=mono>{}</td><td>{when}</td><td class=mono>{}</td></tr>",
                        d.parties[*i], hex(&d.terms_hash)
                    )
                })
                .collect::<String>();
            let m = &d.milestones[0];
            let mrow = format!(
                "<tr><td>1</td><td>Item delivered as described</td><td>1,200.000000 dUSD</td><td>{}</td></tr>",
                if m.released { "released to Seller".to_string() } else { format!("{:?}", d.state).to_lowercase() }
            );
            (format!("{:?}", d.state), rows, mrow)
        }
        None => ("unknown".into(), String::new(), String::new()),
    };

    let tx_rows: String = sess
        .log
        .iter()
        .flat_map(|l| {
            l.txs.iter().map(move |(tl, sig)| {
                format!(
                    "<tr><td>{}</td><td>{}</td><td class=mono><a href=\"https://explorer.solana.com/tx/{sig}?cluster=devnet\">{}…</a></td></tr>",
                    l.step, tl, &sig[..20]
                )
            })
        })
        .collect();

    format!(
        r#"<!doctype html><html><head><meta charset="utf-8"><title>Signature & Escrow Certificate</title>
<style>
 body {{ font-family: Georgia, 'Times New Roman', serif; color:#1a1a1a; max-width: 820px; margin: 40px auto; padding: 0 24px; line-height:1.5; }}
 h1 {{ text-align:center; font-variant: small-caps; letter-spacing:.06em; border-bottom: 3px double #1a1a1a; padding-bottom:12px; }}
 h2 {{ font-size: 1.05em; font-variant: small-caps; letter-spacing:.05em; border-bottom:1px solid #999; padding-bottom:4px; margin-top:28px; }}
 table {{ width:100%; border-collapse: collapse; font-size:.85em; }}
 td,th {{ border:1px solid #bbb; padding:6px 8px; text-align:left; vertical-align:top; }}
 th {{ background:#f2efe9; }}
 .mono {{ font-family: ui-monospace, Menlo, monospace; font-size:.82em; word-break: break-all; }}
 .meta td {{ border:none; padding:2px 8px; }}
 .demo {{ text-align:center; color:#8a6d3b; background:#fcf8e3; border:1px solid #d6c98f; padding:8px; font-size:.85em; }}
 pre {{ background:#faf8f4; border:1px solid #ddd; padding:16px; font-size:.72em; white-space:pre-wrap; }}
 .print {{ text-align:center; margin:24px 0; }}
 .print button {{ font-size:1em; padding:10px 28px; cursor:pointer; }}
 @media print {{ .print {{ display:none; }} body {{ margin:0; }} }}
</style></head><body>
<h1>Signature &amp; Escrow Certificate</h1>
<p class=demo>DEMONSTRATION — Solana devnet. No real funds or obligations.</p>
<table class=meta>
<tr><td><b>Network</b></td><td>Solana devnet</td></tr>
<tr><td><b>Escrow program</b></td><td class=mono>{program}</td></tr>
<tr><td><b>Deal account</b></td><td class=mono><a href="https://explorer.solana.com/address/{deal}?cluster=devnet">{deal}</a></td></tr>
<tr><td><b>Document digest (SHA-256)</b></td><td class=mono>{hash}</td></tr>
<tr><td><b>Current state</b></td><td>{state}</td></tr>
<tr><td><b>Certificate generated</b></td><td>{now}</td></tr>
</table>

<h2>Execution record</h2>
<p>Each party executed the agreement by submitting a transaction, signed by their wallet's private key, recording approval of the document digest on the deal account. Timestamps are Solana consensus (cluster) time.</p>
<table><tr><th>Party</th><th>Wallet</th><th>Signed (cluster time)</th><th>Digest approved</th></tr>{signed_rows}</table>

<h2>Escrow &amp; milestone record</h2>
<table><tr><th>#</th><th>Milestone</th><th>Amount</th><th>Status</th></tr>{milestone_row}</table>

<h2>Transaction log</h2>
<table><tr><th>Step</th><th>Action</th><th>Transaction signature</th></tr>{tx_rows}</table>

<h2>Independent verification</h2>
<ol style="font-size:.9em">
<li>Download the agreement text (<a href="/agreement.txt">agreement.txt</a>) and compute its digest: <span class=mono>shasum -a 256 agreement.txt</span>. It must equal the digest above.</li>
<li>Open the deal account on the Solana Explorer (link above) and confirm the same digest is stored in the account, and that each transaction in the log is signed by the listed wallet.</li>
<li>No signature or timestamp on this certificate is asserted by any company — every fact above is reconstructed from the public chain.</li>
</ol>

<h2>Appendix — executed agreement</h2>
<pre>{agreement}</pre>
<div class=print><button onclick="window.print()">Print / Save as PDF</button></div>
</body></html>"#,
        program = latch::id(),
        deal = sess.deal,
        hash = hex(&sess.terms_hash),
        state = state,
        now = ts(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64),
        signed_rows = signed_rows,
        milestone_row = milestone_row,
        tx_rows = tx_rows,
        agreement = sess.agreement.replace('<', "&lt;"),
    )
}

// ---------------------------------------------------------------- server

fn main() {
    let client = RpcClient::new(URL.to_string());
    let app = Mutex::new(App { env: None, sess: None });
    let server = Server::http(("127.0.0.1", PORT)).expect("bind");
    println!("Latch demo → http://localhost:{PORT}");

    for mut request in server.incoming_requests() {
        let url = request.url().to_string();
        let method = request.method().clone();
        let mut body = String::new();
        let _ = request.as_reader().read_to_string(&mut body);

        let (content, ctype): (String, &str) = match (method, url.as_str()) {
            (Method::Get, "/") => (include_str!("index.html").to_string(), "text/html; charset=utf-8"),
            (Method::Get, "/api/state") => (state_json(&app.lock().unwrap(), &client).to_string(), "application/json"),
            (Method::Post, "/api/next") => {
                let mut a = app.lock().unwrap();
                let out = match run_step(&mut a, &client) {
                    Ok(v) => v,
                    Err(v) => v,
                };
                (out.to_string(), "application/json")
            }
            (Method::Post, "/api/path") => {
                let mut a = app.lock().unwrap();
                let choice = serde_json::from_str::<Value>(&body)
                    .ok()
                    .and_then(|v| v["path"].as_str().map(String::from));
                let out = match (choice.as_deref(), a.sess.as_mut()) {
                    (Some("complete"), Some(s)) => { s.path = Some("complete"); json!({"ok": true}) }
                    (Some("dispute"), Some(s)) => { s.path = Some("dispute"); json!({"ok": true}) }
                    _ => json!({"error": "invalid path"}),
                };
                (out.to_string(), "application/json")
            }
            (Method::Post, "/api/reset") => {
                let mut a = app.lock().unwrap();
                a.sess = None;
                (json!({"ok": true}).to_string(), "application/json")
            }
            (Method::Get, "/certificate") => (certificate_html(&app.lock().unwrap(), &client), "text/html; charset=utf-8"),
            (Method::Get, "/agreement.txt") => {
                let a = app.lock().unwrap();
                (
                    a.sess.as_ref().map(|s| s.agreement.clone()).unwrap_or_else(|| "no deal yet".into()),
                    "text/plain; charset=utf-8",
                )
            }
            _ => ("not found".into(), "text/plain"),
        };
        let response = Response::from_string(content)
            .with_header(Header::from_bytes(&b"Content-Type"[..], ctype.as_bytes()).unwrap());
        let _ = request.respond(response);
    }
}
