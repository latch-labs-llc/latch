mod common;

use common::*;
use solana_keypair::Keypair;
use solana_signer::Signer;
use latch::state::{DealState, DeadlockRule};

fn deadlocked_fixture(tweak: impl FnOnce(&mut latch::CreateDealParams)) -> Fixture {
    let mut f = Fixture::new(vec![1000], tweak);
    f.to_active();
    let ix = ix_raise_deadlock(&f.bob.pubkey(), &f.deal);
    send(&mut f.svm, &[ix], &f.bob.pubkey(), &[&f.bob]).unwrap();
    assert_eq!(f.deal_state().state, DealState::Deadlocked);
    f
}

fn resolve_ix(f: &Fixture) -> anchor_lang::solana_program::instruction::Instruction {
    ix_resolve(&f.deal, &f.mint, &f.alice_ata, &f.bob_ata, &f.token_program)
}

#[test]
fn timeout_release_pays_payee_after_timeout() {
    let mut f = deadlocked_fixture(|p| {
        p.deadlock_rule = DeadlockRule::TimeoutRelease;
        p.timeout_secs = 3600;
    });
    let rix = resolve_ix(&f);
    // Too early.
    assert_err(send(&mut f.svm, &[rix.clone()], &f.alice.pubkey(), &[&f.alice]), "TimeoutNotElapsed");
    warp(&mut f.svm, 3601);
    send(&mut f.svm, &[rix.clone()], &f.alice.pubkey(), &[&f.alice]).unwrap();
    assert_eq!(f.deal_state().state, DealState::Completed);
    assert_eq!(token_balance(&f.svm, &f.bob_ata), 1000);
}

#[test]
fn timeout_refund_pays_payer_after_timeout() {
    let mut f = deadlocked_fixture(|p| {
        p.deadlock_rule = DeadlockRule::TimeoutRefund;
        p.timeout_secs = 3600;
    });
    let rix = resolve_ix(&f);
    let alice_before = token_balance(&f.svm, &f.alice_ata);
    warp(&mut f.svm, 3601);
    send(&mut f.svm, &[rix.clone()], &f.bob.pubkey(), &[&f.bob]).unwrap();
    assert_eq!(token_balance(&f.svm, &f.alice_ata), alice_before + 1000);
    assert_eq!(token_balance(&f.svm, &f.bob_ata), 0);
}

#[test]
fn auto_split_divides_by_bps() {
    let mut f = deadlocked_fixture(|p| {
        p.deadlock_rule = DeadlockRule::AutoSplit;
        p.timeout_secs = 100;
        p.split_bps = 3000; // 30% to payee
    });
    let rix = resolve_ix(&f);
    let alice_before = token_balance(&f.svm, &f.alice_ata);
    warp(&mut f.svm, 101);
    send(&mut f.svm, &[rix.clone()], &f.alice.pubkey(), &[&f.alice]).unwrap();
    assert_eq!(token_balance(&f.svm, &f.bob_ata), 300);
    assert_eq!(token_balance(&f.svm, &f.alice_ata), alice_before + 700);
}

#[test]
fn tie_breaker_decides() {
    let arbiter = Keypair::new();
    let arbiter_pub = arbiter.pubkey();
    let mut f = deadlocked_fixture(move |p| {
        p.deadlock_rule = DeadlockRule::TieBreaker;
        p.tie_breaker = arbiter_pub;
    });
    let rix = resolve_ix(&f);
    f.svm.airdrop(&arbiter.pubkey(), SOL).unwrap();

    // No timeout path exists for this rule.
    warp(&mut f.svm, 1_000_000);
    assert_err(
        send(&mut f.svm, &[rix.clone()], &f.alice.pubkey(), &[&f.alice]),
        "ResolutionConditionsNotMet",
    );
    // A party cannot decide alone.
    let ix = ix_resolution_sign(&f.bob.pubkey(), &f.deal, 1000);
    send(&mut f.svm, &[ix], &f.bob.pubkey(), &[&f.bob]).unwrap();
    assert_err(
        send(&mut f.svm, &[rix.clone()], &f.bob.pubkey(), &[&f.bob]),
        "ResolutionConditionsNotMet",
    );
    // The tie-breaker's decision resolves: 40% to payee.
    let ix = ix_resolution_sign(&arbiter.pubkey(), &f.deal, 400);
    send(&mut f.svm, &[ix], &arbiter.pubkey(), &[&arbiter]).unwrap();
    let alice_before = token_balance(&f.svm, &f.alice_ata);
    send(&mut f.svm, &[rix.clone()], &f.alice.pubkey(), &[&f.alice]).unwrap();
    assert_eq!(token_balance(&f.svm, &f.bob_ata), 400);
    assert_eq!(token_balance(&f.svm, &f.alice_ata), alice_before + 600);
}

#[test]
fn long_sunset_refunds_after_long_wait_but_mutual_resolves_earlier() {
    let year: i64 = 365 * 24 * 3600;
    let mut f = deadlocked_fixture(move |p| {
        p.deadlock_rule = DeadlockRule::LongSunset;
        p.timeout_secs = year;
    });
    let rix = resolve_ix(&f);
    // Mutual agreement works before the sunset: both sign the same payout.
    for (kp, amt) in [(f.alice.insecure_clone(), 500u64), (f.bob.insecure_clone(), 500u64)] {
        let ix = ix_resolution_sign(&kp.pubkey(), &f.deal, amt);
        send(&mut f.svm, &[ix], &kp.pubkey(), &[&kp]).unwrap();
    }
    let alice_before = token_balance(&f.svm, &f.alice_ata);
    send(&mut f.svm, &[rix.clone()], &f.alice.pubkey(), &[&f.alice]).unwrap();
    assert_eq!(token_balance(&f.svm, &f.bob_ata), 500);
    assert_eq!(token_balance(&f.svm, &f.alice_ata), alice_before + 500);

    // And the sunset path alone refunds the payer.
    let mut f = deadlocked_fixture(move |p| {
        p.deadlock_rule = DeadlockRule::LongSunset;
        p.timeout_secs = year;
    });
    let rix = resolve_ix(&f);
    assert_err(send(&mut f.svm, &[rix.clone()], &f.alice.pubkey(), &[&f.alice]), "TimeoutNotElapsed");
    warp(&mut f.svm, year + 1);
    let alice_before = token_balance(&f.svm, &f.alice_ata);
    send(&mut f.svm, &[rix.clone()], &f.alice.pubkey(), &[&f.alice]).unwrap();
    assert_eq!(token_balance(&f.svm, &f.alice_ata), alice_before + 1000);
}

#[test]
fn true_deadlock_only_mutual() {
    let mut f = deadlocked_fixture(|p| p.deadlock_rule = DeadlockRule::TrueDeadlock);
    let rix = resolve_ix(&f);
    warp(&mut f.svm, 10 * 365 * 24 * 3600);
    assert_err(
        send(&mut f.svm, &[rix.clone()], &f.alice.pubkey(), &[&f.alice]),
        "ResolutionConditionsNotMet",
    );
    for kp in [f.alice.insecure_clone(), f.bob.insecure_clone()] {
        let ix = ix_resolution_sign(&kp.pubkey(), &f.deal, 0);
        send(&mut f.svm, &[ix], &kp.pubkey(), &[&kp]).unwrap();
    }
    let alice_before = token_balance(&f.svm, &f.alice_ata);
    send(&mut f.svm, &[rix.clone()], &f.bob.pubkey(), &[&f.bob]).unwrap();
    assert_eq!(token_balance(&f.svm, &f.alice_ata), alice_before + 1000);
}

#[test]
fn changing_proposal_resets_approvals() {
    let mut f = deadlocked_fixture(|_| {});
    let rix = resolve_ix(&f);
    // Alice proposes 900-to-payee; Bob counter-proposes 100 — Alice's approval must not carry over.
    let ix = ix_resolution_sign(&f.alice.pubkey(), &f.deal, 900);
    send(&mut f.svm, &[ix], &f.alice.pubkey(), &[&f.alice]).unwrap();
    let ix = ix_resolution_sign(&f.bob.pubkey(), &f.deal, 100);
    send(&mut f.svm, &[ix], &f.bob.pubkey(), &[&f.bob]).unwrap();
    let d = f.deal_state();
    assert_eq!(d.proposed_to_payee, 100);
    assert_eq!(d.resolution_approvals, 0b10);
    assert_err(
        send(&mut f.svm, &[rix.clone()], &f.alice.pubkey(), &[&f.alice]),
        "TimeoutNotElapsed",
    );
    // Alice joins Bob's proposal → resolves.
    let ix = ix_resolution_sign(&f.alice.pubkey(), &f.deal, 100);
    send(&mut f.svm, &[ix], &f.alice.pubkey(), &[&f.alice]).unwrap();
    send(&mut f.svm, &[rix.clone()], &f.alice.pubkey(), &[&f.alice]).unwrap();
    assert_eq!(token_balance(&f.svm, &f.bob_ata), 100);
}
