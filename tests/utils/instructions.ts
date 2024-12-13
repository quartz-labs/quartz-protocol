import { BN, Program } from "@coral-xyz/anchor";
import { PublicKey, SystemProgram } from "@solana/web3.js";
import { BanksClient } from "solana-bankrun";
import { Quartz } from "../../target/types/quartz";
import { processTransaction } from "./helpers";
import { WSOL_MINT } from "../config/constants";
import { createAssociatedTokenAccountInstruction, createSyncNativeInstruction } from "@solana/spl-token";


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
  
    const meta = await processTransaction(banksClient, accounts.owner, [ix]);
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
  
    const meta = await processTransaction(banksClient, accounts.owner, [ix]);
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
  
    const meta = await processTransaction(banksClient, accounts.owner, [ix]);
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

    const meta = await processTransaction(banksClient, accounts.owner, [ix]);
    return meta;
};


export interface DepositAccouts {
    vault: PublicKey;
    vaultSpl: PublicKey;
    owner: PublicKey;
    ownerSpl: PublicKey;
    splMint: PublicKey;
    driftUser: PublicKey;
    driftUserStats: PublicKey;
    driftState: PublicKey;
    spotMarketVault: PublicKey;
    tokenProgram: PublicKey;
    associatedTokenProgram: PublicKey;
    driftProgram: PublicKey;
    systemProgram: PublicKey;
}

export const deposit = async (
    quartzProgram: Program<Quartz>,
    banksClient: BanksClient,
    amountBaseUnits: number,
    marketIndex: number,
    accounts: DepositAccouts
) => {
    const ix = await quartzProgram.methods
        .deposit(new BN(amountBaseUnits), marketIndex, false)
        .accounts(accounts)
        .instruction();

    const meta = await processTransaction(banksClient, accounts.owner, [ix]);
    return meta;
}


export interface WithdrawAccounts {
    vault: PublicKey;
    vaultSpl: PublicKey;
    owner: PublicKey;
    ownerSpl: PublicKey;
    splMint: PublicKey;
    driftUser: PublicKey;
    driftUserStats: PublicKey;
    driftState: PublicKey;
    spotMarketVault: PublicKey;
    driftSigner: PublicKey;
    tokenProgram: PublicKey;
    associatedTokenProgram: PublicKey;
    driftProgram: PublicKey;
    systemProgram: PublicKey;
}

export const withdraw = async (
    quartzProgram: Program<Quartz>,
    banksClient: BanksClient,
    amountBaseUnits: number,
    marketIndex: number,
    accounts: WithdrawAccounts
) => {
    const ix = await quartzProgram.methods
        .withdraw(new BN(amountBaseUnits), marketIndex, true)
        .accounts(accounts)
        .instruction();

    const meta = await processTransaction(banksClient, accounts.owner, [ix]);
    return meta;
}


export interface WrapSolAccounts {
    user: PublicKey;
    walletWsol: PublicKey;
}

export const makeWrapSolIxs = async (
    quartzProgram: Program<Quartz>,
    banksClient: BanksClient,
    amount: number,
    accounts: WrapSolAccounts
) => {
    const ix_createWSolAta = createAssociatedTokenAccountInstruction(
        accounts.user,
        accounts.walletWsol,
        accounts.user,
        WSOL_MINT
    );

    const ix_wrapSol = SystemProgram.transfer({
        fromPubkey: accounts.user,
        toPubkey: accounts.walletWsol,
        lamports: amount,
    });

    const ix_syncNative = createSyncNativeInstruction(accounts.walletWsol);  

    return [ix_createWSolAta, ix_wrapSol, ix_syncNative];
}