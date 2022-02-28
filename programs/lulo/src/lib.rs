use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

declare_id!("TFZyJb1CNZzTTHTojYaVYhqtmro9gwoP9HHKCfinUs9");

#[program]
pub mod lulo {
    use super::*;

    /* Initialize program */
    pub fn initialize(ctx: Context<Initialize>, fee: u64, fee_scalar: u64) -> Result<()> {
        let state = &mut ctx.accounts.state;
        state.fee = fee;
        state.fee_scalar = fee_scalar;
        state.admin = ctx.accounts.signer.key();
        Ok(())
    }

    /* Create vault to support SPL token */
    pub fn create_vault(ctx: Context<CreateVault>) -> Result<()> {
        Ok(())
    }

    /* Create a Receivable */
    pub fn create(ctx: Context<Create>, amount_due: u64) -> Result<()> {
        let receivable = &mut ctx.accounts.receivable;
        let clock = Clock::get()?;
        let mint_bump = *ctx.bumps.get("mint").unwrap();
        // Set receivable metadata
        receivable.mint = ctx.accounts.mint.key();
        receivable.create_ts = clock.unix_timestamp;
        receivable.create_slot = clock.slot;
        receivable.amount_due = amount_due;
        receivable.creator = ctx.accounts.signer.key();
        receivable.pay_mint = ctx.accounts.pay_mint.key();
        receivable.paid = false;
        // Mint receivable to creator
        anchor_spl::token::mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                anchor_spl::token::MintTo {
                    mint: ctx.accounts.mint.to_account_info(),
                    to: ctx.accounts.mint_account.to_account_info(),
                    authority: ctx.accounts.mint.to_account_info(),
                },
                &[&[
                    &b"mint"[..],
                    &ctx.accounts.receivable.key().as_ref(),
                    &[mint_bump],
                ]],
            ),
            1,
        )?;
        Ok(())
    }

    /* Sign receivable */
    pub fn sign(ctx: Context<Sign>) -> Result<()> {
        let receivable = &mut ctx.accounts.receivable;
        let clock = Clock::get()?;
        // Require unsigned

        // Set signer metadata
        receivable.signer = ctx.accounts.signer.key();
        receivable.sign_ts = clock.unix_timestamp;
        receivable.sign_slot = clock.slot;
        Ok(())
    }

    /* Pay a receivable */
    pub fn pay(ctx: Context<Pay>) -> Result<()> {
        // Update Receivable state
        let receivable = &mut ctx.accounts.receivable;
        receivable.paid = true;
        // Transfer funds to Vault
        anchor_spl::token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                anchor_spl::token::Transfer {
                    from: ctx.accounts.source.to_account_info(),
                    to: ctx.accounts.vault.to_account_info(),
                    authority: ctx.accounts.signer.to_account_info(),
                },
                &[],
            ),
            receivable.amount_due,
        )?;
        Ok(())
    }

    /* Redeem funds for a paid Receivable */
    pub fn redeem(ctx: Context<Redeem>) -> Result<()> {
        let receivable = &mut ctx.accounts.receivable;
        let vault_bump = *ctx.bumps.get("vault").unwrap();

        // Transfer funds to recipient
        anchor_spl::token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                anchor_spl::token::Transfer {
                    from: ctx.accounts.vault.to_account_info(),
                    to: ctx.accounts.recipient.to_account_info(),
                    authority: ctx.accounts.vault.to_account_info(),
                },
                &[&[
                    &b"vault"[..],
                    &ctx.accounts.pay_mint.key().as_ref(),
                    &[vault_bump],
                ]],
            ),
            receivable.amount_due,
        )?;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        // TODO: Change to init
        init_if_needed,
        payer = signer,
        space = 300,
        seeds = [b"state"],
        bump
    )]
    pub state: Box<Account<'info, State>>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}
#[derive(Accounts)]
pub struct Create<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        init,
        payer = signer,
        space = 350,
    )]
    pub receivable: Box<Account<'info, Receivable>>,
    #[account(
        init,
        payer = signer,
        mint::decimals = 0,
        mint::authority = mint,
        seeds = [b"mint", receivable.key().as_ref()],
        bump
    )]
    pub mint: Box<Account<'info, Mint>>,
    #[account(
        init,
        payer = signer,
        token::mint = mint,
        token::authority = signer,
    )]
    pub mint_account: Box<Account<'info, TokenAccount>>,
    #[account()]
    pub pay_mint: Box<Account<'info, Mint>>,
    #[account(
        mut,
        seeds = [b"vault", pay_mint.key().as_ref()],
        bump
    )]
    pub vault: Box<Account<'info, TokenAccount>>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}
#[derive(Accounts)]
pub struct Sign<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(mut)]
    pub receivable: Box<Account<'info, Receivable>>,
}
#[derive(Accounts)]
pub struct Pay<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        mut,
        constraint = source.mint == pay_mint.key())]
    pub source: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub receivable: Box<Account<'info, Receivable>>,
    #[account(
        mut,
        seeds = [b"vault", pay_mint.key().as_ref()],
        bump
    )]
    pub vault: Box<Account<'info, TokenAccount>>,
    #[account()]
    pub pay_mint: Box<Account<'info, Mint>>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct Redeem<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    /// CHECK: Constraint checks for valid creator pubkey
    #[account(
        mut,
        constraint = creator.key() == receivable.creator
    )]
    pub creator: UncheckedAccount<'info>,
    #[account(
        mut,
        close = creator,
    )]
    pub receivable: Box<Account<'info, Receivable>>,
    #[account(
        mut,
        constraint = nft_account.mint == receivable.mint,
        constraint = nft_account.amount == 1,
    )]
    pub nft_account: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub recipient: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        seeds = [b"vault", pay_mint.key().as_ref()],
        bump
    )]
    pub vault: Box<Account<'info, TokenAccount>>,
    #[account()]
    pub pay_mint: Box<Account<'info, Mint>>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct CreateVault<'info> {
    #[account(
        mut,
        constraint = signer.key() == state.admin)]
    pub signer: Signer<'info>,
    #[account(
        init,
        payer = signer,
        token::mint = mint,
        token::authority = vault,
        seeds = [b"vault", mint.key().as_ref()],
        bump
    )]
    pub vault: Box<Account<'info, TokenAccount>>,
    #[account()]
    pub mint: Box<Account<'info, Mint>>,
    #[account()]
    pub state: Box<Account<'info, State>>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[account]
pub struct Receivable {
    // Token
    mint: Pubkey,
    // Amount due
    amount_due: u64,
    // Payment SPL
    pay_mint: Pubkey,
    // Creator info
    creator: Pubkey,
    create_ts: i64,
    create_slot: u64,
    // Signer info
    signer: Pubkey,
    sign_ts: i64,
    sign_slot: u64,
    // Paid status
    paid: bool,
}
#[account]
pub struct State {
    admin: Pubkey,
    fee: u64,
    fee_scalar: u64,
}
