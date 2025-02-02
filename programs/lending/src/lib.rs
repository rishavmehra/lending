pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("3dPENSGTAQ9XCpjUbay57ExdmdrXJakYAYMCWqmuuVwV");

#[program]
pub mod lending {
    use super::*;

    pub fn initialize_bank(ctx: Context<InitBank>, liquidation_threshold: u64,max_ltv: u64 ) -> Result<()> {
        ctx.accounts.process_init_bank(liquidation_threshold, max_ltv)
    }

    pub fn initialize_user(ctx: Context<InitUser>, usdc_address: Pubkey)->Result<()>{
        ctx.accounts.process_init_user(usdc_address)
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64)->Result<()>{
        ctx.accounts.process_deposit(amount)
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()>{
        ctx.accounts.process_withdraw(amount, &ctx.bumps)
    }

    pub fn borrow(ctx: Context<Borrow>, amount: u64) -> Result<()>{
        ctx.accounts.process_borrow(amount, &ctx.bumps)
    }

    pub fn repay(ctx: Context<Repay>, amount: u64) -> Result<()>{
        ctx.accounts.process_repay(amount)
    }

}