import { BN, Program, web3 } from "@coral-xyz/anchor";
import { BankrunProvider } from "anchor-bankrun";
import { beforeAll, expect, test } from "@jest/globals";
import { ProgramTestContext, BanksClient, startAnchor } from "solana-bankrun";
import {
  Keypair,
  PublicKey,
  SystemProgram,
  Connection,
  LAMPORTS_PER_SOL,
  SYSVAR_INSTRUCTIONS_PUBKEY,
  VersionedTransaction,
  TransactionMessage,
  AddressLookupTableAccount,
  AddressLookupTableProgram
} from "@solana/web3.js";
import { IDL as QuartzIDL, Quartz } from "../../target/types/quartz";
import {
  getAssociatedTokenAddress,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { ASSOCIATED_PROGRAM_ID } from "@coral-xyz/anchor/dist/cjs/utils/token";
import { fetchAddressLookupTable, fetchPricesCoingecko, getJupiterSwapIx, getPythOracle, processTransaction, setupAddressLookupTable, setupATA, } from "../utils/helpers";
import { 
  DRIFT_SIGNER, 
  DRIFT_ORACLE_SOL, 
  DRIFT_ORACLE_USDC, 
  DRIFT_MARKET_INDEX_USDC, 
  DRIFT_MARKET_INDEX_SOL, 
  USDC_MINT, 
  WSOL_MINT, 
  DRIFT_PROGRAM_ID,
  QUARTZ_PROGRAM_ID,
  QUARTZ_ADDRESS_TABLE,
  MARGINFI_PROGRAM_ID
} from "../config/constants";
import config from "../config/config";
import { deposit, initUser, makeWrapSolIxs, withdraw, wrapSol } from "../utils/instructions";
import { initDriftAccount } from "../utils/instructions";
import { getDriftSpotMarketVault, getDriftUserStats, getDriftState, getDriftUser, getVaultPda, getVaultSplPda, toRemainingAccount, getTokenLedgerPda, getDriftSpotMarket } from "../utils/accounts";
import { QuoteResponse } from "@jup-ag/api";

describe("collateral repay", () => {
  const connection = new Connection(config.RPC_URL);

  let provider: BankrunProvider;
  let user: Keypair;
  let context: ProgramTestContext;
  let banksClient: BanksClient;
  let quartzProgram: Program<Quartz>;
  
  const driftState = getDriftState();
  const solSpotMarketVault = getDriftSpotMarketVault(DRIFT_MARKET_INDEX_SOL);
  const usdcSpotMarketVault = getDriftSpotMarketVault(DRIFT_MARKET_INDEX_USDC);
  const solSpotMarket = getDriftSpotMarket(DRIFT_MARKET_INDEX_SOL);
  const usdcSpotMarket = getDriftSpotMarket(DRIFT_MARKET_INDEX_USDC);

  let quartzLookupTable: PublicKey;
  let vault: PublicKey;
  let driftUser: PublicKey;
  let driftUserStats: PublicKey;
  let walletWsol: PublicKey;
  let walletUsdc: PublicKey;
  let tokenLedger: PublicKey;
  beforeEach(async () => {
    user = Keypair.generate();
    vault = getVaultPda(user.publicKey);
    driftUser = getDriftUser(vault);
    driftUserStats = getDriftUserStats(vault);
    walletWsol = await getAssociatedTokenAddress(WSOL_MINT, user.publicKey);
    walletUsdc = await getAssociatedTokenAddress(USDC_MINT, user.publicKey);
    tokenLedger = getTokenLedgerPda(user.publicKey);
    
    const driftStateAccount = await connection.getAccountInfo(driftState);
    const driftSignerAccountInfo = await connection.getAccountInfo(DRIFT_SIGNER);
    const usdcMintAccountInfo = await connection.getAccountInfo(USDC_MINT);
    const solMintAccountInfo = await connection.getAccountInfo(WSOL_MINT);
    const oracleSolAccountInfo = await connection.getAccountInfo(DRIFT_ORACLE_SOL);
    const oracleUsdcAccountInfo = await connection.getAccountInfo(DRIFT_ORACLE_USDC);
    const solSpotMarketAccountInfo = await connection.getAccountInfo(solSpotMarket);
    const usdcSpotMarketAccountInfo = await connection.getAccountInfo(usdcSpotMarket);
    const solSpotMarketVaultAccountInfo = await connection.getAccountInfo(solSpotMarketVault);
    const usdcSpotMarketVaultAccountInfo = await connection.getAccountInfo(usdcSpotMarketVault);
    const pythOracleUsdcAccountInfo = await connection.getAccountInfo(getPythOracle(0));
    const pythOracleSolAccountInfo = await connection.getAccountInfo(getPythOracle(1));


    context = await startAnchor(
      "./",
      [
        { name: "drift", programId: DRIFT_PROGRAM_ID },
        { name: "marginfi", programId: MARGINFI_PROGRAM_ID }
      ],
      [
        {
          address: user.publicKey,
          info: {
            lamports: 100 * LAMPORTS_PER_SOL,
            data: Buffer.alloc(0),
            owner: SystemProgram.programId,
            executable: false,
          },
        },
        {
          address: driftState,
          info: driftStateAccount,
        },
        {
          address: solSpotMarketVault,
          info: solSpotMarketVaultAccountInfo,
        },
        {
          address: usdcSpotMarketVault,
          info: usdcSpotMarketVaultAccountInfo,
        },
        {
          address: solSpotMarket,
          info: solSpotMarketAccountInfo,
        },
        {
          address: usdcSpotMarket,
          info: usdcSpotMarketAccountInfo,
        },
        {
          address: DRIFT_ORACLE_SOL,
          info: oracleSolAccountInfo,
        },
        {
          address: DRIFT_ORACLE_USDC,
          info: oracleUsdcAccountInfo,
        },
        {
          address: DRIFT_SIGNER,
          info: driftSignerAccountInfo,
        },
        {
          address: USDC_MINT,
          info: usdcMintAccountInfo,
        },
        {
          address: WSOL_MINT,
          info: solMintAccountInfo,
        },
        {
          address: getPythOracle(0),
          info: pythOracleUsdcAccountInfo,
        },
        {
          address: getPythOracle(1),
          info: pythOracleSolAccountInfo,
        }
      ]
    );
    
    banksClient = context.banksClient;
    provider = new BankrunProvider(context);
    quartzProgram = new Program<Quartz>(QuartzIDL, QUARTZ_PROGRAM_ID, provider);
    const addressSetup = await setupAddressLookupTable(connection, banksClient, context, user.publicKey, QUARTZ_ADDRESS_TABLE);
    quartzLookupTable = addressSetup.lookupTable;

    await setupATA(context, USDC_MINT, user.publicKey, 0);

    await initUser(quartzProgram, banksClient, {
      vault: vault,
      owner: user.publicKey,
      systemProgram: SystemProgram.programId,
    });
    await initDriftAccount(quartzProgram, banksClient, {
      vault: vault,
      owner: user.publicKey,
      driftUser: driftUser,
      driftUserStats: driftUserStats,
      driftState: driftState,
      driftProgram: DRIFT_PROGRAM_ID,
      rent: web3.SYSVAR_RENT_PUBKEY,
      systemProgram: SystemProgram.programId,
    });
    await wrapSol(quartzProgram, banksClient, 10 * LAMPORTS_PER_SOL, {
      user: user.publicKey,
      walletWsol: walletWsol,
    });
    await deposit(
      quartzProgram, 
      banksClient, 10 * LAMPORTS_PER_SOL, 
      DRIFT_MARKET_INDEX_SOL, 
      {
        vault: vault,
        vaultSpl: getVaultSplPda(vault, WSOL_MINT),
        owner: user.publicKey,
        ownerSpl: walletWsol,
        splMint: WSOL_MINT,
        driftUser: driftUser,
        driftUserStats: driftUserStats,
        driftState: driftState,
        spotMarketVault: solSpotMarketVault,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_PROGRAM_ID,
        driftProgram: DRIFT_PROGRAM_ID,
        systemProgram: SystemProgram.programId
      },
      [
        toRemainingAccount(DRIFT_ORACLE_SOL, false, false),
        toRemainingAccount(solSpotMarket, true, false),
      ]
    )
    await withdraw(
      quartzProgram, 
      banksClient, 
      10_000_000, 
      DRIFT_MARKET_INDEX_USDC, 
      {
        vault: vault,
        vaultSpl: getVaultSplPda(vault, USDC_MINT),
        owner: user.publicKey,
        ownerSpl: walletUsdc,
        splMint: USDC_MINT,
        driftUser: driftUser,
        driftUserStats: driftUserStats,
        driftState: driftState,
        driftSigner: DRIFT_SIGNER,
        spotMarketVault: usdcSpotMarketVault,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_PROGRAM_ID,
        driftProgram: DRIFT_PROGRAM_ID,
        systemProgram: SystemProgram.programId
      },
      [
        toRemainingAccount(DRIFT_ORACLE_SOL, false, false),
        toRemainingAccount(solSpotMarket, false, false),
        toRemainingAccount(DRIFT_ORACLE_USDC, false, false),
        toRemainingAccount(usdcSpotMarket, true, false),
      ]
    )
  });

  const TIMEOUT = 10_000;
  test("Should repay collateral", async () => {
    const AMOUNT_LOAN = 5_000_000;
    
    const {
      "usd-coin": priceUsdc,
      "solana": priceSol
    } = await fetchPricesCoingecko(["usd-coin", "solana"]);

    const amountCollateral = Math.round(AMOUNT_LOAN * priceUsdc / priceSol);

    const ixs_wrapSol = await makeWrapSolIxs(banksClient, amountCollateral, {
      user: user.publicKey,
      walletWsol: walletWsol,
    });

    const ix_startCollateralRepay = await quartzProgram.methods
      .startCollateralRepay(new BN(AMOUNT_LOAN), DRIFT_MARKET_INDEX_USDC)
      .accounts({
        caller: user.publicKey,
        callerSpl: walletUsdc,
        owner: user.publicKey,
        vault: vault,
        vaultSpl: getVaultSplPda(vault, USDC_MINT),
        splMint: USDC_MINT,
        driftUser: driftUser,
        driftUserStats: driftUserStats,
        driftState: driftState,
        spotMarketVault: usdcSpotMarketVault,
        tokenProgram: TOKEN_PROGRAM_ID,
        driftProgram: DRIFT_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        instructions: SYSVAR_INSTRUCTIONS_PUBKEY,
        tokenLedger: tokenLedger
      })
      .remainingAccounts([
        toRemainingAccount(DRIFT_ORACLE_SOL, false, false),
        toRemainingAccount(solSpotMarket, false, false),
        toRemainingAccount(DRIFT_ORACLE_USDC, false, false),
        toRemainingAccount(usdcSpotMarket, true, false),
      ])
      .instruction();

    const ix_endCollateralRepay = await quartzProgram.methods
      .endCollateralRepay(new BN(amountCollateral), DRIFT_MARKET_INDEX_SOL)
      .accounts({
        caller: user.publicKey,
        callerSpl: walletWsol,
        owner: user.publicKey,
        vault: vault,
        vaultSpl: getVaultSplPda(vault, WSOL_MINT),
        splMint: WSOL_MINT,
        driftUser: driftUser,
        driftUserStats: driftUserStats,
        driftState: driftState,
        spotMarketVault: solSpotMarketVault,
        driftSigner: DRIFT_SIGNER,
        tokenProgram: TOKEN_PROGRAM_ID,
        driftProgram: DRIFT_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        depositPriceUpdate: getPythOracle(0),
        withdrawPriceUpdate: getPythOracle(1),
        instructions: SYSVAR_INSTRUCTIONS_PUBKEY,
        tokenLedger: tokenLedger
      })
      .remainingAccounts([
        toRemainingAccount(DRIFT_ORACLE_SOL, false, false),
        toRemainingAccount(solSpotMarket, false, false),
        toRemainingAccount(DRIFT_ORACLE_USDC, false, false),
        toRemainingAccount(usdcSpotMarket, true, false)
      ])
      .instruction();

    const [ blockhash ] = await banksClient.getLatestBlockhash();
    const lookupTable = await fetchAddressLookupTable(banksClient, quartzLookupTable);
    const messagev0 = new TransactionMessage({
      payerKey: user.publicKey,
      recentBlockhash: blockhash,
      instructions: [
        ...ixs_wrapSol, 
        ix_startCollateralRepay,
        ix_endCollateralRepay
      ]
    }).compileToV0Message([lookupTable]);
    const transaction = new VersionedTransaction(messagev0);
    const meta = await banksClient.processTransaction(transaction);
  }, TIMEOUT);
});
