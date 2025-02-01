use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct User {
    pub owner: Pubkey,
    pub deposit_sol: u64,
    pub deposit_sol_share: u64,
    pub borrow_sol: u64,
    pub borrow_sol_share: u64,
    pub deposit_usdc: u64,
    pub deposit_usdc_share: u64,
    pub borrow_usdc: u64,
    pub borrow_usdc_share: u64,
    pub usdc_address: Pubkey,
    pub last_updated: u64
}

#[account]
#[derive(InitSpace)]
pub struct Bank{
    pub authority: Pubkey,
    pub mint_address: Pubkey,
    pub total_deposit: u64,
    pub total_deposit_share: u64,
    pub liquity_threshold: u64,
    pub liquity_bonus: u64,
    pub liquity_close_factor: u64,
    pub max_ltv: u64,
    pub last_updated: u64,
}
