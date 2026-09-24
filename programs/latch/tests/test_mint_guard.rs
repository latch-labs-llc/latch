mod common;

use common::*;
use solana_signer::Signer;
use latch::state::{risk_flags, DealState};

fn try_create_2022(
    exts: &[TestExt],
    accepted: u16,
) -> (litesvm::LiteSVM, litesvm::types::TransactionResult, anchor_lang::prelude::Pubkey) {
    let (mut svm, alice, bob) = setup();
    let mint = create_mint_2022_with_extensions(&mut svm, &alice, &alice.pubkey(), exts);
    let mut params = default_params(&alice.pubkey(), &bob.pubkey(), vec![1000]);
    params.accepted_risk_flags = accepted;
    let deal = deal_pda(&alice.pubkey(), params.deal_id);
    let ix = ix_create_deal(&alice.pubkey(), &mint, &token_2022_id(), params);
    let res = send(&mut svm, &[ix], &alice.pubkey(), &[&alice]);
    (svm, res, deal)
}

#[test]
fn non_transferable_mint_rejected() {
    let (_svm, res, _) = try_create_2022(&[TestExt::NonTransferable], u16::MAX);
    assert_err(res, "NonTransferableMint");
}

#[test]
fn default_frozen_mint_rejected() {
    let (_svm, res, _) = try_create_2022(&[TestExt::DefaultFrozen], u16::MAX);
    assert_err(res, "DefaultFrozenMint");
}

#[test]
fn permanent_delegate_flagged_and_requires_acceptance() {
    let delegate = anchor_lang::prelude::Pubkey::new_unique();
    // Not accepted → rejected.
    let (_svm, res, _) = try_create_2022(&[TestExt::PermanentDelegate(delegate)], 0);
    assert_err(res, "RiskFlagsNotAccepted");
    // Accepted → recorded in the deal.
    let (svm, res, deal) =
        try_create_2022(&[TestExt::PermanentDelegate(delegate)], risk_flags::PERMANENT_DELEGATE);
    res.unwrap();
    let d = get_deal(&svm, &deal);
    assert_eq!(d.risk_flags & risk_flags::PERMANENT_DELEGATE, risk_flags::PERMANENT_DELEGATE);
}

#[test]
fn freeze_authority_flagged_on_classic_mint() {
    let (mut svm, alice, bob) = setup();
    let tp = classic_token_id();
    let freeze = anchor_lang::prelude::Pubkey::new_unique();
    let mint = create_mint(&mut svm, &alice, &alice.pubkey(), Some(&freeze), &tp);

    // Not accepted → rejected.
    let mut params = default_params(&alice.pubkey(), &bob.pubkey(), vec![1000]);
    params.accepted_risk_flags = 0;
    let ix = ix_create_deal(&alice.pubkey(), &mint, &tp, params);
    assert_err(send(&mut svm, &[ix], &alice.pubkey(), &[&alice]), "RiskFlagsNotAccepted");

    // Accepted → flag recorded (this is the USDC/USDT disclosure path).
    let mut params = default_params(&alice.pubkey(), &bob.pubkey(), vec![1000]);
    params.deal_id = 2;
    params.accepted_risk_flags = risk_flags::FREEZE_AUTHORITY;
    let deal = deal_pda(&alice.pubkey(), 2);
    let ix = ix_create_deal(&alice.pubkey(), &mint, &tp, params);
    send(&mut svm, &[ix], &alice.pubkey(), &[&alice]).unwrap();
    assert_eq!(get_deal(&svm, &deal).risk_flags, risk_flags::FREEZE_AUTHORITY);
}

#[test]
fn transfer_fee_mint_credits_by_balance_delta() {
    let (mut svm, alice, bob) = setup();
    let tp = token_2022_id();
    // 1% fee. Deal total 990: a 1000-token deposit nets exactly 990 after fee.
    let mint = create_mint_2022_with_extensions(
        &mut svm,
        &alice,
        &alice.pubkey(),
        &[TestExt::TransferFee { bps: 100, max: 1_000_000 }],
    );
    let alice_ata = create_token_account(&mut svm, &alice, &alice.pubkey(), &mint, &tp);
    let alice_kp = alice.insecure_clone();
    mint_to(&mut svm, &alice, &alice_kp, &mint, &alice_ata, 100_000, &tp);

    let mut params = default_params(&alice.pubkey(), &bob.pubkey(), vec![990]);
    params.accepted_risk_flags = risk_flags::TRANSFER_FEE;
    let deal = deal_pda(&alice.pubkey(), params.deal_id);
    let ix = ix_create_deal(&alice.pubkey(), &mint, &tp, params);
    send(&mut svm, &[ix], &alice.pubkey(), &[&alice]).unwrap();
    for kp in [alice.insecure_clone(), bob.insecure_clone()] {
        svm.airdrop(&kp.pubkey(), SOL).unwrap();
        send(&mut svm, &[ix_sign_terms(&kp.pubkey(), &deal, true)], &kp.pubkey(), &[&kp]).unwrap();
    }

    // Sending 1000 credits 990 — the deal counts what the vault received.
    let ix = ix_deposit(&alice.pubkey(), &deal, &mint, &alice_ata, &tp, 1000);
    send(&mut svm, &[ix], &alice.pubkey(), &[&alice]).unwrap();
    let d = get_deal(&svm, &deal);
    assert_eq!(d.deposited, 990);
    assert_eq!(d.state, DealState::Funded);
}
