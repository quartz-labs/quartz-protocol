import { PublicKey } from "@solana/web3.js";
import { web3 } from "@coral-xyz/anchor";
import { BN } from "bn.js";

export const DRIFT_PROGRAM_ID = new PublicKey(
  "dRiftyHA39MWEi3m9aunc5MzRF1JYuBsbn6VPcn33UH"
);
export const DRIFT_MARKET_INDEX_USDC = 0;
export const DRIFT_MARKET_INDEX_SOL = 1;
export const DRIFT_ORACLE_1 = new PublicKey(
  "BAtFj4kQttZRVep3UZS2aZRDixkGYgWsbqTBVDbnSsPF"
); // Mainnet
export const DRIFT_ORACLE_2 = new PublicKey(
  "En8hkHLkRe9d9DraYmBTrus518BvmVH448YcvmrFM6Ce"
);
export const DRIFT_SPOT_MARKET_SOL = new PublicKey(
  "3x85u7SWkmmr7YQGYhtjARgxwegTLJgkSLRprfXod6rh"
);
export const DRIFT_SPOT_MARKET_USDC = new PublicKey(
  "6gMq3mRCKf8aP3ttTyYhuijVZ2LGi14oDsBbkgubfLB3"
);
export const DRIFT_SIGNER = new PublicKey(
  "JCNCMFXo5M5qwUPg2Utu1u6YWp3MbygxqBsBeXXJfrw"
);

export const getDriftSpotMarketVault = (marketIndex: number) => {
  const [spotMarketVaultPda] = web3.PublicKey.findProgramAddressSync(
    [
      Buffer.from("spot_market_vault"),
      new BN(marketIndex).toArrayLike(Buffer, "le", 2),
    ],
    DRIFT_PROGRAM_ID
  );
  return spotMarketVaultPda;
};

export const getDriftUser = (authority: PublicKey) => {
  const [userPda] = web3.PublicKey.findProgramAddressSync(
    [
      Buffer.from("user"),
      authority.toBuffer(),
      new BN(0).toArrayLike(Buffer, "le", 2),
    ],
    DRIFT_PROGRAM_ID
  );
  return userPda;
};

export const getDriftUserStats = (authority: PublicKey) => {
  const [userStatsPda] = web3.PublicKey.findProgramAddressSync(
    [Buffer.from("user_stats"), authority.toBuffer()],
    DRIFT_PROGRAM_ID
  );
  return userStatsPda;
};

export const getDriftState = () => {
  const [statePda] = web3.PublicKey.findProgramAddressSync(
    [Buffer.from("drift_state")],
    DRIFT_PROGRAM_ID
  );
  return statePda;
};
