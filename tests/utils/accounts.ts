import { PublicKey } from "@solana/web3.js";
import { DOMAIN_BASE, DRIFT_PROGRAM_ID, MESSAGE_TRANSMITTER_PROGRAM_ID, QUARTZ_PROGRAM_ID, TOKEN_MESSAGE_MINTER_PROGRAM_ID, USDC_MINT } from "../config/constants";
import { BN, web3 } from "@coral-xyz/anchor";

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

export const getTokenLedgerPda = (owner: PublicKey) => {
    const [tokenLedger] = web3.PublicKey.findProgramAddressSync(
        [Buffer.from("token_ledger"), owner.toBuffer()],
        QUARTZ_PROGRAM_ID
    );
    return tokenLedger;
};

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

export const getDriftSpotMarket = (marketIndex: number) => {
    const [spotMarketPda] = PublicKey.findProgramAddressSync(
        [
            Buffer.from("spot_market"), 
            new BN(marketIndex).toArrayLike(Buffer, 'le', 2)    
        ],
        DRIFT_PROGRAM_ID
    );
    return spotMarketPda;
}
  
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

export const getSenderAuthority = () => {
    const [senderAuthorityPda] = web3.PublicKey.findProgramAddressSync(
        [Buffer.from("sender_authority")],
        TOKEN_MESSAGE_MINTER_PROGRAM_ID
    );
    return senderAuthorityPda;
};

export const getMessageTransmitter = () => {
    const [messageTransmitter] = web3.PublicKey.findProgramAddressSync(
        [Buffer.from("message_transmitter")],
        MESSAGE_TRANSMITTER_PROGRAM_ID
    );
    return messageTransmitter;
};

export const getTokenMessenger = () => {
    const [tokenMessenger] = web3.PublicKey.findProgramAddressSync(
        [Buffer.from("token_messenger")],
        TOKEN_MESSAGE_MINTER_PROGRAM_ID
    );
    return tokenMessenger;
};

export const getTokenMinter = () => {
    const [tokenMinter] = web3.PublicKey.findProgramAddressSync(
        [Buffer.from("token_minter")],
        TOKEN_MESSAGE_MINTER_PROGRAM_ID
    );
    return tokenMinter;
};

export const getLocalToken = () => {
    const [localToken] = web3.PublicKey.findProgramAddressSync(
        [Buffer.from("local_token"), USDC_MINT.toBuffer()],
        TOKEN_MESSAGE_MINTER_PROGRAM_ID,
    );
    return localToken;
};

export const getRemoteTokenMessenger = () => {
    const [remoteTokenMessenger] = web3.PublicKey.findProgramAddressSync(
        [Buffer.from("remote_token_messenger"), Buffer.from(DOMAIN_BASE.toString())],
        TOKEN_MESSAGE_MINTER_PROGRAM_ID
    );
    return remoteTokenMessenger;
};

export const getEventAuthority = () => {
    const [eventAuthority] = web3.PublicKey.findProgramAddressSync(
        [Buffer.from("__event_authority")],
        TOKEN_MESSAGE_MINTER_PROGRAM_ID
    );
    return eventAuthority;
};

export const getBridgeRentPayer = () => {
    const [bridgeRentPayer] = web3.PublicKey.findProgramAddressSync(
        [Buffer.from("bridge_rent_payer")],
        QUARTZ_PROGRAM_ID
    );
    return bridgeRentPayer;
};