import { Program } from "@coral-xyz/anchor";
import { BankrunProvider } from "anchor-bankrun";
import { beforeEach, describe, expect, test } from "@jest/globals";
import { startAnchor, ProgramTestContext, BanksClient } from "solana-bankrun";
import { Connection, Keypair, PublicKey, SystemProgram, SYSVAR_RENT_PUBKEY } from "@solana/web3.js";
import { IDL as QuartzIDL, Quartz } from "../../target/types/quartz";
import { getDriftState, getDriftUser, getDriftUserStats, getInitRentPayer, getVaultPda } from "../utils/accounts";
import { closeUser, initUser } from "../utils/instructions";
import { CLOSE_GAS_FEE, DRIFT_PROGRAM_ID, INIT_ACCOUNT_RENT_FEE, INIT_GAS_FEE, MARGINFI_GROUP_1, MARGINFI_PROGRAM_ID, QUARTZ_PROGRAM_ID } from "../config/constants";
import config from "../config/config";
import { hash } from "@coral-xyz/anchor/dist/cjs/utils/sha256";


const TIMEOUT = 10_000;
describe("init_user", () => {
  let provider: BankrunProvider;
  let user: Keypair;
  let marginfiAccount: Keypair;
  let vaultPda: PublicKey;
  let context: ProgramTestContext;
  let banksClient: BanksClient;
  let quartzProgram: Program<Quartz>;
  let driftState: PublicKey;
  let driftUser: PublicKey;
  let driftUserStats: PublicKey;
  let initRentPayer: PublicKey;

  beforeEach(async () => {
    user = Keypair.generate();
    marginfiAccount = Keypair.generate();
    vaultPda = getVaultPda(user.publicKey);
    driftState = getDriftState();
    driftUser = getDriftUser(vaultPda);
    driftUserStats = getDriftUserStats(vaultPda);
    initRentPayer = getInitRentPayer();

    const connection = new Connection(config.RPC_URL);
    const driftStateAccount = await connection.getAccountInfo(driftState);
    const initRentPayerAccount = await connection.getAccountInfo(initRentPayer);
    const marginfiGroupAccount = await connection.getAccountInfo(MARGINFI_GROUP_1);

    context = await startAnchor(
      "./",
      [
        { name: "drift", programId: DRIFT_PROGRAM_ID },
        { name: "marginfi", programId: MARGINFI_PROGRAM_ID }
      ],
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
          address: driftState,
          info: driftStateAccount
        },
        {
          address: initRentPayer,
          info: initRentPayerAccount
        },
        {
          address: MARGINFI_GROUP_1,
          info: marginfiGroupAccount
        }
      ]
    );

    provider = new BankrunProvider(context);
    quartzProgram = new Program<Quartz>(QuartzIDL, QUARTZ_PROGRAM_ID, provider);
    banksClient = context.banksClient;
  }, TIMEOUT);

  test("Should init user", async () => {
    const startBalance = await banksClient.getBalance(user.publicKey);
    
    const meta = await initUser(
      quartzProgram, 
      banksClient,
      [user],
      {
        requiresMarginfiAccount: true,
        spendLimitPerTransaction: 1000_000_000,
        spendLimitPerTimeframe: 1000_000_000,
        extendSpendLimitPerTimeframeResetSlotAmount: 1000_000_000,
      },
      {
        vault: vaultPda,
        owner: user.publicKey,
        initRentPayer: initRentPayer,
        driftUser: driftUser,
        driftUserStats: driftUserStats,
        driftState: driftState,
        driftProgram: DRIFT_PROGRAM_ID,
        marginfiGroup: MARGINFI_GROUP_1,
        marginfiAccount: marginfiAccount.publicKey,
        marginfiProgram: MARGINFI_PROGRAM_ID,
        rent: SYSVAR_RENT_PUBKEY,
        systemProgram: SystemProgram.programId,
      }
    );

    const endBalance = await banksClient.getBalance(user.publicKey);
    expect(endBalance).toBe(startBalance - INIT_ACCOUNT_RENT_FEE - INIT_GAS_FEE);

    expect(meta.logMessages[1]).toBe("Program log: Instruction: InitUser");
    expect(meta.logMessages[3]).toBe("Program 11111111111111111111111111111111 success");
    expect(meta.logMessages[5]).toBe("Program 11111111111111111111111111111111 success");
    expect(meta.logMessages[7]).toBe("Program log: Instruction: InitializeUserStats");
    expect(meta.logMessages[11]).toBe("Program dRiftyHA39MWEi3m9aunc5MzRF1JYuBsbn6VPcn33UH success");
    expect(meta.logMessages[13]).toBe("Program log: Instruction: InitializeUser");
    expect(meta.logMessages[18]).toBe("Program dRiftyHA39MWEi3m9aunc5MzRF1JYuBsbn6VPcn33UH success");
    expect(meta.logMessages[20]).toBe("Program log: Instruction: MarginfiAccountInitialize");
    expect(meta.logMessages[25]).toBe("Program MFv2hWf31Z9kbCa1snEPYctwafyhdvnV7FZnsebVacA success");
    expect(meta.logMessages[27]).toBe("Program 6JjHXLheGSNvvexgzMthEcgjkcirDrGduc3HAKB2P1v2 success");

    const vaultAccount = await quartzProgram.account.vault.fetch(vaultPda);
    expect(vaultAccount.owner.toString()).toBe(user.publicKey.toString());
  }, TIMEOUT);

  // TODO: Add expected fail tests
});


describe("close_user", () => {
  let provider: BankrunProvider;
  let user: Keypair;
  let marginfiAccount: Keypair;
  let vaultPda: PublicKey;
  let context: ProgramTestContext;
  let banksClient: BanksClient;
  let quartzProgram: Program<Quartz>;
  let driftState: PublicKey;
  let driftUser: PublicKey;
  let driftUserStats: PublicKey;
  let initRentPayer: PublicKey;


  beforeEach(async () => {
    user = Keypair.generate();
    marginfiAccount = Keypair.generate();
    vaultPda = getVaultPda(user.publicKey);
    driftState = getDriftState();
    driftUser = getDriftUser(vaultPda);
    driftUserStats = getDriftUserStats(vaultPda);
    initRentPayer = getInitRentPayer();

    const connection = new Connection(config.RPC_URL);
    const driftStateAccount = await connection.getAccountInfo(driftState);
    const initRentPayerAccount = await connection.getAccountInfo(initRentPayer);
    const marginfiGroupAccount = await connection.getAccountInfo(MARGINFI_GROUP_1);
    const rentAccount = await connection.getAccountInfo(SYSVAR_RENT_PUBKEY);
    
    context = await startAnchor(
      "./",
      [
        { name: "drift", programId: DRIFT_PROGRAM_ID },
        { name: "marginfi", programId: MARGINFI_PROGRAM_ID }
      ],
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
          address: driftState,
          info: driftStateAccount
        },
        {
          address: initRentPayer,
          info: initRentPayerAccount
        },
        {
          address: MARGINFI_GROUP_1,
          info: marginfiGroupAccount
        },
        {
          address: SYSVAR_RENT_PUBKEY,
          info: rentAccount
        }
      ]
    );

    provider = new BankrunProvider(context);
    quartzProgram = new Program<Quartz>(QuartzIDL, QUARTZ_PROGRAM_ID, provider);
    banksClient = context.banksClient;

    await initUser(
      quartzProgram, 
      banksClient,
      [user, marginfiAccount],
      {
        requiresMarginfiAccount: true,
        spendLimitPerTransaction: 1000_000_000,
        spendLimitPerTimeframe: 1000_000_000,
        extendSpendLimitPerTimeframeResetSlotAmount: 1000_000_000,
      },
      {
        vault: vaultPda,
        owner: user.publicKey,
        initRentPayer: initRentPayer,
        driftUser: driftUser,
        driftUserStats: driftUserStats,
        driftState: driftState,
        driftProgram: DRIFT_PROGRAM_ID,
        marginfiGroup: MARGINFI_GROUP_1,
        marginfiAccount: marginfiAccount.publicKey,
        marginfiProgram: MARGINFI_PROGRAM_ID,
        rent: SYSVAR_RENT_PUBKEY,
        systemProgram: SystemProgram.programId,
      }
    );
  }, TIMEOUT);

  test("Should close user", async () => {
    const startBalance = await banksClient.getBalance(user.publicKey);

    const meta = await closeUser(
      quartzProgram, 
      banksClient, 
      {
        vault: vaultPda,
        owner: user.publicKey,
        initRentPayer: initRentPayer,
        driftUser: driftUser,
        driftUserStats: driftUserStats,
        driftState: driftState,
        driftProgram: DRIFT_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      }
    );

    const endBalance = await banksClient.getBalance(user.publicKey);
    expect(endBalance).toBe(startBalance + INIT_ACCOUNT_RENT_FEE - CLOSE_GAS_FEE);

    expect(meta.logMessages[1]).toBe("Program log: Instruction: CloseUser");
    expect(meta.logMessages[3]).toBe("Program log: Instruction: DeleteUser");
    expect(meta.logMessages[5]).toBe("Program dRiftyHA39MWEi3m9aunc5MzRF1JYuBsbn6VPcn33UH success");
    expect(meta.logMessages[7]).toBe("Program 11111111111111111111111111111111 success");
    expect(meta.logMessages[9]).toBe("Program 6JjHXLheGSNvvexgzMthEcgjkcirDrGduc3HAKB2P1v2 success");

    try {
      await quartzProgram.account.vault.fetch(vaultPda);
      expect(false).toBe(true); // Should not reach this point
    } catch (error: any) {
      expect(error.message).toContain("Could not find");
    }
  }, TIMEOUT);

  // TODO: Add expected fail tests
});
