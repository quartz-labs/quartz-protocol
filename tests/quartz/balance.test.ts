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
  SYSVAR_RENT_PUBKEY
} from "@solana/web3.js";
import { IDL as QuartzIDL, Quartz } from "../../target/types/quartz";
import {
  createAssociatedTokenAccountInstruction,
  createSyncNativeInstruction,
  getAssociatedTokenAddress,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { ASSOCIATED_PROGRAM_ID } from "@coral-xyz/anchor/dist/cjs/utils/token";
import { processTransaction } from "../utils/helpers";
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
  MARGINFI_PROGRAM_ID,
  MARGINFI_GROUP_1
} from "../config/constants";
import config from "../config/config";
import { initUser, makeWrapSolIxs } from "../utils/instructions";
import { getDriftSpotMarketVault, getDriftUserStats, getDriftState, getDriftUser, getVaultPda, getVaultSplPda, toRemainingAccount, getDriftSpotMarket, getInitRentPayer } from "../utils/accounts";

const TIMEOUT = 15_000;
describe("deposit, withdraw", () => {
  let provider: BankrunProvider;
  let user: Keypair;
  let context: ProgramTestContext;
  let banksClient: BanksClient;
  let quartzProgram: Program<Quartz>;

  let vault: PublicKey;
  let driftUser: PublicKey;
  let driftUserStats: PublicKey;
  let walletWsol: PublicKey;
  let walletUsdc: PublicKey;

  const driftState = getDriftState();
  const solSpotMarketVault = getDriftSpotMarketVault(DRIFT_MARKET_INDEX_SOL);
  const usdcSpotMarketVault = getDriftSpotMarketVault(DRIFT_MARKET_INDEX_USDC);
  const solSpotMarket = getDriftSpotMarket(DRIFT_MARKET_INDEX_SOL);
  const usdcSpotMarket = getDriftSpotMarket(DRIFT_MARKET_INDEX_USDC);
  const initRentPayer = getInitRentPayer();

  beforeEach(async () => {
    user = Keypair.generate();
    vault = getVaultPda(user.publicKey);
    driftUser = getDriftUser(vault);
    driftUserStats = getDriftUserStats(vault);
    walletWsol = await getAssociatedTokenAddress(WSOL_MINT, user.publicKey);
    walletUsdc = await getAssociatedTokenAddress(USDC_MINT, user.publicKey);
    
    const connection = new Connection(config.RPC_URL);
    const driftStateAccount = await connection.getAccountInfo(driftState);
    const solSpotMarketAccountInfo = await connection.getAccountInfo(solSpotMarket);
    const usdcSpotMarketAccountInfo = await connection.getAccountInfo(usdcSpotMarket);
    const oracle1AccountInfo = await connection.getAccountInfo(DRIFT_ORACLE_SOL);
    const oracle2AccountInfo = await connection.getAccountInfo(DRIFT_ORACLE_USDC);
    const driftSignerAccountInfo = await connection.getAccountInfo(DRIFT_SIGNER);
    const usdcMintAccountInfo = await connection.getAccountInfo(USDC_MINT);
    const solMintAccountInfo = await connection.getAccountInfo(WSOL_MINT);
    const solSpotMarketVaultAccountInfo = await connection.getAccountInfo(solSpotMarketVault);
    const usdcSpotMarketVaultAccountInfo = await connection.getAccountInfo(usdcSpotMarketVault);
    const initRentPayerAccountInfo = await connection.getAccountInfo(initRentPayer);
    const marginfiGroupAccountInfo = await connection.getAccountInfo(MARGINFI_GROUP_1);

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
          info: oracle1AccountInfo,
        },
        {
          address: DRIFT_ORACLE_USDC,
          info: oracle2AccountInfo,
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
          address: initRentPayer,
          info: initRentPayerAccountInfo,
        },
        {
          address: MARGINFI_GROUP_1,
          info: marginfiGroupAccountInfo,
        }
      ]
    );
  
    banksClient = context.banksClient;
    provider = new BankrunProvider(context);
    quartzProgram = new Program<Quartz>(QuartzIDL, QUARTZ_PROGRAM_ID, provider);

    const marginfiAccount = Keypair.generate();
    await initUser(
      quartzProgram, 
      banksClient,
      [user, marginfiAccount],
      {
        requiresMarginfiAccount: true,
        spendLimitPerTransaction: 1000_000_000,
        spendLimitPerTimeframe: 1000_000_000,
        nextTimeframeResetTimestamp: 1000_000_000,
        timeframeInSeconds: 1000_000_000,
      },
      {
        vault: vault,
        owner: user.publicKey,
        initRentPayer: initRentPayer,
        driftUser: driftUser,
        driftUserStats: driftUserStats,
        driftState: driftState,
        driftProgram: DRIFT_PROGRAM_ID,
        marginfiGroup: MARGINFI_GROUP_1,
        marginfiAccount: marginfiAccount.publicKey,
        marginfiProgram: MARGINFI_PROGRAM_ID,
        rent: SYSVAR_RENT_PUBKEY,
        systemProgram: SystemProgram.programId,
      }
    );
  }, TIMEOUT);

  test("Should deposit lamports", async () => {
    const amount = 10 * LAMPORTS_PER_SOL;

    const ixs_wrapSol = await makeWrapSolIxs(banksClient, amount, {
      user: user.publicKey,
      walletWsol: walletWsol,
    });

    const ix_deposit = await quartzProgram.methods
      .deposit(new BN(amount), DRIFT_MARKET_INDEX_SOL, false)
      .accounts({
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
        systemProgram: SystemProgram.programId,
      })
      .remainingAccounts([
        toRemainingAccount(DRIFT_ORACLE_SOL, false, false),
        toRemainingAccount(solSpotMarket, true, false),
      ])
      .instruction();

    const meta = await processTransaction(banksClient, user.publicKey, [...ixs_wrapSol, ix_deposit]);

    expect(meta.logMessages[28]).toBe("Program log: Instruction: Deposit");
    expect(meta.logMessages[36]).toBe("Program log: Instruction: TransferChecked");
    expect(meta.logMessages[43]).toBe("Program log: Instruction: Transfer");
    expect(meta.logMessages[48]).toBe("Program dRiftyHA39MWEi3m9aunc5MzRF1JYuBsbn6VPcn33UH success");
    expect(meta.logMessages[54]).toBe("Program 6JjHXLheGSNvvexgzMthEcgjkcirDrGduc3HAKB2P1v2 success");

    // TODO - Add Drift balance check
  }, TIMEOUT);

  test("Should fail if not enough wrapped lamports", async () => {
    const amountWrap = 5 * LAMPORTS_PER_SOL;
    const amountDeposit = 10 * LAMPORTS_PER_SOL;

    const ixs_wrapSol = await makeWrapSolIxs(banksClient, amountWrap, {
      user: user.publicKey,
      walletWsol: walletWsol,
    });

    const ix_deposit = await quartzProgram.methods
      .deposit(new BN(amountDeposit), DRIFT_MARKET_INDEX_SOL, false)
      .accounts({
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
        systemProgram: SystemProgram.programId,
      })
      .remainingAccounts([
        toRemainingAccount(DRIFT_ORACLE_SOL, false, false),
        toRemainingAccount(solSpotMarket, true, false),
      ])
      .instruction();

    try {
      await processTransaction(banksClient, user.publicKey, [...ixs_wrapSol, ix_deposit]);
      expect(true).toBe(false); // Should not reach this line
    } catch (error: any) {
      expect(error.message).toContain("Error processing Instruction 3: custom program error: 0x1");
    }
  }, TIMEOUT);

  test("Should withdraw lamports", async () => {
    const amountDeposit = 10 * LAMPORTS_PER_SOL;
    const amountWithdraw = 5 * LAMPORTS_PER_SOL;

    const ixs_wrapSol = await makeWrapSolIxs(banksClient, amountDeposit, {
      user: user.publicKey,
      walletWsol: walletWsol,
    });

    const ix_deposit = await quartzProgram.methods
      .deposit(new BN(amountDeposit), DRIFT_MARKET_INDEX_SOL, false)
      .accounts({
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
        systemProgram: SystemProgram.programId,
      })
      .remainingAccounts([
        toRemainingAccount(DRIFT_ORACLE_SOL, false, false),
        toRemainingAccount(solSpotMarket, true, false),
      ])
      .instruction();

    const ix_withdraw = await quartzProgram.methods
      .withdraw(new BN(amountWithdraw), DRIFT_MARKET_INDEX_SOL, true)
      .accounts({
        vault: vault,
        vaultSpl: getVaultSplPda(vault, WSOL_MINT),
        owner: user.publicKey,
        ownerSpl: walletWsol,
        splMint: WSOL_MINT,
        driftUser: driftUser,
        driftUserStats: driftUserStats,
        driftState: driftState,
        driftSigner: DRIFT_SIGNER,
        spotMarketVault: solSpotMarketVault,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_PROGRAM_ID,
        driftProgram: DRIFT_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .remainingAccounts([
        toRemainingAccount(DRIFT_ORACLE_SOL, false, false),
        toRemainingAccount(solSpotMarket, true, false),
      ])
      .instruction();

    const meta = await processTransaction(banksClient, user.publicKey, [...ixs_wrapSol, ix_deposit, ix_withdraw]);
    
    expect(meta.logMessages[56]).toBe("Program log: Instruction: Withdraw");
    expect(meta.logMessages[67]).toBe("Program log: Instruction: Transfer");
    expect(meta.logMessages[71]).toBe("Program dRiftyHA39MWEi3m9aunc5MzRF1JYuBsbn6VPcn33UH success");
    expect(meta.logMessages[73]).toBe("Program log: Instruction: TransferChecked");
    expect(meta.logMessages[81]).toBe("Program 6JjHXLheGSNvvexgzMthEcgjkcirDrGduc3HAKB2P1v2 success");

    // TODO - Add Drift balance check
  }, TIMEOUT);
});


// TODO - Add more deposit tests
// TODO - Add more withdraw tests

// describe("Quartz Balance", () => {
//   //all the things that need to be done before each test
//   let provider: BankrunProvider,
//     user: Keypair,
//     context: ProgramTestContext,
//     banksClient: BanksClient,
//     quartzProgram: Program<Quartz>,
//     vaultPda: PublicKey;

//   user = Keypair.generate();

//   beforeAll(async () => {
//     ({ user, context, banksClient, quartzProgram, vaultPda } =
//       await setupTestEnvironment());

//     await setupQuartzAndDriftAccount(
//       quartzProgram,
//       banksClient,
//       vaultPda,
//       user
//     );
//     await makeDriftLamportDeposit(
//       quartzProgram,
//       user,
//       100_000_000_000,
//       banksClient,
//       WSOL_MINT
//     );
//   });

//   test("Withdraw Lamports", async () => {
//     await makeDriftLamportWithdraw(
//       quartzProgram,
//       user,
//       90_000_000,
//       banksClient
//     );
//   });

//   test("Withdraw USDC", async () => {
//     await makeDriftUSDCWithdraw(quartzProgram, user, 90_000, banksClient);
//   });
// });

// export const makeDriftLamportWithdraw = async (
//   program: Program<Quartz>,
//   wallet: Keypair,
//   amountLamports: number,
//   banksClient: BanksClient
// ) => {
//   const walletWSol = await getAssociatedTokenAddress(
//     WSOL_MINT,
//     wallet.publicKey
//   );
//   const vaultPda = getVaultPda(wallet.publicKey);

//   const oix_createWSolAta = createAssociatedTokenAccountInstruction(
//     wallet.publicKey,
//     walletWSol,
//     wallet.publicKey,
//     WSOL_MINT
//   );

//   const ix_withdraw = await program.methods
//     .withdraw(new BN(amountLamports), DRIFT_MARKET_INDEX_SOL, true)
//     .accounts({
//       vault: vaultPda,
//       vaultSpl: getVaultSplPda(vaultPda, WSOL_MINT),
//       owner: wallet.publicKey,
//       ownerSpl: walletWSol,
//       splMint: WSOL_MINT,
//       driftUser: getDriftUser(vaultPda),
//       driftUserStats: getDriftUserStats(vaultPda),
//       driftState: getDriftState(),
//       spotMarketVault: getDriftSpotMarketVault(DRIFT_MARKET_INDEX_SOL),
//       driftSigner: DRIFT_SIGNER,
//       tokenProgram: TOKEN_PROGRAM_ID,
//       associatedTokenProgram: ASSOCIATED_PROGRAM_ID,
//       driftProgram: DRIFT_PROGRAM_ID,
//       systemProgram: SystemProgram.programId,
//     })
//     .remainingAccounts([
//       toRemainingAccount(DRIFT_ORACLE_2, false, false),
//       toRemainingAccount(DRIFT_ORACLE_1, false, false),
//       toRemainingAccount(DRIFT_SPOT_MARKET_SOL, true, false),
//       toRemainingAccount(DRIFT_SPOT_MARKET_USDC, false, false),
//     ])
//     .instruction();

//   const ix_closeWSolAta = createCloseAccountInstruction(
//     walletWSol,
//     wallet.publicKey,
//     wallet.publicKey
//   );

//   const instructions = [oix_createWSolAta, ix_withdraw, ix_closeWSolAta];

//   const latestBlockhash = await banksClient.getLatestBlockhash();
//   const messageV0 = new TransactionMessage({
//     payerKey: wallet.publicKey,
//     recentBlockhash: latestBlockhash[0],
//     instructions: instructions,
//   }).compileToV0Message();
//   const tx = new VersionedTransaction(messageV0);

//   const simRes = await banksClient.simulateTransaction(tx);
//   const meta = await banksClient.processTransaction(tx);

//   expect(simRes.meta?.logMessages).toEqual(meta?.logMessages);
//   expect(meta.logMessages[1]).toBe("Program log: Create");
//   expect(meta.logMessages[22]).toBe("Program log: Instruction: Withdraw");
//   expect(meta.logMessages[26]).toBe(
//     "Program log: Instruction: InitializeAccount3"
//   );
//   expect(meta.logMessages[30]).toBe("Program log: Instruction: Withdraw");
//   expect(meta.logMessages[33]).toBe("Program log: Instruction: Transfer");
//   expect(meta.logMessages[37]).toBe(
//     "Program dRiftyHA39MWEi3m9aunc5MzRF1JYuBsbn6VPcn33UH success"
//   );
//   expect(meta.logMessages[47]).toBe(
//     "Program 6JjHXLheGSNvvexgzMthEcgjkcirDrGduc3HAKB2P1v2 success"
//   );
// };

// export const makeDriftUSDCWithdraw = async (
//   program: Program<Quartz>,
//   wallet: Keypair,
//   amountMicroCents: number,
//   banksClient: BanksClient
// ) => {
//   const walletUsdc = await getAssociatedTokenAddress(
//     USDC_MINT,
//     wallet.publicKey
//   );
//   const vaultPda = getVaultPda(wallet.publicKey);

//   const oix_createWSolAta = createAssociatedTokenAccountInstruction(
//     wallet.publicKey,
//     walletUsdc,
//     wallet.publicKey,
//     USDC_MINT
//   );

//   const ix_withdraw = await program.methods
//     .withdraw(new BN(amountMicroCents), DRIFT_MARKET_INDEX_USDC, false)
//     .accounts({
//       vault: vaultPda,
//       vaultSpl: getVaultSplPda(vaultPda, USDC_MINT),
//       owner: wallet.publicKey,
//       ownerSpl: walletUsdc,
//       splMint: USDC_MINT,
//       driftUser: getDriftUser(vaultPda),
//       driftUserStats: getDriftUserStats(vaultPda),
//       driftState: getDriftState(),
//       spotMarketVault: getDriftSpotMarketVault(DRIFT_MARKET_INDEX_USDC),
//       driftSigner: DRIFT_SIGNER,
//       tokenProgram: TOKEN_PROGRAM_ID,
//       associatedTokenProgram: ASSOCIATED_PROGRAM_ID,
//       driftProgram: DRIFT_PROGRAM_ID,
//       systemProgram: SystemProgram.programId,
//     })
//     .remainingAccounts([
//       toRemainingAccount(DRIFT_ORACLE_1, false, false),
//       toRemainingAccount(DRIFT_ORACLE_2, false, false),
//       toRemainingAccount(DRIFT_SPOT_MARKET_SOL, true, false),
//       toRemainingAccount(DRIFT_SPOT_MARKET_USDC, true, false),
//     ])
//     .instruction();

//   const instructions = [oix_createWSolAta, ix_withdraw];

//   const latestBlockhash = await banksClient.getLatestBlockhash();
//   const messageV0 = new TransactionMessage({
//     payerKey: wallet.publicKey,
//     recentBlockhash: latestBlockhash[0],
//     instructions: instructions,
//   }).compileToV0Message();
//   const tx = new VersionedTransaction(messageV0);

//   const simRes = await banksClient.simulateTransaction(tx);
//   const meta = await banksClient.processTransaction(tx);

//   expect(simRes.meta?.logMessages).toEqual(meta?.logMessages);
//   expect(meta.logMessages[1]).toBe("Program log: Create");
//   expect(meta.logMessages[22]).toBe("Program log: Instruction: Withdraw");
//   expect(meta.logMessages[26]).toBe(
//     "Program log: Instruction: InitializeAccount3"
//   );
//   expect(meta.logMessages[30]).toBe("Program log: Instruction: Withdraw");
//   expect(meta.logMessages[34]).toBe("Program log: Instruction: Transfer");
//   expect(meta.logMessages[38]).toBe(
//     "Program dRiftyHA39MWEi3m9aunc5MzRF1JYuBsbn6VPcn33UH success"
//   );
//   expect(meta.logMessages[48]).toBe(
//     "Program 6JjHXLheGSNvvexgzMthEcgjkcirDrGduc3HAKB2P1v2 success"
//   );
// };
