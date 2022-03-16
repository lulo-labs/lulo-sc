use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
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
    pub fn create(ctx: Context<Create>, amount_due: u64, due_date: i64) -> Result<()> {
        let contract = &mut ctx.accounts.contract;
        let clock = Clock::get()?;
        let mint_bump = *ctx.bumps.get("mint").unwrap();

        // Due date is in the future
        require!(due_date > clock.unix_timestamp, ErrorCode::InvalidDueDate);

        // Set contract metadata
        contract.recipient = ctx.accounts.recipient.key();
        contract.mint = ctx.accounts.mint.key();
        contract.due_date = due_date;
        contract.create_ts = clock.unix_timestamp;
        contract.create_slot = clock.slot;
        contract.amount_due = amount_due;
        contract.creator = ctx.accounts.signer.key();
        contract.pay_mint = ctx.accounts.pay_mint.key();
        contract.status = 0;
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

    /* Set an account as an approver for an address */
    pub fn set_approver(ctx: Context<SetApprover>) -> Result<()> {
        let approver = &mut ctx.accounts.approver;
        approver.admin = ctx.accounts.signer.key();
        approver.key = ctx.accounts.delegate.key();
        Ok(())
    }

    /* Approve contract */
    pub fn approve(ctx: Context<Approve>) -> Result<()> {
        let contract = &mut ctx.accounts.contract;
        let approver = &mut ctx.accounts.approver;
        let clock = Clock::get()?;
        // Require contract is unsigned
        require!(
            contract.approver.eq(&Pubkey::default()),
            ErrorCode::ExistingApproval
        );
        // Signer is the wallet recipient
        if contract.recipient.eq(&ctx.accounts.signer.key()) {
            // Set approver metadata
            contract.approver = ctx.accounts.signer.key();
            contract.approve_ts = clock.unix_timestamp;
            contract.approve_slot = clock.slot;
            contract.status = 1;
        }
        // Signer is an approver of recipient
        else if approver.admin.eq(&contract.recipient)
            && approver.key.eq(&ctx.accounts.signer.key())
        {
            // Set approver metadata
            contract.approver = ctx.accounts.signer.key();
            contract.approve_ts = clock.unix_timestamp;
            contract.approve_slot = clock.slot;
            contract.status = 1;
        }
        // Unauthorized approver
        else {
            return Err(ErrorCode::UnauthorizedApprover.into());
        }
        Ok(())
    }

    /* Pay a contract */
    pub fn pay(ctx: Context<Pay>) -> Result<()> {
        // Update Contract state
        let contract = &mut ctx.accounts.contract;
        let clock = Clock::get()?;
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
        contract.payer = ctx.accounts.signer.key();
        contract.pay_ts = clock.unix_timestamp;
        contract.pay_slot = clock.slot;
        contract.status = 2;
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
        // Burn contract token
        anchor_spl::token::burn(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                anchor_spl::token::Burn {
                    mint: ctx.accounts.mint.to_account_info(),
                    to: ctx.accounts.nft_account.to_account_info(),
                    authority: ctx.accounts.signer.to_account_info(),
                },
                &[],
            ),
            1,
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
    /// CHECK: TODO: Use AccountInfo
    pub recipient: UncheckedAccount<'info>,
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
        associated_token::mint = mint,
        associated_token::authority = signer,
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
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}
#[derive(Accounts)]
pub struct Approve<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(mut)]
    pub contract: Box<Account<'info, Contract>>,
    #[account(mut)]
    pub approver: Box<Account<'info, Approver>>,
}

#[derive(Accounts)]
pub struct SetApprover<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    /// CHECK: TODO: Use AccountInfo?
    pub delegate: UncheckedAccount<'info>,
    #[account(
        init,
        payer = signer,
        space = 200,
        seeds = [b"approver", signer.key().as_ref(), delegate.key().as_ref()],
        bump
    )]
    pub approver: Box<Account<'info, Approver>>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct Pay<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        mut,
        constraint = source.mint == pay_mint.key())]
    pub source: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        constraint = signer.key() == contract.recipient)]
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
        constraint = nft_account.owner == signer.key(),
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
    #[account(mut)]
    pub mint: Box<Account<'info, Mint>>,
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
    due_date: i64,
    // Creator info
    creator: Pubkey,
    create_ts: i64,
    create_slot: u64,
    // Approver info
    approver: Pubkey,
    approve_ts: i64,
    approve_slot: u64,
    // Paid status TODO: Do this with a type, or keep track of status codes in account
    status: u64,
    // Payer info
    payer: Pubkey,
    pay_ts: i64,
    pay_slot: u64,
}

#[account]
pub struct Approver {
    admin: Pubkey,
    key: Pubkey,
    balance: u64,
    budget: u64,
    budget_mint: Pubkey,
}

#[account]
pub struct State {
    admin: Pubkey,
    fee: u64,
    fee_scalar: u64,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Contract is already approved")]
    ExistingApproval,
    #[msg("Not an authorized approver")]
    UnauthorizedApprover,
    #[msg("Due date must be in the future")]
    InvalidDueDate,
}
