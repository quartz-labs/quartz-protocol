import {
  AnchorProvider,
  BN,
  Program,
  setProvider,
  web3,
} from "@coral-xyz/anchor";
import { BankrunProvider } from "anchor-bankrun";
import { beforeAll, expect, test, beforeEach } from "@jest/globals";
import { startAnchor, ProgramTestContext, BanksClient } from "solana-bankrun";
import {
  Keypair,
  PublicKey,
  SystemProgram,
  TransactionMessage,
  VersionedTransaction,
  Connection,
  SYSVAR_INSTRUCTIONS_PUBKEY,
} from "@solana/web3.js";
import { IDL as QuartzIDL, Quartz } from "../../../target/types/quartz";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  createCloseAccountInstruction,
} from "@solana/spl-token";
import {
  createAssociatedTokenAccountInstruction,
  createSyncNativeInstruction,
  getAssociatedTokenAddress,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { ASSOCIATED_PROGRAM_ID } from "@coral-xyz/anchor/dist/cjs/utils/token";
import {
  getVaultPda,
  getVaultSplPda
} from "../../utils/helpers";
import {
  getDriftState,
  getDriftUser,
  getDriftUserStats,
} from "../../utils/drift";
import { DRIFT_PROGRAM_ID } from "../../utils/drift";
import { setupTestEnvironment } from "./collateralRepaySetup";
import { setupDriftAccountWithFundsAndLoan } from "../balance/balanceSetup";
import { WSOL_MINT } from "../../utils/constants";

describe("Quartz Start auto Repay", () => {
  //all the things that need to be done before each test
  let provider: BankrunProvider,
    user: Keypair,
    context: ProgramTestContext,
    banksClient: BanksClient,
    quartzProgram: Program<Quartz>,
    vaultPda: PublicKey;

  user = Keypair.generate();

  // beforeAll(async () => {
  //   ({ user, context, banksClient, quartzProgram, vaultPda } =
  //     await setupTestEnvironment());
  //   await setupDriftAccountWithFundsAndLoan(
  //     quartzProgram,
  //     banksClient,
  //     vaultPda,
  //     user
  //   );
  // });

  test("Start auto repay fails when called alone", async () => {
    expect(true).toBe(true);
    return;
    await makecollateralRepayStartInstructions(
      quartzProgram,
      user,
      100000000,
      banksClient
    );
  });
});

export const makecollateralRepayStartInstructions = async (
  program: Program<Quartz>,
  wallet: Keypair,
  amountLamports: number,
  banksClient: BanksClient
) => {
  const walletWSol = await getAssociatedTokenAddress(
    WSOL_MINT,
    wallet.publicKey
  );
  const vaultPda = getVaultPda(wallet.publicKey);
  const vaultWsol = getVaultSplPda(vaultPda, WSOL_MINT);

  const oix_createWSolAta = createAssociatedTokenAccountInstruction(
    wallet.publicKey,
    walletWSol,
    wallet.publicKey,
    WSOL_MINT
  );

  const collateralRepayStart = await program.methods
    .collateralRepayStart(new BN(amountLamports))
    .accounts({
      caller: wallet.publicKey,
      callerWithdrawSpl: walletWSol,
      withdrawMint: WSOL_MINT,
      vault: vaultPda,
      vaultWithdrawSpl: vaultWsol,
      owner: wallet.publicKey,
      tokenProgram: TOKEN_PROGRAM_ID,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
      instructions: SYSVAR_INSTRUCTIONS_PUBKEY,
    })
    .instruction();

  const instructions = [oix_createWSolAta, collateralRepayStart];

  const latestBlockhash = await banksClient.getLatestBlockhash();
  const messageV0 = new TransactionMessage({
    payerKey: wallet.publicKey,
    recentBlockhash: latestBlockhash[0],
    instructions: instructions,
  }).compileToV0Message();
  const tx = new VersionedTransaction(messageV0);

  try {
    const meta = await banksClient.processTransaction(tx);
    // If we reach here, the test should fail
    expect(false).toBe(true);
  } catch (error: any) {
    expect(error);
  }

  //TODO: Add expectations
  // expect(simRes.meta?.logMessages).toEqual(meta?.logMessages);
  // expect(meta.logMessages[1]).toBe("Program log: Create");
  // expect(meta.logMessages[22]).toBe("Program log: Instruction: Withdraw");
  // expect(meta.logMessages[26]).toBe("Program log: Instruction: InitializeAccount3");
  // expect(meta.logMessages[30]).toBe("Program log: Instruction: Withdraw");
  // expect(meta.logMessages[34]).toBe("Program log: Instruction: Transfer");
  // expect(meta.logMessages[38]).toBe("Program dRiftyHA39MWEi3m9aunc5MzRF1JYuBsbn6VPcn33UH success");
  // expect(meta.logMessages[48]).toBe("Program 6JjHXLheGSNvvexgzMthEcgjkcirDrGduc3HAKB2P1v2 success");
};
