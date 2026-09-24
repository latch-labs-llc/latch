//! End-to-end devnet run: creates a test mint, walks a two-party deal through
//! the full lifecycle (create → sign → fund → activate → approve → release),
//! then runs a second deal into deadlock and resolves it by timeout refund.
//!
//! Usage:
//!   anchor build --arch v0 && anchor deploy --provider.cluster devnet
//!   cargo run --example devnet_e2e
//!
//! Requires ~0.5 devnet SOL in ~/.config/solana/id.json (faucet.solana.com).

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
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_rpc_client::rpc_client::RpcClient,
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
    latch::{state::*, CreateDealParams},
    std::{thread::sleep, time::Duration},
};

const MINT_LEN: usize = 82;
const TOKEN_ACCOUNT_LEN: usize = 165;
const URL: &str = "https://api.devnet.solana.com";

fn send(client: &RpcClient, ixs: &[Instruction], payer: &Pubkey, signers: &[&Keypair]) -> String {
    let blockhash = client.get_latest_blockhash().expect("blockhash");
    let msg = Message::new_with_blockhash(ixs, Some(payer), &blockhash);
    let mut unique: Vec<&Keypair> = Vec::new();
    for s in signers {
        if !unique.iter().any(|u| u.pubkey() == s.pubkey()) {
            unique.push(s);
        }
    }
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &unique).expect("sign");
    let sig = client.send_and_confirm_transaction(&tx).expect("tx failed");
    sig.to_string()
}

fn event_authority() -> Pubkey {
    Pubkey::find_program_address(&[b"__event_authority"], &latch::id()).0
}

fn main() {
    let client = RpcClient::new(URL.to_string());
    let home = std::env::var("HOME").unwrap();
    let alice = solana_keypair::read_keypair_file(format!("{home}/.config/solana/id.json"))
        .expect("no keypair at ~/.config/solana/id.json");
    let bob = Keypair::new();
    let tp = anchor_spl::token::ID;
    println!("payer/creator (alice): {}", alice.pubkey());
    println!("counterparty (bob):    {}", bob.pubkey());

    let balance = client.get_balance(&alice.pubkey()).unwrap();
    println!("alice devnet balance: {} SOL", balance as f64 / 1e9);
    assert!(balance > 400_000_000, "need ≥0.4 devnet SOL — use faucet.solana.com");

    // Give bob a little SOL to sign his own transactions.
    let sig = send(
        &client,
        &[system_instruction::transfer(&alice.pubkey(), &bob.pubkey(), 50_000_000)],
        &alice.pubkey(),
        &[&alice],
    );
    println!("funded bob: {sig}");

    // Test mint (stand-in stablecoin, 6 decimals) + token accounts.
    let mint_kp = Keypair::new();
    let rent = client
        .get_minimum_balance_for_rent_exemption(MINT_LEN)
        .unwrap();
    let sig = send(
        &client,
        &[
            system_instruction::create_account(
                &alice.pubkey(),
                &mint_kp.pubkey(),
                rent,
                MINT_LEN as u64,
                &tp,
            ),
            spl_token_2022::instruction::initialize_mint2(
                &tp,
                &mint_kp.pubkey(),
                &alice.pubkey(),
                None,
                6,
            )
            .unwrap(),
        ],
        &alice.pubkey(),
        &[&alice, &mint_kp],
    );
    let mint = mint_kp.pubkey();
    println!("test mint {mint}: {sig}");

    let make_ata = |owner: &Pubkey| -> Pubkey {
        let kp = Keypair::new();
        let rent = client
            .get_minimum_balance_for_rent_exemption(TOKEN_ACCOUNT_LEN)
            .unwrap();
        send(
            &client,
            &[
                system_instruction::create_account(
                    &alice.pubkey(),
                    &kp.pubkey(),
                    rent,
                    TOKEN_ACCOUNT_LEN as u64,
                    &tp,
                ),
                spl_token_2022::instruction::initialize_account3(&tp, &kp.pubkey(), &mint, owner)
                    .unwrap(),
            ],
            &alice.pubkey(),
            &[&alice, &kp],
        );
        kp.pubkey()
    };
    let alice_ta = make_ata(&alice.pubkey());
    let bob_ta = make_ata(&bob.pubkey());
    let sig = send(
        &client,
        &[spl_token_2022::instruction::mint_to(&tp, &mint, &alice_ta, &alice.pubkey(), &[], 2_000_000_000).unwrap()],
        &alice.pubkey(),
        &[&alice],
    );
    println!("minted 2000 tokens to alice: {sig}");

    // ---------- Deal 1: full happy path ----------
    let deal_id: u64 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let run_deal = |deal_id: u64, rule: DeadlockRule, timeout_secs: i64| -> Pubkey {
        let params = CreateDealParams {
            deal_id,
            parties: vec![alice.pubkey(), bob.pubkey()],
            payer_idx: 0,
            payee_idx: 1,
            approval_threshold: 2,
            terms_hash: [7u8; 32],
            price_risk_term: 0,
            total_amount: 1_000_000_000,
            milestone_amounts: vec![600_000_000, 400_000_000],
            deadlock_rule: rule,
            timer_mode: TimerMode::FromDeadlock,
            timeout_secs,
            split_bps: 0,
            tie_breaker: Pubkey::default(),
            recovery_signers: vec![Pubkey::new_unique()],
            recovery_threshold: 1,
            recovery_delay_secs: 0,
            accepted_risk_flags: u16::MAX,
        };
        let deal = Pubkey::find_program_address(
            &[b"deal", alice.pubkey().as_ref(), &deal_id.to_le_bytes()],
            &latch::id(),
        )
        .0;
        let vault = get_associated_token_address_with_program_id(&deal, &mint, &tp);
        let ix = Instruction::new_with_bytes(
            latch::id(),
            &latch::instruction::CreateDeal { params }.data(),
            latch::accounts::CreateDeal {
                creator: alice.pubkey(),
                deal,
                mint,
                vault,
                token_program: tp,
                associated_token_program: anchor_spl::associated_token::ID,
                system_program: system_program::ID,
                event_authority: event_authority(),
                program: latch::id(),
            }
            .to_account_metas(None),
        );
        let sig = send(&client, &[ix], &alice.pubkey(), &[&alice]);
        println!("  create_deal {deal}: {sig}");

        for kp in [&alice, &bob] {
            let ix = Instruction::new_with_bytes(
                latch::id(),
                &latch::instruction::SignTerms { consent_true_deadlock: true }.data(),
                latch::accounts::SignTerms {
                    party: kp.pubkey(),
                    deal,
                    event_authority: event_authority(),
                    program: latch::id(),
                }
                .to_account_metas(None),
            );
            let sig = send(&client, &[ix], &kp.pubkey(), &[kp]);
            println!("  sign_terms({}): {sig}", kp.pubkey());
        }

        let ix = Instruction::new_with_bytes(
            latch::id(),
            &latch::instruction::Deposit { amount: 1_000_000_000 }.data(),
            latch::accounts::Deposit {
                payer: alice.pubkey(),
                deal,
                mint,
                vault,
                payer_token_account: alice_ta,
                token_program: tp,
                event_authority: event_authority(),
                program: latch::id(),
            }
            .to_account_metas(None),
        );
        let sig = send(&client, &[ix], &alice.pubkey(), &[&alice]);
        println!("  deposit 1000 tokens: {sig}");

        for kp in [&alice, &bob] {
            let ix = Instruction::new_with_bytes(
                latch::id(),
                &latch::instruction::ConfirmReady {}.data(),
                latch::accounts::ConfirmReady {
                    party: kp.pubkey(),
                    deal,
                    event_authority: event_authority(),
                    program: latch::id(),
                }
                .to_account_metas(None),
            );
            send(&client, &[ix], &kp.pubkey(), &[kp]);
        }
        println!("  both parties confirmed ready → Active");
        deal
    };

    println!("\n=== Deal 1: happy path (two milestones) ===");
    let deal = run_deal(deal_id, DeadlockRule::TimeoutRefund, 3600);
    let vault = get_associated_token_address_with_program_id(&deal, &mint, &tp);
    for index in 0u8..2 {
        for kp in [&alice, &bob] {
            let ix = Instruction::new_with_bytes(
                latch::id(),
                &latch::instruction::ApproveMilestone { index }.data(),
                latch::accounts::ApproveMilestone {
                    party: kp.pubkey(),
                    deal,
                    event_authority: event_authority(),
                    program: latch::id(),
                }
                .to_account_metas(None),
            );
            send(&client, &[ix], &kp.pubkey(), &[kp]);
        }
        let ix = Instruction::new_with_bytes(
            latch::id(),
            &latch::instruction::ReleaseMilestone { index }.data(),
            latch::accounts::ReleaseMilestone {
                deal,
                mint,
                vault,
                payee_token_account: bob_ta,
                token_program: tp,
                event_authority: event_authority(),
                program: latch::id(),
            }
            .to_account_metas(None),
        );
        let sig = send(&client, &[ix], &alice.pubkey(), &[&alice]);
        println!("  milestone {index} approved by both and released: {sig}");
    }
    let data = client.get_account_data(&deal).unwrap();
    let state = Deal::try_deserialize(&mut data.as_slice()).unwrap();
    println!("  final state: {:?}, released_total: {}", state.state, state.released_total);
    assert_eq!(state.state, DealState::Completed);

    // ---------- Deal 2: deadlock → timeout refund ----------
    println!("\n=== Deal 2: deadlock resolved by timeout refund (10s) ===");
    let deal = run_deal(deal_id + 1, DeadlockRule::TimeoutRefund, 10);
    let vault = get_associated_token_address_with_program_id(&deal, &mint, &tp);
    let ix = Instruction::new_with_bytes(
        latch::id(),
        &latch::instruction::RaiseDeadlock {}.data(),
        latch::accounts::RaiseDeadlock {
            party: bob.pubkey(),
            deal,
            event_authority: event_authority(),
            program: latch::id(),
        }
        .to_account_metas(None),
    );
    let sig = send(&client, &[ix], &bob.pubkey(), &[&bob]);
    println!("  bob raised deadlock: {sig}");
    println!("  waiting 15s for the timeout...");
    sleep(Duration::from_secs(15));
    let ix = Instruction::new_with_bytes(
        latch::id(),
        &latch::instruction::Resolve {}.data(),
        latch::accounts::Resolve {
            deal,
            mint,
            vault,
            payer_token_account: alice_ta,
            payee_token_account: bob_ta,
            token_program: tp,
            event_authority: event_authority(),
            program: latch::id(),
        }
        .to_account_metas(None),
    );
    let sig = send(&client, &[ix], &alice.pubkey(), &[&alice]);
    println!("  resolve (timeout refund to payer): {sig}");
    let data = client.get_account_data(&deal).unwrap();
    let state = Deal::try_deserialize(&mut data.as_slice()).unwrap();
    assert_eq!(state.state, DealState::Completed);

    println!("\nAll devnet scenarios completed. Explorer: https://explorer.solana.com/address/{}?cluster=devnet", latch::id());
}
