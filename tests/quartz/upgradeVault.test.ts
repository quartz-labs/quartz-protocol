import { Program } from "@coral-xyz/anchor";
import { BankrunProvider } from "anchor-bankrun";
import { beforeEach, describe, expect, test } from "@jest/globals";
import { startAnchor, ProgramTestContext, BanksClient, start } from "solana-bankrun";
import { Connection, Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import { IDL as QuartzIDL, Quartz } from "../../target/types/quartz";
import { getInitRentPayer, getVaultPda } from "../utils/accounts";
import { upgradeVault } from "../utils/instructions";
import { QUARTZ_PROGRAM_ID } from "../config/constants";
import config from "../config/config";

const LOOKUP_TABLE = new PublicKey("F39jBi6T9YtnqVFaLYTFdPGX7vKoeeChZPYCuEDLA4mB");
const OLD_VAULT = new PublicKey("Ainxsfb6sFumCTucuLNRY6qeZiRwcewxBFsS9cKbjDhQ");
const OLD_VAULT_OWNER = new PublicKey("CPdx23eqz7NtZ5GSzgJB7WnyA2LrC4gFs343W5gEkWbv");

const TIMEOUT = 10_000;
describe("upgrade_vault", () => {
    let provider: BankrunProvider;
    let user: Keypair;
    let vaultPda: PublicKey;
    let context: ProgramTestContext;
    let banksClient: BanksClient;
    let quartzProgram: Program<Quartz>;
    let initRentPayer: PublicKey;

    beforeEach(async () => {
        user = Keypair.generate();
        vaultPda = getVaultPda(user.publicKey);
        initRentPayer = getInitRentPayer();

        const connection = new Connection(config.RPC_URL);
        const oldVaultAccount = await connection.getAccountInfo(OLD_VAULT);
        const initRentPayerAccount = await connection.getAccountInfo(initRentPayer);

        context = await startAnchor(
            "./",
            [],
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
                    address: OLD_VAULT,
                    info: oldVaultAccount,
                },
                {
                    address: OLD_VAULT_OWNER,
                    info: {
                        lamports: 1_000_000_000,
                        data: Buffer.alloc(0),
                        owner: SystemProgram.programId,
                        executable: false,
                    }
                },
                {
                    address: initRentPayer,
                    info: initRentPayerAccount,
                }
            ]
        );

        provider = new BankrunProvider(context);
        quartzProgram = new Program<Quartz>(QuartzIDL, QUARTZ_PROGRAM_ID, provider);
        banksClient = context.banksClient;
    }, TIMEOUT);

    test("Should upgrade vault, adding new data", async () => {
        const OLD_VAULT_SIZE = 41;
        const oldVaultAccountInfo = await provider.connection.getAccountInfo(OLD_VAULT);
        expect(oldVaultAccountInfo.data.length).toBe(OLD_VAULT_SIZE);

        const oldVaultAccount = await quartzProgram.account.vault.fetch(OLD_VAULT);
        expect(oldVaultAccount.spendLimitPerTransaction.toNumber()).toBe(0);
        expect(oldVaultAccount.spendLimitPerTimeframe.toNumber()).toBe(0);
        expect(oldVaultAccount.remainingSpendLimitPerTimeframe.toNumber()).toBe(0);
        expect(oldVaultAccount.nextSpendLimitPerTimeframeResetSlot.toNumber()).toBe(0);
        expect(oldVaultAccount.extendSpendLimitPerTimeframeResetSlotAmount.toNumber()).toBe(0);

        const meta = await upgradeVault(
            quartzProgram, 
            banksClient, 
            {
                spendLimitPerTransaction: 1000_000_000,
                spendLimitPerTimeframe: 1000_000_000,
                extendSpendLimitPerTimeframeResetSlotAmount: 1000_000_000,
            },
            {
                vault: OLD_VAULT,
                owner: OLD_VAULT_OWNER,
                initRentPayer: initRentPayer,
                systemProgram: SystemProgram.programId
            }
        );

        expect(meta.logMessages[1]).toBe("Program log: Instruction: UpgradeVault");
        expect(meta.logMessages[3]).toBe("Program 11111111111111111111111111111111 success");
        expect(meta.logMessages[5]).toBe("Program 6JjHXLheGSNvvexgzMthEcgjkcirDrGduc3HAKB2P1v2 success");

        const updatedVaultAccount = await quartzProgram.account.vault.fetch(OLD_VAULT);

        expect(updatedVaultAccount.spendLimitPerTransaction.toNumber()).toBe(1000_000_000);
        expect(updatedVaultAccount.spendLimitPerTimeframe.toNumber()).toBe(1000_000_000);
        expect(updatedVaultAccount.remainingSpendLimitPerTimeframe.toNumber()).toBe(1000_000_000);
        expect(updatedVaultAccount.nextSpendLimitPerTimeframeResetSlot.toNumber()).toBe(1000_000_001); // Add 1 for current slot
        expect(updatedVaultAccount.extendSpendLimitPerTimeframeResetSlotAmount.toNumber()).toBe(1000_000_000);
        expect(updatedVaultAccount.owner.toBase58()).toBe(OLD_VAULT_OWNER.toBase58());
    }, TIMEOUT);

    // TODO: Add expected fail tests
});