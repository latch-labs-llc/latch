mod common;

use common::*;
use solana_signer::Signer;
use latch::state::{DealState, DeadlockRule, TimerMode};

fn activation_fixture(rule: DeadlockRule, timeout: i64) -> Fixture {
    let mut f = Fixture::new(vec![1000], move |p| {
        p.deadlock_rule = rule;
        p.timer_mode = TimerMode::FromActivation;
        p.timeout_secs = timeout;
    });
    f.to_active();
    f
}

#[test]
fn from_activation_auto_release_without_any_deadlock() {
    // Marketplace-style: funds auto-release to the payee N seconds after
    // activation unless someone disputes — no deadlock ever raised.
    let mut f = activation_fixture(DeadlockRule::TimeoutRelease, 3600);
    let rix = ix_resolve(&f.deal, &f.mint, &f.alice_ata, &f.bob_ata, &f.token_program);

    assert_err(send(&mut f.svm, &[rix.clone()], &f.alice.pubkey(), &[&f.alice]), "TimeoutNotElapsed");
    warp(&mut f.svm, 3601);
    send(&mut f.svm, &[rix], &f.alice.pubkey(), &[&f.alice]).unwrap();
    assert_eq!(f.deal_state().state, DealState::Completed);
    assert_eq!(token_balance(&f.svm, &f.bob_ata), 1000);
}

#[test]
fn from_activation_dispute_pauses_and_withdraw_resumes() {
    let mut f = activation_fixture(DeadlockRule::TimeoutRelease, 3600);
    let rix = ix_resolve(&f.deal, &f.mint, &f.alice_ata, &f.bob_ata, &f.token_program);

    // Half the clock runs, then the payer disputes.
    warp(&mut f.svm, 1800);
    let ix = ix_raise_deadlock(&f.alice.pubkey(), &f.deal);
    send(&mut f.svm, &[ix], &f.alice.pubkey(), &[&f.alice]).unwrap();
    assert_eq!(f.deal_state().elapsed_at_pause, 1800);

    // Paused: even far past the nominal deadline, timeout cannot fire.
    warp(&mut f.svm, 1_000_000);
    assert_err(
        send(&mut f.svm, &[rix.clone()], &f.bob.pubkey(), &[&f.bob]),
        "TimeoutPausedByDispute",
    );

    // Only the raiser can withdraw.
    let ix = ix_withdraw_deadlock(&f.bob.pubkey(), &f.deal);
    assert_err(send(&mut f.svm, &[ix], &f.bob.pubkey(), &[&f.bob]), "OnlyRaiserMayWithdraw");
    let ix = ix_withdraw_deadlock(&f.alice.pubkey(), &f.deal);
    send(&mut f.svm, &[ix], &f.alice.pubkey(), &[&f.alice]).unwrap();
    assert_eq!(f.deal_state().state, DealState::Active);

    // Clock resumes with 1800s banked: 1799 more isn't enough, 1801 is.
    warp(&mut f.svm, 1799);
    assert_err(send(&mut f.svm, &[rix.clone()], &f.bob.pubkey(), &[&f.bob]), "TimeoutNotElapsed");
    warp(&mut f.svm, 2);
    send(&mut f.svm, &[rix], &f.bob.pubkey(), &[&f.bob]).unwrap();
    assert_eq!(token_balance(&f.svm, &f.bob_ata), 1000);
}

#[test]
fn from_activation_mutual_settlement_still_works_while_disputed() {
    let mut f = activation_fixture(DeadlockRule::TimeoutRefund, 3600);
    let ix = ix_raise_deadlock(&f.bob.pubkey(), &f.deal);
    send(&mut f.svm, &[ix], &f.bob.pubkey(), &[&f.bob]).unwrap();

    for kp in [f.alice.insecure_clone(), f.bob.insecure_clone()] {
        let ix = ix_resolution_sign(&kp.pubkey(), &f.deal, 250);
        send(&mut f.svm, &[ix], &kp.pubkey(), &[&kp]).unwrap();
    }
    let rix = ix_resolve(&f.deal, &f.mint, &f.alice_ata, &f.bob_ata, &f.token_program);
    let alice_before = token_balance(&f.svm, &f.alice_ata);
    send(&mut f.svm, &[rix], &f.alice.pubkey(), &[&f.alice]).unwrap();
    assert_eq!(token_balance(&f.svm, &f.bob_ata), 250);
    assert_eq!(token_balance(&f.svm, &f.alice_ata), alice_before + 750);
}

#[test]
fn withdraw_discards_pending_proposal() {
    let mut f = activation_fixture(DeadlockRule::TimeoutRefund, 3600);
    let ix = ix_raise_deadlock(&f.alice.pubkey(), &f.deal);
    send(&mut f.svm, &[ix], &f.alice.pubkey(), &[&f.alice]).unwrap();
    let ix = ix_resolution_sign(&f.bob.pubkey(), &f.deal, 900);
    send(&mut f.svm, &[ix], &f.bob.pubkey(), &[&f.bob]).unwrap();

    let ix = ix_withdraw_deadlock(&f.alice.pubkey(), &f.deal);
    send(&mut f.svm, &[ix], &f.alice.pubkey(), &[&f.alice]).unwrap();
    let d = f.deal_state();
    assert!(!d.proposal_active);
    assert_eq!(d.resolution_approvals, 0);
}

#[test]
fn from_deadlock_mode_cannot_resolve_while_active() {
    // Existing behavior preserved: in FromDeadlock mode the timeout only ever
    // runs after a deadlock is raised.
    let mut f = Fixture::new(vec![1000], |p| {
        p.deadlock_rule = DeadlockRule::TimeoutRelease;
        p.timer_mode = TimerMode::FromDeadlock;
        p.timeout_secs = 60;
    });
    f.to_active();
    warp(&mut f.svm, 1_000_000);
    let rix = ix_resolve(&f.deal, &f.mint, &f.alice_ata, &f.bob_ata, &f.token_program);
    assert_err(send(&mut f.svm, &[rix], &f.alice.pubkey(), &[&f.alice]), "InvalidState");
}

#[test]
fn from_activation_invalid_for_tiebreaker_and_true_deadlock() {
    // No configuration may create a timeout backdoor into a true deadlock.
    for rule in [DeadlockRule::TrueDeadlock, DeadlockRule::TieBreaker] {
        let (mut svm, alice, bob) = setup();
        let tp = classic_token_id();
        let mint = create_mint(&mut svm, &alice, &alice.pubkey(), None, &tp);
        let mut params = default_params(&alice.pubkey(), &bob.pubkey(), vec![1000]);
        params.deadlock_rule = rule;
        params.timer_mode = TimerMode::FromActivation;
        params.tie_breaker = anchor_lang::prelude::Pubkey::new_unique();
        let ix = ix_create_deal(&alice.pubkey(), &mint, &tp, params);
        assert_err(send(&mut svm, &[ix], &alice.pubkey(), &[&alice]), "InvalidTimerMode");
    }
}

#[test]
fn true_deadlock_still_never_times_out() {
    // The core guarantee: a true-deadlock contract stays locked
    // until mutual sign-off, no matter how much time passes, in every mode.
    let mut f = Fixture::new(vec![1000], |p| p.deadlock_rule = DeadlockRule::TrueDeadlock);
    f.to_active();
    let rix = ix_resolve(&f.deal, &f.mint, &f.alice_ata, &f.bob_ata, &f.token_program);
    // Not resolvable while Active (FromDeadlock mode).
    warp(&mut f.svm, 100 * 365 * 24 * 3600);
    assert_err(send(&mut f.svm, &[rix.clone()], &f.alice.pubkey(), &[&f.alice]), "InvalidState");
    // Nor by time once deadlocked.
    let ix = ix_raise_deadlock(&f.alice.pubkey(), &f.deal);
    send(&mut f.svm, &[ix], &f.alice.pubkey(), &[&f.alice]).unwrap();
    warp(&mut f.svm, 100 * 365 * 24 * 3600);
    assert_err(
        send(&mut f.svm, &[rix], &f.alice.pubkey(), &[&f.alice]),
        "ResolutionConditionsNotMet",
    );
}
