use std::f32::consts::E;

use anchor_lang::prelude::*;
use anchor_spl::{associated_token::AssociatedToken, token_interface::{transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked}};
use pyth_solana_receiver_sdk::price_update::{get_feed_id_from_hex, PriceUpdateV2};

use crate::{error::ErrorCode, Bank, User, MAXIMUM_AGE, SOL_USD_FEED_ID, USDC_USD_FEED_ID};

#[derive(Accounts)]
pub struct Liquidate<'info>{
    #[account(mut)]
    pub liquidator: Signer<'info>,
    pub price_update: Account<'info, PriceUpdateV2>,
    pub collateral_mint: InterfaceAccount<'info, Mint>,
    pub borrowed_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        seeds=[b"bank", collateral_mint.key().as_ref()],
        bump
    )]
    pub collateral_bank: Account<'info, Bank>,

    #[account(
        mut,
        seeds=[b"treasury", collateral_mint.key().as_ref()],
        bump
    )]
    pub collateral_bank_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut, 
        seeds = [b"borrowed", borrowed_mint.key().as_ref()],
        bump,
    )]  
    pub borrowed_bank: Account<'info, Bank>,


    #[account(
        mut,
        seeds=[b"treasury", borrowed_mint.key().as_ref()],
        bump,
    )]
    pub borrowed_bank_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        seeds=[liquidator.key().as_ref()],
        bump,
    )]
    pub user_account: Account<'info, User>,

    #[account(
        init_if_needed,
        payer=liquidator,
        associated_token::mint=collateral_mint,
        associated_token::authority=liquidator,
        associated_token::token_program=token_program,
    )]
    pub liquidator_collateral_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer=liquidator,
        associated_token::mint=borrowed_mint,
        associated_token::authority=liquidator,
        associated_token::token_program=token_program,
    )]
    pub liquidator_borrowed_token_account: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}


impl <'info>Liquidate<'info> {
    pub fn process_liquidate(&mut self, bumps: &LiquidateBumps) -> Result<()>{
        let collateral_bank = &mut self.collateral_bank;
        let user = &mut self.user_account;
        let borrowed_bank = &mut self.borrowed_bank;

        let price_update = &mut self.price_update;

        let sol_feed_id = get_feed_id_from_hex(SOL_USD_FEED_ID)?;
        let usdc_feed_id = get_feed_id_from_hex(USDC_USD_FEED_ID)?;

        let sol_price = price_update.get_price_no_older_than(&Clock::get()?, MAXIMUM_AGE, &sol_feed_id)?;
        let usdc_price = price_update.get_price_no_older_than(&Clock::get()?, MAXIMUM_AGE, &usdc_feed_id)?;
        
        let total_collateral: u64;
        let total_borrowed: u64;

        match self.collateral_mint.key() {
            key if key == user.usdc_address =>{
                let new_usdc = calculate_accrued_interest(user.deposit_usdc, collateral_bank.interest_rate, user.last_updated)?;
                total_collateral= usdc_price.price as u64 * new_usdc;
                let new_sol = calculate_accrued_interest(user.deposit_sol, borrowed_bank.interest_rate, user.last_updated_borrowed)?;
                total_borrowed= sol_price.price as u64 * new_sol;   
            }
            _=> {
                let new_sol = calculate_accrued_interest(user.deposit_sol, collateral_bank.interest_rate, user.last_updated)?;
                total_collateral= sol_price.price as u64 * new_sol;
                let new_usdc = calculate_accrued_interest(user.deposit_usdc, borrowed_bank.interest_rate, user.last_updated_borrowed)?;
                total_borrowed= usdc_price.price as u64 * new_usdc;
            }
        }

        let health_factor = ((total_collateral as f64 * collateral_bank.liquity_threshold as f64)/total_borrowed as f64) as f64;

        if health_factor >= 1.0 {
            return Err(ErrorCode::NotUnderCollecteralized.into());
        }

        let transfer_to_bank= TransferChecked{
            from: self.liquidator_borrowed_token_account.to_account_info(),
            to: self.borrowed_bank_token_account.to_account_info(),
            mint: self.borrowed_mint.to_account_info(),
            authority: self.liquidator.to_account_info()
        };

        let cpi_program = self.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program.clone(), transfer_to_bank);
        let decimals = self.borrowed_mint.decimals;

        let liquidation_amount = total_borrowed.checked_mul(borrowed_bank.liquity_close_factor).unwrap();
        transfer_checked(cpi_ctx, liquidation_amount, decimals);

        
        let transfer_to_liquidator = TransferChecked {
            from: self.collateral_bank_token_account.to_account_info(),
            to: self.liquidator_collateral_token_account.to_account_info(),
            mint: self.collateral_mint.to_account_info(),
            authority: self.collateral_bank_token_account.to_account_info()
        };
        let liquidator_amount = (liquidation_amount * collateral_bank.liquity_bonus) + liquidation_amount;
        
        let mint_key = self.collateral_mint.key();
        let signer_seeds: &[&[&[u8]]] =&[ 
                &[
                    b"treasury",
                    mint_key.as_ref(),
                    &[bumps.collateral_bank_token_account]
                ]
            ];
            
            let cpi_ctx_to_liquidator = CpiContext::new(cpi_program.clone(), transfer_to_liquidator).with_signer(signer_seeds);
            let collateral_decimal = self.collateral_mint.decimals;
            transfer_checked(cpi_ctx_to_liquidator, liquidator_amount, collateral_decimal)?;
        Ok(())
    }
}

fn calculate_accrued_interest(deposited: u64, interest_rate: u64, last_update: i64) -> Result<u64>{
    let current_time = Clock::get()?.unix_timestamp;
    let time_elapsed = current_time - last_update;
    let new_value = (deposited as f64 * E.powf(interest_rate as f32 * time_elapsed as f32) as f64) as u64;
    Ok(new_value)
}
