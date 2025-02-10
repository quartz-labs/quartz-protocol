import { Program } from "@coral-xyz/anchor";
import { BankrunProvider } from "anchor-bankrun";
import { beforeEach, describe, expect, test } from "@jest/globals";
import { startAnchor, ProgramTestContext, BanksClient, start } from "solana-bankrun";
import { Connection, Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import { IDL as QuartzIDL, Quartz } from "../../target/types/quartz";
import { getVaultPda } from "../utils/accounts";
import { closeUser, initUser, migrateVault } from "../utils/instructions";
import { QUARTZ_ADDRESS_TABLE, QUARTZ_PROGRAM_ID } from "../config/constants";
import config from "../config/config";

const LOOKUP_TABLE = new PublicKey("F39jBi6T9YtnqVFaLYTFdPGX7vKoeeChZPYCuEDLA4mB");
const OLD_VAULT = new PublicKey("Ainxsfb6sFumCTucuLNRY6qeZiRwcewxBFsS9cKbjDhQ");
const OLD_VAULT_OWNER = new PublicKey("CPdx23eqz7NtZ5GSzgJB7WnyA2LrC4gFs343W5gEkWbv");

const TIMEOUT = 10_000;
describe("migrate_vault", () => {
    let provider: BankrunProvider;
    let user: Keypair;
    let vaultPda: PublicKey;
    let context: ProgramTestContext;
    let banksClient: BanksClient;
    let quartzProgram: Program<Quartz>;
    beforeEach(async () => {
        user = Keypair.generate();
        vaultPda = getVaultPda(user.publicKey);

        const connection = new Connection(config.RPC_URL);
        const lookupTableAccountInfo = await connection.getAccountInfo(QUARTZ_ADDRESS_TABLE);
        const oldVaultAccount = await connection.getAccountInfo(OLD_VAULT);

        console.log(oldVaultAccount);

        console.log('Old vault account:', {
            data: oldVaultAccount.data,
            owner: oldVaultAccount.owner.toBase58(),
            lamports: oldVaultAccount.lamports,
            executable: oldVaultAccount.executable,
            rentEpoch: oldVaultAccount.rentEpoch
        });

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
                    address: LOOKUP_TABLE,
                    info: lookupTableAccountInfo
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
                }
            ]
        );

        provider = new BankrunProvider(context);
        quartzProgram = new Program<Quartz>(QuartzIDL, QUARTZ_PROGRAM_ID, provider);
        banksClient = context.banksClient;
    }, TIMEOUT);

    test("Should new init user", async () => {
        const meta = await initUser(quartzProgram, banksClient, {
            vault: vaultPda,
            owner: user.publicKey,
            systemProgram: SystemProgram.programId,
            lookupTable: LOOKUP_TABLE
        });

        expect(meta.logMessages[1]).toBe("Program log: Instruction: InitUser");
        expect(meta.logMessages[3]).toBe("Program 11111111111111111111111111111111 success");
        expect(meta.logMessages[5]).toBe("Program 6JjHXLheGSNvvexgzMthEcgjkcirDrGduc3HAKB2P1v2 success");

        const vaultAccount = await quartzProgram.account.vault.fetch(vaultPda);

        expect(vaultAccount.owner.toString()).toBe(user.publicKey.toString());
    }, TIMEOUT);

    test("Should Migrate Vault to new vault", async () => {

        //I want to test that the old vault does not have the new vault data eg: spendBalanceAmount
        const oldVaultAccount = await quartzProgram.account.vault.fetch(OLD_VAULT);
        expect(oldVaultAccount.lookupTable.toBase58()).toBe("11111111111111111111111111111111");

        const migrateMeta = await migrateVault(quartzProgram, banksClient, {
            vault: OLD_VAULT,
            owner: OLD_VAULT_OWNER,
            lookupTable: LOOKUP_TABLE,
            systemProgram: SystemProgram.programId
        });

        expect(migrateMeta.logMessages[1]).toBe("Program log: Instruction: MigrateVault");
        expect(migrateMeta.logMessages[3]).toBe("Program 11111111111111111111111111111111 success");

        const updatedVaultAccount = await quartzProgram.account.vault.fetch(OLD_VAULT);

        expect(Number(updatedVaultAccount.spendBalanceAmount)).toBe(0);
        expect(updatedVaultAccount.owner.toBase58()).toBe(OLD_VAULT_OWNER.toBase58());
        expect(updatedVaultAccount.lookupTable.toBase58()).toBe(LOOKUP_TABLE.toBase58());
    }, TIMEOUT);
});

