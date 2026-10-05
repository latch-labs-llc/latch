mod common;

use common::*;
use latch::state::{DeadlockRule, DealState, DisputePolicy, TimerMode};
use solana_signer::Signer;

const DAY: i64 = 86_400;

/// A 1,000-token FromActivation deal (7-day clock) with the given rule and,
/// optionally, a non-default dispute policy — Active, with Alice's dispute open.
fn disputed(rule: DeadlockRule, policy: Option<(DisputePolicy, u32)>, split_bps: u16) -> Fixture {
    let mut f = Fixture::new(vec![1000], |p| {
        p.deadlock_rule = rule;
        p.timer_mode = TimerMode::FromActivation;
        p.timeout_secs = 7 * DAY;
        p.split_bps = split_bps;
    });
    if let Some((pol, window)) = policy {
        let ix = ix_set_dispute_policy(&f.alice.pubkey(), &f.deal, pol, window);
        send(&mut f.svm, &[ix], &f.alice.pubkey(), &[&f.alice]).unwrap();
    }
    f.to_active();
    warp(&mut f.svm, DAY);
    let ix = ix_raise_deadlock(&f.alice.pubkey(), &f.deal);
    send(&mut f.svm, &[ix], &f.alice.pubkey(), &[&f.alice]).unwrap();
    f
}

fn stranger(f: &mut Fixture) -> solana_keypair::Keypair {
    let kp = solana_keypair::Keypair::new();
    f.svm.airdrop(&kp.pubkey(), SOL).unwrap();
    kp
}

fn resolve(f: &mut Fixture) -> litesvm::types::TransactionResult {
    let ix = ix_resolve(&f.deal, &f.mint, &f.alice_ata, &f.bob_ata, &f.token_program);
    let bob = f.bob.insecure_clone();
    send(&mut f.svm, &[ix], &bob.pubkey(), &[&bob])
}

fn withdraw_as(f: &mut Fixture, kp: &solana_keypair::Keypair) -> litesvm::types::TransactionResult {
    let ix = ix_withdraw_deadlock(&kp.pubkey(), &f.deal);
    send(&mut f.svm, &[ix], &kp.pubkey(), &[kp])
}

#[test]
fn by_default_a_dispute_holds_until_the_parties_agree() {
    let mut f = disputed(DeadlockRule::TimeoutRelease, None, 0);
    assert_eq!(f.deal_state().dispute_policy, DisputePolicy::NeverExpire);
    warp(&mut f.svm, 400 * DAY);
    let mallory = stranger(&mut f);
    assert_err(withdraw_as(&mut f, &mallory), "OnlyRaiserMayWithdraw");
    assert_err(resolve(&mut f), "TimeoutPausedByDispute");
    // Settlement still works at any time.
    for kp in [f.alice.insecure_clone(), f.bob.insecure_clone()] {
        let ix = ix_resolution_sign(&kp.pubkey(), &f.deal, 400);
        send(&mut f.svm, &[ix], &kp.pubkey(), &[&kp]).unwrap();
    }
    resolve(&mut f).unwrap();
    assert_eq!(f.deal_state().state, DealState::Completed);
    assert_eq!(token_balance(&f.svm, &f.bob_ata), 400);
}

#[test]
fn resume_rule_lets_anyone_lift_an_expired_dispute_and_the_signed_rule_applies() {
    let mut f = disputed(
        DeadlockRule::TimeoutRelease,
        Some((DisputePolicy::ResumeRule, DAY as u32)),
        0,
    );
    let mallory = stranger(&mut f);
    warp(&mut f.svm, DAY - 1);
    assert_err(withdraw_as(&mut f, &mallory), "OnlyRaiserMayWithdraw");
    warp(&mut f.svm, 1);
    withdraw_as(&mut f, &mallory).unwrap();
    let d = f.deal_state();
    assert_eq!(d.state, DealState::Active);
    assert_eq!(d.elapsed_at_pause, DAY); // one Active day banked before the dispute
    assert_eq!(d.disputed_secs, DAY as u32);
    warp(&mut f.svm, 6 * DAY - 1);
    assert_err(resolve(&mut f), "TimeoutNotElapsed");
    warp(&mut f.svm, 1);
    resolve(&mut f).unwrap();
    assert_eq!(token_balance(&f.svm, &f.bob_ata), 1000); // TimeoutRelease: payee
}

#[test]
fn resume_rule_expiry_goes_through_withdraw_not_resolve() {
    let mut f = disputed(
        DeadlockRule::TimeoutRelease,
        Some((DisputePolicy::ResumeRule, DAY as u32)),
        0,
    );
    warp(&mut f.svm, DAY);
    assert_err(resolve(&mut f), "ExpiredDisputeMustBeWithdrawn");
}

#[test]
fn repeated_disputes_share_one_window() {
    let mut f = disputed(
        DeadlockRule::TimeoutRelease,
        Some((DisputePolicy::ResumeRule, 2 * DAY as u32)),
        0,
    );
    let alice = f.alice.insecure_clone();
    warp(&mut f.svm, DAY);
    withdraw_as(&mut f, &alice).unwrap(); // raiser lifts it after one day
    let ix = ix_raise_deadlock(&alice.pubkey(), &f.deal);
    send(&mut f.svm, &[ix], &alice.pubkey(), &[&alice]).unwrap(); // and raises again
    let mallory = stranger(&mut f);
    warp(&mut f.svm, DAY - 1);
    assert_err(withdraw_as(&mut f, &mallory), "OnlyRaiserMayWithdraw");
    warp(&mut f.svm, 1);
    withdraw_as(&mut f, &mallory).unwrap(); // two days of disputes in total
    assert_eq!(f.deal_state().disputed_secs, 2 * DAY as u32);
}

#[test]
fn split_policy_halves_the_vault() {
    let mut f = disputed(
        DeadlockRule::TimeoutRelease,
        Some((DisputePolicy::Split, DAY as u32)),
        0,
    );
    let alice_before = token_balance(&f.svm, &f.alice_ata);
    assert_err(resolve(&mut f), "TimeoutPausedByDispute");
    warp(&mut f.svm, DAY);
    resolve(&mut f).unwrap();
    assert_eq!(token_balance(&f.svm, &f.bob_ata), 500);
    assert_eq!(token_balance(&f.svm, &f.alice_ata), alice_before + 500);
}

#[test]
fn split_policy_uses_the_deals_own_split_under_autosplit() {
    let mut f = disputed(
        DeadlockRule::AutoSplit,
        Some((DisputePolicy::Split, DAY as u32)),
        3000,
    );
    warp(&mut f.svm, DAY);
    resolve(&mut f).unwrap();
    assert_eq!(token_balance(&f.svm, &f.bob_ata), 300);
}

#[test]
fn refund_policy_returns_the_vault_to_the_payer() {
    let mut f = disputed(
        DeadlockRule::TimeoutRelease,
        Some((DisputePolicy::RefundPayer, DAY as u32)),
        0,
    );
    let alice_before = token_balance(&f.svm, &f.alice_ata);
    warp(&mut f.svm, DAY);
    resolve(&mut f).unwrap();
    assert_eq!(token_balance(&f.svm, &f.alice_ata), alice_before + 1000);
    assert_eq!(token_balance(&f.svm, &f.bob_ata), 0);
}

#[test]
fn only_the_creator_sets_the_policy_and_only_before_anyone_signs() {
    let activation = |p: &mut latch::CreateDealParams| {
        p.deadlock_rule = DeadlockRule::TimeoutRelease;
        p.timer_mode = TimerMode::FromActivation;
        p.timeout_secs = 7 * DAY;
    };
    let mut f = Fixture::new(vec![1000], activation);
    let set = |f: &mut Fixture, who: &solana_keypair::Keypair, window: u32| {
        let ix = ix_set_dispute_policy(&who.pubkey(), &f.deal, DisputePolicy::Split, window);
        send(&mut f.svm, &[ix], &who.pubkey(), &[who])
    };
    let (alice, bob) = (f.alice.insecure_clone(), f.bob.insecure_clone());
    assert_err(set(&mut f, &bob, DAY as u32), "OnlyCreatorMaySetPolicy");
    for bad in [0u32, (DAY - 1) as u32, 31_536_001] {
        assert_err(set(&mut f, &alice, bad), "InvalidDisputeWindow");
    }
    set(&mut f, &alice, 365 * DAY as u32).unwrap();
    set(&mut f, &alice, DAY as u32).unwrap(); // may still change before signing
    let ix = ix_sign_terms(&bob.pubkey(), &f.deal, true);
    send(&mut f.svm, &[ix], &bob.pubkey(), &[&bob]).unwrap();
    warp(&mut f.svm, 1);
    assert_err(set(&mut f, &alice, 2 * DAY as u32), "DisputePolicyLocked");
    assert_eq!(f.deal_state().dispute_window_secs, DAY as u32);

    // FromDeadlock deals already have a dispute clock; the policy doesn't apply.
    let mut g = Fixture::new(vec![1000], |_| {});
    let ga = g.alice.insecure_clone();
    assert_err(set(&mut g, &ga, DAY as u32), "InvalidTimerMode");
}
