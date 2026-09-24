mod common;

use common::*;
use solana_signer::Signer;
use latch::state::DealState;

#[test]
fn any_party_cancels_draft() {
    let mut f = Fixture::new(vec![1000], |_| {});
    let ix = ix_cancel_draft(&f.bob.pubkey(), &f.deal);
    send(&mut f.svm, &[ix], &f.bob.pubkey(), &[&f.bob]).unwrap();
    assert_eq!(f.deal_state().state, DealState::Cancelled);
}

#[test]
fn stranger_cannot_cancel_draft() {
    let mut f = Fixture::new(vec![1000], |_| {});
    let mallory = solana_keypair::Keypair::new();
    f.svm.airdrop(&mallory.pubkey(), SOL).unwrap();
    let ix = ix_cancel_draft(&mallory.pubkey(), &f.deal);
    assert_err(send(&mut f.svm, &[ix], &mallory.pubkey(), &[&mallory]), "NotAParty");
}

#[test]
fn draft_cancel_invalid_after_signing() {
    let mut f = Fixture::new(vec![1000], |_| {});
    f.sign_all();
    let ix = ix_cancel_draft(&f.alice.pubkey(), &f.deal);
    assert_err(send(&mut f.svm, &[ix], &f.alice.pubkey(), &[&f.alice]), "InvalidState");
}

#[test]
fn mutual_cancel_refunds_payer() {
    let mut f = Fixture::new(vec![1000], |_| {});
    f.sign_all();
    f.fund();
    let alice_before = token_balance(&f.svm, &f.alice_ata);

    // One party alone does not cancel.
    let ix = ix_cancel_sign(&f.alice.pubkey(), &f.deal, &f.mint, &f.alice_ata, &f.token_program);
    send(&mut f.svm, &[ix], &f.alice.pubkey(), &[&f.alice]).unwrap();
    assert_eq!(f.deal_state().state, DealState::Funded);

    // Second signature completes the mutual cancel and refunds the deposit.
    let ix = ix_cancel_sign(&f.bob.pubkey(), &f.deal, &f.mint, &f.alice_ata, &f.token_program);
    send(&mut f.svm, &[ix], &f.bob.pubkey(), &[&f.bob]).unwrap();
    let d = f.deal_state();
    assert_eq!(d.state, DealState::Cancelled);
    assert_eq!(token_balance(&f.svm, &f.alice_ata), alice_before + 1000);
    assert_eq!(token_balance(&f.svm, &vault_ata(&f.deal, &f.mint, &f.token_program)), 0);
}

#[test]
fn mutual_cancel_not_available_once_active() {
    let mut f = Fixture::new(vec![1000], |_| {});
    f.to_active();
    let ix = ix_cancel_sign(&f.alice.pubkey(), &f.deal, &f.mint, &f.alice_ata, &f.token_program);
    assert_err(send(&mut f.svm, &[ix], &f.alice.pubkey(), &[&f.alice]), "InvalidState");
}
