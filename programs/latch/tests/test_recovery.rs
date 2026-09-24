mod common;

use common::*;
use solana_keypair::Keypair;
use solana_signer::Signer;
use latch::state::DealState;

#[test]
fn two_of_three_recovery() {
    let r1 = Keypair::new();
    let r2 = Keypair::new();
    let r3 = Keypair::new();
    let (p1, p2, p3) = (r1.pubkey(), r2.pubkey(), r3.pubkey());
    let mut f = Fixture::new(vec![1000], move |p| {
        p.recovery_signers = vec![p1, p2, p3];
        p.recovery_threshold = 2;
    });
    for kp in [&r1, &r2, &r3] {
        f.svm.airdrop(&kp.pubkey(), SOL).unwrap();
    }
    f.sign_all();
    f.fund();

    let exec = ix_recovery_execute(&f.deal, &f.mint, &f.alice_ata, &f.bob_ata, &f.token_program);

    // No proposal yet.
    assert_err(send(&mut f.svm, &[exec.clone()], &f.alice.pubkey(), &[&f.alice]), "NoActiveProposal");

    // One signature is below the threshold.
    let ix = ix_recovery_sign(&r1.pubkey(), &f.deal, 1000);
    send(&mut f.svm, &[ix], &r1.pubkey(), &[&r1]).unwrap();
    assert_err(
        send(&mut f.svm, &[exec.clone()], &f.alice.pubkey(), &[&f.alice]),
        "InsufficientRecoverySignatures",
    );

    // A second signer on a DIFFERENT amount resets the proposal.
    let ix = ix_recovery_sign(&r2.pubkey(), &f.deal, 600);
    send(&mut f.svm, &[ix], &r2.pubkey(), &[&r2]).unwrap();
    assert_err(
        send(&mut f.svm, &[exec.clone()], &f.alice.pubkey(), &[&f.alice]),
        "InsufficientRecoverySignatures",
    );

    // r3 matches r2's amount → threshold met; anyone executes; funds go only to parties.
    let ix = ix_recovery_sign(&r3.pubkey(), &f.deal, 600);
    send(&mut f.svm, &[ix], &r3.pubkey(), &[&r3]).unwrap();
    let alice_before = token_balance(&f.svm, &f.alice_ata);
    send(&mut f.svm, &[exec], &f.bob.pubkey(), &[&f.bob]).unwrap();
    let d = f.deal_state();
    assert_eq!(d.state, DealState::Completed);
    assert_eq!(token_balance(&f.svm, &f.bob_ata), 600);
    assert_eq!(token_balance(&f.svm, &f.alice_ata), alice_before + 400);
}

#[test]
fn recovery_gating() {
    let r1 = Keypair::new();
    let p1 = r1.pubkey();
    let mut f = Fixture::new(vec![1000], move |p| {
        p.recovery_signers = vec![p1];
        p.recovery_threshold = 1;
    });
    f.svm.airdrop(&r1.pubkey(), SOL).unwrap();

    // Not valid before funding.
    let ix = ix_recovery_sign(&r1.pubkey(), &f.deal, 0);
    assert_err(send(&mut f.svm, &[ix], &r1.pubkey(), &[&r1]), "InvalidState");
    f.sign_all();
    let ix = ix_recovery_sign(&r1.pubkey(), &f.deal, 0);
    assert_err(send(&mut f.svm, &[ix], &r1.pubkey(), &[&r1]), "InvalidState");

    f.fund();
    // A party is not a recovery signer.
    let ix = ix_recovery_sign(&f.alice.pubkey(), &f.deal, 0);
    assert_err(send(&mut f.svm, &[ix], &f.alice.pubkey(), &[&f.alice]), "NotARecoverySigner");

    // Payout above the vault balance cannot execute.
    let ix = ix_recovery_sign(&r1.pubkey(), &f.deal, 2000);
    send(&mut f.svm, &[ix], &r1.pubkey(), &[&r1]).unwrap();
    let exec = ix_recovery_execute(&f.deal, &f.mint, &f.alice_ata, &f.bob_ata, &f.token_program);
    assert_err(send(&mut f.svm, &[exec], &f.alice.pubkey(), &[&f.alice]), "PayoutExceedsVault");
}

#[test]
fn recovery_notice_delay() {
    let r1 = Keypair::new();
    let r2 = Keypair::new();
    let (p1, p2) = (r1.pubkey(), r2.pubkey());
    let mut f = Fixture::new(vec![1000], move |p| {
        p.recovery_signers = vec![p1, p2];
        p.recovery_threshold = 2;
        p.recovery_delay_secs = 72 * 3600; // 72h notice window
    });
    for kp in [&r1, &r2] {
        f.svm.airdrop(&kp.pubkey(), SOL).unwrap();
    }
    f.sign_all();
    f.fund();

    for kp in [&r1, &r2] {
        let ix = ix_recovery_sign(&kp.pubkey(), &f.deal, 1000);
        send(&mut f.svm, &[ix], &kp.pubkey(), &[kp]).unwrap();
    }
    let exec = ix_recovery_execute(&f.deal, &f.mint, &f.alice_ata, &f.bob_ata, &f.token_program);

    // Threshold met, but the notice window hasn't elapsed.
    assert_err(
        send(&mut f.svm, &[exec.clone()], &f.alice.pubkey(), &[&f.alice]),
        "RecoveryDelayNotElapsed",
    );
    warp(&mut f.svm, 72 * 3600 - 60);
    assert_err(
        send(&mut f.svm, &[exec.clone()], &f.alice.pubkey(), &[&f.alice]),
        "RecoveryDelayNotElapsed",
    );
    warp(&mut f.svm, 61);
    send(&mut f.svm, &[exec], &f.alice.pubkey(), &[&f.alice]).unwrap();
    assert_eq!(f.deal_state().state, DealState::Completed);
    assert_eq!(token_balance(&f.svm, &f.bob_ata), 1000);
}

#[test]
fn changing_recovery_proposal_restarts_notice_clock() {
    let r1 = Keypair::new();
    let r2 = Keypair::new();
    let (p1, p2) = (r1.pubkey(), r2.pubkey());
    let mut f = Fixture::new(vec![1000], move |p| {
        p.recovery_signers = vec![p1, p2];
        p.recovery_threshold = 2;
        p.recovery_delay_secs = 3600;
    });
    for kp in [&r1, &r2] {
        f.svm.airdrop(&kp.pubkey(), SOL).unwrap();
    }
    f.sign_all();
    f.fund();

    for kp in [&r1, &r2] {
        let ix = ix_recovery_sign(&kp.pubkey(), &f.deal, 1000);
        send(&mut f.svm, &[ix], &kp.pubkey(), &[kp]).unwrap();
    }
    // Wait out the original notice window, then change the proposal:
    // the clock must restart from zero for the new amount.
    warp(&mut f.svm, 3601);
    for kp in [&r1, &r2] {
        let ix = ix_recovery_sign(&kp.pubkey(), &f.deal, 500);
        send(&mut f.svm, &[ix], &kp.pubkey(), &[kp]).unwrap();
    }
    let exec = ix_recovery_execute(&f.deal, &f.mint, &f.alice_ata, &f.bob_ata, &f.token_program);
    assert_err(
        send(&mut f.svm, &[exec.clone()], &f.alice.pubkey(), &[&f.alice]),
        "RecoveryDelayNotElapsed",
    );
    warp(&mut f.svm, 3601);
    send(&mut f.svm, &[exec], &f.alice.pubkey(), &[&f.alice]).unwrap();
    assert_eq!(token_balance(&f.svm, &f.bob_ata), 500);
}

#[test]
fn recovery_works_from_deadlock() {
    let r1 = Keypair::new();
    let p1 = r1.pubkey();
    let mut f = Fixture::new(vec![1000], move |p| {
        p.recovery_signers = vec![p1];
        p.recovery_threshold = 1;
        p.deadlock_rule = latch::state::DeadlockRule::TrueDeadlock;
    });
    f.svm.airdrop(&r1.pubkey(), SOL).unwrap();
    f.to_active();
    let ix = ix_raise_deadlock(&f.alice.pubkey(), &f.deal);
    send(&mut f.svm, &[ix], &f.alice.pubkey(), &[&f.alice]).unwrap();

    // Even a true-deadlock contract has the recovery path (court order / key loss).
    let ix = ix_recovery_sign(&r1.pubkey(), &f.deal, 1000);
    send(&mut f.svm, &[ix], &r1.pubkey(), &[&r1]).unwrap();
    let exec = ix_recovery_execute(&f.deal, &f.mint, &f.alice_ata, &f.bob_ata, &f.token_program);
    send(&mut f.svm, &[exec], &f.bob.pubkey(), &[&f.bob]).unwrap();
    assert_eq!(token_balance(&f.svm, &f.bob_ata), 1000);
    assert_eq!(f.deal_state().state, DealState::Completed);
}
