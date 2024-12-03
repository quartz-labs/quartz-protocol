import { BN, Program} from "@coral-xyz/anchor";
import { BankrunProvider } from "anchor-bankrun";
import { beforeAll, expect, test } from '@jest/globals';
import {
    ProgramTestContext,
    BanksClient
} from "solana-bankrun";
import { Keypair, PublicKey, SystemProgram, TransactionMessage, VersionedTransaction } from "@solana/web3.js";
import { IDL as QuartzIDL, Quartz } from "../../../target/types/quartz";
import { createCloseAccountInstruction } from "@solana/spl-token";
import { createAssociatedTokenAccountInstruction, getAssociatedTokenAddress, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { ASSOCIATED_PROGRAM_ID } from "@coral-xyz/anchor/dist/cjs/utils/token";
import { getVault, getVaultSpl, toRemainingAccount, USDC_MINT, WSOL_MINT } from "../../utils/helpers";
import { DRIFT_MARKET_INDEX_SOL, DRIFT_MARKET_INDEX_USDC, DRIFT_ORACLE_1, DRIFT_ORACLE_2, DRIFT_SIGNER, DRIFT_SPOT_MARKET_USDC, getDriftSpotMarketVault, getDriftState, getDriftUser, getDriftUserStats } from "../../utils/drift";
import { DRIFT_SPOT_MARKET_SOL } from "../../utils/drift";
import { DRIFT_PROGRAM_ID } from "../../utils/drift";
import { makeDriftLamportDeposit } from "./deposit.test";
import { setupQuartzAndDriftAccount, setupTestEnvironment } from "./balanceSetup";

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

        await setupQuartzAndDriftAccount(quartzProgram, banksClient, vaultPda, user);
        await makeDriftLamportDeposit(quartzProgram, user, 100_000_000_000, banksClient, WSOL_MINT);
    });

    test("Withdraw Lamports", async () => {
        await makeDriftLamportWithdraw(quartzProgram, user, 90_000_000, banksClient);
    });

    test("Withdraw USDC", async () => {
        await makeDriftUSDCWithdraw(quartzProgram, user, 90_000, banksClient);
    });
});

export const makeDriftLamportWithdraw = async (program: Program<Quartz>, wallet: Keypair, amountLamports: number, banksClient: BanksClient) => {

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

    const latestBlockhash = await banksClient.getLatestBlockhash();
    const messageV0 = new TransactionMessage({
        payerKey: wallet.publicKey,
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
}


export const makeDriftUSDCWithdraw = async (program: Program<Quartz>, wallet: Keypair, amountMicroCents: number, banksClient: BanksClient) => {

    const walletUsdc = await getAssociatedTokenAddress(USDC_MINT, wallet.publicKey);
    const vaultPda = getVault(wallet.publicKey);

    const oix_createWSolAta = createAssociatedTokenAccountInstruction(
        wallet.publicKey,
        walletUsdc,
        wallet.publicKey,
        USDC_MINT
    )

    const ix_withdraw = await program.methods
    .withdraw(new BN(amountMicroCents), DRIFT_MARKET_INDEX_USDC, false)
    .accounts({
        vault: vaultPda,
        vaultSpl: getVaultSpl(vaultPda, USDC_MINT),
        owner: wallet.publicKey,
        ownerSpl: walletUsdc,
        splMint: USDC_MINT,
        driftUser: getDriftUser(vaultPda),
        driftUserStats: getDriftUserStats(vaultPda),
        driftState: getDriftState(),
        spotMarketVault: getDriftSpotMarketVault(DRIFT_MARKET_INDEX_USDC),
        driftSigner: DRIFT_SIGNER,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_PROGRAM_ID,
        driftProgram: DRIFT_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
    })
    .remainingAccounts([
        toRemainingAccount(DRIFT_ORACLE_1, false, false),
        toRemainingAccount(DRIFT_ORACLE_2, false, false),
        toRemainingAccount(DRIFT_SPOT_MARKET_SOL, true, false),
        toRemainingAccount(DRIFT_SPOT_MARKET_USDC, true, false)
    ])
    .instruction();

    const instructions = [oix_createWSolAta, ix_withdraw];

    const latestBlockhash = await banksClient.getLatestBlockhash();
    const messageV0 = new TransactionMessage({
        payerKey: wallet.publicKey,
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
    expect(meta.logMessages[34]).toBe("Program log: Instruction: Transfer");
    expect(meta.logMessages[38]).toBe("Program dRiftyHA39MWEi3m9aunc5MzRF1JYuBsbn6VPcn33UH success");
    expect(meta.logMessages[48]).toBe("Program 6JjHXLheGSNvvexgzMthEcgjkcirDrGduc3HAKB2P1v2 success");
}