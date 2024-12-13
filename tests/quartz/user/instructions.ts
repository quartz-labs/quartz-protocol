import {
  PublicKey,
  Keypair,
  SystemProgram,
  TransactionMessage,
  VersionedTransaction,
} from "@solana/web3.js";
import {
  getDriftUserStats,
  getDriftState,
  getDriftUser,
  DRIFT_PROGRAM_ID,
} from "../../utils/drift";
import { Program, web3 } from "@coral-xyz/anchor";
import { Quartz } from "../../../target/types/quartz";
import { BanksClient } from "solana-bankrun";
import { expect } from "@jest/globals";


export interface InitUserAccounts {
  vault: PublicKey;
  owner: PublicKey;
  systemProgram: PublicKey;
}

export const initUser = async (
  quartzProgram: Program<Quartz>,
  banksClient: BanksClient,
  accounts: InitUserAccounts
) => {
  const ix = await quartzProgram.methods
    .initUser()
    .accounts(accounts)
    .instruction();

  const latestBlockhash = await banksClient.getLatestBlockhash();
  const messageV0 = new TransactionMessage({
    payerKey: accounts.owner,
    recentBlockhash: latestBlockhash[0],
    instructions: [ix],
  }).compileToV0Message();

  const tx = new VersionedTransaction(messageV0);
  const meta = await banksClient.processTransaction(tx);
  return meta;
};


export interface CloseUserAccounts {
  vault: PublicKey;
  owner: PublicKey;
}

export const closeUser = async (
  quartzProgram: Program<Quartz>,
  banksClient: BanksClient,
  accounts: CloseUserAccounts
) => {
  const ix = await quartzProgram.methods
    .closeUser()
    .accounts(accounts)
    .instruction();

  const latestBlockhash = await banksClient.getLatestBlockhash();
  const messageV0 = new TransactionMessage({
    payerKey: accounts.owner,
    recentBlockhash: latestBlockhash[0],
    instructions: [ix],
  }).compileToV0Message();

  const tx = new VersionedTransaction(messageV0);
  const meta = await banksClient.processTransaction(tx);
  return meta;
}


export const initDriftAccount = async (
  quartzProgram: Program<Quartz>,
  banksClient: BanksClient,
  vaultPda: PublicKey,
  user: Keypair
) => {
  const ix = await quartzProgram.methods
    .initDriftAccount()
    .accounts({
      vault: vaultPda,
      owner: user.publicKey,
      driftUser: getDriftUser(vaultPda),
      driftUserStats: getDriftUserStats(vaultPda),
      driftState: getDriftState(),
      driftProgram: DRIFT_PROGRAM_ID,
      rent: web3.SYSVAR_RENT_PUBKEY,
      systemProgram: SystemProgram.programId,
    })
    .instruction();

  const latestBlockhash = await banksClient.getLatestBlockhash();
  const messageV0 = new TransactionMessage({
    payerKey: user.publicKey,
    recentBlockhash: latestBlockhash[0],
    instructions: [ix],
  }).compileToV0Message();

  const tx = new VersionedTransaction(messageV0);
  const meta = await banksClient.processTransaction(tx);

  expect(meta.logMessages[1]).toBe(
    "Program log: Instruction: InitDriftAccount"
  );
  expect(meta.logMessages[9]).toBe("Program log: Instruction: InitializeUser");
  expect(meta.logMessages[14]).toBe(
    "Program dRiftyHA39MWEi3m9aunc5MzRF1JYuBsbn6VPcn33UH success"
  );
  expect(meta.logMessages[16]).toBe(
    "Program 6JjHXLheGSNvvexgzMthEcgjkcirDrGduc3HAKB2P1v2 success"
  );
};
