import { Program } from "@coral-xyz/anchor";
import { BankrunProvider } from "anchor-bankrun";
import { expect, test } from "@jest/globals";
import { startAnchor, ProgramTestContext, BanksClient } from "solana-bankrun";
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import { IDL as QuartzIDL, Quartz } from "../../../target/types/quartz";
import { getVault, QUARTZ_PROGRAM_ID } from "../../utils/helpers";
import { closeUser, initUser } from "./instructions";
import NodeWallet from "@coral-xyz/anchor/dist/cjs/nodewallet";


describe("init_user", () => {
  let provider: BankrunProvider;
  let user: Keypair;
  let vaultPda: PublicKey;
  let context: ProgramTestContext;
  let banksClient: BanksClient;
  let quartzProgram: Program<Quartz>;

  beforeEach(async () => {
    user = Keypair.generate();
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
    vaultPda = getVault(user.publicKey);
  });

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
  });

  test("Should fail to init user with wrong vault PDA seed", async () => {
    const [badVaultPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("bad_vault"), user.publicKey.toBuffer()],
      new PublicKey(QUARTZ_PROGRAM_ID)
    );

    try {
      const meta = await initUser(quartzProgram, banksClient, {
        vault: badVaultPda,
        owner: user.publicKey,
        systemProgram: SystemProgram.programId,
      });

      expect(meta.logMessages[1]).toBe("Program log: Instruction: InitUser");
      expect(meta.logMessages[2]).toBe(
        "Program log: AnchorError caused by account: vault. Error Code: ConstraintSeeds. Error Number: 2006. Error Message: A seeds constraint was violated."
      );

      expect(false).toBe(true); // Should not reach this point
    } catch (error: any) {
      expect(error.message).toContain("Error processing Instruction 0: custom program error: 0x7d6");
    }
  });

  test("Should fail to init user with wrong vault PDA owner", async () => {
    const otherVault = getVault(Keypair.generate().publicKey);

    try {
      const meta = await initUser(quartzProgram, banksClient, {
        vault: otherVault,
        owner: user.publicKey,
        systemProgram: SystemProgram.programId,
      });

      expect(meta.logMessages[1]).toBe("Program log: Instruction: InitUser");
      expect(meta.logMessages[2]).toBe(
        "Program log: AnchorError caused by account: vault. Error Code: ConstraintSeeds. Error Number: 2006. Error Message: A seeds constraint was violated."
      );

      expect(false).toBe(true); // Should not reach this point
    } catch (error: any) {
      expect(error.message).toContain("Error processing Instruction 0: custom program error: 0x7d6");
    }
  });

  test("Should fail to init user with wrong system program", async () => {
    try {
      const meta = await initUser(quartzProgram, banksClient, {
        vault: vaultPda,
        owner: user.publicKey,
        systemProgram: Keypair.generate().publicKey,
      });

      expect(meta.logMessages[1]).toBe("Program log: Instruction: InitUser");

      expect(false).toBe(true); // Should not reach this point
    } catch (error: any) {
      expect(error.message).toContain("Error processing Instruction 0: custom program error: 0xbc0");
    }
  });
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

    vaultPda = getVault(user.publicKey);
    await initUser(quartzProgram, banksClient, {
      vault: vaultPda,
      owner: user.publicKey,
      systemProgram: SystemProgram.programId,
    });
  });

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
  });

  test("Should fail to close user of vault that doesn't exist", async () => {
    const randomVault = getVault(Keypair.generate().publicKey);

    try {
      const meta = await closeUser(quartzProgram, banksClient, {
        vault: randomVault,
        owner: user.publicKey,
      });

      expect(meta.logMessages[1]).toBe("Program log: Instruction: CloseUser");
      expect(meta.logMessages[2]).toBe(
        "Program log: AnchorError caused by account: vault. Error Code: AccountNotInitialized. Error Number: 3012. Error Message: The program expected this account to be already initialized."
      );

      expect(false).toBe(true); // Should not reach this point
    } catch (error: any) {
      expect(error.message).toContain("Error processing Instruction 0: custom program error: 0xbc4");
    }
  });

  test("Should fail to close user with wrong vault PDA", async () => {
    const otherProvider = new BankrunProvider(context);
    otherProvider.wallet = new NodeWallet(otherUser);
    const otherQuartzProgram = new Program<Quartz>(QuartzIDL, QUARTZ_PROGRAM_ID, otherProvider);
    const otherVault = getVault(otherUser.publicKey);

    await initUser(otherQuartzProgram, banksClient, {
      vault: otherVault,
      owner: otherUser.publicKey,
      systemProgram: SystemProgram.programId,
    });
    const vaultAccount = await quartzProgram.account.vault.fetch(otherVault);
    expect(vaultAccount.owner.toString()).toBe(otherUser.publicKey.toString());

    try {
      const meta = await closeUser(quartzProgram, banksClient, {
        vault: otherVault,
        owner: user.publicKey,
      });

      expect(meta.logMessages[1]).toBe("Program log: Instruction: CloseUser");
      expect(meta.logMessages[2]).toBe(
        "Program log: AnchorError caused by account: vault. Error Code: ConstraintSeeds. Error Number: 2006. Error Message: A seeds constraint was violated."
      );

      expect(false).toBe(true); // Should not reach this point
    } catch (error: any) {
      expect(error.message).toContain("Error processing Instruction 0: custom program error: 0x7d6");
    }
  });
});
