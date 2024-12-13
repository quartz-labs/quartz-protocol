import { assert } from "chai";
import { Clock, ProgramTestContext } from "solana-bankrun";
import dotenv from "dotenv";
import { PublicKey } from "@solana/web3.js";
import { web3 } from "@coral-xyz/anchor";

dotenv.config();

export const RPC_URL =
  process.env.RPC_URL || "https://api.mainnet-beta.solana.com";
export const QUARTZ_PROGRAM_ID = new PublicKey(
  "6JjHXLheGSNvvexgzMthEcgjkcirDrGduc3HAKB2P1v2"
);
export const WSOL_MINT = new PublicKey(
  "So11111111111111111111111111111111111111112"
);
export const USDC_MINT = new PublicKey(
  "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
); // Mainnet devnet mint

export const expectError = (
  expectedError: string,
  message: string
): [() => void, (e: any) => void] => {
  return [
    () => assert.fail(message),
    (e) => {
      assert(e.error != undefined, `problem retrieving program error: ${e}`);
      assert(
        e.error.errorCode != undefined,
        "problem retrieving program error code"
      );
      //for (let idlError of program.idl.errors) {
      //  if (idlError.code == e.code) {
      //    assert.equal(idlError.name, expectedError);
      //    return;
      //  }
      //}
      assert.equal(
        e.error.errorCode.code,
        expectedError,
        `the program threw for a reason that we didn't expect. error : ${e}`
      );
      /* assert.fail("error doesn't match idl"); */
      /* console.log(program.idl.errors); */
      /* assert( */
      /*   e["error"] != undefined, */
      /*   `the program threw for a reason that we didn't expect. error: ${e}` */
      /* ); */
      /* assert.equal(e.error.errorCode.code, expectedErrorCode); */
    },
  ];
};

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

export const getVault = (owner: PublicKey) => {
  const [vault] = PublicKey.findProgramAddressSync(
    [Buffer.from("vault"), owner.toBuffer()],
    new PublicKey(QUARTZ_PROGRAM_ID)
  );
  return vault;
};

export const getVaultSpl = (vaultPda: PublicKey, mint: PublicKey) => {
  const [vaultWSol] = web3.PublicKey.findProgramAddressSync(
    [vaultPda.toBuffer(), mint.toBuffer()],
    QUARTZ_PROGRAM_ID
  );
  return vaultWSol;
};
