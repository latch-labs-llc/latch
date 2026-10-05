//! Randomized state-machine test. Each run builds a random deal configuration
//! and plays random instructions from four actors — payer, payee, a third
//! party (tie-breaker / recovery signer) and a stranger — with randomly chosen
//! (often wrong) accounts and random clock warps. After every step the
//! invariants below are checked against an independent model.
//!
//! LATCH_FUZZ_RUNS=<n> raises the run count for longer local sessions.

mod common;

use anchor_lang::prelude::{Clock, Pubkey};
use common::*;
use latch::state::{DeadlockRule, DealState, DisputePolicy, TimerMode};
use latch::ACTIVATION_WINDOW_SECS;
use solana_keypair::Keypair;
use solana_signer::Signer;

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
    fn chance(&mut self, pct: u64) -> bool {
        self.below(100) < pct
    }
}

const ALICE: usize = 0; // payer
const BOB: usize = 1; // payee
const CAROL: usize = 2; // non-party: tie-breaker / recovery signer
const MALLORY: usize = 3; // stranger

#[derive(Clone, Copy, PartialEq, Debug)]
enum Recovery {
    PartiesBoth, // [alice, bob] 2-of-2
    WithCarol,   // [alice, bob, carol] 2-of-3
    CarolOnly,   // [carol] 1-of-1
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Act {
    Sign,
    Deposit,
    Ready,
    Approve,
    Release,
    Raise,
    Withdraw,
    ResSign,
    Resolve,
    RecSign,
    RecExec,
    CancelDraft,
    CancelSign,
    RefundUnactivated,
    RefundByPayee,
    SetPolicy,
}

const ACTS: [Act; 16] = [
    Act::Sign,
    Act::Deposit,
    Act::Ready,
    Act::Approve,
    Act::Release,
    Act::Raise,
    Act::Withdraw,
    Act::ResSign,
    Act::Resolve,
    Act::RecSign,
    Act::RecExec,
    Act::CancelDraft,
    Act::CancelSign,
    Act::RefundUnactivated,
    Act::RefundByPayee,
    Act::SetPolicy,
];

fn terminal(s: DealState) -> bool {
    matches!(s, DealState::Completed | DealState::Cancelled)
}

fn now(svm: &litesvm::LiteSVM) -> i64 {
    svm.get_sysvar::<Clock>().unix_timestamp
}

#[test]
fn random_sequences_preserve_invariants() {
    let runs: u64 = std::env::var("LATCH_FUZZ_RUNS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(120);
    let mut cov = [0u64; 16];
    let mut ends = std::collections::BTreeMap::<String, u64>::new();
    let mut successes = 0u64;
    for seed in 1..=runs {
        successes += run(
            seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1,
            &mut cov,
            &mut ends,
        );
    }
    println!("fuzz: {runs} runs, {successes} successful txs");
    for (i, a) in ACTS.iter().enumerate() {
        println!("  {:<18} {}", format!("{a:?}"), cov[i]);
    }
    println!("  end states: {ends:?}");
    // Guard against a harness that silently stops exercising paths.
    assert!(successes > runs * 8, "too few successful txs: {successes}");
    for (i, a) in ACTS.iter().enumerate() {
        assert!(cov[i] > 0, "{a:?} never succeeded — coverage gap");
    }
}

fn run(seed: u64, cov: &mut [u64; 16], ends: &mut std::collections::BTreeMap<String, u64>) -> u64 {
    let mut rng = Rng(seed);
    let carol = Keypair::new();
    let n_ms = 1 + rng.below(3) as usize;
    let milestones: Vec<u64> = (0..n_ms).map(|_| 100 + rng.below(900)).collect();
    let rule = [
        DeadlockRule::TimeoutRelease,
        DeadlockRule::TimeoutRefund,
        DeadlockRule::AutoSplit,
        DeadlockRule::TieBreaker,
        DeadlockRule::LongSunset,
        DeadlockRule::TrueDeadlock,
    ][rng.below(6) as usize];
    let timer = if matches!(rule, DeadlockRule::TieBreaker | DeadlockRule::TrueDeadlock)
        || rng.chance(50)
    {
        TimerMode::FromDeadlock
    } else {
        TimerMode::FromActivation
    };
    let timeout = 50 + rng.below(450) as i64;
    let split = rng.below(10_001) as u16;
    let threshold = 1 + rng.below(2) as u8;
    let recovery = [
        Recovery::PartiesBoth,
        Recovery::WithCarol,
        Recovery::CarolOnly,
    ][rng.below(3) as usize];
    let rec_delay = rng.below(300) as i64;
    let carol_pk = carol.pubkey();

    let mut f = Fixture::new(milestones.clone(), |p| {
        p.deadlock_rule = rule;
        p.timer_mode = timer;
        p.timeout_secs = if matches!(rule, DeadlockRule::TieBreaker | DeadlockRule::TrueDeadlock) {
            0
        } else {
            timeout
        };
        p.split_bps = split;
        if rule == DeadlockRule::TieBreaker {
            p.tie_breaker = carol_pk;
        }
        p.approval_threshold = threshold;
        let (signers, th) = match recovery {
            Recovery::PartiesBoth => (vec![p.parties[0], p.parties[1]], 2),
            Recovery::WithCarol => (vec![p.parties[0], p.parties[1], carol_pk], 2),
            Recovery::CarolOnly => (vec![carol_pk], 1),
        };
        p.recovery_signers = signers;
        p.recovery_threshold = th;
        p.recovery_delay_secs = rec_delay;
    });

    let mallory = Keypair::new();
    f.svm.airdrop(&carol.pubkey(), 10 * SOL).unwrap();
    f.svm.airdrop(&mallory.pubkey(), 10 * SOL).unwrap();
    let carol_ata = create_token_account(
        &mut f.svm,
        &f.alice,
        &carol.pubkey(),
        &f.mint,
        &f.token_program,
    );
    let mallory_ata = create_token_account(
        &mut f.svm,
        &f.alice,
        &mallory.pubkey(),
        &f.mint,
        &f.token_program,
    );
    let keys = [
        f.alice.insecure_clone(),
        f.bob.insecure_clone(),
        carol,
        mallory,
    ];
    let atas = [f.alice_ata, f.bob_ata, carol_ata, mallory_ata];
    let vault = vault_ata(&f.deal, &f.mint, &f.token_program);
    let (deal, mint, tp) = (f.deal, f.mint, f.token_program);
    let total: u64 = milestones.iter().sum();

    let balances = |svm: &litesvm::LiteSVM| -> [u64; 5] {
        [
            token_balance(svm, &atas[0]),
            token_balance(svm, &atas[1]),
            token_balance(svm, &atas[2]),
            token_balance(svm, &atas[3]),
            token_balance(svm, &vault),
        ]
    };
    let supply: u64 = balances(&f.svm).iter().sum();

    // One party may "go dark" from a point on; single-party checks apply then.
    let dark: Option<usize> = match rng.below(3) {
        0 => None,
        1 => Some(ALICE),
        _ => Some(BOB),
    };
    let dark_from_active = rng.chance(50); // else from Funded
    let mut dark_now = false;

    // Independent model of the FromActivation clock.
    let mut banked: i64 = 0;
    let mut disputed: i64 = 0;
    let mut raised_at: i64 = 0;
    let mut resumed_at: i64 = 0;
    let mut terminal_snapshot: Option<([u64; 5], DealState)> = None;
    let mut ok_txs = 0u64;

    for _step in 0..90 {
        if rng.chance(20) {
            warp(&mut f.svm, rng.below(700) as i64);
        }
        if rng.chance(3) {
            // Occasionally jump past the activation window.
            warp(
                &mut f.svm,
                ACTIVATION_WINDOW_SECS - 100 + rng.below(300) as i64,
            );
        }
        if f.deal_state().state == DealState::Deadlocked && rng.chance(15) {
            // Let open disputes run into their windows.
            warp(&mut f.svm, 86_400 + rng.below(86_400) as i64);
        }
        let pre = f.deal_state();
        let pre_bal = balances(&f.svm);
        let t = now(&f.svm);

        if let Some(d) = dark {
            let point = if dark_from_active {
                matches!(pre.state, DealState::Active | DealState::Deadlocked)
                    || terminal(pre.state)
            } else {
                !matches!(pre.state, DealState::Draft | DealState::Signed)
            };
            if point {
                dark_now = true;
            }
            let _ = d;
        }

        // 70%: a move that fits the current state, by a plausible actor.
        // 30%: anything, by anyone — the adversarial half.
        let (act, mut who) = if rng.chance(70) {
            let party = if rng.chance(50) { ALICE } else { BOB };
            match pre.state {
                DealState::Draft => match rng.below(100) {
                    0..=44 if pre.signed == 0 => (Act::SetPolicy, ALICE),
                    0..=96 => (Act::Sign, party),
                    _ => (Act::CancelDraft, party),
                },
                DealState::Signed => match rng.below(10) {
                    0..=6 => (Act::Deposit, ALICE),
                    7..=8 => (Act::CancelSign, party),
                    _ => (
                        Act::RefundUnactivated,
                        [ALICE, BOB, MALLORY][rng.below(3) as usize],
                    ),
                },
                DealState::Funded => match rng.below(10) {
                    0..=6 => (Act::Ready, party),
                    7 => (Act::CancelSign, party),
                    8 => (
                        Act::RefundUnactivated,
                        [ALICE, BOB, MALLORY][rng.below(3) as usize],
                    ),
                    _ => (Act::RecSign, [ALICE, BOB, CAROL][rng.below(3) as usize]),
                },
                DealState::Active => match rng.below(10) {
                    0..=3 => (Act::Approve, party),
                    4..=5 => (Act::Release, [ALICE, BOB, MALLORY][rng.below(3) as usize]),
                    6 => (Act::Raise, party),
                    7 => (Act::Resolve, MALLORY),
                    8 => (Act::RecSign, [ALICE, BOB, CAROL][rng.below(3) as usize]),
                    _ => {
                        if rng.chance(50) {
                            (Act::RecExec, MALLORY)
                        } else {
                            (
                                Act::RefundByPayee,
                                [BOB, BOB, ALICE, MALLORY][rng.below(4) as usize],
                            )
                        }
                    }
                },
                DealState::Deadlocked => match rng.below(10) {
                    0..=3 => (Act::ResSign, [ALICE, BOB, CAROL][rng.below(3) as usize]),
                    4 => (Act::Withdraw, [ALICE, BOB, MALLORY][rng.below(3) as usize]),
                    5..=6 => (Act::Resolve, MALLORY),
                    7 => (Act::RecSign, [ALICE, BOB, CAROL][rng.below(3) as usize]),
                    _ => (Act::RecExec, MALLORY),
                },
                _ => (ACTS[rng.below(ACTS.len() as u64) as usize], party),
            }
        } else {
            (
                ACTS[rng.below(ACTS.len() as u64) as usize],
                [ALICE, ALICE, ALICE, BOB, BOB, BOB, CAROL, MALLORY, MALLORY]
                    [rng.below(9) as usize],
            )
        };
        if dark_now && Some(who) == dark {
            who = if who == ALICE { BOB } else { ALICE };
            if Some(who) == dark {
                continue;
            }
        }
        let signer = &keys[who];
        let any_ata = |r: &mut Rng| atas[r.below(4) as usize];
        // Mostly-correct accounts, sometimes adversarial.
        let payer_acct = if rng.chance(75) {
            atas[ALICE]
        } else {
            any_ata(&mut rng)
        };
        let payee_acct = if rng.chance(75) {
            atas[BOB]
        } else {
            any_ata(&mut rng)
        };
        let remaining = pre_bal[4];
        // Usually target the next unreleased milestone (releases are in order).
        let next_ms = (0..pre.num_milestones)
            .find(|&i| !pre.milestones[i as usize].released)
            .unwrap_or(0);
        let ms_index = |r: &mut Rng| {
            if r.chance(80) {
                next_ms
            } else {
                r.below(4) as u8
            }
        };
        // Converge on matching proposals often, so mutual paths get exercised.
        let pending = pre.proposed_to_payee;
        let pending_rec = pre.recovery_proposed_to_payee;
        let amount_choice = |r: &mut Rng, current: u64| match r.below(7) {
            0 => 0,
            1 => remaining / 2,
            2 => remaining,
            3 => remaining.saturating_add(1),
            4 | 5 => current,
            _ => r.below(total + 1),
        };

        let ix = match act {
            Act::Sign => ix_sign_terms(&signer.pubkey(), &deal, rng.chance(80)),
            Act::Deposit => {
                let left = total.saturating_sub(pre.deposited);
                let amt = match rng.below(4) {
                    0 => left,
                    1 => (left / 2).max(1),
                    2 => 1,
                    _ => left.saturating_add(1),
                };
                ix_deposit(&signer.pubkey(), &deal, &mint, &atas[who], &tp, amt)
            }
            Act::Ready => ix_confirm_ready(&signer.pubkey(), &deal),
            Act::Approve => ix_approve_milestone(&signer.pubkey(), &deal, ms_index(&mut rng)),
            Act::Release => {
                ix_release_milestone(&deal, &mint, &payee_acct, &tp, ms_index(&mut rng))
            }
            Act::Raise => ix_raise_deadlock(&signer.pubkey(), &deal),
            Act::Withdraw => ix_withdraw_deadlock(&signer.pubkey(), &deal),
            Act::ResSign => {
                ix_resolution_sign(&signer.pubkey(), &deal, amount_choice(&mut rng, pending))
            }
            Act::Resolve => ix_resolve(&deal, &mint, &payer_acct, &payee_acct, &tp),
            Act::RecSign => ix_recovery_sign(
                &signer.pubkey(),
                &deal,
                amount_choice(&mut rng, pending_rec),
            ),
            Act::RecExec => ix_recovery_execute(&deal, &mint, &payer_acct, &payee_acct, &tp),
            Act::CancelDraft => ix_cancel_draft(&signer.pubkey(), &deal),
            Act::CancelSign => ix_cancel_sign(&signer.pubkey(), &deal, &mint, &payer_acct, &tp),
            Act::RefundUnactivated => {
                ix_refund_unactivated(&signer.pubkey(), &deal, &mint, &payer_acct, &tp)
            }
            Act::RefundByPayee => {
                ix_refund_by_payee(&signer.pubkey(), &deal, &mint, &payer_acct, &tp)
            }
            Act::SetPolicy => {
                let policy = [
                    DisputePolicy::NeverExpire,
                    DisputePolicy::ResumeRule,
                    DisputePolicy::Split,
                    DisputePolicy::RefundPayer,
                ][rng.below(4) as usize];
                let window = match rng.below(10) {
                    0 => 0, // invalid
                    1..=6 => 86_400 * (1 + rng.below(2) as u32),
                    _ => 86_400 + rng.below(365 * 86_400) as u32,
                };
                ix_set_dispute_policy(&signer.pubkey(), &deal, policy, window)
            }
        };
        let res = send(&mut f.svm, &[ix], &signer.pubkey(), &[signer]);
        let post = f.deal_state();
        let post_bal = balances(&f.svm);
        let ctx = format!(
            "seed={seed} act={act:?} who={who} rule={rule:?} timer={timer:?} th={threshold} rec={recovery:?} pre={:?} post={:?}",
            pre.state, post.state
        );

        // I1: the program never creates or destroys tokens.
        assert_eq!(post_bal.iter().sum::<u64>(), supply, "conservation: {ctx}");
        // I2: non-parties are never paid.
        assert!(post_bal[CAROL] <= pre_bal[CAROL], "carol paid: {ctx}");
        assert!(
            post_bal[MALLORY] <= pre_bal[MALLORY] || (act == Act::Deposit && who == MALLORY),
            "stranger paid: {ctx}"
        );
        assert!(
            post_bal[MALLORY] <= pre_bal[MALLORY],
            "stranger balance grew: {ctx}"
        );
        // I3: terminal states are final — no state or balance change afterwards.
        if let Some((snap, st)) = terminal_snapshot {
            assert_eq!(post.state, st, "terminal state changed: {ctx}");
            assert_eq!(post_bal, snap, "balances moved after terminal: {ctx}");
        } else if terminal(post.state) {
            terminal_snapshot = Some((post_bal, post.state));
        }
        // I4: accounting. Before a terminal state, the vault holds exactly
        // what was deposited minus what was released.
        assert!(
            post.deposited <= total && post.released_total <= total,
            "over-accounting: {ctx}"
        );
        if !terminal(post.state) {
            assert_eq!(
                post_bal[4],
                post.deposited - post.released_total,
                "vault != deposited - released: {ctx}"
            );
        }

        if res.is_err() {
            assert_eq!(
                post.event_seq, pre.event_seq,
                "failed tx changed state: {ctx}"
            );
            continue;
        }
        ok_txs += 1;
        cov[ACTS.iter().position(|a| *a == act).unwrap()] += 1;
        if terminal(post.state) && !terminal(pre.state) {
            *ends
                .entry(format!("{:?} via {act:?}", post.state))
                .or_default() += 1;
        }
        // I5: every successful instruction emits exactly one event.
        assert!(
            post.event_seq > pre.event_seq,
            "event seq did not advance: {ctx}"
        );

        // I6: a dark party's counterparty can gain only through paths that
        // don't need the dark party: consented single-approval releases,
        // time-based rules, the tie-breaker, a recovery set without them, or
        // the activation-lapse refund (payer only).
        if dark_now {
            let gain_bob = post_bal[BOB] > pre_bal[BOB];
            let gain_alice = post_bal[ALICE] > pre_bal[ALICE] && !(act == Act::Deposit);
            let rec_without = |p: usize| match recovery {
                Recovery::PartiesBoth => false,
                Recovery::WithCarol | Recovery::CarolOnly => {
                    let _ = p;
                    true
                }
            };
            let tb = rule == DeadlockRule::TieBreaker && pre.tie_breaker_decided;
            let expiry_payout = act == Act::Resolve
                && pre.state == DealState::Deadlocked
                && timer == TimerMode::FromActivation;
            if dark == Some(ALICE) && gain_bob {
                let ok = (act == Act::Release && threshold == 1)
                    || (act == Act::Resolve
                        && (tb
                            || matches!(
                                rule,
                                DeadlockRule::TimeoutRelease | DeadlockRule::AutoSplit
                            )))
                    || (expiry_payout && pre.dispute_policy == DisputePolicy::Split)
                    || (act == Act::RecExec && rec_without(ALICE));
                assert!(ok, "payee gained without the dark payer: {ctx}");
            }
            if dark == Some(BOB) && gain_alice {
                let ok = (act == Act::Resolve
                    && (tb
                        || matches!(
                            rule,
                            DeadlockRule::TimeoutRefund
                                | DeadlockRule::LongSunset
                                | DeadlockRule::AutoSplit
                        )))
                    || (expiry_payout
                        && matches!(
                            pre.dispute_policy,
                            DisputePolicy::Split | DisputePolicy::RefundPayer
                        ))
                    || (act == Act::RecExec && rec_without(BOB))
                    || act == Act::RefundUnactivated;
                assert!(ok, "payer gained without the dark payee: {ctx}");
            }
        }

        // I7: the lapse refund fires only for a deal holding a deposit that
        // never activated, only after the window, and only to the payer.
        if act == Act::RefundUnactivated {
            let start = match pre.state {
                DealState::Funded => pre.funded_at,
                DealState::Signed if pre.deposited > 0 => pre.signed_at,
                s => panic!("lapse refund from {s:?}: {ctx}"),
            };
            assert!(
                t >= start + ACTIVATION_WINDOW_SECS,
                "lapse refund too early: {ctx}"
            );
            assert_eq!(
                post_bal[ALICE],
                pre_bal[ALICE] + pre_bal[4],
                "lapse refund not to payer: {ctx}"
            );
        }

        // I9: a payee refund is signed by the payee and pays only the payer.
        if act == Act::RefundByPayee {
            assert_eq!(who, BOB, "refund_by_payee succeeded for a non-payee: {ctx}");
            assert_eq!(
                post_bal[ALICE],
                pre_bal[ALICE] + pre_bal[4],
                "payee refund not to payer: {ctx}"
            );
            assert_eq!(
                post_bal[BOB], pre_bal[BOB],
                "payee refund paid the payee: {ctx}"
            );
        }

        // I8: the FromActivation clock matches an independent model.
        if timer == TimerMode::FromActivation {
            let tn = now(&f.svm);
            match (pre.state, post.state) {
                (DealState::Funded, DealState::Active) => {
                    banked = 0;
                    resumed_at = tn;
                }
                (DealState::Active, DealState::Deadlocked) => banked += tn - resumed_at,
                (DealState::Deadlocked, DealState::Active) => resumed_at = tn,
                _ => {}
            }
            if matches!(post.state, DealState::Active | DealState::Deadlocked) {
                assert_eq!(post.elapsed_at_pause, banked, "banked clock drift: {ctx}");
            }
            if post.state == DealState::Active {
                assert_eq!(post.last_resumed_at, resumed_at, "resume time drift: {ctx}");
            }
            if act == Act::Resolve && pre.state == DealState::Active {
                let elapsed = banked + (tn - resumed_at);
                assert!(
                    elapsed >= pre.timeout_secs,
                    "resolved before the active clock ran out: {ctx}"
                );
            }

            // I10: dispute expiry follows the policy both parties signed.
            match (pre.state, post.state) {
                (DealState::Active, DealState::Deadlocked) => raised_at = tn,
                (DealState::Deadlocked, DealState::Active) => disputed += tn - raised_at,
                _ => {}
            }
            if !terminal(post.state) {
                assert_eq!(
                    post.disputed_secs as i64, disputed,
                    "dispute time drift: {ctx}"
                );
            }
            let open = if pre.state == DealState::Deadlocked {
                tn - raised_at
            } else {
                0
            };
            let expired = pre.dispute_policy != DisputePolicy::NeverExpire
                && disputed + open >= pre.dispute_window_secs as i64;
            if act == Act::Withdraw
                && pre.state == DealState::Deadlocked
                && pre.deadlock_raised_by != keys[who].pubkey()
            {
                assert!(
                    expired && pre.dispute_policy == DisputePolicy::ResumeRule,
                    "non-raiser lifted a dispute that hadn't expired under ResumeRule: {ctx}"
                );
                *ends
                    .entry("expiry: dispute lifted by non-raiser".into())
                    .or_default() += 1;
            }
            let mutual = pre.proposal_active && pre.resolution_approvals == 0b11;
            if act == Act::Resolve && pre.state == DealState::Deadlocked && !mutual {
                assert!(
                    expired,
                    "dispute resolved by time before expiry / under NeverExpire: {ctx}"
                );
                *ends
                    .entry(format!("expiry: payout under {:?}", pre.dispute_policy))
                    .or_default() += 1;
                match pre.dispute_policy {
                    DisputePolicy::Split => {
                        let bps = if rule == DeadlockRule::AutoSplit {
                            pre.split_bps as u64
                        } else {
                            5_000
                        };
                        assert_eq!(
                            post_bal[BOB] - pre_bal[BOB],
                            pre_bal[4] * bps / 10_000,
                            "split payout: {ctx}"
                        );
                    }
                    DisputePolicy::RefundPayer => {
                        assert_eq!(
                            post_bal[ALICE] - pre_bal[ALICE],
                            pre_bal[4],
                            "refund payout: {ctx}"
                        );
                    }
                    p => panic!("expiry payout under {p:?}: {ctx}"),
                }
            }
        }
    }
    let _ = Pubkey::default();
    ok_txs
}
