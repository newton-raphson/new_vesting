use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, SetAuthority, TokenAccount, Transfer};
use spl_token::instruction::AuthorityType;


declare_id!("Bgr3WiJ2PpKZW6mun2a6JCDugfZ8ZaRfJhuhQsV9KQHz");

#[program]
pub mod new_vesting {
    use super::*;
    pub fn initialize(ctx: Context<Initialize>,
    schedule:Vec<Schedule> ,
    total_amount:u64) -> ProgramResult 
    {
        ctx.accounts.escrow_account.sender_key = *ctx.accounts.sender.key;
        ctx.accounts.escrow_account.receiver_key = *ctx.accounts.receiver_account.key;
        ctx.accounts.escrow_account.mint_address=*ctx.accounts.mint.to_account_info().key;
        ctx.accounts.escrow_account.schedule=schedule;
        ctx.accounts.escrow_account.total_amount=total_amount;
        let (vault_authority, _vault_authority_bump) =
        Pubkey::find_program_address(&[&ctx.accounts.receiver_account.key.to_bytes()], ctx.program_id);
       
        token::set_authority(
        ctx.accounts.into_set_authority_context(),
        AuthorityType::AccountOwner,
        Some(vault_authority),)?;
    
        token::transfer(
            ctx.accounts.into_transfer_to_pda_context(),
            ctx.accounts.escrow_account.vested_amount,
        )?;    
        Ok(())
    }
    pub fn unlock(ctx:Context<UnLock>)->ProgramResult
    {
        let (_vault_authority, vault_authority_bump) =Pubkey::find_program_address(&[&ctx.accounts.receiver.key.to_bytes()], ctx.program_id);
        let authority_seeds= &[&ctx.accounts.receiver.key.to_bytes()[..], &[vault_authority_bump]];
        
        let time_now = Clock::get().unwrap().unix_timestamp as u64;

        let escrow= ctx.accounts.escrow_account.as_mut();
        let mut transfer_amount =0;

        for scheduling in escrow.schedule.iter_mut() {
            if time_now>= scheduling.time {
                transfer_amount+= scheduling.amount;
                scheduling.amount = 0;
            }
        }
        if transfer_amount == 0 {
            msg!("Vesting contract has not yet reached release time");
            return Err(ProgramError::InvalidArgument);
        }

        token::transfer(
        ctx.accounts.into_transfer_to_taker_context().with_signer(&[&authority_seeds[..]]),
        transfer_amount,)?;
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(vested_amount: u64,unlock_time:u64 )]
pub struct Initialize<'info> {
    #[account(mut, signer)]
    pub sender: AccountInfo<'info>,
    pub mint: Account<'info, Mint>,
    #[account(
        init,
        payer = sender,
        token::mint = mint,
        token::authority = sender,
    )]
    pub pda_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = sender_token_account.amount >= vested_amount
    )]
    pub sender_token_account: Account<'info, TokenAccount>,
    pub receiver_account: AccountInfo<'info>,
    #[account(zero)]
    pub escrow_account: Box<Account<'info, EscrowAccount>>,
    pub system_program: SystemAccount<'info>,
    pub rent: Sysvar<'info, Rent>,
    #[account(
        constraint = token_program.key != &spl_token::id()
    )]
    pub token_program: AccountInfo<'info>,
   
}


#[derive(Accounts)]
pub struct UnLock<'info> {
    #[account(mut, signer)]
    pub receiver: AccountInfo<'info>,
    pub sender: AccountInfo<'info>,
    pub receiver_token_account: Account<'info, TokenAccount>,
    pub mint: Account<'info, Mint>,
    pub pda_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = escrow_account.sender_key == *sender.key,
        constraint = escrow_account.receiver_key== *receiver.key,
    )]
    pub escrow_account: Box<Account<'info, EscrowAccount>>,
    pub system_program: SystemAccount<'info>,
    pub vault_authority: AccountInfo<'info>,
    pub rent: Sysvar<'info, Rent>,
    #[account(
        constraint = token_program.key != &spl_token::id()
    )]
    pub token_program: AccountInfo<'info>,
  
}

#[account]
pub struct EscrowAccount {
    pub sender_key: Pubkey,
    pub receiver_key: Pubkey,
    pub vested_amount: u64,
    pub unlock_time: u64,
    pub mint_address:Pubkey,
    pub schedule: Vec<Schedule>,
    pub total_amount:u64,
}
#[account]
pub struct Schedule {
pub time:u64,
pub amount:u64,
}
impl<'info> Initialize<'info> {
    fn into_transfer_to_pda_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self
                .sender_token_account
                .to_account_info()
                .clone(),
            to: self.pda_token_account.to_account_info().clone(),
            authority: self.sender.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }

    fn into_set_authority_context(&self) -> CpiContext<'_, '_, '_, 'info, SetAuthority<'info>> {
        let cpi_accounts = SetAuthority {
            account_or_mint: self.pda_token_account.to_account_info().clone(),
            current_authority: self.sender.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }
}

impl<'info> UnLock<'info> {
    fn into_transfer_to_taker_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.pda_token_account.to_account_info().clone(),
            to: self.receiver_token_account.to_account_info().clone(),
            authority: self.vault_authority.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }
}

