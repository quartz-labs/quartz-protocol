import { AnchorProvider, BN, Program, setProvider, web3 } from "@coral-xyz/anchor";
import { BankrunProvider } from "anchor-bankrun";
import { beforeAll, expect, test, beforeEach } from '@jest/globals';
import {
    startAnchor,
    ProgramTestContext,
    BanksClient
} from "solana-bankrun";
import { Keypair, PublicKey, SystemProgram, TransactionMessage, VersionedTransaction, Connection } from "@solana/web3.js";
import { IDL as QuartzIDL, Quartz } from "../../../target/types/quartz";
import { createCloseAccountInstruction } from "@solana/spl-token";
import { createAssociatedTokenAccountInstruction, createSyncNativeInstruction, getAssociatedTokenAddress, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { ASSOCIATED_PROGRAM_ID } from "@coral-xyz/anchor/dist/cjs/utils/token";
import { getVault, getVaultSpl, QUARTZ_PROGRAM_ID, RPC_URL, toRemainingAccount, WSOL_MINT } from "../../utils/helpers";
import { DRIFT_MARKET_INDEX_SOL, DRIFT_ORACLE_1, DRIFT_ORACLE_2, DRIFT_SIGNER, DRIFT_SPOT_MARKET_USDC, getDriftSpotMarketVault, getDriftState, getDriftUser, getDriftUserStats } from "../../utils/drift";
import { DRIFT_SPOT_MARKET_SOL } from "../../utils/drift";
import { DRIFT_PROGRAM_ID } from "../../utils/drift";
import { makeDepositLamportsInstructions } from "./deposit.test";
import { setupTestEnvironment } from "./balanceSetup";

describe("Quartz Balance", () => {
    //all the things that need to be done before each test
    let provider: BankrunProvider,
        user: Keypair,
        context: ProgramTestContext,
        banksClient: BanksClient,
        quartzProgram: Program<Quartz>,
        vaultPda: PublicKey;

    user = Keypair.generate();

    beforeAll(async () => {
        ({ user, context, banksClient, quartzProgram, vaultPda } = await setupTestEnvironment());

        const vaultAccount = await quartzProgram.account.vault.fetch(vaultPda);
        expect(vaultAccount.owner.toString()).toBe(user.publicKey.toString());


        const ix_initVaultDriftAccount = await quartzProgram.methods
            .initDriftAccount()
            .accounts({
                vault: vaultPda,
                owner: user.publicKey,
                driftUser: getDriftUser(vaultPda),
                driftUserStats: getDriftUserStats(vaultPda),
                driftState: getDriftState(),
                driftProgram: DRIFT_PROGRAM_ID,
                rent: web3.SYSVAR_RENT_PUBKEY,
                systemProgram: SystemProgram.programId,
            })
            .instruction();

        const latestBlockhash = await banksClient.getLatestBlockhash();
        const messageV0 = new TransactionMessage({
            payerKey: user.publicKey,
            recentBlockhash: latestBlockhash[0],
            instructions: [ix_initVaultDriftAccount],
        }).compileToV0Message();
        const tx = new VersionedTransaction(messageV0);

        const simResInitDriftAccount = await banksClient.simulateTransaction(tx);
        const metaInitDriftAccount = await banksClient.processTransaction(tx);

        expect(simResInitDriftAccount.meta?.logMessages).toEqual(metaInitDriftAccount?.logMessages);
        expect(metaInitDriftAccount.logMessages[1]).toBe("Program log: Instruction: InitDriftAccount");
        expect(metaInitDriftAccount.logMessages[9]).toBe("Program log: Instruction: InitializeUser");
        expect(metaInitDriftAccount.logMessages[14]).toBe("Program dRiftyHA39MWEi3m9aunc5MzRF1JYuBsbn6VPcn33UH success");
        expect(metaInitDriftAccount.logMessages[16]).toBe("Program 6JjHXLheGSNvvexgzMthEcgjkcirDrGduc3HAKB2P1v2 success");

        const instructions = await makeDepositLamportsInstructions(quartzProgram, user, 100_000_000);

        const latestBlockhashDeposit = await banksClient.getLatestBlockhash();
        const messageV0Deposit = new TransactionMessage({
            payerKey: user.publicKey,
            recentBlockhash: latestBlockhashDeposit[0],
            instructions: instructions,
        }).compileToV0Message();
        const txDeposit = new VersionedTransaction(messageV0Deposit);

        const simResDeposit = await banksClient.simulateTransaction(txDeposit);
        const metaDeposit = await banksClient.processTransaction(txDeposit);

        expect(simResDeposit.meta?.logMessages).toEqual(metaDeposit?.logMessages);
        expect(metaDeposit.logMessages[1]).toBe("Program log: Create");
        expect(metaDeposit.logMessages[28]).toBe("Program log: Instruction: Deposit");
        expect(metaDeposit.logMessages[36]).toBe("Program log: Instruction: Transfer");
        expect(metaDeposit.logMessages[48]).toBe("Program dRiftyHA39MWEi3m9aunc5MzRF1JYuBsbn6VPcn33UH success");
        expect(metaDeposit.logMessages[54]).toBe("Program 6JjHXLheGSNvvexgzMthEcgjkcirDrGduc3HAKB2P1v2 success");
    });

    test("Withdraw", async () => {
        const instructions = await makeWithdrawLamportsInstructions(quartzProgram, user, 90_000_000);

        const latestBlockhash = await banksClient.getLatestBlockhash();
        const messageV0 = new TransactionMessage({
            payerKey: user.publicKey,
            recentBlockhash: latestBlockhash[0],
            instructions: instructions,
        }).compileToV0Message();
        const tx = new VersionedTransaction(messageV0);

        const simRes = await banksClient.simulateTransaction(tx);
        const meta = await banksClient.processTransaction(tx);

        expect(simRes.meta?.logMessages).toEqual(meta?.logMessages);
        expect(meta.logMessages[1]).toBe("Program log: Create");
        expect(meta.logMessages[22]).toBe("Program log: Instruction: Withdraw");
        expect(meta.logMessages[26]).toBe("Program log: Instruction: InitializeAccount3");
        expect(meta.logMessages[30]).toBe("Program log: Instruction: Withdraw");
        expect(meta.logMessages[33]).toBe("Program log: Instruction: Transfer");
        expect(meta.logMessages[37]).toBe("Program dRiftyHA39MWEi3m9aunc5MzRF1JYuBsbn6VPcn33UH success");
        expect(meta.logMessages[47]).toBe("Program 6JjHXLheGSNvvexgzMthEcgjkcirDrGduc3HAKB2P1v2 success");


    });
});

export const makeWithdrawLamportsInstructions = async (program: Program<Quartz>, wallet: Keypair, amountLamports: number) => {

    const walletWSol = await getAssociatedTokenAddress(WSOL_MINT, wallet.publicKey);
    const vaultPda = getVault(wallet.publicKey);

    const oix_createWSolAta = createAssociatedTokenAccountInstruction(
        wallet.publicKey,
        walletWSol,
        wallet.publicKey,
        WSOL_MINT
    )

    const ix_withdraw = await program.methods
    .withdraw(new BN(amountLamports), DRIFT_MARKET_INDEX_SOL, true)
    .accounts({
        vault: vaultPda,
        vaultSpl: getVaultSpl(vaultPda, WSOL_MINT),
        owner: wallet.publicKey,
        ownerSpl: walletWSol,
        splMint: WSOL_MINT,
        driftUser: getDriftUser(vaultPda),
        driftUserStats: getDriftUserStats(vaultPda),
        driftState: getDriftState(),
        spotMarketVault: getDriftSpotMarketVault(DRIFT_MARKET_INDEX_SOL),
        driftSigner: DRIFT_SIGNER,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_PROGRAM_ID,
        driftProgram: DRIFT_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
    })
    .remainingAccounts([
        toRemainingAccount(DRIFT_ORACLE_2, false, false),
        toRemainingAccount(DRIFT_ORACLE_1, false, false),
        toRemainingAccount(DRIFT_SPOT_MARKET_SOL, true, false),
        toRemainingAccount(DRIFT_SPOT_MARKET_USDC, false, false)
    ])
    .instruction();

    const ix_closeWSolAta = createCloseAccountInstruction(
        walletWSol,
        wallet.publicKey,
        wallet.publicKey
    );

    const instructions = [oix_createWSolAta, ix_withdraw, ix_closeWSolAta];
    return instructions;
}

