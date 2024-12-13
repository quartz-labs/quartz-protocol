import { Clock, ProgramTestContext } from "solana-bankrun";
import { PublicKey } from "@solana/web3.js";
import { web3 } from "@coral-xyz/anchor";
import { QUARTZ_PROGRAM_ID } from "./constants";

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

export const toRemainingAccount = (
  pubkey: PublicKey,
  isWritable: boolean,
  isSigner: boolean
) => {
  return { pubkey, isWritable, isSigner };
};

export const getVaultPda = (owner: PublicKey) => {
  const [vault] = PublicKey.findProgramAddressSync(
    [Buffer.from("vault"), owner.toBuffer()],
    new PublicKey(QUARTZ_PROGRAM_ID)
  );
  return vault;
};

export const getVaultSplPda = (vaultPda: PublicKey, mint: PublicKey) => {
  const [vaultWSol] = web3.PublicKey.findProgramAddressSync(
    [vaultPda.toBuffer(), mint.toBuffer()],
    QUARTZ_PROGRAM_ID
  );
  return vaultWSol;
};
