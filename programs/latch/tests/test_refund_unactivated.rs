mod common;

use common::*;
use latch::state::DealState;
use latch::ACTIVATION_WINDOW_SECS;
use solana_signer::Signer;

/// A stranger with SOL for fees — proves the crank is permissionless.
fn stranger(f: &mut Fixture) -> solana_keypair::Keypair {
    let kp = solana_keypair::Keypair::new();
    f.svm.airdrop(&kp.pubkey(), SOL).unwrap();
    kp
}

fn funded() -> Fixture {
    let mut f = Fixture::new(vec![1000], |_| {});
    f.sign_all();
    f.fund();
    assert_eq!(f.deal_state().state, DealState::Funded);
    f
}

#[test]
fn lapsed_deal_refunds_payer_via_any_cranker() {
    let mut f = funded();
    let alice_before = token_balance(&f.svm, &f.alice_ata);
    warp(&mut f.svm, ACTIVATION_WINDOW_SECS);

    let mallory = stranger(&mut f);
    let ix = ix_refund_unactivated(
        &mallory.pubkey(),
        &f.deal,
        &f.mint,
        &f.alice_ata,
        &f.token_program,
    );
    send(&mut f.svm, &[ix], &mallory.pubkey(), &[&mallory]).unwrap();

    assert_eq!(f.deal_state().state, DealState::Cancelled);
    assert_eq!(token_balance(&f.svm, &f.alice_ata), alice_before + 1000);
    assert_eq!(
        token_balance(&f.svm, &vault_ata(&f.deal, &f.mint, &f.token_program)),
        0
    );
}

#[test]
fn refund_rejected_until_window_lapses() {
    let mut f = funded();
    warp(&mut f.svm, ACTIVATION_WINDOW_SECS - 1);
    let ix = ix_refund_unactivated(
        &f.alice.pubkey(),
        &f.deal,
        &f.mint,
        &f.alice_ata,
        &f.token_program,
    );
    assert_err(
        send(
            &mut f.svm,
            std::slice::from_ref(&ix),
            &f.alice.pubkey(),
            &[&f.alice],
        ),
        "ActivationWindowOpen",
    );
    assert_eq!(f.deal_state().state, DealState::Funded);

    // One second later the window has lapsed (boundary is inclusive).
    warp(&mut f.svm, 1);
    send(&mut f.svm, &[ix], &f.alice.pubkey(), &[&f.alice]).unwrap();
    assert_eq!(f.deal_state().state, DealState::Cancelled);
}

#[test]
fn one_party_ready_does_not_block_the_lapse() {
    let mut f = funded();
    // The payee confirms; the deal still waits on the payer and stays Funded.
    let ix = ix_confirm_ready(&f.bob.pubkey(), &f.deal);
    send(&mut f.svm, &[ix], &f.bob.pubkey(), &[&f.bob]).unwrap();
    assert_eq!(f.deal_state().state, DealState::Funded);

    warp(&mut f.svm, ACTIVATION_WINDOW_SECS);
    let ix = ix_refund_unactivated(
        &f.bob.pubkey(),
        &f.deal,
        &f.mint,
        &f.alice_ata,
        &f.token_program,
    );
    send(&mut f.svm, &[ix], &f.bob.pubkey(), &[&f.bob]).unwrap();
    assert_eq!(f.deal_state().state, DealState::Cancelled);
}

#[test]
fn not_available_once_active() {
    let mut f = Fixture::new(vec![1000], |_| {});
    f.to_active();
    warp(&mut f.svm, ACTIVATION_WINDOW_SECS);
    let ix = ix_refund_unactivated(
        &f.alice.pubkey(),
        &f.deal,
        &f.mint,
        &f.alice_ata,
        &f.token_program,
    );
    assert_err(
        send(&mut f.svm, &[ix], &f.alice.pubkey(), &[&f.alice]),
        "InvalidState",
    );
}

#[test]
fn not_available_before_funding() {
    let mut f = Fixture::new(vec![1000], |_| {});
    f.sign_all();
    warp(&mut f.svm, ACTIVATION_WINDOW_SECS);
    let ix = ix_refund_unactivated(
        &f.alice.pubkey(),
        &f.deal,
        &f.mint,
        &f.alice_ata,
        &f.token_program,
    );
    assert_err(
        send(&mut f.svm, &[ix], &f.alice.pubkey(), &[&f.alice]),
        "InvalidState",
    );
}

#[test]
fn cannot_refund_twice() {
    let mut f = funded();
    warp(&mut f.svm, ACTIVATION_WINDOW_SECS);
    let ix = ix_refund_unactivated(
        &f.alice.pubkey(),
        &f.deal,
        &f.mint,
        &f.alice_ata,
        &f.token_program,
    );
    send(
        &mut f.svm,
        std::slice::from_ref(&ix),
        &f.alice.pubkey(),
        &[&f.alice],
    )
    .unwrap();
    warp(&mut f.svm, 1);
    assert_err(
        send(&mut f.svm, &[ix], &f.alice.pubkey(), &[&f.alice]),
        "InvalidState",
    );
}

#[test]
fn refund_cannot_be_redirected_to_another_account() {
    let mut f = funded();
    warp(&mut f.svm, ACTIVATION_WINDOW_SECS);
    let mallory = stranger(&mut f);
    let mallory_ata = create_token_account(
        &mut f.svm,
        &f.alice,
        &mallory.pubkey(),
        &f.mint,
        &f.token_program,
    );
    // Neither a stranger's account nor the payee's account is accepted.
    for wrong in [mallory_ata, f.bob_ata] {
        let ix = ix_refund_unactivated(
            &mallory.pubkey(),
            &f.deal,
            &f.mint,
            &wrong,
            &f.token_program,
        );
        assert_err(
            send(&mut f.svm, &[ix], &mallory.pubkey(), &[&mallory]),
            "TokenAccountMismatch",
        );
    }
    assert_eq!(f.deal_state().state, DealState::Funded);
}

#[test]
fn refund_rejects_a_substituted_vault() {
    let mut f = funded();
    warp(&mut f.svm, ACTIVATION_WINDOW_SECS);
    // Any account other than the deal's own vault fails the has_one check.
    let ix = ix_refund_unactivated_with_vault(
        &f.alice.pubkey(),
        &f.deal,
        &f.mint,
        &f.bob_ata,
        &f.alice_ata,
        &f.token_program,
    );
    assert_err(
        send(&mut f.svm, &[ix], &f.alice.pubkey(), &[&f.alice]),
        "ConstraintHasOne",
    );
}

/// Partial funding: the deal stays Signed with tokens in the vault. Mutual
/// cancel needs the counterparty and recovery is post-funding only, so the
/// lapse refund must also cover a partially funded Signed deal.
fn partially_funded() -> Fixture {
    let mut f = Fixture::new(vec![1000], |_| {});
    f.sign_all();
    let ix = ix_deposit(
        &f.alice.pubkey(),
        &f.deal,
        &f.mint,
        &f.alice_ata,
        &f.token_program,
        400,
    );
    send(&mut f.svm, &[ix], &f.alice.pubkey(), &[&f.alice]).unwrap();
    assert_eq!(f.deal_state().state, DealState::Signed);
    f
}

#[test]
fn partial_deposit_lapses_back_to_payer() {
    let mut f = partially_funded();
    let alice_before = token_balance(&f.svm, &f.alice_ata);
    warp(&mut f.svm, ACTIVATION_WINDOW_SECS);
    let mallory = stranger(&mut f);
    let ix = ix_refund_unactivated(
        &mallory.pubkey(),
        &f.deal,
        &f.mint,
        &f.alice_ata,
        &f.token_program,
    );
    send(&mut f.svm, &[ix], &mallory.pubkey(), &[&mallory]).unwrap();
    assert_eq!(f.deal_state().state, DealState::Cancelled);
    assert_eq!(token_balance(&f.svm, &f.alice_ata), alice_before + 400);
}

#[test]
fn partial_deposit_refund_waits_for_window_from_signing() {
    let mut f = partially_funded();
    warp(&mut f.svm, ACTIVATION_WINDOW_SECS - 1);
    let ix = ix_refund_unactivated(
        &f.alice.pubkey(),
        &f.deal,
        &f.mint,
        &f.alice_ata,
        &f.token_program,
    );
    assert_err(
        send(
            &mut f.svm,
            std::slice::from_ref(&ix),
            &f.alice.pubkey(),
            &[&f.alice],
        ),
        "ActivationWindowOpen",
    );
    warp(&mut f.svm, 1);
    send(&mut f.svm, &[ix], &f.alice.pubkey(), &[&f.alice]).unwrap();
    assert_eq!(f.deal_state().state, DealState::Cancelled);
}
