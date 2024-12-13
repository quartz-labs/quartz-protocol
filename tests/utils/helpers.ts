import { BanksClient, Clock, ProgramTestContext } from "solana-bankrun";
import { PublicKey, TransactionMessage, VersionedTransaction, TransactionInstruction } from "@solana/web3.js";
import { web3 } from "@coral-xyz/anchor";
import { QUARTZ_PROGRAM_ID } from "../config/constants";

export const advanceBySlots = async (
  context: ProgramTestContext,
  slots: bigint
) => {
  const currentClock = await context.banksClient.getClock();
  context.setClock(
    new Clock(
      currentClock.slot + slots,
      currentClock.epochStartTimestamp,
      currentClock.epoch,
      currentClock.leaderScheduleEpoch,
      50n
    )
  );
};

export const processTransaction = async (
  banksClient: BanksClient,
  payer: PublicKey,
  instructions: TransactionInstruction[],
) => {
  const latestBlockhash = await banksClient.getLatestBlockhash();
  const messageV0 = new TransactionMessage({
      payerKey: payer,
      recentBlockhash: latestBlockhash[0],
      instructions: instructions,
  }).compileToV0Message();

  const tx = new VersionedTransaction(messageV0);
  const meta = await banksClient.processTransaction(tx);
  return meta;
};
