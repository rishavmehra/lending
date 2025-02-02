use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken, 
    token_interface::{Mint, TokenAccount, TokenInterface,  TransferChecked, transfer_checked}
};

use crate::{Bank, User};

#[derive(Accounts)]
pub struct Deposit<'info>{
    #[account(mut)]
    pub signer: Signer<'info>,

    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        seeds=[b"bank", mint.key().as_ref()],
        bump
    )]
    pub bank: Account<'info, Bank>,

    #[account(
        mut,
        seeds=[b"treasury", mint.key().as_ref()],
        bump
    )]
    pub bank_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        seeds=[b"user", signer.key().as_ref()],
        bump
    )]
    pub user_account: Account<'info, User>,

    #[account(
        mut,
        associated_token::mint=mint,
        associated_token::authority=signer,
        associated_token::token_program=token_program
    )]
    pub user_token_account: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_account: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>
}

impl <'info>Deposit<'info> {
    pub fn process_deposit(&mut self, amount: u64)->Result<()>{
        let transfer_cpi_accounts = TransferChecked{
            from: self.user_token_account.to_account_info(),
            to: self.bank_token_account.to_account_info(),
            authority: self.signer.to_account_info(),
            mint: self.mint.to_account_info()
        };

        let cpi_program = self.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, transfer_cpi_accounts);

        let decimals = self.mint.decimals;

        transfer_checked(cpi_ctx, amount, decimals)?;

        let bank = &mut self.bank;

        if bank.total_deposit == 0 {
            bank.total_deposit = amount;
            bank.total_deposit_share = amount;
        }

        let deposit_ratio = amount.checked_div(bank.total_deposit).unwrap(); // 10
        let user_share = bank.total_deposit_share.checked_mul(deposit_ratio).unwrap();

        let user = &mut self.user_account;

        match self.mint.to_account_info().key(){
            key if key == user.usdc_address => {
                user.deposit_usdc += amount;
                user.deposit_usdc_share += user_share;
            },
            _ => {
                user.deposit_sol += amount;
                user.deposit_sol_share += user_share;
            }
        }

        bank.total_deposit += amount;
        bank.total_deposit_share += user_share;


        user.last_updated = Clock::get()?.unix_timestamp;

        Ok(())
    }
}
