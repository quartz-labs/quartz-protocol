import { Program } from "@coral-xyz/anchor";
import { BankrunProvider } from "anchor-bankrun";
import { beforeEach, describe, expect, test } from "@jest/globals";
import { startAnchor, ProgramTestContext, BanksClient } from "solana-bankrun";
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import { IDL as QuartzIDL, Quartz } from "../../target/types/quartz";
import { getVaultPda } from "../utils/accounts";
import { closeUser, initUser } from "../utils/instructions";
import NodeWallet from "@coral-xyz/anchor/dist/cjs/nodewallet";
import { QUARTZ_PROGRAM_ID } from "../config/constants";


const TIMEOUT = 10_000;
describe("init_user", () => {
  let provider: BankrunProvider;
  let user: Keypair;
  let vaultPda: PublicKey;
  let context: ProgramTestContext;
  let banksClient: BanksClient;
  let quartzProgram: Program<Quartz>;

  beforeEach(async () => {
    user = Keypair.generate();
    vaultPda = getVaultPda(user.publicKey);

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
      ]
    );

    provider = new BankrunProvider(context);
    quartzProgram = new Program<Quartz>(QuartzIDL, QUARTZ_PROGRAM_ID, provider);
    banksClient = context.banksClient;
  }, TIMEOUT);

  test("Should init user", async () => {
    const meta = await initUser(quartzProgram, banksClient, {
      vault: vaultPda,
      owner: user.publicKey,
      systemProgram: SystemProgram.programId,
    });

    expect(meta.logMessages[1]).toBe("Program log: Instruction: InitUser");
    expect(meta.logMessages[3]).toBe("Program 11111111111111111111111111111111 success");
    expect(meta.logMessages[5]).toBe("Program 6JjHXLheGSNvvexgzMthEcgjkcirDrGduc3HAKB2P1v2 success");

    const vaultAccount = await quartzProgram.account.vault.fetch(vaultPda);
    expect(vaultAccount.owner.toString()).toBe(user.publicKey.toString());
  }, TIMEOUT);

  test("Should fail to init user with wrong vault PDA seed", async () => {
    const [badVaultPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("bad_vault"), user.publicKey.toBuffer()],
      new PublicKey(QUARTZ_PROGRAM_ID)
    );

    try {
      await initUser(quartzProgram, banksClient, {
        vault: badVaultPda,
        owner: user.publicKey,
        systemProgram: SystemProgram.programId,
      });

      expect(false).toBe(true); // Should not reach this point
    } catch (error: any) {
      expect(error.message).toContain("Error processing Instruction 0: custom program error: 0x7d6");
    }
  }, TIMEOUT);

  test("Should fail to init user with wrong vault PDA owner", async () => {
    const otherVault = getVaultPda(Keypair.generate().publicKey);

    try {
      await initUser(quartzProgram, banksClient, {
        vault: otherVault,
        owner: user.publicKey,
        systemProgram: SystemProgram.programId,
      });

      expect(false).toBe(true); // Should not reach this point
    } catch (error: any) {
      expect(error.message).toContain("Error processing Instruction 0: custom program error: 0x7d6");
    }
  }, TIMEOUT);

  test("Should fail to init user with wrong system program", async () => {
    try {
      await initUser(quartzProgram, banksClient, {
        vault: vaultPda,
        owner: user.publicKey,
        systemProgram: Keypair.generate().publicKey,
      });

      expect(false).toBe(true); // Should not reach this point
    } catch (error: any) {
      expect(error.message).toContain("Error processing Instruction 0: custom program error: 0xbc0");
    }
  }, TIMEOUT);
});


describe("close_user", () => {
  let provider: BankrunProvider;
  let user: Keypair;
  let otherUser: Keypair;
  let vaultPda: PublicKey;
  let context: ProgramTestContext;
  let banksClient: BanksClient;
  let quartzProgram: Program<Quartz>;


  beforeEach(async () => {
    user = Keypair.generate();
    otherUser = Keypair.generate();
    vaultPda = getVaultPda(user.publicKey);
    
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
          address: otherUser.publicKey,
          info: {
            lamports: 1_000_000_000,
            data: Buffer.alloc(0),
            owner: SystemProgram.programId,
            executable: false,
          },
        },
      ]
    );

    provider = new BankrunProvider(context);
    quartzProgram = new Program<Quartz>(QuartzIDL, QUARTZ_PROGRAM_ID, provider);
    banksClient = context.banksClient;

    await initUser(quartzProgram, banksClient, {
      vault: vaultPda,
      owner: user.publicKey,
      systemProgram: SystemProgram.programId,
    });
  }, TIMEOUT);

  test("Should close user", async () => {
    const meta = await closeUser(quartzProgram, banksClient, {
      vault: vaultPda,
      owner: user.publicKey,
    });

    expect(meta.logMessages[1]).toBe("Program log: Instruction: CloseUser");
    expect(meta.logMessages[3]).toBe("Program 6JjHXLheGSNvvexgzMthEcgjkcirDrGduc3HAKB2P1v2 success");

    try {
      await quartzProgram.account.vault.fetch(vaultPda);
      expect(false).toBe(true); // Should not reach this point
    } catch (error: any) {
      expect(error.message).toContain("Could not find");
    }
  }, TIMEOUT);

  test("Should fail to close user of vault that doesn't exist", async () => {
    const randomVault = getVaultPda(Keypair.generate().publicKey);

    try {
      await closeUser(quartzProgram, banksClient, {
        vault: randomVault,
        owner: user.publicKey,
      });

      expect(false).toBe(true); // Should not reach this point
    } catch (error: any) {
      expect(error.message).toContain("Error processing Instruction 0: custom program error: 0xbc4");
    }
  }, TIMEOUT);

  test("Should fail to close user with wrong vault PDA", async () => {
    const otherProvider = new BankrunProvider(context);
    otherProvider.wallet = new NodeWallet(otherUser);
    const otherQuartzProgram = new Program<Quartz>(QuartzIDL, QUARTZ_PROGRAM_ID, otherProvider);
    const otherVault = getVaultPda(otherUser.publicKey);

    await initUser(otherQuartzProgram, banksClient, {
      vault: otherVault,
      owner: otherUser.publicKey,
      systemProgram: SystemProgram.programId,
    });
    const vaultAccount = await quartzProgram.account.vault.fetch(otherVault);
    expect(vaultAccount.owner.toString()).toBe(otherUser.publicKey.toString());

    try {
      await closeUser(quartzProgram, banksClient, {
        vault: otherVault,
        owner: user.publicKey,
      });

      expect(false).toBe(true); // Should not reach this point
    } catch (error: any) {
      expect(error.message).toContain("Error processing Instruction 0: custom program error: 0x7d6");
    }
  }, TIMEOUT);
});
