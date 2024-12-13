import { BN, Program } from "@coral-xyz/anchor";
import { BankrunProvider } from "anchor-bankrun";
import { beforeAll, expect, test } from "@jest/globals";
import { ProgramTestContext, BanksClient } from "solana-bankrun";
import {
  Keypair,
  PublicKey,
  SystemProgram,
  TransactionMessage,
  VersionedTransaction,
} from "@solana/web3.js";
import { IDL as QuartzIDL, Quartz } from "../../../target/types/quartz";
import { createCloseAccountInstruction } from "@solana/spl-token";
import {
  createAssociatedTokenAccountInstruction,
  createSyncNativeInstruction,
  getAssociatedTokenAddress,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { ASSOCIATED_PROGRAM_ID } from "@coral-xyz/anchor/dist/cjs/utils/token";
import {
  getVaultPda,
  getVaultSplPda,
  toRemainingAccount
} from "../../utils/helpers";
import { WSOL_MINT } from "../../utils/constants";
import {
  DRIFT_MARKET_INDEX_SOL,
  DRIFT_ORACLE_1,
  getDriftSpotMarketVault,
  getDriftState,
  getDriftUser,
  getDriftUserStats,
} from "../../utils/drift";
import { DRIFT_SPOT_MARKET_SOL } from "../../utils/drift";
import { DRIFT_PROGRAM_ID } from "../../utils/drift";
import {
  setupQuartzAndDriftAccount,
  setupTestEnvironment,
} from "./balanceSetup";

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
    ({ user, context, banksClient, quartzProgram, vaultPda } =
      await setupTestEnvironment());
    await setupQuartzAndDriftAccount(
      quartzProgram,
      banksClient,
      vaultPda,
      user
    );
  });

  test("Deposit Lamports", async () => {
    await makeDriftLamportDeposit(
      quartzProgram,
      user,
      100_000_000,
      banksClient,
      WSOL_MINT
    );
  });
});

export const makeDriftLamportDeposit = async (
  program: Program<Quartz>,
  wallet: Keypair,
  amountLamports: number,
  banksClient: BanksClient,
  splMint: PublicKey
) => {
  const walletWSol = await getAssociatedTokenAddress(splMint, wallet.publicKey);
  const vaultPda = getVaultPda(wallet.publicKey);

  const oix_createWSolAta = createAssociatedTokenAccountInstruction(
    wallet.publicKey,
    walletWSol,
    wallet.publicKey,
    splMint
  );
  const ix_wrapSol = SystemProgram.transfer({
    fromPubkey: wallet.publicKey,
    toPubkey: walletWSol,
    lamports: amountLamports,
  });

  const ix_syncNative = createSyncNativeInstruction(walletWSol);

  const ix_deposit = await program.methods
    .deposit(new BN(amountLamports), DRIFT_MARKET_INDEX_SOL, false)
    .accounts({
      vault: vaultPda,
      vaultSpl: getVaultSplPda(vaultPda, splMint),
      owner: wallet.publicKey,
      ownerSpl: walletWSol,
      splMint: splMint,
      driftUser: getDriftUser(vaultPda),
      driftUserStats: getDriftUserStats(vaultPda),
      driftState: getDriftState(),
      spotMarketVault: getDriftSpotMarketVault(DRIFT_MARKET_INDEX_SOL),
      tokenProgram: TOKEN_PROGRAM_ID,
      associatedTokenProgram: ASSOCIATED_PROGRAM_ID,
      driftProgram: DRIFT_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
    })
    .remainingAccounts([
      toRemainingAccount(DRIFT_ORACLE_1, false, false),
      toRemainingAccount(DRIFT_SPOT_MARKET_SOL, true, false),
    ])
    .instruction();

  const ix_closeWSolAta = createCloseAccountInstruction(
    walletWSol,
    wallet.publicKey,
    wallet.publicKey
  );

  const instructions = [
    oix_createWSolAta,
    ix_wrapSol,
    ix_syncNative,
    ix_deposit,
    ix_closeWSolAta,
  ];

  const latestBlockhash = await banksClient.getLatestBlockhash();
  const messageV0 = new TransactionMessage({
    payerKey: wallet.publicKey,
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
  expect(meta.logMessages[48]).toBe(
    "Program dRiftyHA39MWEi3m9aunc5MzRF1JYuBsbn6VPcn33UH success"
  );
  expect(meta.logMessages[54]).toBe(
    "Program 6JjHXLheGSNvvexgzMthEcgjkcirDrGduc3HAKB2P1v2 success"
  );
};
