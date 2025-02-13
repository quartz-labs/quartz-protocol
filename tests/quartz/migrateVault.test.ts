import { BN, Program, Provider } from "@coral-xyz/anchor";
import { BankrunProvider } from "anchor-bankrun";
import { beforeEach, describe, expect, test } from "@jest/globals";
import { startAnchor, ProgramTestContext, BanksClient, start } from "solana-bankrun";
import { AddressLookupTableProgram, Connection, Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import { IDL as QuartzIDL, Quartz } from "../../target/types/quartz";
import { getVaultPda } from "../utils/accounts";
import { closeUser, initUser, migrateVault } from "../utils/instructions";
import { ADDRESS_LOOKUP_TABLE_PROGRAM_ID, QUARTZ_ADDRESS_TABLE, QUARTZ_PROGRAM_ID } from "../config/constants";
import config from "../config/config";
import { processTransaction } from "../utils/helpers";
import { TOKENS } from "../utils/tokens";

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
        const oldVaultAccount = await connection.getAccountInfo(OLD_VAULT);

        //loop through all TOKENS and getAccountInfo for each.
        const tokenMintAdddresses = []
        const tokenMintAccounts = []
        for (const token of Object.values(TOKENS)) {
            const tokenAccount = await connection.getAccountInfo(token.mint);
            tokenMintAdddresses.push(token.mint);
            tokenMintAccounts.push(tokenAccount);
        }
        console.log("TokenMint length ", tokenMintAdddresses.length);
        console.log("Old vault account:", oldVaultAccount);

        console.log("AddressLookupTable Program ID:", AddressLookupTableProgram.programId.toString());
        console.log("Used Program ID:", ADDRESS_LOOKUP_TABLE_PROGRAM_ID.toString());

        context = await startAnchor(
            "./",
            [{ name: "lut", programId: new PublicKey("AddressLookupTab1e1111111111111111111111111") }],
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
                    address: tokenMintAdddresses[0],
                    info: tokenMintAccounts[0],
                },
                {
                    address: tokenMintAdddresses[1],
                    info: tokenMintAccounts[1],
                },
                {
                    address: tokenMintAdddresses[2],
                    info: tokenMintAccounts[2],
                },
                {
                    address: tokenMintAdddresses[3],
                    info: tokenMintAccounts[3],
                },
                {
                    address: tokenMintAdddresses[4],
                    info: tokenMintAccounts[4],
                },
                {
                    address: tokenMintAdddresses[5],
                    info: tokenMintAccounts[5],
                }
            ]
        );

        provider = new BankrunProvider(context);
        //@ts-ignore
        quartzProgram = new Program<Quartz>(QuartzIDL, QUARTZ_PROGRAM_ID, provider);
        banksClient = context.banksClient;
    }, TIMEOUT);

    test("Should init new user", async () => {
        // Calculate the lookup table address that will be created

        const slot = Number(await banksClient.getSlot());

        const [_ix, lookupTableAddress] = AddressLookupTableProgram.createLookupTable({
            authority: vaultPda,
            payer: vaultPda,
            recentSlot: slot,
        });

        console.log("Lookup table address:", lookupTableAddress);

        // Initialize user with the derived lookup table address
        // const meta = await initUser(quartzProgram, banksClient, {
        //     vault: vaultPda,
        //     owner: user.publicKey,
        //     systemProgram: SystemProgram.programId,
        //     lookupTable: lookupTableAddress,   
        //     addressLookupTableProgram: AddressLookupTableProgram.programId,
        // }, slot);

        const ix = await quartzProgram.methods
            .initUser(new BN(10), new BN(slot))
            .accounts({
                vault: vaultPda,
                owner: user.publicKey,
                systemProgram: SystemProgram.programId,
                lookupTable: lookupTableAddress,
                addressLookupTableProgram: new PublicKey("AddressLookupTab1e1111111111111111111111111"),
            })
            .remainingAccounts(Object.values(TOKENS).map(token => ({
                pubkey: token.mint,
                isWritable: false,
                isSigner: false,
            })))
            .instruction();

        const meta = await processTransaction(banksClient, user.publicKey, [ix]);

        console.log(meta);

        expect(meta.logMessages[1]).toBe("Program log: Instruction: InitUser");
        expect(meta.logMessages[3]).toBe("Program 11111111111111111111111111111111 success");
        expect(meta.logMessages[5]).toBe("Program 6JjHXLheGSNvvexgzMthEcgjkcirDrGduc3HAKB2P1v2 success");

        const vaultAccount = await quartzProgram.account.vault.fetch(vaultPda);

        console.log("Vault account:", vaultAccount);

        expect(vaultAccount.owner.toString()).toBe(user.publicKey.toString());
    }, TIMEOUT);

    //programs/address-lookup-table
    // test("Should Migrate Vault to new vault", async () => {

    //     //I want to test that the old vault does not have the new vault data eg: spendBalanceAmount
    //     const oldVaultAccount = await quartzProgram.account.vault.fetch(OLD_VAULT);
    //     expect(oldVaultAccount.lookupTable.toBase58()).toBe("11111111111111111111111111111111");

    //     const migrateMeta = await migrateVault(quartzProgram, banksClient, {
    //         vault: OLD_VAULT,
    //         owner: OLD_VAULT_OWNER,
    //         lookupTable: LOOKUP_TABLE,
    //         systemProgram: SystemProgram.programId
    //     });

    //     expect(migrateMeta.logMessages[1]).toBe("Program log: Instruction: MigrateVault");
    //     expect(migrateMeta.logMessages[3]).toBe("Program 11111111111111111111111111111111 success");

    //     const updatedVaultAccount = await quartzProgram.account.vault.fetch(OLD_VAULT);

    //     expect(Number(updatedVaultAccount.spendBalanceAmount)).toBe(0);
    //     expect(updatedVaultAccount.owner.toBase58()).toBe(OLD_VAULT_OWNER.toBase58());
    //     expect(updatedVaultAccount.lookupTable.toBase58()).toBe(LOOKUP_TABLE.toBase58());
    // }, TIMEOUT);
});

