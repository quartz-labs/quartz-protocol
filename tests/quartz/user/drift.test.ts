import { BN, Program, web3 } from "@coral-xyz/anchor";
import { BankrunProvider } from "anchor-bankrun";
import { expect, test } from '@jest/globals';
import {
    startAnchor,
    ProgramTestContext,
    BanksClient,
    Clock
} from "solana-bankrun";
import { Keypair, PublicKey, SystemProgram, TransactionMessage, VersionedTransaction, Connection } from "@solana/web3.js";
import { IDL as QuartzIDL, Quartz } from "../../../target/types/quartz";
import { expectError, getVault, QUARTZ_PROGRAM_ID, RPC_URL } from "../../utils/helpers";
import { DRIFT_PROGRAM_ID, getDriftState, getDriftUser, getDriftUserStats } from "../../utils/drift";
import { initDriftAccount, initUser } from "./userSetup";

describe("Quartz User", () => {
    let provider: BankrunProvider,
        user: Keypair,
        context: ProgramTestContext,
        banksClient: BanksClient,
        quartzProgram: Program<Quartz>,
        vaultPda: PublicKey;

    const setupTest = async () => {
        user = Keypair.generate();
        const connection = new Connection(RPC_URL);
        const driftStateAccount = await connection.getAccountInfo(new PublicKey("5zpq7DvB6UdFFvpmBPspGPNfUGoBRRCE2HHg5u3gxcsN"));
        const driftAuthorityAccount = await connection.getAccountInfo(new PublicKey("rxEaSMXqKx9GvYY8rrZB1SG5CQUXTfnXbZSaceaaPzA"));

        context = await startAnchor("./", 
            [{ name: "drift", programId: DRIFT_PROGRAM_ID }],
            [
                {
                    address: user.publicKey,
                    info: {
                        lamports: 1_000_000_000,
                        data: Buffer.alloc(0),
                        owner: SystemProgram.programId,
                        executable: false,
                    },
                },
                {
                    address: new PublicKey("rxEaSMXqKx9GvYY8rrZB1SG5CQUXTfnXbZSaceaaPzA"),
                    info: {
                        ...driftAuthorityAccount,
                        executable: false,
                        owner: DRIFT_PROGRAM_ID,
                    }
                },
                {
                    address: new PublicKey("5zpq7DvB6UdFFvpmBPspGPNfUGoBRRCE2HHg5u3gxcsN"),
                    info: {
                        ...driftStateAccount,
                        executable: false,
                        owner: DRIFT_PROGRAM_ID,
                    }
                }
            ]
        );

        banksClient = context.banksClient;
        provider = new BankrunProvider(context);
        quartzProgram = new Program<Quartz>(QuartzIDL, QUARTZ_PROGRAM_ID, provider);
        vaultPda = getVault(user.publicKey);

        // Initialize user
        await initUser(quartzProgram, banksClient, vaultPda, user);
    };

    test("Init Drift User", async () => {
        await setupTest();
        
        const vaultAccount = await quartzProgram.account.vault.fetch(vaultPda);
        expect(vaultAccount.owner.toString()).toBe(user.publicKey.toString());

        await initDriftAccount(quartzProgram, banksClient, vaultPda, user);
    });

    test("Close Drift Account", async () => {
        await setupTest();
        await initDriftAccount(quartzProgram, banksClient, vaultPda, user);

        const ix_closeDriftAccount = await quartzProgram.methods
            .closeDriftAccount()
            .accounts({
                vault: vaultPda,
                owner: user.publicKey,
                driftUser: getDriftUser(vaultPda),
                driftUserStats: getDriftUserStats(vaultPda),
                driftState: getDriftState(),
                driftProgram: DRIFT_PROGRAM_ID
            })
            .instruction();

        const latestBlockhash = await banksClient.getLatestBlockhash();
        const messageV0 = new TransactionMessage({
            payerKey: user.publicKey,
            recentBlockhash: latestBlockhash[0],
            instructions: [ix_closeDriftAccount],
        }).compileToV0Message();

        const tx = new VersionedTransaction(messageV0);
        tx.sign([user]);
        
        const simRes = await banksClient.simulateTransaction(tx);
        console.log("simRes", simRes);

        // TODO: Add expectations
    });
});