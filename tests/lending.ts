import {describe, it} from "node:test";
import { BanksClient, ProgramTestContext, startAnchor } from 'solana-bankrun'
import { clusterApiUrl, Connection, PublicKey } from "@solana/web3.js";
import { BankrunProvider } from 'anchor-bankrun'

import IDL from "../target/idl/lending.json";
import { BankrunContextWrapper } from '../bankrun-utils/bankrunConnection';
import { PythSolanaReceiver } from '@pythnetwork/pyth-solana-receiver';
import { Lending } from '../target/types/lending';
import { BN, Program } from '@coral-xyz/anchor';
import { Keypair } from '@solana/web3.js';
import { createAccount, createMint, mintTo } from 'spl-token-bankrun';
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";

describe("lending program tests",  async () => {
    const devnetConnection = new Connection(clusterApiUrl("devnet"), "confirmed") 
    const PROJECT_PATH=""
    
    let signer: Keypair;
    let provider: BankrunProvider;
    let context: ProgramTestContext;
    let bankrunContextWrapper: BankrunContextWrapper;
    let program: Program<Lending>;
    let banksClient: BanksClient;
    let usdcBankAccount: PublicKey;
    let solBankAccount: PublicKey;

    const pyth = new PublicKey('7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE');
    const accountInfo = await devnetConnection.getAccountInfo(pyth);

    context = await startAnchor(
        PROJECT_PATH,
        [{name:"lending", programId: new PublicKey(IDL.address)}],
        [{
            address:pyth, 
            info: accountInfo
        }]
    )
    
    provider = new BankrunProvider(context);
    bankrunContextWrapper = new BankrunContextWrapper(context);

    const connection = bankrunContextWrapper.connection.toConnection();
    const pythSolanaReceiver = new PythSolanaReceiver({
        connection, 
        wallet: provider.wallet
    })

    const SOL_FEED_ID = "0xeaa020c61cc479712813461ce153894a96a6c00b21ed0cfc2798d1f9a9e9c94a";
    const solUsdPriceFeedAccount = pythSolanaReceiver.getPriceFeedAccountAddress(0, SOL_FEED_ID).toBase58();
    const solUsdPriceFeedAccountPubKey = new PublicKey(solUsdPriceFeedAccount)

    const feedAccountInfo = await devnetConnection.getAccountInfo(solUsdPriceFeedAccountPubKey);

    context.setAccount(solUsdPriceFeedAccountPubKey, feedAccountInfo);

    console.log(`Price Feed: ${solUsdPriceFeedAccount}`);
    console.log(`Pyth Account Info: ${JSON.stringify(accountInfo)}`);

    program = new Program<Lending>(IDL as Lending, provider);

    banksClient = context.banksClient;
    signer = provider.wallet.payer;

    const mintUSDC = await createMint(
        //@ts-ignore
        banksClient,
        signer,
        signer.publicKey,
        null,
        2
    );

    const mintSOL = await createMint(
        //@ts-ignore
        banksClient,
        signer,
        signer.publicKey,
        null,
        2
    );

    [usdcBankAccount] = PublicKey.findProgramAddressSync(
        [Buffer.from('treasury'), mintUSDC.toBuffer()],
        program.programId
    );

    [solBankAccount] = PublicKey.findProgramAddressSync(
        [Buffer.from('treasury'), mintSOL.toBuffer()],
        program.programId
    );

    console.log("USDC Bank Account: ", usdcBankAccount.toBase58());
    console.log("SOL Bank Account: ", solBankAccount.toBase58());
    
    it("Test init User", async() =>{
        const initUserTx = await program.methods
            .initializeUser(mintUSDC)
            .accounts({
                signer: signer.publicKey
            })
            .rpc({commitment:"confirmed"})
        console.log("Create User Account: ", initUserTx);
    })

    it("test init and Fund USDC Bank ",  async() =>{
        const initUSDCBankTx = await program.methods
        .initializeBank(new BN(1), new BN(1))
        .accounts({
            mint: mintUSDC,
            signer: signer.publicKey,
            tokenProgram: TOKEN_PROGRAM_ID
        })
        .rpc({commitment:"confirmed"})
        console.log("Create USDC Bank Account: ", initUSDCBankTx);

        const amount = 10_000 * 10 ** 9;

        const mintTx = await mintTo(
            // @ts-ignore
            banksClient,
            signer,
            mintUSDC, 
            usdcBankAccount,
            signer,
            amount
        )
        console.log("Mint to USDC Bank Signature: ", mintTx);
    });

    it("test init and Fund SOL Bank ",  async() =>{
        const initSOLBankTx = await program.methods
        .initializeBank(new BN(1), new BN(1))
        .accounts({
            signer: signer.publicKey,
            mint: mintSOL,
            tokenProgram: TOKEN_PROGRAM_ID
        })
        .rpc({commitment:"confirmed"})
        console.log("Create SOL Bank Account: ", initSOLBankTx);

        const amount = 10_000 * 10 ** 9;
        const mintTx = await mintTo(
            // @ts-ignore
            banksClient,
            signer,
            mintSOL, 
            solBankAccount,
            signer,
            amount
        )
        console.log("Mint to SOL Bank Signature: ", mintTx);
    })

    it("Create and Fund Token Account", async()=>{
        const USDCTokenAccount = await createAccount(
            // @ts-ignore
            banksClient,
            signer,
            mintUSDC,
            signer.publicKey
        )
        console.log("USDC Token Account Created: ", USDCTokenAccount);

        const amount = 10_000 * 10 ** 9;

        const mintUSDCTx = await mintTo(
            // @ts-ignore
            banksClient,
            signer,
            mintUSDC,
            USDCTokenAccount,
            signer,
            amount
        )
        console.log("Mint to USDC Bank Signature:", mintUSDCTx);
    })

    it("Test Deposite", async()=>{
        const depositeUSDC = await program.methods
        .deposit(new BN(10000000000))
        .accounts({
            signer: signer.publicKey,
            mint: mintUSDC,
            tokenProgram: TOKEN_PROGRAM_ID
        })
        .rpc({commitment: "confirmed"});
        console.log("Deposite USDC: ", depositeUSDC);
    })

    it("Test Borrow", async()=>{
        const borrowSOl = await program.methods
        .borrow(new BN(1))
        .accounts({
            signer: signer.publicKey,
            mint: mintSOL,
            tokenProgram: TOKEN_PROGRAM_ID,
            priceUpdate: solUsdPriceFeedAccount
        })
        .rpc({commitment: "confirmed"});
        console.log("Borrow Sol: ", borrowSOl);
    })

    it("Test Repay: ", async () =>{
        const repaySol = await program.methods
        .repay(new BN(1))
        .accounts({
            signer: signer.publicKey,
            mint: mintSOL,
            tokenProgram: TOKEN_PROGRAM_ID,
        })
        .rpc({commitment: "confirmed"});
        console.log("Repay Sol: ", repaySol);
    })

    it("Test Withdraw: ", async () =>{
        const withdrawUSDC = await program.methods
        .withdraw(new BN(1))
        .accounts({
            signer: signer.publicKey,
            mint: mintUSDC,
            tokenProgram: TOKEN_PROGRAM_ID,
        })
        .rpc({commitment: "confirmed"});
        console.log("WithDraw USDC: ", withdrawUSDC);
    })
});
