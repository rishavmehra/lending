use std::f32::consts::E;

use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken, 
    token_interface::{Mint, TokenAccount, TokenInterface, TransferChecked, transfer_checked}
};

use crate::{Bank, User};
use crate::error::ErrorCode;

#[derive(Accounts)]
pub struct Withdraw<'info>{
    #[account(mut)]
    pub signer: Signer<'info>,

    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        seeds=[b"bank",mint.key().as_ref()],
        bump
    )]
    pub bank:Account<'info, Bank>,

    #[account(
        mut,
        seeds=[b"treasury", mint.key().as_ref()],
        bump
    )]
    pub bank_token_account: InterfaceAccount<'info, TokenAccount>,


    #[account(
        mut,
        seeds=[b"user", signer.key().as_ref()],
        bump,
    )]
    pub user_account: Account<'info, User>,

    #[account( 
        init_if_needed,
        payer=signer, 
        associated_token::mint = mint, 
        associated_token::authority = signer,
        associated_token::token_program = token_program,
    )]
    pub user_token_account: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

impl <'info>Withdraw<'info> {
    pub fn process_withdraw(&mut self, amount: u64, bumps: &WithdrawBumps )->Result<()> {
        let user = &mut self.user_account;

        let deposite_value;
        if self.mint.to_account_info().key() == user.usdc_address{
            deposite_value=user.deposit_usdc;
        } else {
            deposite_value=user.deposit_sol;
        }

        if amount > deposite_value {
            return Err(ErrorCode::InsufficientFunds.into());
        }

        let time_diff = user.last_updated-Clock::get()?.unix_timestamp;

        let bank = &mut self.bank;
        // Continuous Compound Interest: A = P × e^rt
        bank.total_deposit = (bank.total_deposit as f64 * E.powf(bank.interest_rate as f32 * time_diff as f32) as f64) as u64;

        let value_per_share = bank.total_deposit as f64 / bank.total_deposit_share as f64;

        let user_value = deposite_value as f64 / value_per_share;

        if user_value < amount as f64 {
            return Err(ErrorCode::InsufficientFunds.into());
        }

        let transfer_cpi_accounts = TransferChecked{
            from: self.bank_token_account.to_account_info(),
            to: self.user_account.to_account_info(),
            authority: self.bank_token_account.to_account_info(),
            mint: self.mint.to_account_info()
        };

        let cpi_program = self.token_program.to_account_info();
        let mint_key = self.mint.key();

        let signer_seeds:&[&[&[u8]]]= &[&[
            b"treasury",
            mint_key.as_ref(),
            &[bumps.bank_token_account],
        ]];

        let cpi_ctx = CpiContext::new(cpi_program, transfer_cpi_accounts).with_signer(signer_seeds);

        let decimals = self.mint.decimals; 

        transfer_checked(cpi_ctx, amount, decimals)?;

        let bank = &mut self.bank;

        let shares_to_remove = (amount as f64/bank.total_deposit as f64) * bank.total_deposit_share as f64;

        let user = &mut self.user_account;

        if self.mint.to_account_info().key() == user.usdc_address {
            user.deposit_usdc -= shares_to_remove as u64;
        } else {
            user.deposit_sol -= shares_to_remove as u64;
        }

        bank.total_deposit -= amount;
        bank.total_deposit_share -= shares_to_remove as u64;

        Ok(())
    }
}

