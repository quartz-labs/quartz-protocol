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
import { DRIFT_MARKET_INDEX_SOL, DRIFT_ORACLE_1, DRIFT_SPOT_MARKET_USDC, getDriftSpotMarketVault, getDriftState, getDriftUser, getDriftUserStats } from "../../utils/drift";
import { DRIFT_SPOT_MARKET_SOL } from "../../utils/drift";
import { DRIFT_PROGRAM_ID } from "../../utils/drift";
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

        const simRes = await banksClient.simulateTransaction(tx);
        const meta = await banksClient.processTransaction(tx);

        expect(simRes.meta?.logMessages).toEqual(meta?.logMessages);
        expect(meta.logMessages[1]).toBe("Program log: Instruction: InitDriftAccount");
        expect(meta.logMessages[9]).toBe("Program log: Instruction: InitializeUser");
        expect(meta.logMessages[14]).toBe("Program dRiftyHA39MWEi3m9aunc5MzRF1JYuBsbn6VPcn33UH success");
        expect(meta.logMessages[16]).toBe("Program 6JjHXLheGSNvvexgzMthEcgjkcirDrGduc3HAKB2P1v2 success");
    });

    test("Deposit", async () => {

        const instructions = await makeDepositLamportsInstructions(quartzProgram, user, 100_000_000);

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
        expect(meta.logMessages[28]).toBe("Program log: Instruction: Deposit");
        expect(meta.logMessages[36]).toBe("Program log: Instruction: Transfer");
        expect(meta.logMessages[48]).toBe("Program dRiftyHA39MWEi3m9aunc5MzRF1JYuBsbn6VPcn33UH success");
        expect(meta.logMessages[54]).toBe("Program 6JjHXLheGSNvvexgzMthEcgjkcirDrGduc3HAKB2P1v2 success");
    });
});

export const makeDepositLamportsInstructions = async (program: Program<Quartz>, wallet: Keypair, amountLamports: number) => {

    const walletWSol = await getAssociatedTokenAddress(WSOL_MINT, wallet.publicKey);
    const vaultPda = getVault(wallet.publicKey);

    const oix_createWSolAta = createAssociatedTokenAccountInstruction(
        wallet.publicKey,
        walletWSol,
        wallet.publicKey,
        WSOL_MINT
    )
    const ix_wrapSol = SystemProgram.transfer({
        fromPubkey: wallet.publicKey,
        toPubkey: walletWSol,
        lamports: amountLamports
    });

    const ix_syncNative = createSyncNativeInstruction(walletWSol);

    const ix_deposit = await program.methods
        .deposit(new BN(amountLamports), DRIFT_MARKET_INDEX_SOL, false)
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
            tokenProgram: TOKEN_PROGRAM_ID,
            associatedTokenProgram: ASSOCIATED_PROGRAM_ID,
            driftProgram: DRIFT_PROGRAM_ID,
            systemProgram: SystemProgram.programId
        })
        .remainingAccounts([
            toRemainingAccount(DRIFT_ORACLE_1, false, false),
            toRemainingAccount(DRIFT_SPOT_MARKET_SOL, true, false)
        ])
        .instruction();

    const ix_closeWSolAta = createCloseAccountInstruction(
        walletWSol,
        wallet.publicKey,
        wallet.publicKey
    );

    const instructions = [oix_createWSolAta, ix_wrapSol, ix_syncNative, ix_deposit, ix_closeWSolAta];
    return instructions;
}

