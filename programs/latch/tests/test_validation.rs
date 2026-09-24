mod common;

use anchor_lang::prelude::Pubkey;
use common::*;
use solana_signer::Signer;
use latch::state::{DealState, DeadlockRule};

fn try_create(params_tweak: impl FnOnce(&mut latch::CreateDealParams)) -> litesvm::types::TransactionResult {
    let (mut svm, alice, bob) = setup();
    let tp = classic_token_id();
    let mint = create_mint(&mut svm, &alice, &alice.pubkey(), None, &tp);
    let mut params = default_params(&alice.pubkey(), &bob.pubkey(), vec![1000]);
    params_tweak(&mut params);
    let ix = ix_create_deal(&alice.pubkey(), &mint, &tp, params);
    send(&mut svm, &[ix], &alice.pubkey(), &[&alice])
}

#[test]
fn create_rejects_bad_params() {
    assert_err(try_create(|p| p.parties = vec![p.parties[0]]), "InvalidParties");
    assert_err(try_create(|p| p.parties = vec![p.parties[0], p.parties[0]]), "DuplicateParty");
    assert_err(try_create(|p| { p.parties = vec![Pubkey::new_unique(), Pubkey::new_unique()]; }), "CreatorNotParty");
    assert_err(try_create(|p| p.payee_idx = 0), "InvalidParties");
    assert_err(try_create(|p| p.approval_threshold = 0), "InvalidThreshold");
    assert_err(try_create(|p| p.approval_threshold = 3), "InvalidThreshold");
    assert_err(try_create(|p| p.milestone_amounts = vec![]), "InvalidMilestones");
    assert_err(try_create(|p| p.milestone_amounts = vec![999]), "MilestoneSumMismatch");
    assert_err(try_create(|p| p.timeout_secs = 0), "InvalidDeadlockParams");
    assert_err(
        try_create(|p| {
            p.deadlock_rule = DeadlockRule::TieBreaker;
            p.tie_breaker = Pubkey::default();
        }),
        "InvalidDeadlockParams",
    );
    assert_err(
        try_create(|p| {
            p.deadlock_rule = DeadlockRule::AutoSplit;
            p.split_bps = 10_001;
        }),
        "InvalidDeadlockParams",
    );
    assert_err(try_create(|p| p.recovery_signers = vec![]), "InvalidRecoveryConfig");
    assert_err(try_create(|p| p.recovery_threshold = 2), "InvalidRecoveryConfig");
}

#[test]
fn sign_terms_gating() {
    let mut f = Fixture::new(vec![1000], |_| {});
    // A stranger cannot sign.
    let mallory = solana_keypair::Keypair::new();
    f.svm.airdrop(&mallory.pubkey(), SOL).unwrap();
    assert_err(
        send(&mut f.svm, &[ix_sign_terms(&mallory.pubkey(), &f.deal, true)], &mallory.pubkey(), &[&mallory]),
        "NotAParty",
    );
    // Double-sign fails.
    let ix = ix_sign_terms(&f.alice.pubkey(), &f.deal, true);
    send(&mut f.svm, &[ix.clone()], &f.alice.pubkey(), &[&f.alice]).unwrap();
    assert_err(send(&mut f.svm, &[ix], &f.alice.pubkey(), &[&f.alice]), "AlreadySigned");
}

#[test]
fn true_deadlock_requires_consent() {
    let mut f = Fixture::new(vec![1000], |p| p.deadlock_rule = DeadlockRule::TrueDeadlock);
    let ix = ix_sign_terms(&f.alice.pubkey(), &f.deal, false);
    assert_err(
        send(&mut f.svm, &[ix], &f.alice.pubkey(), &[&f.alice]),
        "TrueDeadlockConsentRequired",
    );
    let ix = ix_sign_terms(&f.alice.pubkey(), &f.deal, true);
    send(&mut f.svm, &[ix], &f.alice.pubkey(), &[&f.alice]).unwrap();
    assert_eq!(f.deal_state().deadlock_consent, 0b01);
}

#[test]
fn deposit_gating() {
    let mut f = Fixture::new(vec![1000], |_| {});
    // Deposit before Signed fails.
    let ix = ix_deposit(&f.alice.pubkey(), &f.deal, &f.mint, &f.alice_ata, &f.token_program, 100);
    assert_err(send(&mut f.svm, &[ix], &f.alice.pubkey(), &[&f.alice]), "InvalidState");

    f.sign_all();
    // Only the payer party may deposit (bob owns no funded ATA anyway; use his key with alice's ATA delegated? — simplest: bob signs with own ATA).
    let bob_ata = f.bob_ata;
    let ix = ix_deposit(&f.bob.pubkey(), &f.deal, &f.mint, &bob_ata, &f.token_program, 100);
    assert_err(send(&mut f.svm, &[ix], &f.bob.pubkey(), &[&f.bob]), "OnlyPayerMayDeposit");
    // Zero deposit fails.
    let ix = ix_deposit(&f.alice.pubkey(), &f.deal, &f.mint, &f.alice_ata, &f.token_program, 0);
    assert_err(send(&mut f.svm, &[ix], &f.alice.pubkey(), &[&f.alice]), "ZeroDeposit");
    // Over-funding fails and rolls back.
    let ix = ix_deposit(&f.alice.pubkey(), &f.deal, &f.mint, &f.alice_ata, &f.token_program, 1001);
    assert_err(send(&mut f.svm, &[ix], &f.alice.pubkey(), &[&f.alice]), "OverFunded");
    assert_eq!(f.deal_state().deposited, 0);
}

#[test]
fn release_gating() {
    let mut f = Fixture::new(vec![600, 400], |_| {});
    f.to_active();

    // No approvals yet.
    let ix = ix_release_milestone(&f.deal, &f.mint, &f.bob_ata, &f.token_program, 0);
    assert_err(send(&mut f.svm, &[ix], &f.alice.pubkey(), &[&f.alice]), "InsufficientApprovals");

    // Approve milestone 1 fully, but it cannot release before milestone 0.
    for kp in [f.alice.insecure_clone(), f.bob.insecure_clone()] {
        let ix = ix_approve_milestone(&kp.pubkey(), &f.deal, 1);
        send(&mut f.svm, &[ix], &kp.pubkey(), &[&kp]).unwrap();
    }
    let ix = ix_release_milestone(&f.deal, &f.mint, &f.bob_ata, &f.token_program, 1);
    assert_err(send(&mut f.svm, &[ix], &f.alice.pubkey(), &[&f.alice]), "MilestonesOutOfOrder");

    // Out-of-range index.
    let ix = ix_release_milestone(&f.deal, &f.mint, &f.bob_ata, &f.token_program, 5);
    assert_err(send(&mut f.svm, &[ix], &f.alice.pubkey(), &[&f.alice]), "MilestoneOutOfRange");

    // A stranger's token account cannot be the payee account.
    let mallory = solana_keypair::Keypair::new();
    f.svm.airdrop(&mallory.pubkey(), SOL).unwrap();
    let mallory_ata =
        create_token_account(&mut f.svm, &f.alice, &mallory.pubkey(), &f.mint, &f.token_program.clone());
    for kp in [f.alice.insecure_clone(), f.bob.insecure_clone()] {
        let ix = ix_approve_milestone(&kp.pubkey(), &f.deal, 0);
        send(&mut f.svm, &[ix], &kp.pubkey(), &[&kp]).unwrap();
    }
    let ix = ix_release_milestone(&f.deal, &f.mint, &mallory_ata, &f.token_program, 0);
    assert_err(send(&mut f.svm, &[ix], &mallory.pubkey(), &[&mallory]), "TokenAccountMismatch");
}

#[test]
fn wrong_state_transitions_rejected() {
    let mut f = Fixture::new(vec![1000], |_| {});
    // Cannot confirm_ready, approve, raise deadlock, or resolve from Draft.
    assert_err(
        send(&mut f.svm, &[ix_confirm_ready(&f.alice.pubkey(), &f.deal)], &f.alice.pubkey(), &[&f.alice]),
        "InvalidState",
    );
    assert_err(
        send(&mut f.svm, &[ix_approve_milestone(&f.alice.pubkey(), &f.deal, 0)], &f.alice.pubkey(), &[&f.alice]),
        "InvalidState",
    );
    assert_err(
        send(&mut f.svm, &[ix_raise_deadlock(&f.alice.pubkey(), &f.deal)], &f.alice.pubkey(), &[&f.alice]),
        "InvalidState",
    );
    // After completion, nothing moves.
    let mut f = Fixture::new(vec![1000], |p| p.approval_threshold = 1);
    f.to_active();
    let ix = ix_approve_milestone(&f.bob.pubkey(), &f.deal, 0);
    send(&mut f.svm, &[ix], &f.bob.pubkey(), &[&f.bob]).unwrap();
    let ix = ix_release_milestone(&f.deal, &f.mint, &f.bob_ata, &f.token_program, 0);
    send(&mut f.svm, &[ix], &f.bob.pubkey(), &[&f.bob]).unwrap();
    assert_eq!(f.deal_state().state, DealState::Completed);
    assert_err(
        send(&mut f.svm, &[ix_raise_deadlock(&f.alice.pubkey(), &f.deal)], &f.alice.pubkey(), &[&f.alice]),
        "InvalidState",
    );
}
