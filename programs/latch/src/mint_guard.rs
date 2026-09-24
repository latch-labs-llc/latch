use anchor_lang::prelude::*;
use anchor_spl::token_2022::spl_token_2022::{
    extension::{
        default_account_state::DefaultAccountState, transfer_hook::TransferHook,
        BaseStateWithExtensions, ExtensionType, StateWithExtensions,
    },
    state::{AccountState, Mint as Mint2022},
};

use crate::error::EscrowError;
use crate::state::risk_flags;

/// Inspect a mint (classic SPL Token or Token-2022) and return its risk-flag bitfield.
///
/// Posture (see DESIGN.md): unknown extensions and extensions that make escrow
/// impossible are rejected outright; extensions that put the funds at issuer risk
/// (freeze, permanent delegate, ...) are flagged and must be explicitly accepted
/// by the parties via `accepted_risk_flags`.
pub fn vet_mint(mint_ai: &AccountInfo) -> Result<u16> {
    let mut flags: u16 = 0;
    let data = mint_ai.data.borrow();
    // Base mint layout is shared between SPL Token and Token-2022; a classic mint
    // simply has no extension TLV area.
    let state = StateWithExtensions::<Mint2022>::unpack(&data)?;

    if state.base.freeze_authority.is_some() {
        flags |= risk_flags::FREEZE_AUTHORITY;
    }

    if *mint_ai.owner == anchor_spl::token::ID {
        return Ok(flags);
    }

    for ext in state.get_extension_types()? {
        match ext {
            ExtensionType::PermanentDelegate => flags |= risk_flags::PERMANENT_DELEGATE,
            ExtensionType::TransferHook => {
                let hook = state.get_extension::<TransferHook>()?;
                let program_id: Option<Pubkey> = hook.program_id.into();
                // A live hook program can block or reorder transfers; not supported.
                // A dormant hook (PYUSD's placeholder) is flag-and-proceed.
                require!(program_id.is_none(), EscrowError::ActiveTransferHook);
                flags |= risk_flags::DORMANT_TRANSFER_HOOK;
            }
            ExtensionType::TransferFeeConfig => flags |= risk_flags::TRANSFER_FEE,
            ExtensionType::ConfidentialTransferMint => flags |= risk_flags::CONFIDENTIAL_CAPABLE,
            ExtensionType::ConfidentialTransferFeeConfig => {
                flags |= risk_flags::TRANSFER_FEE | risk_flags::CONFIDENTIAL_CAPABLE
            }
            ExtensionType::DefaultAccountState => {
                let dstate = state.get_extension::<DefaultAccountState>()?;
                require!(
                    dstate.state != (AccountState::Frozen as u8),
                    EscrowError::DefaultFrozenMint
                );
            }
            ExtensionType::NonTransferable => return err!(EscrowError::NonTransferableMint),
            ExtensionType::InterestBearingConfig => flags |= risk_flags::INTEREST_BEARING,
            ExtensionType::MintCloseAuthority => flags |= risk_flags::MINT_CLOSE_AUTHORITY,
            // Cosmetic/metadata extensions: no escrow impact.
            ExtensionType::MetadataPointer
            | ExtensionType::TokenMetadata
            | ExtensionType::GroupPointer
            | ExtensionType::TokenGroup
            | ExtensionType::GroupMemberPointer
            | ExtensionType::TokenGroupMember => {}
            // Anything else (pausable, scaled UI, permissioned burn, future
            // extensions) is rejected: default-deny is the escrow-safe posture.
            _ => return err!(EscrowError::UnknownMintExtension),
        }
    }

    Ok(flags)
}
