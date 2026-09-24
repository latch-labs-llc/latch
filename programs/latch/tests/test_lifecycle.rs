mod common;

use common::*;
use solana_signer::Signer;
use latch::state::DealState;

#[test]
fn happy_path_two_party_two_milestones() {
    let mut f = Fixture::new(vec![600, 400], |_| {});
    assert_eq!(f.deal_state().state, DealState::Draft);

    // Both parties sign the terms hash.
    f.sign_all();
    let d = f.deal_state();
    assert_eq!(d.state, DealState::Signed);
    assert_eq!(d.signed, 0b11);
    assert!(d.parties_signed_at[0] > 0 && d.parties_signed_at[1] > 0);

    // Payer funds the vault in two partial deposits.
    let ix = ix_deposit(&f.alice.pubkey(), &f.deal, &f.mint, &f.alice_ata, &f.token_program, 250);
    send(&mut f.svm, &[ix], &f.alice.pubkey(), &[&f.alice]).unwrap();
    assert_eq!(f.deal_state().state, DealState::Signed);
    assert_eq!(f.deal_state().deposited, 250);
    let ix = ix_deposit(&f.alice.pubkey(), &f.deal, &f.mint, &f.alice_ata, &f.token_program, 750);
    send(&mut f.svm, &[ix], &f.alice.pubkey(), &[&f.alice]).unwrap();
    let d = f.deal_state();
    assert_eq!(d.state, DealState::Funded);
    assert_eq!(d.deposited, 1000);
    assert_eq!(token_balance(&f.svm, &vault_ata(&f.deal, &f.mint, &f.token_program)), 1000);

    // Everyone confirms readiness.
    f.activate();
    assert_eq!(f.deal_state().state, DealState::Active);

    // Milestone 0: both approve (threshold 2), anyone cranks the release.
    for kp in [f.alice.insecure_clone(), f.bob.insecure_clone()] {
        let ix = ix_approve_milestone(&kp.pubkey(), &f.deal, 0);
        send(&mut f.svm, &[ix], &kp.pubkey(), &[&kp]).unwrap();
    }
    let ix = ix_release_milestone(&f.deal, &f.mint, &f.bob_ata, &f.token_program, 0);
    send(&mut f.svm, &[ix], &f.alice.pubkey(), &[&f.alice]).unwrap();
    let d = f.deal_state();
    assert_eq!(d.state, DealState::Active);
    assert!(d.milestones[0].released);
    assert_eq!(d.released_total, 600);
    assert_eq!(token_balance(&f.svm, &f.bob_ata), 600);

    // Milestone 1 completes the deal.
    for kp in [f.alice.insecure_clone(), f.bob.insecure_clone()] {
        let ix = ix_approve_milestone(&kp.pubkey(), &f.deal, 1);
        send(&mut f.svm, &[ix], &kp.pubkey(), &[&kp]).unwrap();
    }
    let ix = ix_release_milestone(&f.deal, &f.mint, &f.bob_ata, &f.token_program, 1);
    send(&mut f.svm, &[ix], &f.bob.pubkey(), &[&f.bob]).unwrap();
    let d = f.deal_state();
    assert_eq!(d.state, DealState::Completed);
    assert_eq!(d.released_total, 1000);
    assert_eq!(token_balance(&f.svm, &f.bob_ata), 1000);
    assert_eq!(token_balance(&f.svm, &vault_ata(&f.deal, &f.mint, &f.token_program)), 0);

    // Event sequence advanced monotonically the whole way.
    assert!(d.event_seq >= 12);
}

#[test]
fn threshold_release_without_full_approval() {
    // Threshold 1 of 2: payee's own approval is enough to crank the release.
    let mut f = Fixture::new(vec![1000], |p| p.approval_threshold = 1);
    f.to_active();

    let ix = ix_approve_milestone(&f.bob.pubkey(), &f.deal, 0);
    send(&mut f.svm, &[ix], &f.bob.pubkey(), &[&f.bob]).unwrap();
    let ix = ix_release_milestone(&f.deal, &f.mint, &f.bob_ata, &f.token_program, 0);
    send(&mut f.svm, &[ix], &f.bob.pubkey(), &[&f.bob]).unwrap();
    assert_eq!(f.deal_state().state, DealState::Completed);
    assert_eq!(token_balance(&f.svm, &f.bob_ata), 1000);
}

#[test]
fn token_2022_lifecycle() {
    // Same happy path on Token-2022 with a plain (extension-free) mint.
    let (mut svm, alice, bob) = setup();
    let tp = token_2022_id();
    let mint = create_mint(&mut svm, &alice, &alice.pubkey(), None, &tp);
    let alice_ata = create_token_account(&mut svm, &alice, &alice.pubkey(), &mint, &tp);
    let bob_ata = create_token_account(&mut svm, &alice, &bob.pubkey(), &mint, &tp);
    let alice_kp = alice.insecure_clone();
    mint_to(&mut svm, &alice, &alice_kp, &mint, &alice_ata, 5000, &tp);

    let params = default_params(&alice.pubkey(), &bob.pubkey(), vec![5000]);
    let deal = deal_pda(&alice.pubkey(), params.deal_id);
    send(&mut svm, &[ix_create_deal(&alice.pubkey(), &mint, &tp, params)], &alice.pubkey(), &[&alice])
        .unwrap();
    for kp in [alice.insecure_clone(), bob.insecure_clone()] {
        send(&mut svm, &[ix_sign_terms(&kp.pubkey(), &deal, true)], &kp.pubkey(), &[&kp]).unwrap();
    }
    let ix = ix_deposit(&alice.pubkey(), &deal, &mint, &alice_ata, &tp, 5000);
    send(&mut svm, &[ix], &alice.pubkey(), &[&alice]).unwrap();
    for kp in [alice.insecure_clone(), bob.insecure_clone()] {
        send(&mut svm, &[ix_confirm_ready(&kp.pubkey(), &deal)], &kp.pubkey(), &[&kp]).unwrap();
    }
    for kp in [alice.insecure_clone(), bob.insecure_clone()] {
        send(&mut svm, &[ix_approve_milestone(&kp.pubkey(), &deal, 0)], &kp.pubkey(), &[&kp]).unwrap();
    }
    let ix = ix_release_milestone(&deal, &mint, &bob_ata, &tp, 0);
    send(&mut svm, &[ix], &alice.pubkey(), &[&alice]).unwrap();

    assert_eq!(get_deal(&svm, &deal).state, DealState::Completed);
    assert_eq!(token_balance(&svm, &bob_ata), 5000);
}
