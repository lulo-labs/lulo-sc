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

    /* Create a Contract */
    pub fn create(ctx: Context<Create>, amount_due: u64) -> Result<()> {
        let contract = &mut ctx.accounts.contract;
        let clock = Clock::get()?;
        let mint_bump = *ctx.bumps.get("mint").unwrap();
        // Set contract metadata
        contract.mint = ctx.accounts.mint.key();
        contract.create_ts = clock.unix_timestamp;
        contract.create_slot = clock.slot;
        contract.amount_due = amount_due;
        contract.creator = ctx.accounts.signer.key();
        contract.pay_mint = ctx.accounts.pay_mint.key();
        contract.paid = false;
        // Mint contract to creator
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
                    &ctx.accounts.contract.key().as_ref(),
                    &[mint_bump],
                ]],
            ),
            1,
        )?;
        Ok(())
    }

    /* Sign contract */
    pub fn sign(ctx: Context<Sign>) -> Result<()> {
        let contract = &mut ctx.accounts.contract;
        let clock = Clock::get()?;
        // Require unsigned

        // Set approver metadata
        contract.approver = ctx.accounts.signer.key();
        contract.approve_ts = clock.unix_timestamp;
        contract.approve_slot = clock.slot;
        Ok(())
    }

    /* Pay a contract */
    pub fn pay(ctx: Context<Pay>) -> Result<()> {
        // Update Contract state
        let contract = &mut ctx.accounts.contract;
        contract.paid = true;
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
            contract.amount_due,
        )?;
        Ok(())
    }

    /* Redeem funds for a paid Contract */
    pub fn redeem(ctx: Context<Redeem>) -> Result<()> {
        let contract = &mut ctx.accounts.contract;
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
            contract.amount_due,
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
    pub contract: Box<Account<'info, Contract>>,
    #[account(
        init,
        payer = signer,
        mint::decimals = 0,
        mint::authority = mint,
        seeds = [b"mint", contract.key().as_ref()],
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
    pub contract: Box<Account<'info, Contract>>,
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
    pub contract: Box<Account<'info, Contract>>,
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
        constraint = creator.key() == contract.creator
    )]
    pub creator: UncheckedAccount<'info>,
    #[account(
        mut,
        close = creator,
    )]
    pub contract: Box<Account<'info, Contract>>,
    #[account(
        mut,
        constraint = nft_account.mint == contract.mint,
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
pub struct Contract {
    // Recipient sent the contract
    recipient: Pubkey,
    // Token representing this contract
    mint: Pubkey,
    // SPL to settle funds
    pay_mint: Pubkey,
    // Amount due
    amount_due: u64,
    // Due date
    due_date: u64,
    // Creator info
    creator: Pubkey,
    create_ts: i64,
    create_slot: u64,
    // Approver info
    approver: Pubkey,
    approve_ts: i64,
    approve_slot: u64,
    // Paid status
    paid: bool,
    // Payer info
    payer: Pubkey,
    pay_ts: i64,
    pay_slot: u64,
}
#[account]
pub struct State {
    admin: Pubkey,
    fee: u64,
    fee_scalar: u64,
}
