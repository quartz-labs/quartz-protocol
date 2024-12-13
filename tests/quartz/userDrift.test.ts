import { BN, Program, web3 } from "@coral-xyz/anchor";
import { BankrunProvider } from "anchor-bankrun";
import { beforeEach, describe, expect, test } from "@jest/globals";
import {
  startAnchor,
  ProgramTestContext,
  BanksClient
} from "solana-bankrun";
import {
  Keypair,
  PublicKey,
  SystemProgram,
  Connection,
} from "@solana/web3.js";
import { IDL as QuartzIDL, Quartz } from "../../target/types/quartz";
import {
  getDriftState,
  getDriftUser,
  getDriftUserStats,
  getVaultPda
} from "../utils/accounts";
import { closeDriftAccount, closeUser, initDriftAccount, initUser } from "../utils/instructions";
import { QUARTZ_PROGRAM_ID, DRIFT_PROGRAM_ID } from "../config/constants";
import config from "../config/config";


describe("init_drift_account, close_drift_account", () => {
  let provider: BankrunProvider;
  let user: Keypair;
  let otherUser: Keypair;
  let context: ProgramTestContext;
  let banksClient: BanksClient;
  let quartzProgram: Program<Quartz>;
  let vaultPda: PublicKey;

  let driftUser: PublicKey;
  let driftUserStats: PublicKey;
  let driftState: PublicKey;

  beforeEach(async () => {
    user = Keypair.generate();
    otherUser = Keypair.generate();
    vaultPda = getVaultPda(user.publicKey);
    driftState = getDriftState();
    driftUser = getDriftUser(vaultPda);
    driftUserStats = getDriftUserStats(vaultPda);

    const connection = new Connection(config.RPC_URL);
    const driftStateAccount = await connection.getAccountInfo(driftState);

    context = await startAnchor(
      "./",
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
          address: otherUser.publicKey,
          info: {
            lamports: 1_000_000_000,
            data: Buffer.alloc(0),
            owner: SystemProgram.programId,
            executable: false,
          },
        },
        {
          address: driftState,
          info: {
            ...driftStateAccount,
            executable: false,
            owner: DRIFT_PROGRAM_ID,
          },
        },
      ]
    );

    banksClient = context.banksClient;
    provider = new BankrunProvider(context);
    quartzProgram = new Program<Quartz>(QuartzIDL, QUARTZ_PROGRAM_ID, provider);

    await initUser(quartzProgram, banksClient, {
      vault: vaultPda,
      owner: user.publicKey,
      systemProgram: SystemProgram.programId,
    });

    const vaultAccount = await quartzProgram.account.vault.fetch(vaultPda);
    expect(vaultAccount.owner.toString()).toBe(user.publicKey.toString());
  });

  test("Should init Drift account", async () => {
    const meta = await initDriftAccount(quartzProgram, banksClient, {
      vault: vaultPda,
      owner: user.publicKey,
      driftUser: driftUser,
      driftUserStats: driftUserStats,
      driftState: driftState,
      driftProgram: DRIFT_PROGRAM_ID,
      rent: web3.SYSVAR_RENT_PUBKEY,
      systemProgram: SystemProgram.programId,
    });

    expect(meta.logMessages[1]).toBe("Program log: Instruction: InitDriftAccount");
    expect(meta.logMessages[9]).toBe("Program log: Instruction: InitializeUser");
    expect(meta.logMessages[14]).toBe("Program dRiftyHA39MWEi3m9aunc5MzRF1JYuBsbn6VPcn33UH success");
    expect(meta.logMessages[16]).toBe("Program 6JjHXLheGSNvvexgzMthEcgjkcirDrGduc3HAKB2P1v2 success");

    const driftAccount = await banksClient.getAccount(driftUser);
    expect(driftAccount).not.toBeNull();
    expect(driftAccount.owner.toBase58()).toBe(DRIFT_PROGRAM_ID.toBase58());
  });

  test("Should not init Drift account after user is closed", async () => {
    await closeUser(quartzProgram, banksClient, {
      vault: vaultPda,
      owner: user.publicKey,
    });

    try {
      await initDriftAccount(quartzProgram, banksClient, {
        vault: vaultPda,
        owner: user.publicKey,
        driftUser: driftUser,
        driftUserStats: driftUserStats,
        driftState: driftState,
        driftProgram: DRIFT_PROGRAM_ID,
        rent: web3.SYSVAR_RENT_PUBKEY,
        systemProgram: SystemProgram.programId,
      });

      expect(false).toBe(true); // Should not reach this point
    } catch (error: any) {
      expect(error.message).toContain("Error processing Instruction 0: custom program error: 0xbc4");
    }
  });

  test("Should not init Drift account before user is initted", async () => {
    const otherVaultPda = getVaultPda(otherUser.publicKey);

    try {
      await initDriftAccount(quartzProgram, banksClient, {
        vault: otherVaultPda,
        owner: otherUser.publicKey,
        driftUser: getDriftUser(otherVaultPda),
        driftUserStats: getDriftUserStats(otherVaultPda),
        driftState: driftState,
        driftProgram: DRIFT_PROGRAM_ID,
        rent: web3.SYSVAR_RENT_PUBKEY,
        systemProgram: SystemProgram.programId,
      });

      expect(false).toBe(true); // Should not reach this point
    } catch (error: any) {
      expect(error.message).toContain("Error processing Instruction 0: custom program error: 0xbc4");
    }
  });


  test("Should close Drift account", async () => {
    await initDriftAccount(quartzProgram, banksClient, {
      vault: vaultPda,
      owner: user.publicKey,
      driftUser: driftUser,
      driftUserStats: driftUserStats,
      driftState: driftState,
      driftProgram: DRIFT_PROGRAM_ID,
      rent: web3.SYSVAR_RENT_PUBKEY,
      systemProgram: SystemProgram.programId,
    });

    const driftAccountBefore = await banksClient.getAccount(driftUser);
    expect(driftAccountBefore).not.toBeNull();
    expect(driftAccountBefore.owner.toBase58()).toBe(DRIFT_PROGRAM_ID.toBase58());

    const meta = await closeDriftAccount(quartzProgram, banksClient, {
      vault: vaultPda,
      owner: user.publicKey,
      driftUser: driftUser,
      driftUserStats: driftUserStats,
      driftState: driftState,
      driftProgram: DRIFT_PROGRAM_ID,
    });

    expect(meta.logMessages[1]).toBe("Program log: Instruction: CloseDriftAccount");
    expect(meta.logMessages[3]).toBe("Program log: Instruction: DeleteUser");
    expect(meta.logMessages[5]).toBe("Program dRiftyHA39MWEi3m9aunc5MzRF1JYuBsbn6VPcn33UH success");
    expect(meta.logMessages[7]).toBe("Program 6JjHXLheGSNvvexgzMthEcgjkcirDrGduc3HAKB2P1v2 success");

    const driftAccountAfter = await banksClient.getAccount(driftUser);
    expect(driftAccountAfter).toBeNull();
  });

  test("Should not close Drift account if drift account is not initted", async () => {
    try {
      await closeDriftAccount(quartzProgram, banksClient, {
        vault: vaultPda,
        owner: user.publicKey,
        driftUser: driftUser,
        driftUserStats: driftUserStats,
        driftState: driftState,
        driftProgram: DRIFT_PROGRAM_ID,
      });

      expect(false).toBe(true); // Should not reach this point
    } catch (error: any) {
      console.log(error);
      expect(error.message).toContain("Error processing Instruction 0: custom program error: 0xbbf");
    }
  });

  test("Should not close Drift account if user is not initted", async () => {
    const otherVaultPda = getVaultPda(otherUser.publicKey);
    const otherDriftUser = getDriftUser(otherVaultPda);
    const otherDriftUserStats = getDriftUserStats(otherVaultPda);
    
    try {
      await closeDriftAccount(quartzProgram, banksClient, {
        vault: otherVaultPda,
        owner: otherUser.publicKey,
        driftUser: otherDriftUser,
        driftUserStats: otherDriftUserStats,
        driftState: driftState,
        driftProgram: DRIFT_PROGRAM_ID,
      });

      expect(false).toBe(true); // Should not reach this point
    } catch (error: any) {
      console.log(error);
      expect(error.message).toContain("Error processing Instruction 0: custom program error: 0xbc4");
    }
  });

  test("Should not close Drift account with incorrect vault", async () => {
    const otherVaultPda = getVaultPda(otherUser.publicKey);

    try {
      await closeDriftAccount(quartzProgram, banksClient, {
        vault: otherVaultPda,
        owner: user.publicKey,
        driftUser: driftUser,
        driftUserStats: driftUserStats,
        driftState: driftState,
        driftProgram: DRIFT_PROGRAM_ID,
      });

      expect(false).toBe(true); // Should not reach this point
    } catch (error: any) {
      console.log(error);
      expect(error.message).toContain("Error processing Instruction 0: custom program error: 0xbc4");
    }
  });

  test("Should not close Drift account with incorrect owner", async () => {
    try {
      await closeDriftAccount(quartzProgram, banksClient, {
        vault: vaultPda,
        owner: otherUser.publicKey,
        driftUser: driftUser,
        driftUserStats: driftUserStats,
        driftState: driftState,
        driftProgram: DRIFT_PROGRAM_ID,
      });

      expect(false).toBe(true); // Should not reach this point
    } catch (error: any) {
      console.log(error);
      expect(error.message).toContain("Error processing Instruction 0: custom program error: 0xbbf");
    }
  });

  test("Should not close Drift account with incorrect drift user", async () => {
    const otherDriftUser = getDriftUser(otherUser.publicKey);

    try {
      await closeDriftAccount(quartzProgram, banksClient, {
        vault: vaultPda,
        owner: user.publicKey,
        driftUser: otherDriftUser,
        driftUserStats: driftUserStats,
        driftState: driftState,
        driftProgram: DRIFT_PROGRAM_ID,
      });

      expect(false).toBe(true); // Should not reach this point
    } catch (error: any) {
      console.log(error);
      expect(error.message).toContain("Error processing Instruction 0: custom program error: 0xbbf");
    }
  });

  test("Should not close Drift account with incorrect drift user stats", async () => {
    const otherDriftUserStats = getDriftUserStats(otherUser.publicKey);

    try {
      await closeDriftAccount(quartzProgram, banksClient, {
        vault: vaultPda,
        owner: user.publicKey,
        driftUser: driftUser,
        driftUserStats: otherDriftUserStats,
        driftState: driftState,
        driftProgram: DRIFT_PROGRAM_ID,
      });

      expect(false).toBe(true); // Should not reach this point
    } catch (error: any) {
      console.log(error);
      expect(error.message).toContain("Error processing Instruction 0: custom program error: 0xbbf");
    }
  });

  test("Should not close Drift account with incorrect drift state", async () => {
    try {
      await closeDriftAccount(quartzProgram, banksClient, {
        vault: vaultPda,
        owner: user.publicKey,
        driftUser: driftUser,
        driftUserStats: driftUserStats,
        driftState: Keypair.generate().publicKey,
        driftProgram: DRIFT_PROGRAM_ID,
      });

      expect(false).toBe(true); // Should not reach this point
    } catch (error: any) {
      console.log(error);
      expect(error.message).toContain("Error processing Instruction 0: custom program error: 0xbbf");
    }
  });

  test("Should not close Drift account with incorrect program", async () => {
    try {
      await closeDriftAccount(quartzProgram, banksClient, {
        vault: vaultPda,
        owner: user.publicKey,
        driftUser: driftUser,
        driftUserStats: driftUserStats,
        driftState: driftState,
        driftProgram: Keypair.generate().publicKey,
      });

      expect(false).toBe(true); // Should not reach this point
    } catch (error: any) {
      console.log(error);
      expect(error.message).toContain("Error processing Instruction 0: custom program error: 0xbbf");
    }
  });
});
