use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken, 
    token_interface::{TokenAccount, TokenInterface, Mint, TransferChecked, transfer_checked}
};

use crate::{ Bank, User};
use crate::error::ErrorCode;

#[derive(Accounts)]
pub struct Repay<'info>{
    #[account(mut)]
    pub signer: Signer<'info>,

    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        seeds= [b"bank", mint.key().as_ref()],
        bump
    )]
    pub bank: Account<'info, Bank>,

    #[account(
        mut,
        seeds= [b"treasury",mint.key().as_ref()],
        bump,
    )]
    pub bank_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        seeds= [b"user", signer.key().as_ref()],
        bump
    )]
    pub user_account: Account<'info, User>,

    #[account(
        mut,
        associated_token::mint=mint,
        associated_token::authority=signer,
        associated_token::token_program=token_program,
    )]
    pub user_token_account: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,

}


impl <'info>Repay<'info> {
    pub fn process_repay(&mut self, amount: u64)-> Result<()>{
        let user = &mut self.user_account;

        let borrowed_asset;

        match self.mint.to_account_info().key() {
            key if key == user.usdc_address => {
                borrowed_asset = user.borrow_usdc;
            }
            _ => {
                borrowed_asset = user.borrow_sol;
            }
        }

        if amount > borrowed_asset {
            return Err(ErrorCode::OverBorrowableAmount.into());
        }

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

        let borrowed_ratio = amount.checked_div(bank.total_borrow).unwrap();
        let user_shares = bank.total_borrow_share.checked_mul(borrowed_ratio).unwrap();

        match self.mint.to_account_info().key() {
            key if key == user.usdc_address => {
                user.borrow_usdc -= amount;
                user.borrow_usdc_share -= user_shares;
            }
            _=>{
                user.borrow_sol -= amount;
                user.borrow_sol_share -= user_shares;
            }
        }

        bank.total_borrow -= amount;
        bank.total_borrow_share -= user_shares;

        Ok(())

    }

}