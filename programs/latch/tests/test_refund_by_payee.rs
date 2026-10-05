mod common;

use common::*;
use latch::state::DealState;
use solana_signer::Signer;

fn refund(f: &mut Fixture) -> litesvm::types::TransactionResult {
    let bob = f.bob.insecure_clone();
    let ix = ix_refund_by_payee(
        &bob.pubkey(),
        &f.deal,
        &f.mint,
        &f.alice_ata,
        &f.token_program,
    );
    send(&mut f.svm, &[ix], &bob.pubkey(), &[&bob])
}

fn vault_of(f: &Fixture) -> u64 {
    token_balance(&f.svm, &vault_ata(&f.deal, &f.mint, &f.token_program))
}

#[test]
fn payee_refunds_a_funded_deal() {
    let mut f = Fixture::new(vec![1000], |_| {});
    f.sign_all();
    f.fund();
    let before = token_balance(&f.svm, &f.alice_ata);
    refund(&mut f).unwrap();
    assert_eq!(f.deal_state().state, DealState::Cancelled);
    assert_eq!(token_balance(&f.svm, &f.alice_ata), before + 1000);
    assert_eq!(vault_of(&f), 0);
}

#[test]
fn payee_refunds_an_active_deal() {
    let mut f = Fixture::new(vec![1000], |_| {});
    f.to_active();
    let before = token_balance(&f.svm, &f.alice_ata);
    refund(&mut f).unwrap();
    assert_eq!(f.deal_state().state, DealState::Cancelled);
    assert_eq!(token_balance(&f.svm, &f.alice_ata), before + 1000);
}

#[test]
fn payee_refunds_the_remainder_after_a_release_and_the_deal_completes() {
    let mut f = Fixture::new(vec![300, 700], |_| {});
    f.to_active();
    for kp in [f.alice.insecure_clone(), f.bob.insecure_clone()] {
        let ix = ix_approve_milestone(&kp.pubkey(), &f.deal, 0);
        send(&mut f.svm, &[ix], &kp.pubkey(), &[&kp]).unwrap();
    }
    let ix = ix_release_milestone(&f.deal, &f.mint, &f.bob_ata, &f.token_program, 0);
    send(&mut f.svm, &[ix], &f.alice.pubkey(), &[&f.alice]).unwrap();
    let before = token_balance(&f.svm, &f.alice_ata);
    refund(&mut f).unwrap();
    // The first milestone stands; the deal concludes with the rest returned.
    assert_eq!(f.deal_state().state, DealState::Completed);
    assert_eq!(token_balance(&f.svm, &f.alice_ata), before + 700);
    assert_eq!(token_balance(&f.svm, &f.bob_ata), 300);
}

#[test]
fn payee_refunds_during_a_dispute() {
    let mut f = Fixture::new(vec![1000], |_| {});
    f.to_active();
    let ix = ix_raise_deadlock(&f.alice.pubkey(), &f.deal);
    send(&mut f.svm, &[ix], &f.alice.pubkey(), &[&f.alice]).unwrap();
    let before = token_balance(&f.svm, &f.alice_ata);
    refund(&mut f).unwrap();
    assert_eq!(f.deal_state().state, DealState::Cancelled);
    assert_eq!(token_balance(&f.svm, &f.alice_ata), before + 1000);
}

#[test]
fn payee_refunds_a_partial_deposit() {
    let mut f = Fixture::new(vec![1000], |_| {});
    f.sign_all();
    let ix = ix_deposit(
        &f.alice.pubkey(),
        &f.deal,
        &f.mint,
        &f.alice_ata,
        &f.token_program,
        250,
    );
    send(&mut f.svm, &[ix], &f.alice.pubkey(), &[&f.alice]).unwrap();
    let before = token_balance(&f.svm, &f.alice_ata);
    refund(&mut f).unwrap();
    assert_eq!(f.deal_state().state, DealState::Cancelled);
    assert_eq!(token_balance(&f.svm, &f.alice_ata), before + 250);
}

#[test]
fn only_the_payee_can_refund() {
    let mut f = Fixture::new(vec![1000], |_| {});
    f.to_active();
    let mallory = solana_keypair::Keypair::new();
    f.svm.airdrop(&mallory.pubkey(), SOL).unwrap();
    for kp in [f.alice.insecure_clone(), mallory] {
        let ix = ix_refund_by_payee(
            &kp.pubkey(),
            &f.deal,
            &f.mint,
            &f.alice_ata,
            &f.token_program,
        );
        assert_err(
            send(&mut f.svm, &[ix], &kp.pubkey(), &[&kp]),
            "OnlyPayeeMayRefund",
        );
    }
    assert_eq!(f.deal_state().state, DealState::Active);
    assert_eq!(vault_of(&f), 1000);
}

#[test]
fn refund_goes_only_to_the_payer() {
    let mut f = Fixture::new(vec![1000], |_| {});
    f.to_active();
    let bob = f.bob.insecure_clone();
    // The payee cannot route the "refund" to themselves.
    let ix = ix_refund_by_payee(
        &bob.pubkey(),
        &f.deal,
        &f.mint,
        &f.bob_ata,
        &f.token_program,
    );
    assert_err(
        send(&mut f.svm, &[ix], &bob.pubkey(), &[&bob]),
        "TokenAccountMismatch",
    );
    let ix = ix_refund_by_payee_with_vault(
        &bob.pubkey(),
        &f.deal,
        &f.mint,
        &f.bob_ata,
        &f.alice_ata,
        &f.token_program,
    );
    assert_err(
        send(&mut f.svm, &[ix], &bob.pubkey(), &[&bob]),
        "ConstraintHasOne",
    );
    assert_eq!(vault_of(&f), 1000);
}

#[test]
fn nothing_to_refund_before_a_deposit_or_after_the_end() {
    let mut f = Fixture::new(vec![1000], |_| {});
    assert_err(refund(&mut f), "InvalidState"); // Draft
    f.sign_all();
    assert_err(refund(&mut f), "InvalidState"); // Signed, nothing deposited
    f.fund();
    refund(&mut f).unwrap();
    warp(&mut f.svm, 1);
    assert_err(refund(&mut f), "InvalidState"); // already Cancelled
}
