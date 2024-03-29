use std::convert::Into;

use boolinator::Boolinator;

use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, MintTo, Token, TokenAccount, Transfer};

use crate::{
    errors::ErrorCode,
    state::{Vault, VaultFlags},
};

#[event]
pub struct DepositEvent {
    vault: Pubkey,
    user: Pubkey,
    amount: u64,
}

#[derive(Accounts)]
pub struct Deposit<'info> {
    /// Vault state account
    /// Checks that refresh has been called in the same slot
    /// Checks that the accounts passed in are correct
    #[account(mut, constraint = !vault.value.last_update.is_stale(clock.slot)? @ ErrorCode::VaultIsNotRefreshed, has_one = lp_token_mint, has_one = vault_authority, has_one = vault_reserve_token)]
    pub vault: Box<Account<'info, Vault>>,

    /// Authority that the vault uses for lp token mints/burns and transfers to/from downstream assets
    pub vault_authority: AccountInfo<'info>,

    /// Token account for the vault's reserve tokens
    #[account(mut)]
    pub vault_reserve_token: Box<Account<'info, TokenAccount>>,

    /// Mint for the vault's lp token
    #[account(mut)]
    pub lp_token_mint: Box<Account<'info, Mint>>,

    /// Token account from which reserve tokens are transfered
    #[account(mut)]
    pub user_reserve_token: Box<Account<'info, TokenAccount>>,

    /// Account where vault LP tokens are minted to
    #[account(mut)]
    pub user_lp_token: Box<Account<'info, TokenAccount>>,

    /// Authority of the user_reserve_token account
    /// Must be signer
    pub user_authority: Signer<'info>,

    pub token_program: Program<'info, Token>,

    pub clock: Sysvar<'info, Clock>,
}

impl<'info> Deposit<'info> {
    /// CpiContext for minting vault lp tokens to user account
    fn mint_to_context(&self) -> CpiContext<'_, '_, '_, 'info, MintTo<'info>> {
        let context = CpiContext::new(
            self.token_program.to_account_info(),
            MintTo {
                mint: self.lp_token_mint.to_account_info(),
                to: self.user_lp_token.to_account_info(),
                authority: self.vault_authority.clone(),
            },
        );
        context
    }

    /// CpiContext for transfering reserve tokens from user to vault
    fn transfer_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        CpiContext::new(
            self.token_program.to_account_info(),
            Transfer {
                from: self.user_reserve_token.to_account_info(),
                to: self.vault_reserve_token.to_account_info(),
                authority: self.user_authority.to_account_info(),
            },
        )
    }
}

/// Deposit to the vault
///
/// Transfers reserve tokens from user to vault and mints their share of lp tokens
pub fn handler(ctx: Context<Deposit>, reserve_token_amount: u64) -> Result<()> {
    #[cfg(feature = "debug")]
    msg!("Depositting {} reserve tokens", reserve_token_amount);

    // Check that deposits are not halted
    (!ctx
        .accounts
        .vault
        .flags()
        .contains(VaultFlags::HALT_DEPOSITS_WITHDRAWS))
    .ok_or(ErrorCode::HaltedVault)?;

    let vault = &ctx.accounts.vault;

    let lp_tokens_to_mint = crate::math::calc_reserve_to_lp(
        reserve_token_amount,
        ctx.accounts.lp_token_mint.supply,
        vault.value.value,
    )
    .ok_or(ErrorCode::MathError)?;

    let total_value = ctx
        .accounts
        .vault
        .value
        .value
        .checked_add(reserve_token_amount)
        .ok_or(ErrorCode::OverflowError)?;

    if total_value > ctx.accounts.vault.config.deposit_cap {
        msg!("Dposit cap reached");
        return Err(ErrorCode::DepositCapError.into());
    }

    token::transfer(ctx.accounts.transfer_context(), reserve_token_amount)?;

    #[cfg(feature = "debug")]
    msg!("Minting {} LP Tokens", lp_tokens_to_mint);

    token::mint_to(
        ctx.accounts
            .mint_to_context()
            .with_signer(&[&vault.authority_seeds()]),
        lp_tokens_to_mint,
    )?;

    // This is so that the SDK can read an up-to-date total value without calling refresh
    ctx.accounts.vault.value.value = ctx
        .accounts
        .vault
        .value
        .value
        .checked_add(reserve_token_amount)
        .ok_or(ErrorCode::MathError)?;

    emit!(DepositEvent {
        vault: ctx.accounts.vault.key(),
        user: ctx.accounts.user_authority.key(),
        amount: reserve_token_amount,
    });
    Ok(())
}
