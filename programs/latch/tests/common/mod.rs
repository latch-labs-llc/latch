#![allow(dead_code)]

use {
    anchor_lang::{
        prelude::Pubkey,
        solana_program::{instruction::Instruction, system_instruction, system_program},
        AccountDeserialize, InstructionData, ToAccountMetas,
    },
    anchor_spl::{
        associated_token::get_associated_token_address_with_program_id,
        token_2022::spl_token_2022,
    },
    litesvm::{types::TransactionResult, LiteSVM},
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
    latch::{state::*, CreateDealParams},
};

pub const SOL: u64 = 1_000_000_000;
// Base account sizes shared by SPL Token and Token-2022.
const MINT_LEN: usize = 82;
const TOKEN_ACCOUNT_LEN: usize = 165;

pub fn setup() -> (LiteSVM, Keypair, Keypair) {
    let mut svm = LiteSVM::new();
    let bytes = include_bytes!(concat!(env!("CARGO_TARGET_TMPDIR"), "/../deploy/latch.so"));
    svm.add_program(latch::id(), bytes).unwrap();
    // LiteSVM's clock starts at 0; give tests a realistic wall time.
    let mut clock: anchor_lang::prelude::Clock = svm.get_sysvar();
    clock.unix_timestamp = 1_758_500_000;
    svm.set_sysvar(&clock);
    let a = Keypair::new();
    let b = Keypair::new();
    svm.airdrop(&a.pubkey(), 10 * SOL).unwrap();
    svm.airdrop(&b.pubkey(), 10 * SOL).unwrap();
    (svm, a, b)
}

pub fn send(
    svm: &mut LiteSVM,
    instructions: &[Instruction],
    payer: &Pubkey,
    signers: &[&Keypair],
) -> TransactionResult {
    // A fresh blockhash per send so repeated identical instructions aren't
    // rejected as duplicate transactions.
    svm.expire_blockhash();
    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(instructions, Some(payer), &blockhash);
    // Dedupe signers by pubkey so helpers can pass the same keypair in two roles.
    let mut unique: Vec<&Keypair> = Vec::new();
    for s in signers {
        if !unique.iter().any(|u| u.pubkey() == s.pubkey()) {
            unique.push(s);
        }
    }
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &unique).unwrap();
    svm.send_transaction(tx)
}

pub fn assert_err(res: TransactionResult, code_name: &str) {
    match res {
        Ok(meta) => panic!("expected error {code_name}, tx succeeded: {:?}", meta.logs),
        Err(fail) => {
            let logs = fail.meta.logs.join("\n");
            assert!(
                logs.contains(code_name),
                "expected error {code_name}, got:\n{logs}\nerr: {:?}",
                fail.err
            );
        }
    }
}

pub fn warp(svm: &mut LiteSVM, secs: i64) {
    let mut clock: anchor_lang::prelude::Clock = svm.get_sysvar();
    clock.unix_timestamp += secs;
    svm.set_sysvar(&clock);
}

// ---------- token helpers ----------

pub fn classic_token_id() -> Pubkey {
    anchor_spl::token::ID
}

pub fn token_2022_id() -> Pubkey {
    anchor_spl::token_2022::ID
}

/// Create a mint with optional freeze authority (classic or 2022, no extensions).
pub fn create_mint(
    svm: &mut LiteSVM,
    payer: &Keypair,
    mint_authority: &Pubkey,
    freeze_authority: Option<&Pubkey>,
    token_program: &Pubkey,
) -> Pubkey {
    let mint = Keypair::new();
    let len = MINT_LEN;
    let rent = svm.minimum_balance_for_rent_exemption(len);
    let ixs = [
        system_instruction::create_account(&payer.pubkey(), &mint.pubkey(), rent, len as u64, token_program),
        spl_token_2022::instruction::initialize_mint2(
            token_program,
            &mint.pubkey(),
            mint_authority,
            freeze_authority,
            6,
        )
        .unwrap(),
    ];
    send(svm, &ixs, &payer.pubkey(), &[payer, &mint]).unwrap();
    mint.pubkey()
}

/// Extensions a hostile/flagged Token-2022 test mint can carry.
pub enum TestExt {
    PermanentDelegate(Pubkey),
    TransferFee { bps: u16, max: u64 },
    NonTransferable,
    DefaultFrozen,
}

pub fn create_mint_2022_with_extensions(
    svm: &mut LiteSVM,
    payer: &Keypair,
    mint_authority: &Pubkey,
    exts: &[TestExt],
) -> Pubkey {
    use spl_token_2022::extension::ExtensionType;
    let mint = Keypair::new();
    let types: Vec<ExtensionType> = exts
        .iter()
        .map(|e| match e {
            TestExt::PermanentDelegate(_) => ExtensionType::PermanentDelegate,
            TestExt::TransferFee { .. } => ExtensionType::TransferFeeConfig,
            TestExt::NonTransferable => ExtensionType::NonTransferable,
            TestExt::DefaultFrozen => ExtensionType::DefaultAccountState,
        })
        .collect();
    let len =
        ExtensionType::try_calculate_account_len::<spl_token_2022::state::Mint>(&types).unwrap();
    let rent = svm.minimum_balance_for_rent_exemption(len);
    let tp = token_2022_id();

    let mut ixs = vec![system_instruction::create_account(
        &payer.pubkey(),
        &mint.pubkey(),
        rent,
        len as u64,
        &tp,
    )];
    for e in exts {
        match e {
            TestExt::PermanentDelegate(delegate) => ixs.push(
                spl_token_2022::instruction::initialize_permanent_delegate(&tp, &mint.pubkey(), delegate)
                    .unwrap(),
            ),
            TestExt::TransferFee { bps, max } => ixs.push(
                spl_token_2022::extension::transfer_fee::instruction::initialize_transfer_fee_config(
                    &tp,
                    &mint.pubkey(),
                    Some(mint_authority),
                    Some(mint_authority),
                    *bps,
                    *max,
                )
                .unwrap(),
            ),
            TestExt::NonTransferable => ixs.push(
                spl_token_2022::instruction::initialize_non_transferable_mint(&tp, &mint.pubkey())
                    .unwrap(),
            ),
            TestExt::DefaultFrozen => ixs.push(
                spl_token_2022::extension::default_account_state::instruction::initialize_default_account_state(
                    &tp,
                    &mint.pubkey(),
                    &spl_token_2022::state::AccountState::Frozen,
                )
                .unwrap(),
            ),
        }
    }
    // A default-frozen mint needs a freeze authority to be valid.
    let freeze = exts.iter().any(|e| matches!(e, TestExt::DefaultFrozen)).then_some(mint_authority);
    ixs.push(
        spl_token_2022::instruction::initialize_mint2(&tp, &mint.pubkey(), mint_authority, freeze, 6)
            .unwrap(),
    );
    send(svm, &ixs, &payer.pubkey(), &[payer, &mint]).unwrap();
    mint.pubkey()
}

pub fn create_token_account(
    svm: &mut LiteSVM,
    payer: &Keypair,
    owner: &Pubkey,
    mint: &Pubkey,
    token_program: &Pubkey,
) -> Pubkey {
    let account = Keypair::new();
    // Token-2022 accounts need space for account extensions the mint requires
    // (e.g. TransferFeeAmount on transfer-fee mints).
    let len = if *token_program == token_2022_id() {
        use spl_token_2022::extension::{BaseStateWithExtensions, ExtensionType, StateWithExtensions};
        let mint_data = svm.get_account(mint).unwrap().data;
        let state = StateWithExtensions::<spl_token_2022::state::Mint>::unpack(&mint_data).unwrap();
        let required = ExtensionType::get_required_init_account_extensions(
            &state.get_extension_types().unwrap(),
        );
        ExtensionType::try_calculate_account_len::<spl_token_2022::state::Account>(&required).unwrap()
    } else {
        TOKEN_ACCOUNT_LEN
    };
    let rent = svm.minimum_balance_for_rent_exemption(len);
    let ixs = [
        system_instruction::create_account(&payer.pubkey(), &account.pubkey(), rent, len as u64, token_program),
        spl_token_2022::instruction::initialize_account3(token_program, &account.pubkey(), mint, owner)
            .unwrap(),
    ];
    send(svm, &ixs, &payer.pubkey(), &[payer, &account]).unwrap();
    account.pubkey()
}

pub fn mint_to(
    svm: &mut LiteSVM,
    payer: &Keypair,
    mint_authority: &Keypair,
    mint: &Pubkey,
    dest: &Pubkey,
    amount: u64,
    token_program: &Pubkey,
) {
    let ix = spl_token_2022::instruction::mint_to(
        token_program,
        mint,
        dest,
        &mint_authority.pubkey(),
        &[],
        amount,
    )
    .unwrap();
    send(svm, &[ix], &payer.pubkey(), &[payer, mint_authority]).unwrap();
}

pub fn token_balance(svm: &LiteSVM, account: &Pubkey) -> u64 {
    use spl_token_2022::extension::{BaseStateWithExtensions, StateWithExtensions};
    let data = svm.get_account(account).unwrap().data;
    let state = StateWithExtensions::<spl_token_2022::state::Account>::unpack(&data).unwrap();
    let _ = state.get_extension_types();
    state.base.amount
}

// ---------- program helpers ----------

pub fn event_authority() -> Pubkey {
    Pubkey::find_program_address(&[b"__event_authority"], &latch::id()).0
}

pub fn deal_pda(creator: &Pubkey, deal_id: u64) -> Pubkey {
    Pubkey::find_program_address(
        &[b"deal", creator.as_ref(), &deal_id.to_le_bytes()],
        &latch::id(),
    )
    .0
}

pub fn vault_ata(deal: &Pubkey, mint: &Pubkey, token_program: &Pubkey) -> Pubkey {
    get_associated_token_address_with_program_id(deal, mint, token_program)
}

pub fn get_deal(svm: &LiteSVM, deal: &Pubkey) -> Deal {
    let data = svm.get_account(deal).unwrap().data;
    Deal::try_deserialize(&mut data.as_slice()).unwrap()
}

/// Two-party params with sensible defaults; tests override fields as needed.
pub fn default_params(payer: &Pubkey, payee: &Pubkey, milestones: Vec<u64>) -> CreateDealParams {
    let total = milestones.iter().sum();
    CreateDealParams {
        deal_id: 1,
        parties: vec![*payer, *payee],
        payer_idx: 0,
        payee_idx: 1,
        approval_threshold: 2,
        terms_hash: [7u8; 32],
        price_risk_term: 0,
        total_amount: total,
        milestone_amounts: milestones,
        deadlock_rule: DeadlockRule::TimeoutRefund,
        timer_mode: TimerMode::FromDeadlock,
        timeout_secs: 3600,
        split_bps: 0,
        tie_breaker: Pubkey::default(),
        recovery_signers: vec![Pubkey::new_unique()],
        recovery_threshold: 1,
        recovery_delay_secs: 0,
        accepted_risk_flags: u16::MAX,
    }
}

pub fn ix_create_deal(
    creator: &Pubkey,
    mint: &Pubkey,
    token_program: &Pubkey,
    params: CreateDealParams,
) -> Instruction {
    let deal = deal_pda(creator, params.deal_id);
    let vault = vault_ata(&deal, mint, token_program);
    Instruction::new_with_bytes(
        latch::id(),
        &latch::instruction::CreateDeal { params }.data(),
        latch::accounts::CreateDeal {
            creator: *creator,
            deal,
            mint: *mint,
            vault,
            token_program: *token_program,
            associated_token_program: anchor_spl::associated_token::ID,
            system_program: system_program::ID,
            event_authority: event_authority(),
            program: latch::id(),
        }
        .to_account_metas(None),
    )
}

pub fn ix_sign_terms(party: &Pubkey, deal: &Pubkey, consent: bool) -> Instruction {
    Instruction::new_with_bytes(
        latch::id(),
        &latch::instruction::SignTerms { consent_true_deadlock: consent }.data(),
        latch::accounts::SignTerms {
            party: *party,
            deal: *deal,
            event_authority: event_authority(),
            program: latch::id(),
        }
        .to_account_metas(None),
    )
}

pub fn ix_deposit(
    payer: &Pubkey,
    deal: &Pubkey,
    mint: &Pubkey,
    payer_token_account: &Pubkey,
    token_program: &Pubkey,
    amount: u64,
) -> Instruction {
    Instruction::new_with_bytes(
        latch::id(),
        &latch::instruction::Deposit { amount }.data(),
        latch::accounts::Deposit {
            payer: *payer,
            deal: *deal,
            mint: *mint,
            vault: vault_ata(deal, mint, token_program),
            payer_token_account: *payer_token_account,
            token_program: *token_program,
            event_authority: event_authority(),
            program: latch::id(),
        }
        .to_account_metas(None),
    )
}

pub fn ix_confirm_ready(party: &Pubkey, deal: &Pubkey) -> Instruction {
    Instruction::new_with_bytes(
        latch::id(),
        &latch::instruction::ConfirmReady {}.data(),
        latch::accounts::ConfirmReady {
            party: *party,
            deal: *deal,
            event_authority: event_authority(),
            program: latch::id(),
        }
        .to_account_metas(None),
    )
}

pub fn ix_approve_milestone(party: &Pubkey, deal: &Pubkey, index: u8) -> Instruction {
    Instruction::new_with_bytes(
        latch::id(),
        &latch::instruction::ApproveMilestone { index }.data(),
        latch::accounts::ApproveMilestone {
            party: *party,
            deal: *deal,
            event_authority: event_authority(),
            program: latch::id(),
        }
        .to_account_metas(None),
    )
}

pub fn ix_release_milestone(
    deal: &Pubkey,
    mint: &Pubkey,
    payee_token_account: &Pubkey,
    token_program: &Pubkey,
    index: u8,
) -> Instruction {
    Instruction::new_with_bytes(
        latch::id(),
        &latch::instruction::ReleaseMilestone { index }.data(),
        latch::accounts::ReleaseMilestone {
            deal: *deal,
            mint: *mint,
            vault: vault_ata(deal, mint, token_program),
            payee_token_account: *payee_token_account,
            token_program: *token_program,
            event_authority: event_authority(),
            program: latch::id(),
        }
        .to_account_metas(None),
    )
}

pub fn ix_raise_deadlock(party: &Pubkey, deal: &Pubkey) -> Instruction {
    Instruction::new_with_bytes(
        latch::id(),
        &latch::instruction::RaiseDeadlock {}.data(),
        latch::accounts::RaiseDeadlock {
            party: *party,
            deal: *deal,
            event_authority: event_authority(),
            program: latch::id(),
        }
        .to_account_metas(None),
    )
}

pub fn ix_withdraw_deadlock(party: &Pubkey, deal: &Pubkey) -> Instruction {
    Instruction::new_with_bytes(
        latch::id(),
        &latch::instruction::WithdrawDeadlock {}.data(),
        latch::accounts::WithdrawDeadlock {
            party: *party,
            deal: *deal,
            event_authority: event_authority(),
            program: latch::id(),
        }
        .to_account_metas(None),
    )
}

pub fn ix_resolution_sign(signer: &Pubkey, deal: &Pubkey, amount_to_payee: u64) -> Instruction {
    Instruction::new_with_bytes(
        latch::id(),
        &latch::instruction::ResolutionSign { amount_to_payee }.data(),
        latch::accounts::ResolutionSign {
            signer: *signer,
            deal: *deal,
            event_authority: event_authority(),
            program: latch::id(),
        }
        .to_account_metas(None),
    )
}

pub fn ix_resolve(
    deal: &Pubkey,
    mint: &Pubkey,
    payer_token_account: &Pubkey,
    payee_token_account: &Pubkey,
    token_program: &Pubkey,
) -> Instruction {
    Instruction::new_with_bytes(
        latch::id(),
        &latch::instruction::Resolve {}.data(),
        latch::accounts::Resolve {
            deal: *deal,
            mint: *mint,
            vault: vault_ata(deal, mint, token_program),
            payer_token_account: *payer_token_account,
            payee_token_account: *payee_token_account,
            token_program: *token_program,
            event_authority: event_authority(),
            program: latch::id(),
        }
        .to_account_metas(None),
    )
}

pub fn ix_recovery_sign(signer: &Pubkey, deal: &Pubkey, amount_to_payee: u64) -> Instruction {
    Instruction::new_with_bytes(
        latch::id(),
        &latch::instruction::RecoverySign { amount_to_payee }.data(),
        latch::accounts::RecoverySign {
            signer: *signer,
            deal: *deal,
            event_authority: event_authority(),
            program: latch::id(),
        }
        .to_account_metas(None),
    )
}

pub fn ix_recovery_execute(
    deal: &Pubkey,
    mint: &Pubkey,
    payer_token_account: &Pubkey,
    payee_token_account: &Pubkey,
    token_program: &Pubkey,
) -> Instruction {
    Instruction::new_with_bytes(
        latch::id(),
        &latch::instruction::RecoveryExecute {}.data(),
        latch::accounts::RecoveryExecute {
            deal: *deal,
            mint: *mint,
            vault: vault_ata(deal, mint, token_program),
            payer_token_account: *payer_token_account,
            payee_token_account: *payee_token_account,
            token_program: *token_program,
            event_authority: event_authority(),
            program: latch::id(),
        }
        .to_account_metas(None),
    )
}

pub fn ix_cancel_draft(party: &Pubkey, deal: &Pubkey) -> Instruction {
    Instruction::new_with_bytes(
        latch::id(),
        &latch::instruction::CancelDraft {}.data(),
        latch::accounts::CancelDraft {
            party: *party,
            deal: *deal,
            event_authority: event_authority(),
            program: latch::id(),
        }
        .to_account_metas(None),
    )
}

pub fn ix_cancel_sign(
    party: &Pubkey,
    deal: &Pubkey,
    mint: &Pubkey,
    payer_token_account: &Pubkey,
    token_program: &Pubkey,
) -> Instruction {
    Instruction::new_with_bytes(
        latch::id(),
        &latch::instruction::CancelSign {}.data(),
        latch::accounts::CancelSign {
            party: *party,
            deal: *deal,
            mint: *mint,
            vault: vault_ata(deal, mint, token_program),
            payer_token_account: *payer_token_account,
            token_program: *token_program,
            event_authority: event_authority(),
            program: latch::id(),
        }
        .to_account_metas(None),
    )
}

// ---------- fixture: a standard 2-party classic-SPL deal ----------

pub struct Fixture {
    pub svm: LiteSVM,
    pub alice: Keypair, // payer party, creator, mint authority
    pub bob: Keypair,   // payee party
    pub mint: Pubkey,
    pub token_program: Pubkey,
    pub deal: Pubkey,
    pub alice_ata: Pubkey,
    pub bob_ata: Pubkey,
}

impl Fixture {
    /// Create a deal in Draft with the given params-modifier applied.
    pub fn new(milestones: Vec<u64>, tweak: impl FnOnce(&mut CreateDealParams)) -> Self {
        let (mut svm, alice, bob) = setup();
        let tp = classic_token_id();
        let mint = create_mint(&mut svm, &alice, &alice.pubkey(), None, &tp);
        let alice_ata = create_token_account(&mut svm, &alice, &alice.pubkey(), &mint, &tp);
        let bob_ata = create_token_account(&mut svm, &alice, &bob.pubkey(), &mint, &tp);
        let alice_kp = alice.insecure_clone();
        mint_to(&mut svm, &alice, &alice_kp, &mint, &alice_ata, 1_000_000_000, &tp);

        let mut params = default_params(&alice.pubkey(), &bob.pubkey(), milestones);
        tweak(&mut params);
        let deal = deal_pda(&alice.pubkey(), params.deal_id);
        send(&mut svm, &[ix_create_deal(&alice.pubkey(), &mint, &tp, params)], &alice.pubkey(), &[&alice])
            .unwrap();

        Fixture { svm, alice, bob, mint, token_program: tp, deal, alice_ata, bob_ata }
    }

    pub fn sign_all(&mut self) {
        for kp in [self.alice.insecure_clone(), self.bob.insecure_clone()] {
            send(
                &mut self.svm,
                &[ix_sign_terms(&kp.pubkey(), &self.deal, true)],
                &kp.pubkey(),
                &[&kp],
            )
            .unwrap();
        }
    }

    pub fn fund(&mut self) {
        let total = self.deal_state().total_amount;
        let ix = ix_deposit(
            &self.alice.pubkey(),
            &self.deal,
            &self.mint,
            &self.alice_ata,
            &self.token_program,
            total,
        );
        send(&mut self.svm, &[ix], &self.alice.pubkey(), &[&self.alice]).unwrap();
    }

    pub fn activate(&mut self) {
        for kp in [self.alice.insecure_clone(), self.bob.insecure_clone()] {
            send(
                &mut self.svm,
                &[ix_confirm_ready(&kp.pubkey(), &self.deal)],
                &kp.pubkey(),
                &[&kp],
            )
            .unwrap();
        }
    }

    /// Draft → Active in one call.
    pub fn to_active(&mut self) {
        self.sign_all();
        self.fund();
        self.activate();
    }

    pub fn deal_state(&self) -> Deal {
        get_deal(&self.svm, &self.deal)
    }
}
