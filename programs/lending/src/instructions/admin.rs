use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use crate::{Bank, User, ANCHOR_DISCRIMINATOR};

#[derive(Accounts)]
pub struct InitBank<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,

    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        init,
        payer=signer,
        space= ANCHOR_DISCRIMINATOR + Bank::INIT_SPACE,
        seeds= [b"bank", mint.key().as_ref()],
        bump
    )]
    pub bank: Account<'info, Bank>,

    #[account(
        init, 
        payer=signer,
        token::mint=mint,
        token::authority=bank_token_account,
        seeds=[b"treasury",mint.key().as_ref()],
        bump
    )]
    pub bank_token_account: InterfaceAccount<'info, TokenAccount>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
pub struct InitUser<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,

    #[account(
        init,
        payer=signer,
        space=ANCHOR_DISCRIMINATOR+User::INIT_SPACE,
        seeds=[b"user", signer.key().as_ref()],
        bump,
    )]
    pub user_account: Account<'info, User>,
    pub system_program: Program<'info, System>,
}

impl <'info>InitBank<'info> {
    pub fn process_init_bank(&mut self, liquidation_threshold:u64, max_ltv:u64 )-> Result<()>{
        let bank = &mut self.bank;
        bank.mint_address = self.mint.key();
        bank.authority = self.signer.key();
        bank.liquity_threshold =liquidation_threshold;
        bank.max_ltv = max_ltv;
        Ok(())
    }
}

impl <'info>InitUser<'info> {
    pub fn process_init_user(&mut self, usdc_address: Pubkey)->Result<()>{
        let user_account = &mut self.user_account;
        user_account.owner = self.signer.key();
        user_account.usdc_address = usdc_address;
        Ok(())
    }
}

