import { Connection, Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import { BankrunProvider } from "anchor-bankrun";
import { startAnchor, BanksClient } from "solana-bankrun";
import { Program } from "@coral-xyz/anchor";
import { Quartz, IDL as QuartzIDL } from "../../../target/types/quartz";
import { getVault, QUARTZ_PROGRAM_ID, RPC_URL, USDC_MINT, WSOL_MINT } from "../../utils/helpers";
import { DRIFT_PROGRAM_ID, DRIFT_SPOT_MARKET_SOL, DRIFT_SPOT_MARKET_USDC, DRIFT_ORACLE_1, DRIFT_ORACLE_2, DRIFT_SIGNER } from "../../utils/drift";
import { initDriftAccount, initUser } from "../user/userSetup";
import { makeDriftLamportDeposit } from "./deposit.test";
import { makeDriftUSDCWithdraw } from "./withdraw.test";

export const setupTestEnvironment = async () => {
    const user = Keypair.generate();
    const connection = new Connection(RPC_URL);
    const accountInfo = await connection.getAccountInfo(new PublicKey("5zpq7DvB6UdFFvpmBPspGPNfUGoBRRCE2HHg5u3gxcsN"));
    const solSpotMarketVaultAccountInfo = await connection.getAccountInfo(new PublicKey("DfYCNezifxAEsQbAJ1b3j6PX3JVBe8fu11KBhxsbw5d2"));
    const usdcSpotMarketVaultAccountInfo = await connection.getAccountInfo(new PublicKey("GXWqPpjQpdz7KZw9p7f5PX2eGxHAhvpNXiviFkAB8zXg"));
    const solSpotMarketAccountInfo = await connection.getAccountInfo(DRIFT_SPOT_MARKET_SOL);
    const usdcSpotMarketAccountInfo = await connection.getAccountInfo(DRIFT_SPOT_MARKET_USDC);
    const oracle1AccountInfo = await connection.getAccountInfo(DRIFT_ORACLE_1);
    const oracle2AccountInfo = await connection.getAccountInfo(DRIFT_ORACLE_2);
    const driftSignerAccountInfo = await connection.getAccountInfo(DRIFT_SIGNER);
    const usdcMintAccountInfo = await connection.getAccountInfo(USDC_MINT);

    const context = await startAnchor("./", [{ name: "drift", programId: DRIFT_PROGRAM_ID }],
        [
            {
                address: user.publicKey,
                info: {
                    lamports: 1_000_000_000_000_000,
                    data: Buffer.alloc(0),
                    owner: SystemProgram.programId,
                    executable: false,
                },
            },
            //drift authority
            {
                address: new PublicKey("rxEaSMXqKx9GvYY8rrZB1SG5CQUXTfnXbZSaceaaPzA"),
                info: {
                    lamports: 1_000_000_000,
                    data: Buffer.alloc(0),
                    owner: new PublicKey("6JjHXLheGSNvvexgzMthEcgjkcirDrGduc3HAKB2P1v2"),
                    executable: false,
                }
            },
            //drift state
            {
                address: new PublicKey("5zpq7DvB6UdFFvpmBPspGPNfUGoBRRCE2HHg5u3gxcsN"),
                info: accountInfo
            },
            // Drift Sol spot market vault
            {
                address: new PublicKey("DfYCNezifxAEsQbAJ1b3j6PX3JVBe8fu11KBhxsbw5d2"),
                info: solSpotMarketVaultAccountInfo
            },
            // Drift USDC spot market vault
            {
                address: new PublicKey("GXWqPpjQpdz7KZw9p7f5PX2eGxHAhvpNXiviFkAB8zXg"),
                info: usdcSpotMarketVaultAccountInfo
            },
            {
                address: DRIFT_SPOT_MARKET_SOL,
                info: solSpotMarketAccountInfo
            },
            {
                address: DRIFT_SPOT_MARKET_USDC,
                info: usdcSpotMarketAccountInfo
            },
            {
                address: DRIFT_ORACLE_1,
                info: oracle1AccountInfo
            },
            {
                address: DRIFT_SIGNER,
                info: driftSignerAccountInfo
            },
            {
                address: DRIFT_ORACLE_2,
                info: oracle2AccountInfo
            },
            {
                address: USDC_MINT,
                info: usdcMintAccountInfo
            }
        ]
    );

    const banksClient = context.banksClient;
    const provider = new BankrunProvider(context);

    const quartzProgram = new Program<Quartz>(
        QuartzIDL,
        QUARTZ_PROGRAM_ID,
        provider,
    );

    const vaultPda = getVault(user.publicKey);

    return { user, context, banksClient, quartzProgram, vaultPda };
};

//Sets up the drift account
export const setupQuartzAndDriftAccount = async (quartzProgram: Program<Quartz>, banksClient: BanksClient, vaultPda: PublicKey, user: Keypair) => {
    await initUser(quartzProgram, banksClient, vaultPda, user);
    await initDriftAccount(quartzProgram, banksClient, vaultPda, user);
}


//Sets up the drift account + funds it with SOL
export const setupDriftAccountWithFunds = async (quartzProgram: Program<Quartz>, banksClient: BanksClient, vaultPda: PublicKey, user: Keypair) => {
    await setupQuartzAndDriftAccount(quartzProgram, banksClient, vaultPda, user);
    await makeDriftLamportDeposit(quartzProgram, user, 100_000_000, banksClient, WSOL_MINT);
}

export const setupDriftAccountWithFundsAndLoan = async (quartzProgram: Program<Quartz>, banksClient: BanksClient, vaultPda: PublicKey, user: Keypair) => {
    await setupQuartzAndDriftAccount(quartzProgram, banksClient, vaultPda, user);
    await makeDriftLamportDeposit(quartzProgram, user, 100_000_000, banksClient, WSOL_MINT);
    await makeDriftUSDCWithdraw(quartzProgram, user, 90_000, banksClient);
}