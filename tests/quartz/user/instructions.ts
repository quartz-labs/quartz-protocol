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


export interface InitDriftAccountAccounts {
  vault: PublicKey;
  owner: PublicKey;
  driftUser: PublicKey;
  driftUserStats: PublicKey;
  driftState: PublicKey;
  driftProgram: PublicKey;
  rent: PublicKey;
  systemProgram: PublicKey;
}

export const initDriftAccount = async (
  quartzProgram: Program<Quartz>,
  banksClient: BanksClient,
  accounts: InitDriftAccountAccounts
) => {
  const ix = await quartzProgram.methods
    .initDriftAccount()
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


export interface CloseDriftAccountAccounts {
  vault: PublicKey;
  owner: PublicKey;
  driftUser: PublicKey;
  driftUserStats: PublicKey;
  driftState: PublicKey;
  driftProgram: PublicKey;
}

export const closeDriftAccount = async (
  quartzProgram: Program<Quartz>,
  banksClient: BanksClient,
  accounts: CloseDriftAccountAccounts
) => {
  const ix = await quartzProgram.methods
    .closeDriftAccount()
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
