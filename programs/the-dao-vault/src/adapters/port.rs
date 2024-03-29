use std::ops::{Deref, DerefMut};

use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};
use port_anchor_adaptor::{port_lending_id, PortReserve};
use port_variable_rate_lending_instructions::state::Reserve;
use solana_maths::Rate;

use crate::{
    errors::ErrorCode,
    impl_has_vault,
    init_yield_source::YieldSourceInitializer,
    reconcile::LendingMarket,
    refresh::Refresher,
    reserves::{Provider, ReserveAccessor},
    state::Vault,
};

#[derive(Accounts)]
pub struct PortAccounts<'info> {
    /// Vault state account
    /// Checks that the accounts passed in are correct
    #[account(
        mut,
        has_one = vault_authority,
        has_one = vault_reserve_token,
        has_one = vault_port_lp_token,
        has_one = port_reserve,
    )]
    pub vault: Box<Account<'info, Vault>>,

    /// Authority that the vault uses for lp token mints/burns and transfers to/from downstream assets
    pub vault_authority: AccountInfo<'info>,

    /// Token account for the vault's reserve tokens
    #[account(mut)]
    pub vault_reserve_token: Box<Account<'info, TokenAccount>>,

    /// Token account for the vault's port lp tokens
    #[account(mut)]
    pub vault_port_lp_token: Box<Account<'info, TokenAccount>>,

    #[account(
        executable,
        address = port_lending_id(),
    )]
    pub port_program: AccountInfo<'info>,

    pub port_market_authority: AccountInfo<'info>,

    pub port_market: AccountInfo<'info>,

    #[account(mut)]
    pub port_reserve: Box<Account<'info, PortReserve>>,

    #[account(mut)]
    pub port_lp_mint: AccountInfo<'info>,

    #[account(mut)]
    pub port_reserve_token: AccountInfo<'info>,

    pub clock: Sysvar<'info, Clock>,

    pub token_program: Program<'info, Token>,
}

impl_has_vault!(PortAccounts<'_>);

impl<'info> LendingMarket for PortAccounts<'info> {
    fn deposit(&self, amount: u64) -> Result<()> {
        let context = CpiContext::new(
            self.port_program.clone(),
            port_anchor_adaptor::Deposit {
                source_liquidity: self.vault_reserve_token.to_account_info(),
                destination_collateral: self.vault_port_lp_token.to_account_info(),
                reserve: self.port_reserve.to_account_info(),
                reserve_collateral_mint: self.port_lp_mint.clone(),
                reserve_liquidity_supply: self.port_reserve_token.clone(),
                lending_market: self.port_market.clone(),
                lending_market_authority: self.port_market_authority.clone(),
                transfer_authority: self.vault_authority.clone(),
                clock: self.clock.to_account_info(),
                token_program: self.token_program.to_account_info(),
            },
        );

        match amount {
            0 => Ok(()),
            _ => port_anchor_adaptor::deposit_reserve(
                context.with_signer(&[&self.vault.authority_seeds()]),
                amount,
            ),
        }
    }

    fn redeem(&self, amount: u64) -> Result<()> {
        let context = CpiContext::new(
            self.port_program.clone(),
            port_anchor_adaptor::Redeem {
                source_collateral: self.vault_port_lp_token.to_account_info(),
                destination_liquidity: self.vault_reserve_token.to_account_info(),
                reserve: self.port_reserve.to_account_info(),
                reserve_collateral_mint: self.port_lp_mint.clone(),
                reserve_liquidity_supply: self.port_reserve_token.clone(),
                lending_market: self.port_market.clone(),
                lending_market_authority: self.port_market_authority.clone(),
                transfer_authority: self.vault_authority.clone(),
                clock: self.clock.to_account_info(),
                token_program: self.token_program.to_account_info(),
            },
        );
        match amount {
            0 => Ok(()),
            _ => port_anchor_adaptor::redeem(
                context.with_signer(&[&self.vault.authority_seeds()]),
                amount,
            ),
        }
    }

    fn convert_amount_reserve_to_lp(&self, amount: u64) -> Result<u64> {
        let exchange_rate = self.port_reserve.collateral_exchange_rate()?;
        match exchange_rate.liquidity_to_collateral(amount) {
            Ok(val) => Ok(val),
            Err(err) => Err(err.into()),
        }
    }

    fn convert_amount_lp_to_reserve(&self, amount: u64) -> Result<u64> {
        let exchange_rate = self.port_reserve.collateral_exchange_rate()?;
        match exchange_rate.collateral_to_liquidity(amount) {
            Ok(val) => Ok(val),
            Err(err) => Err(err.into()),
        }
    }

    fn reserve_tokens_in_vault(&self) -> u64 {
        self.vault_reserve_token.amount
    }

    fn lp_tokens_in_vault(&self) -> u64 {
        self.vault_port_lp_token.amount
    }

    fn provider(&self) -> Provider {
        Provider::Port
    }
}

impl ReserveAccessor for Reserve {
    fn utilization_rate(&self) -> Result<Rate> {
        Ok(Rate::from_scaled_val(
            self.liquidity.utilization_rate()?.to_scaled_val() as u64,
        ))
    }

    fn borrow_rate(&self) -> Result<Rate> {
        Ok(Rate::from_scaled_val(
            self.current_borrow_rate()?.to_scaled_val() as u64,
        ))
    }

    fn reserve_with_deposit(&self, allocation: u64) -> Result<Box<dyn ReserveAccessor>> {
        let mut reserve = Box::new(self.clone());
        reserve.liquidity.available_amount = reserve
            .liquidity
            .available_amount
            .checked_add(allocation)
            .ok_or(ErrorCode::OverflowError)?;
        Ok(reserve)
    }
}

#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct InitializePort<'info> {
    #[account(mut, has_one = owner, has_one = vault_authority)]
    pub vault: Box<Account<'info, Vault>>,

    pub vault_authority: AccountInfo<'info>,

    /// Token account for the vault's port lp tokens
    #[account(init, payer = payer, seeds = [vault.key().as_ref(), port_lp_token_mint.key().as_ref()], bump, token::authority = vault_authority, token::mint = port_lp_token_mint)]
    pub vault_port_lp_token: Box<Account<'info, TokenAccount>>,

    /// Mint of the port lp token
    pub port_lp_token_mint: AccountInfo<'info>,

    pub port_reserve: Box<Account<'info, PortReserve>>,

    pub owner: Signer<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub token_program: Program<'info, Token>,

    pub system_program: Program<'info, System>,

    pub rent: Sysvar<'info, Rent>,
}

impl<'info> YieldSourceInitializer<'info> for InitializePort<'info> {
    fn initialize_yield_source(&mut self) -> Result<()> {
        self.vault.port_reserve = self.port_reserve.key();
        self.vault.vault_port_lp_token = self.vault_port_lp_token.key();
        Ok(())
    }
}

#[derive(Accounts)]
pub struct RefreshPort<'info> {
    /// Vault state account
    /// Checks that the accounts passed in are correct
    #[account(mut, has_one = vault_port_lp_token, has_one = port_reserve)]
    pub vault: Box<Account<'info, Vault>>,

    /// Token account for the vault's port lp tokens
    pub vault_port_lp_token: Box<Account<'info, TokenAccount>>,

    #[account(executable, address = port_lending_id())]
    pub port_program: AccountInfo<'info>,

    #[account(mut)]
    pub port_reserve: Box<Account<'info, PortReserve>>,

    pub clock: Sysvar<'info, Clock>,
}

impl<'info> RefreshPort<'info> {
    fn port_refresh_reserve_context(
        &self,
        remaining_accounts: &[AccountInfo<'info>],
    ) -> CpiContext<'_, '_, '_, 'info, port_anchor_adaptor::RefreshReserve<'info>> {
        CpiContext::new(
            self.port_program.clone(),
            port_anchor_adaptor::RefreshReserve {
                reserve: self.port_reserve.to_account_info(),
                clock: self.clock.to_account_info(),
            },
        )
        .with_remaining_accounts(remaining_accounts.to_vec())
    }
}

impl<'info> Refresher<'info> for RefreshPort<'info> {
    fn update_actual_allocation(
        &mut self,
        remaining_accounts: &[AccountInfo<'info>],
    ) -> Result<()> {
        port_anchor_adaptor::refresh_port_reserve(
            self.port_refresh_reserve_context(remaining_accounts),
        )?;

        let port_exchange_rate = self.port_reserve.collateral_exchange_rate()?;
        let port_value =
            port_exchange_rate.collateral_to_liquidity(self.vault_port_lp_token.amount)?;

        #[cfg(feature = "debug")]
        msg!("Refresh port reserve token value: {}", port_value);

        self.vault.actual_allocations[Provider::Port].update(port_value, self.clock.slot);

        Ok(())
    }
}
