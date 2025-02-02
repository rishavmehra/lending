use std::f32::consts::E;

use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken, 
    token_interface::{Mint, TokenInterface, TokenAccount, TransferChecked, transfer_checked}
};
use pyth_solana_receiver_sdk::price_update::{get_feed_id_from_hex, PriceUpdateV2};

use crate::{state::*, USDC_USD_FEED_ID};
use crate::constants::{MAXIMUM_AGE, SOL_USD_FEED_ID};
use crate::error::ErrorCode;

#[derive(Accounts)]
pub struct Borrow<'info> {
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
        bump,
    )]
    pub bank_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        seeds=[b"user", signer.key().as_ref()],
        bump
    )]
    pub user_account: Account<'info, User>,

    #[account(
        init_if_needed,
        payer=signer,
        associated_token::mint=mint,
        associated_token::authority=signer,
        associated_token::token_program=token_program,
    )]
    pub user_token_account: InterfaceAccount<'info, TokenAccount>,

    pub price_update: Account<'info, PriceUpdateV2>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl <'info>Borrow<'info> {
    pub fn process_borrow(&mut self, amount: u64, bumps: &BorrowBumps)->Result<()>{
        let bank = &mut self.bank;
        let user = &mut self.user_account;

        let price_update = &mut self.price_update;

        let total_collateral: u64;

        match self.mint.to_account_info().key() {
            key if key == user.usdc_address => {
                let sol_feed_id = get_feed_id_from_hex(SOL_USD_FEED_ID)?; 
                let sol_price = price_update.get_price_no_older_than(&Clock::get()?, MAXIMUM_AGE, &sol_feed_id)?;
                let accrued_interest = calculate_accrued_interest(user.deposit_sol, bank.interest_rate, user.last_updated)?;
                total_collateral = sol_price.price as u64 * (user.deposit_sol + accrued_interest);
            },
            _ => {
                let usdc_feed_id= get_feed_id_from_hex(USDC_USD_FEED_ID)?;
                let usdc_price = price_update.get_price_no_older_than(&Clock::get()?, MAXIMUM_AGE, &usdc_feed_id)?;
                total_collateral = usdc_price.price as u64 * user.deposit_usdc;
            }
        }

        let borrowable_amount = total_collateral as u64 * bank.liquity_threshold;

        if borrowable_amount < amount {
            return  Err(ErrorCode::OverBorrowableAmount.into());
        }

        let transfer_cpi_accounts = TransferChecked{
            from: self.bank_token_account.to_account_info(),
            to: self.user_token_account.to_account_info(),
            mint: self.mint.to_account_info(),
            authority: self.bank_token_account.to_account_info(),
        };

        let cpi_program = self.token_program.to_account_info();
        let mint_key = self.mint.key();

        let signer_seeds: &[&[&[u8]]] = &[
            &[
                b"treasury",
                mint_key.as_ref(),    
                &[bumps.bank_token_account]
            ],
        ];

        let cpi_ctx = CpiContext::new(cpi_program, transfer_cpi_accounts).with_signer(signer_seeds);

        transfer_checked(cpi_ctx, amount, self.mint.decimals)?;

        if bank.total_borrow == 0 {
            bank.total_borrow = amount;
            bank.total_borrow_share = amount;
        }

        let borrow_ratio = amount.checked_div(bank.total_borrow).unwrap();
        let user_shares = bank.total_borrow_share.checked_mul(borrow_ratio).unwrap();

        match self.mint.to_account_info().key() {
            key if key == user.usdc_address => {
                user.borrow_usdc += amount;
                user.borrow_usdc_share += user_shares;
            }
            _ => {
                user.borrow_sol += amount;
                user.borrow_sol_share += user_shares;
            }
        }
        Ok(())
    }
}

fn calculate_accrued_interest(deposited: u64, interest_rate: u64, last_update: i64) -> Result<u64>{
    let current_time = Clock::get()?.unix_timestamp;
    let time_elapsed = current_time - last_update;
    let new_value = (deposited as f64 * E.powf(interest_rate as f32 * time_elapsed as f32) as f64) as u64;
    Ok(new_value)
}

