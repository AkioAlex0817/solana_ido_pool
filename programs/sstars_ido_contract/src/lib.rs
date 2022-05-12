//! A Solana SStars IDO program
use anchor_lang::prelude::*;
use anchor_spl::token::{self, TokenAccount, Token, Mint, Transfer};
use std::ops::Deref;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

pub mod constants {
    pub const USER_STAKE_PDA_SEED: &[u8] = b"user_stake";
}

pub mod token_constants {
    pub const USDC_TOKEN_PUBKEY: &str = "G7EY516o2hAWDxQ3g8Z9tSCh5gdkhp5Sz7WhHNFQ9kqA";
}

const DECIMALS: u8 = 9;

#[program]
pub mod sstars_ido_contract {
    use super::*;

    /**
     * ****************************************
     *
     * Initialize Program Config
     * ****************************************
     */
    /// upgradeable Initialize
    /// @param _config_nonce            Program Config Account Address Nonce
    #[access_control(validate_ido_times(ido_times))]
    pub fn initialize(ctx: Context<Initialize>, ido_name: String, ido_times: IdoTimes, _nonce: u8) -> Result<()> {
        msg!("INITIALIZE");
        let ido_account = &mut ctx.accounts.ido_account;

        let name_bytes = ido_name.as_bytes();
        let mut name_data = [b' '; 10];
        name_data[..name_bytes.len()].copy_from_slice(name_bytes);

        ido_account.ido_name = name_data;
        ido_account.ido_authority = ctx.accounts.ido_authority.key();
        ido_account.usdc_mint = ctx.accounts.usdc_mint.key();
        ido_account.service_vault = ctx.accounts.service_vault.key();
        ido_account.total_amount = 0;
        ido_account.nonce = _nonce;
        ido_account.ido_times = ido_times;

        Ok(())
    }

    #[access_control(unrestricted_phase(& ctx.accounts.ido_account))]
    pub fn init_user_stake(ctx: Context<InitUserStake>) -> Result<()> {
        msg!("INIT USER STAKE");
        Ok(())
    }

    #[access_control(unrestricted_phase(& ctx.accounts.ido_account))]
    pub fn stake(ctx: Context<Stake>, amount: u64) -> Result<()>{
        msg!("STAKE USDC");
        // While token::transfer will check this, we prefer a verbose err msg.
        if ctx.accounts.user_usdc.amount < amount {
            return err!(ErrorCode::LowUsdc);
        }
        let ido_account = &mut ctx.accounts.ido_account;
        let user_stake = &mut ctx.accounts.user_stake;
        let now_ts = Clock::get().unwrap().unix_timestamp;

        // Transfer user's USDC to service USDC account.
        let cpi_accounts = Transfer {
            from: ctx.accounts.user_usdc.to_account_info(),
            to: ctx.accounts.service_vault.to_account_info(),
            authority: ctx.accounts.user_authority.to_account_info()
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, amount)?;

        //update ido_account total_amount
        ido_account.total_amount = (ido_account.total_amount as u128)
            .checked_add(amount as u128)
            .unwrap()
            .try_into()
            .unwrap();

        //update user_stake info
        user_stake.authority = ctx.accounts.user_authority.key();
        if user_stake.started_at == 0 {
            user_stake.started_at = now_ts as u64;
        }
        user_stake.updated_at = now_ts as u64;
        user_stake.amount = (user_stake.amount as u128)
            .checked_add(amount as u128)
            .unwrap()
            .try_into()
            .unwrap();

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(ido_name: String)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub ido_authority: Signer<'info>,

    #[account(
    init,
    payer = ido_authority,
    seeds = [ido_name.as_bytes()],
    bump,
    space = IdoAccount::LEN + 8
    )]
    pub ido_account: Box<Account<'info, IdoAccount>>,

    #[account(
    //address = token_constants::USDC_TOKEN_PUBKEY.parse::< Pubkey > ().unwrap(),
    constraint = usdc_mint.decimals == DECIMALS
    )]
    pub usdc_mint: Box<Account<'info, Mint>>,

    #[account(
    constraint = service_vault.mint == usdc_mint.key(),
    )]
    pub service_vault: Box<Account<'info, TokenAccount>>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct InitUserStake<'info> {
    // User Accounts
    #[account(mut)]
    pub user_authority: Signer<'info>,

    #[account(
    init,
    payer = user_authority,
    seeds = [
    user_authority.key().as_ref(),
    ido_account.ido_name.as_ref().trim_ascii_whitespace(),
    constants::USER_STAKE_PDA_SEED.as_ref(),
    ],
    bump,
    space = UserStake::LEN + 8
    )]
    pub user_stake: Box<Account<'info, UserStake>>,

    // IDO Accounts
    #[account(
    seeds = [ido_account.ido_name.as_ref().trim_ascii_whitespace()],
    bump = ido_account.nonce
    )]
    pub ido_account: Box<Account<'info, IdoAccount>>,

    // Programs and Sysvars
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct Stake<'info> {
    // User Accounts
    #[account(mut)]
    pub user_authority: Signer<'info>,
    // IDO Accounts
    #[account(
    mut,
    seeds = [
    ido_account.ido_name.as_ref().trim_ascii_whitespace()
    ],
    bump = ido_account.nonce,
    has_one = usdc_mint,
    has_one = service_vault
    )]
    pub ido_account: Box<Account<'info, IdoAccount>>,
    pub usdc_mint: Box<Account<'info, Mint>>,
    #[account(
    mut,
    constraint = service_vault.mint == usdc_mint.key(),
    )]
    pub service_vault: Box<Account<'info, TokenAccount>>,

    #[account(
    mut,
    seeds = [
    user_authority.key().as_ref(),
    ido_account.ido_name.as_ref().trim_ascii_whitespace(),
    constants::USER_STAKE_PDA_SEED.as_ref(),
    ],
    bump
    )]
    pub user_stake: Box<Account<'info, UserStake>>,

    #[account(
    mut,
    constraint = user_usdc.owner == user_authority.key(),
    constraint = user_usdc.mint == usdc_mint.key()
    )]
    pub user_usdc: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,
}

#[account]
pub struct IdoAccount {
    pub ido_name: [u8; 10],
    //Setting an arbitrary max of ten characters in the ido name. // 10
    pub ido_authority: Pubkey,
    // 32
    pub usdc_mint: Pubkey,
    // 32
    pub service_vault: Pubkey,
    // 32
    pub ido_times: IdoTimes,
    // 16
    pub total_amount: u64,
    // 8
    pub nonce: u8, // 1
}

impl IdoAccount {
    pub const LEN: usize = 10 + 32 + 32 + 32 + 16 + 8 + 1;
}

#[derive(AnchorSerialize, AnchorDeserialize, Default, Clone, Copy)]
pub struct IdoTimes {
    pub start_ido: i64,
    // 8
    pub end_ido: i64,      // 8
}

#[account]
pub struct UserStake {
    pub authority: Pubkey,
    // 32
    pub amount: u64,
    // 8
    pub started_at: u64,
    //8
    pub updated_at: u64, //8
}

impl UserStake {
    pub const LEN: usize = 32 + 8 + 8 + 8;
}

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid action, E5000")]
    PermissionError,
    #[msg("Given nonce is invalid, E1000")]
    InvalidNonce,
    #[msg("IDO has not started, E1001")]
    StartIdoTime,
    #[msg("IDO has ended, E1002")]
    EndIdoTime,
    #[msg("IDO has not finished yet, E1003")]
    IdoNotOver,
    #[msg("Insufficient USDC, E1004")]
    LowUsdc,
    #[msg("IDO times are non-sequential, E1005")]
    SeqTimes,
    #[msg("IDO must start in the future, E1006")]
    IdoFuture,
}

// Access control modifiers
// Asserts the IDO starts in the future
fn validate_ido_times(ido_times: IdoTimes) -> Result<()> {
    let clock = Clock::get()?;
    if ido_times.start_ido <= clock.unix_timestamp {
        return err!(ErrorCode::IdoFuture);
    }
    if ido_times.end_ido <= ido_times.start_ido {
        return err!(ErrorCode::SeqTimes);
    }
    Ok(())
}

// Asserts the IDO is still accepting deposits.
fn unrestricted_phase(ido_account: &IdoAccount) -> Result<()> {
    let clock = Clock::get()?;
    if clock.unix_timestamp <= ido_account.ido_times.start_ido {
        return err!(ErrorCode::StartIdoTime);
    } else if ido_account.ido_times.end_ido <= clock.unix_timestamp {
        return err!(ErrorCode::EndIdoTime);
    }
    Ok(())
}

/// Trait to allow trimming ascii whitespace from a &[u8].
pub trait TrimAsciiWhitespace {
    /// Trim ascii whitespace (based on `is_ascii_whitespace()`) from the
    /// start and end of a slice.
    fn trim_ascii_whitespace(&self) -> &[u8];
}

impl<T: Deref<Target=[u8]>> TrimAsciiWhitespace for T {
    fn trim_ascii_whitespace(&self) -> &[u8] {
        let from = match self.iter().position(|x| !x.is_ascii_whitespace()) {
            Some(i) => i,
            None => return &self[0..0],
        };
        let to = self.iter().rposition(|x| !x.is_ascii_whitespace()).unwrap();
        &self[from..=to]
    }
}
