//! Real zero-knowledge proof generation and on-chain verification, in-suite.
//!
//! These tests inject Solana's native ZK ElGamal Proof Program (the one
//! re-enabled on mainnet/devnet in June 2026) into LiteSVM as a builtin, then
//! generate genuine proofs client-side and have the program verify them.
//! This is the primitive the future confidential-escrow phase builds on:
//! encrypted vault amounts whose properties (validity, zero-ness, ranges)
//! are proven without revealing the numbers.

mod common;

use {
    common::*,
    solana_keypair::Keypair,
    solana_signer::Signer,
    solana_zk_sdk::{
        encryption::elgamal::ElGamalKeypair,
        zk_elgamal_proof_program::{
            self,
            instruction::ProofInstruction,
            proof_data::{PubkeyValidityProofData, ZeroCiphertextProofData},
        },
    },
};

fn zk_svm() -> (litesvm::LiteSVM, Keypair) {
    let (mut svm, payer, _) = setup();
    svm.add_builtin(
        zk_elgamal_proof_program::id(),
        solana_zk_elgamal_proof_program::Entrypoint::vm,
    );
    // add_builtin creates the program account owned by the BPF loader, but
    // native programs must be owned by the native loader (as on devnet) or
    // invocation fails with "Unsupported program id".
    let native_loader: anchor_lang::prelude::Pubkey =
        "NativeLoader1111111111111111111111111111111".parse().unwrap();
    svm.set_account(
        zk_elgamal_proof_program::id(),
        solana_account::Account {
            lamports: 1,
            data: b"zk_elgamal_proof_program".to_vec(),
            owner: native_loader,
            executable: true,
            rent_epoch: 0,
        },
    )
    .unwrap();
    (svm, payer)
}

#[test]
fn verify_pubkey_validity_proof() {
    let (mut svm, payer) = zk_svm();

    // A party generates an ElGamal encryption keypair (this is what a
    // confidential-escrow party would hold alongside their wallet key) and
    // proves, in zero knowledge, that the public key is well-formed.
    let elgamal = ElGamalKeypair::new_rand();
    let proof = PubkeyValidityProofData::new(&elgamal).unwrap();
    let ix = ProofInstruction::VerifyPubkeyValidity.encode_verify_proof(None, &proof);

    let res = send(&mut svm, &[ix], &payer.pubkey(), &[&payer]);
    assert!(res.is_ok(), "valid proof rejected: {:?}", res.err().map(|e| e.meta.logs));
}

#[test]
fn tampered_proof_is_rejected() {
    let (mut svm, payer) = zk_svm();

    let elgamal = ElGamalKeypair::new_rand();
    let proof = PubkeyValidityProofData::new(&elgamal).unwrap();
    let mut ix = ProofInstruction::VerifyPubkeyValidity.encode_verify_proof(None, &proof);
    // Flip one bit inside the proof bytes (past the 1-byte instruction tag).
    let last = ix.data.len() - 1;
    ix.data[last] ^= 0x01;

    let res = send(&mut svm, &[ix], &payer.pubkey(), &[&payer]);
    assert!(res.is_err(), "tampered proof was accepted");
}

#[test]
fn verify_zero_ciphertext_proof() {
    let (mut svm, payer) = zk_svm();

    // The proof a confidential escrow would use to show an encrypted balance
    // is exactly zero (e.g. "the vault is empty, the deal can close") without
    // revealing anything else.
    let elgamal = ElGamalKeypair::new_rand();
    let ciphertext_of_zero = elgamal.pubkey().encrypt(0u64);
    let proof = ZeroCiphertextProofData::new(&elgamal, &ciphertext_of_zero).unwrap();
    let ix = ProofInstruction::VerifyZeroCiphertext.encode_verify_proof(None, &proof);
    let res = send(&mut svm, &[ix], &payer.pubkey(), &[&payer]);
    assert!(res.is_ok(), "zero-ciphertext proof rejected: {:?}", res.err().map(|e| e.meta.logs));

    // And the same proof structure over a NON-zero balance must fail:
    // you cannot prove an encrypted 42 is zero.
    let ciphertext_of_42 = elgamal.pubkey().encrypt(42u64);
    let bogus = ZeroCiphertextProofData::new(&elgamal, &ciphertext_of_42).unwrap();
    let ix = ProofInstruction::VerifyZeroCiphertext.encode_verify_proof(None, &bogus);
    let res = send(&mut svm, &[ix], &payer.pubkey(), &[&payer]);
    assert!(res.is_err(), "proved that 42 == 0 — cryptography is broken");
}
