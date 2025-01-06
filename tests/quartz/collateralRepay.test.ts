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
import { fetchAddressLookupTable, getJupiterSwapIx, getPythOracle, processTransaction, setupAddressLookupTable, setupATA, } from "../utils/helpers";
import { 
  DRIFT_SIGNER, 
  DRIFT_ORACLE_SOL, 
  DRIFT_ORACLE_USDC, 
  DRIFT_MARKET_INDEX_USDC, 
  DRIFT_MARKET_INDEX_SOL, 
  DRIFT_SPOT_MARKET_SOL, 
  DRIFT_SPOT_MARKET_USDC, 
  USDC_MINT, 
  WSOL_MINT, 
  DRIFT_PROGRAM_ID,
  QUARTZ_PROGRAM_ID,
  QUARTZ_ADDRESS_TABLE
} from "../config/constants";
import config from "../config/config";
import { deposit, initUser, makeWrapSolIxs, withdraw, wrapSol } from "../utils/instructions";
import { initDriftAccount } from "../utils/instructions";
import { getDriftSpotMarketVault, getDriftUserStats, getDriftState, getDriftUser, getVaultPda, getVaultSplPda, toRemainingAccount } from "../utils/accounts";
import { QuoteResponse } from "@jup-ag/api";

describe("collateral repay", () => {
  const connection = new Connection(config.RPC_URL);

  let provider: BankrunProvider;
  let user: Keypair;
  let context: ProgramTestContext;
  let banksClient: BanksClient;
  let quartzProgram: Program<Quartz>;
  
  const driftState = getDriftState();
  const solSpotMarket = getDriftSpotMarketVault(DRIFT_MARKET_INDEX_SOL);
  const usdcSpotMarket = getDriftSpotMarketVault(DRIFT_MARKET_INDEX_USDC);

  let quartzLookupTable: PublicKey;
  let vault: PublicKey;
  let driftUser: PublicKey;
  let driftUserStats: PublicKey;
  let walletWsol: PublicKey;
  let walletUsdc: PublicKey;

  beforeEach(async () => {
    user = Keypair.generate();
    vault = getVaultPda(user.publicKey);
    driftUser = getDriftUser(vault);
    driftUserStats = getDriftUserStats(vault);
    walletWsol = await getAssociatedTokenAddress(WSOL_MINT, user.publicKey);
    walletUsdc = await getAssociatedTokenAddress(USDC_MINT, user.publicKey);
    
    const driftStateAccount = await connection.getAccountInfo(driftState);
    const solSpotMarketAccountInfo = await connection.getAccountInfo(DRIFT_SPOT_MARKET_SOL);
    const usdcSpotMarketAccountInfo = await connection.getAccountInfo(DRIFT_SPOT_MARKET_USDC);
    const oracleSolAccountInfo = await connection.getAccountInfo(DRIFT_ORACLE_SOL);
    const oracleUsdcAccountInfo = await connection.getAccountInfo(DRIFT_ORACLE_USDC);
    const driftSignerAccountInfo = await connection.getAccountInfo(DRIFT_SIGNER);
    const usdcMintAccountInfo = await connection.getAccountInfo(USDC_MINT);
    const solMintAccountInfo = await connection.getAccountInfo(WSOL_MINT);
    const solSpotMarketVaultAccountInfo = await connection.getAccountInfo(solSpotMarket);
    const usdcSpotMarketVaultAccountInfo = await connection.getAccountInfo(usdcSpotMarket);

    context = await startAnchor(
      "./",
      [{ name: "drift", programId: DRIFT_PROGRAM_ID }],
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
          address: solSpotMarket,
          info: solSpotMarketVaultAccountInfo,
        },
        {
          address: usdcSpotMarket,
          info: usdcSpotMarketVaultAccountInfo,
        },
        {
          address: DRIFT_SPOT_MARKET_SOL,
          info: solSpotMarketAccountInfo,
        },
        {
          address: DRIFT_SPOT_MARKET_USDC,
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
        spotMarketVault: solSpotMarket,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_PROGRAM_ID,
        driftProgram: DRIFT_PROGRAM_ID,
        systemProgram: SystemProgram.programId
      },
      [
        toRemainingAccount(DRIFT_ORACLE_SOL, false, false),
        toRemainingAccount(DRIFT_SPOT_MARKET_SOL, true, false),
      ]
    )
    await withdraw(
      quartzProgram, 
      banksClient, 
      10_000_000, 
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
        driftSigner: DRIFT_SIGNER,
        spotMarketVault: solSpotMarket,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_PROGRAM_ID,
        driftProgram: DRIFT_PROGRAM_ID,
        systemProgram: SystemProgram.programId
      },
      [
        toRemainingAccount(DRIFT_ORACLE_SOL, false, false),
        toRemainingAccount(DRIFT_SPOT_MARKET_SOL, true, false),
      ]
    )
  });

  test("Should repay collateral", async () => {
    const amountLoan = 5_000_000;

    const mintCollateral = WSOL_MINT;
    const mintLoan = USDC_MINT;
    const slippageBps = 50;

    const jupiterQuoteEndpoint
        = `https://quote-api.jup.ag/v6/quote?inputMint=${mintCollateral.toBase58()}&outputMint=${mintLoan.toBase58()}&amount=${amountLoan}&slippageBps=${slippageBps}&swapMode=ExactOut&onlyDirectRoutes=true`;
    const response = await fetch(jupiterQuoteEndpoint);
    const jupiterQuote = (await response.json()) as QuoteResponse;
    const collateralRequiredForSwap = Math.ceil(Number(jupiterQuote.inAmount) * (1 + (slippageBps / 10_000)));

    const ixs_wrapSol = await makeWrapSolIxs(quartzProgram, banksClient, collateralRequiredForSwap, {
      user: user.publicKey,
      walletWsol: walletWsol,
    });

    const ix_collateralRepayStart = await quartzProgram.methods
      .collateralRepayStart(new BN(collateralRequiredForSwap))
      .accounts({
        caller: user.publicKey,
        callerWithdrawSpl: walletWsol,
        withdrawMint: WSOL_MINT,
        vault: vault,
        vaultWithdrawSpl: getVaultSplPda(vault, WSOL_MINT),
        owner: user.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        instructions: SYSVAR_INSTRUCTIONS_PUBKEY,
      })
      .instruction();

    const { 
      ix_jupiterSwap,
      jupiterLookupTables
    } = await getJupiterSwapIx(user.publicKey, connection, jupiterQuote);

    const ix_collateralRepayDeposit = await quartzProgram.methods
      .collateralRepayDeposit(DRIFT_MARKET_INDEX_USDC)
      .accounts({
        vault: vault,
        vaultSpl: getVaultSplPda(vault, USDC_MINT),
        owner: user.publicKey,
        caller: user.publicKey,
        callerSpl: walletUsdc,
        splMint: USDC_MINT,
        driftUser: driftUser,
        driftUserStats: driftUserStats,
        driftState: driftState,
        spotMarketVault: usdcSpotMarket,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_PROGRAM_ID,
        driftProgram: DRIFT_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        instructions: SYSVAR_INSTRUCTIONS_PUBKEY,
      })
      .remainingAccounts([
        toRemainingAccount(DRIFT_ORACLE_SOL, false, false),
        toRemainingAccount(DRIFT_SPOT_MARKET_SOL, true, false),
        toRemainingAccount(DRIFT_ORACLE_USDC, false, false),
        toRemainingAccount(DRIFT_SPOT_MARKET_USDC, true, false),
      ])
      .instruction();

    const ix_collateralRepayWithdraw = await quartzProgram.methods
      .collateralRepayWithdraw(DRIFT_MARKET_INDEX_SOL)
      .accounts({
        vault: vault,
        vaultSpl: getVaultSplPda(vault, WSOL_MINT),
        owner: user.publicKey,
        caller: user.publicKey,
        callerSpl: walletWsol,
        splMint: WSOL_MINT,
        driftUser: driftUser,
        driftUserStats: driftUserStats,
        driftState: driftState,
        spotMarketVault: usdcSpotMarket,
        driftSigner: DRIFT_SIGNER,
        tokenProgram: TOKEN_PROGRAM_ID,
        driftProgram: DRIFT_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        depositPriceUpdate: getPythOracle(0),
        withdrawPriceUpdate: getPythOracle(1),
        instructions: SYSVAR_INSTRUCTIONS_PUBKEY,
      })
      .remainingAccounts([
        toRemainingAccount(DRIFT_ORACLE_SOL, false, false),
        toRemainingAccount(DRIFT_SPOT_MARKET_SOL, true, false),
        toRemainingAccount(DRIFT_ORACLE_USDC, false, false),
        toRemainingAccount(DRIFT_SPOT_MARKET_USDC, true, false),
      ])
      .instruction();

    const lookupTable = await fetchAddressLookupTable(banksClient, quartzLookupTable);
    const messagev0 = new TransactionMessage({
      payerKey: user.publicKey,
      recentBlockhash: (await connection.getLatestBlockhash()).blockhash,
      instructions: [
        ...ixs_wrapSol, 
        ix_collateralRepayStart, 
        ix_jupiterSwap, 
        ix_collateralRepayDeposit, 
        ix_collateralRepayWithdraw
      ]
    }).compileToV0Message([lookupTable, ...jupiterLookupTables]);
    const transaction = new VersionedTransaction(messagev0);
    const meta = await banksClient.processTransaction(transaction);

    throw new Error("Not implemented");
  });
});
