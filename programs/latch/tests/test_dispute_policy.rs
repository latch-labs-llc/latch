mod common;

use common::*;
use latch::state::{DeadlockRule, DealState, DisputePolicy, TimerMode};
use solana_signer::Signer;

const DAY: i64 = 86_400;

/// A 1,000-token FromActivation deal (7-day clock) with the given rule and,
/// optionally, a non-default dispute policy — Active, with Alice's dispute open.
fn disputed(rule: DeadlockRule, policy: Option<(DisputePolicy, u32)>, split_bps: u16) -> Fixture {
    let mut f = Fixture::new_with_policy(
        vec![1000],
        |p| {
            p.deadlock_rule = rule;
            p.timer_mode = TimerMode::FromActivation;
            p.timeout_secs = 7 * DAY;
            p.split_bps = split_bps;
        },
        policy,
    );
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

fn activation(p: &mut latch::CreateDealParams) {
    p.deadlock_rule = DeadlockRule::TimeoutRelease;
    p.timer_mode = TimerMode::FromActivation;
    p.timeout_secs = 7 * DAY;
}

/// create_deal + set_dispute_policy as one transaction signed by `setter`
/// (plus Alice, the creator). Returns the result and the deal address.
fn create_with_policy(
    f: &mut Fixture,
    deal_id: u64,
    setter: &solana_keypair::Keypair,
    window: u32,
) -> (
    litesvm::types::TransactionResult,
    anchor_lang::prelude::Pubkey,
) {
    let mut params = default_params(&f.alice.pubkey(), &f.bob.pubkey(), vec![1000]);
    params.deal_id = deal_id;
    activation(&mut params);
    let deal = deal_pda(&f.alice.pubkey(), deal_id);
    let alice = f.alice.insecure_clone();
    let ixs = [
        ix_create_deal(&alice.pubkey(), &f.mint, &f.token_program, params),
        ix_set_dispute_policy(&setter.pubkey(), &deal, DisputePolicy::Split, window),
    ];
    (
        send(&mut f.svm, &ixs, &alice.pubkey(), &[&alice, setter]),
        deal,
    )
}

#[test]
fn the_policy_is_set_with_creation_by_the_creator_within_bounds() {
    let mut f = Fixture::new(vec![1000], activation);
    let (alice, bob) = (f.alice.insecure_clone(), f.bob.insecure_clone());
    let (res, _) = create_with_policy(&mut f, 10, &bob, DAY as u32);
    assert_err(res, "OnlyCreatorMaySetPolicy");
    for (id, bad) in [(11, 0u32), (12, (DAY - 1) as u32), (13, 31_536_001)] {
        let (res, _) = create_with_policy(&mut f, id, &alice, bad);
        assert_err(res, "InvalidDisputeWindow");
    }
    let (res, deal) = create_with_policy(&mut f, 14, &alice, 365 * DAY as u32);
    res.unwrap();
    let d = get_deal(&f.svm, &deal);
    assert_eq!(d.dispute_policy, DisputePolicy::Split);
    assert_eq!(d.dispute_window_secs, 365 * DAY as u32);
}

#[test]
fn the_policy_cannot_be_set_after_the_creating_transaction_even_in_the_same_second() {
    // The deal already exists (created in its own transaction). No clock warp:
    // this follow-up transaction lands in the same second as the creation.
    let mut f = Fixture::new(vec![1000], activation);
    let alice = f.alice.insecure_clone();
    assert_eq!(f.deal_state().created_at, {
        let c: anchor_lang::prelude::Clock = f.svm.get_sysvar();
        c.unix_timestamp
    });
    let ix = ix_set_dispute_policy(
        &alice.pubkey(),
        &f.deal,
        DisputePolicy::RefundPayer,
        DAY as u32,
    );
    assert_err(
        send(
            &mut f.svm,
            std::slice::from_ref(&ix),
            &alice.pubkey(),
            &[&alice],
        ),
        "PolicyMustBeSetAtCreation",
    );
    // ...nor later, and the default stands.
    warp(&mut f.svm, 60);
    assert_err(
        send(&mut f.svm, &[ix], &alice.pubkey(), &[&alice]),
        "PolicyMustBeSetAtCreation",
    );
    assert_eq!(f.deal_state().dispute_policy, DisputePolicy::NeverExpire);
}

#[test]
fn the_policy_cannot_be_set_twice_or_after_a_signature_in_the_creating_transaction() {
    let mut f = Fixture::new(vec![1000], activation);
    let alice = f.alice.insecure_clone();
    let mut params = default_params(&alice.pubkey(), &f.bob.pubkey(), vec![1000]);
    params.deal_id = 20;
    activation(&mut params);
    let deal = deal_pda(&alice.pubkey(), 20);
    let set = |p| ix_set_dispute_policy(&alice.pubkey(), &deal, p, DAY as u32);
    // A second set in the same transaction is refused (nothing may follow creation).
    let ixs = [
        ix_create_deal(&alice.pubkey(), &f.mint, &f.token_program, params.clone()),
        set(DisputePolicy::NeverExpire),
        set(DisputePolicy::RefundPayer),
    ];
    assert_err(
        send(&mut f.svm, &ixs, &alice.pubkey(), &[&alice]),
        "PolicyMustBeSetAtCreation",
    );
    // Signing first, then setting, is refused too.
    let ixs = [
        ix_create_deal(&alice.pubkey(), &f.mint, &f.token_program, params),
        ix_sign_terms(&alice.pubkey(), &deal, true),
        set(DisputePolicy::RefundPayer),
    ];
    assert_err(
        send(&mut f.svm, &ixs, &alice.pubkey(), &[&alice]),
        "DisputePolicyLocked",
    );
    // FromDeadlock deals already have a dispute clock; the policy doesn't apply.
    let mut g = Fixture::new(vec![1000], |_| {});
    let ga = g.alice.insecure_clone();
    let mut p2 = default_params(&ga.pubkey(), &g.bob.pubkey(), vec![1000]);
    p2.deal_id = 21;
    let d2 = deal_pda(&ga.pubkey(), 21);
    let ixs = [
        ix_create_deal(&ga.pubkey(), &g.mint, &g.token_program, p2),
        ix_set_dispute_policy(&ga.pubkey(), &d2, DisputePolicy::Split, DAY as u32),
    ];
    assert_err(
        send(&mut g.svm, &ixs, &ga.pubkey(), &[&ga]),
        "InvalidTimerMode",
    );
}
