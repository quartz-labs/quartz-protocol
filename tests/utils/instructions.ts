import { BN, Program } from "@coral-xyz/anchor";
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import { BanksClient } from "solana-bankrun";
import { Quartz } from "../../target/types/quartz";
import { processTransaction } from "./helpers";
import { WSOL_MINT } from "../config/constants";
import { createAssociatedTokenAccountInstruction, createSyncNativeInstruction } from "@solana/spl-token";
import { AccountMeta } from "./interfaces";


export interface InitUserParams {
    requiresMarginfiAccount: boolean;
    spendLimitPerTransaction: number;
    spendLimitPerTimeframe: number;
    extendSpendLimitPerTimeframeResetSlotAmount: number;
}

export interface InitUserAccounts {
    vault: PublicKey;
    owner: PublicKey;
    initRentPayer: PublicKey;
    driftUser: PublicKey;
    driftUserStats: PublicKey;
    driftState: PublicKey;
    driftProgram: PublicKey;
    marginfiGroup: PublicKey;
    marginfiAccount: PublicKey;
    marginfiProgram: PublicKey;
    rent: PublicKey;
    systemProgram: PublicKey;
}
  
export const initUser = async (
    quartzProgram: Program<Quartz>,
    banksClient: BanksClient,
    signers: Keypair[],
    params: InitUserParams,
    accounts: InitUserAccounts,
) => {
    const ix = await quartzProgram.methods
        .initUser(
            params.requiresMarginfiAccount, 
            new BN(params.spendLimitPerTransaction), 
            new BN(params.spendLimitPerTimeframe), 
            new BN(params.extendSpendLimitPerTimeframeResetSlotAmount)
        )
        .accounts(accounts)
        .instruction();
  
    const meta = await processTransaction(
        banksClient, 
        accounts.owner, 
        [ix],
        signers
    );
    return meta;
};
  
  
export interface CloseUserAccounts {
    vault: PublicKey;
    owner: PublicKey;
    initRentPayer: PublicKey;
    driftUser: PublicKey;
    driftUserStats: PublicKey;
    driftState: PublicKey;
    driftProgram: PublicKey;
    systemProgram: PublicKey;
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


export interface UpgradeVaultParams {
    spendLimitPerTransaction: number;
    spendLimitPerTimeframe: number;
    extendSpendLimitPerTimeframeResetSlotAmount: number;
}

export interface UpgradeVaultAccounts {
    vault: PublicKey;
    owner: PublicKey;
    initRentPayer: PublicKey;
    systemProgram: PublicKey;
}
  
export const upgradeVault = async (
    quartzProgram: Program<Quartz>,
    banksClient: BanksClient,
    params: UpgradeVaultParams,
    accounts: UpgradeVaultAccounts,
) => {
    const ix = await quartzProgram.methods
        .upgradeVault(
            new BN(params.spendLimitPerTransaction), 
            new BN(params.spendLimitPerTimeframe), 
            new BN(params.extendSpendLimitPerTimeframeResetSlotAmount)
        )
        .accounts(accounts)
        .instruction();
  
    const meta = await processTransaction(
        banksClient, 
        accounts.owner, 
        [ix]
    );
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
    accounts: DepositAccouts,
    remainingAccounts: AccountMeta[]
) => {
    const ix = await quartzProgram.methods
        .deposit(new BN(amountBaseUnits), marketIndex, false)
        .accounts(accounts)
        .remainingAccounts(remainingAccounts)
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
    accounts: WithdrawAccounts,
    remainingAccounts: AccountMeta[]
) => {
    const ix = await quartzProgram.methods
        .withdraw(new BN(amountBaseUnits), marketIndex, false)
        .accounts(accounts)
        .remainingAccounts(remainingAccounts)
        .instruction();

    const meta = await processTransaction(banksClient, accounts.owner, [ix]);
    return meta;
}


export interface WrapSolAccounts {
    user: PublicKey;
    walletWsol: PublicKey;
}

export const makeWrapSolIxs = async (
    banksClient: BanksClient,
    amount: number,
    accounts: WrapSolAccounts,
) => {
    const oix_createWSolAta = [];
    const ataInfo = await banksClient.getAccount(accounts.walletWsol);
    if (!ataInfo) {
        const ix_createWSolAta = createAssociatedTokenAccountInstruction(
            accounts.user,
            accounts.walletWsol,
            accounts.user,
            WSOL_MINT
        );
        oix_createWSolAta.push(ix_createWSolAta);
    }

    const ix_wrapSol = SystemProgram.transfer({
        fromPubkey: accounts.user,
        toPubkey: accounts.walletWsol,
        lamports: amount,
    });

    const ix_syncNative = createSyncNativeInstruction(accounts.walletWsol);  

    return [...oix_createWSolAta, ix_wrapSol, ix_syncNative];
}

export const wrapSol = async (
    quartzProgram: Program<Quartz>,
    banksClient: BanksClient,
    amount: number,
    accounts: WrapSolAccounts
) => {
    const ixs = await makeWrapSolIxs(banksClient, amount, accounts);
    const meta = await processTransaction(banksClient, accounts.user, ixs);
    return meta;
}