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
  SYSVAR_RENT_PUBKEY,
  SYSVAR_INSTRUCTIONS_PUBKEY
} from "@solana/web3.js";
import { IDL as QuartzIDL, Quartz } from "../../target/types/quartz";
import {
  getAssociatedTokenAddress,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { ASSOCIATED_PROGRAM_ID } from "@coral-xyz/anchor/dist/cjs/utils/token";
import { evmAddressToSolana, processTransaction, setupATA } from "../utils/helpers";
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
  MESSAGE_TRANSMITTER_PROGRAM_ID,
  TOKEN_MESSAGE_MINTER_PROGRAM_ID,
  PROVIDER_BASE_ADDRESS,
  QUARTZ_CALLER_BASE_ADDRESS,
  MARGINFI_GROUP_1,
  MARGINFI_PROGRAM_ID
} from "../config/constants";
import config from "../config/config";
import { deposit, initUser } from "../utils/instructions";
import { getDriftSpotMarketVault, getDriftUserStats, getDriftState, getDriftUser, getVaultPda, getVaultSplPda, toRemainingAccount, getDriftSpotMarket, getSenderAuthority, getRemoteTokenMessenger, getTokenMessenger, getLocalToken, getMessageTransmitter, getTokenMinter, getEventAuthority, getBridgeRentPayer, getInitRentPayer, getSpendMulePda } from "../utils/accounts";
import { readFileSync } from "fs";

const TIMEOUT = 30_000;
describe("top up card", () => {
  let provider: BankrunProvider;
  let user: Keypair;
  let context: ProgramTestContext;
  let banksClient: BanksClient;
  let quartzProgram: Program<Quartz>;

  let vault: PublicKey;
  let driftUser: PublicKey;
  let driftUserStats: PublicKey;
  let walletUsdc: PublicKey;

  const driftState = getDriftState();
  const usdcSpotMarketVault = getDriftSpotMarketVault(DRIFT_MARKET_INDEX_USDC);
  const usdcSpotMarket = getDriftSpotMarket(DRIFT_MARKET_INDEX_USDC);
  const solSpotMarket = getDriftSpotMarket(DRIFT_MARKET_INDEX_SOL);
  const solSpotMarketVault = getDriftSpotMarketVault(DRIFT_MARKET_INDEX_SOL);
  const senderAuthority = getSenderAuthority();
  const messageTransmitter = getMessageTransmitter();
  const tokenMessenger = getTokenMessenger();
  const tokenMinter = getTokenMinter();
  const localToken = getLocalToken();
  const remoteTokenMessenger = getRemoteTokenMessenger();
  const eventAuthority = getEventAuthority();
  const bridgeRentPayer = getBridgeRentPayer();
  const initRentPayer = getInitRentPayer();
  const spendCaller = config.SPEND_CALLER;

  beforeEach(async () => {
    user = Keypair.generate();
    vault = getVaultPda(user.publicKey);
    driftUser = getDriftUser(vault);
    driftUserStats = getDriftUserStats(vault);
    walletUsdc = await getAssociatedTokenAddress(USDC_MINT, user.publicKey);
    
    const connection = new Connection(config.RPC_URL);
    const driftStateAccount = await connection.getAccountInfo(driftState);
    const usdcSpotMarketAccountInfo = await connection.getAccountInfo(usdcSpotMarket);
    const solSpotMarketAccountInfo = await connection.getAccountInfo(solSpotMarket);
    const usdcOracleAccountInfo = await connection.getAccountInfo(DRIFT_ORACLE_USDC);
    const solOracleAccountInfo = await connection.getAccountInfo(DRIFT_ORACLE_SOL);
    const driftSignerAccountInfo = await connection.getAccountInfo(DRIFT_SIGNER);
    const usdcMintAccountInfo = await connection.getAccountInfo(USDC_MINT);
    const usdcSpotMarketVaultAccountInfo = await connection.getAccountInfo(usdcSpotMarketVault);
    const solSpotMarketVaultAccountInfo = await connection.getAccountInfo(solSpotMarketVault);
    const messageTransmitterInfo = await connection.getAccountInfo(messageTransmitter);
    const tokenMessengerInfo = await connection.getAccountInfo(tokenMessenger);
    const tokenMinterInfo = await connection.getAccountInfo(tokenMinter);
    const localTokenAccountInfo = await connection.getAccountInfo(localToken);
    const remoteTokenMessengerAccountInfo = await connection.getAccountInfo(remoteTokenMessenger);
    const bridgeRentPayerAccountInfo = await connection.getAccountInfo(bridgeRentPayer);
    const initRentPayerAccountInfo = await connection.getAccountInfo(initRentPayer);
    const marginfiGroupAccountInfo = await connection.getAccountInfo(MARGINFI_GROUP_1);
    const spendCallerAccountInfo = await connection.getAccountInfo(spendCaller.publicKey);

    const tokenMessageMinterAccountInfo = await connection.getAccountInfo(TOKEN_MESSAGE_MINTER_PROGRAM_ID);
    const programDataAddress = PublicKey.findProgramAddressSync(
      [TOKEN_MESSAGE_MINTER_PROGRAM_ID.toBuffer()],
      new PublicKey('BPFLoaderUpgradeab1e11111111111111111111111')
    )[0];
    const programDataInfo = await connection.getAccountInfo(programDataAddress);

    context = await startAnchor(
      "./",
      [
        { name: "drift", programId: DRIFT_PROGRAM_ID },
        { name: "marginfi", programId: MARGINFI_PROGRAM_ID },
        { name: "token_messenger_minter", programId: TOKEN_MESSAGE_MINTER_PROGRAM_ID },
        { name: "message_transmitter", programId: MESSAGE_TRANSMITTER_PROGRAM_ID }
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
          address: usdcSpotMarketVault,
          info: usdcSpotMarketVaultAccountInfo,
        },
        {
          address: solSpotMarketVault,
          info: solSpotMarketVaultAccountInfo,
        },
        {
          address: usdcSpotMarket,
          info: usdcSpotMarketAccountInfo,
        },
        {
          address: solSpotMarket,
          info: solSpotMarketAccountInfo,
        },
        {
          address: DRIFT_ORACLE_USDC,
          info: usdcOracleAccountInfo,
        },
        {
          address: DRIFT_ORACLE_SOL,
          info: solOracleAccountInfo,
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
          address: messageTransmitter,
          info: messageTransmitterInfo,
        },
        {
          address: tokenMessenger,
          info: tokenMessengerInfo,
        },
        {
          address: tokenMinter,
          info: tokenMinterInfo,
        },
        {
          address: localToken,
          info: localTokenAccountInfo,
        },
        {
          address: remoteTokenMessenger,
          info: remoteTokenMessengerAccountInfo,
        },
        {
          address: bridgeRentPayer,
          info: bridgeRentPayerAccountInfo,
        },
        {
          address: initRentPayer,
          info: initRentPayerAccountInfo
        },
        {
          address: MARGINFI_GROUP_1,
          info: marginfiGroupAccountInfo
        },
        {
          address: spendCaller.publicKey,
          info: spendCallerAccountInfo
        },
        {
          address: TOKEN_MESSAGE_MINTER_PROGRAM_ID,
          info: tokenMessageMinterAccountInfo,
        },
        {
          address: programDataAddress,
          info: programDataInfo,
        },
      ]
    );
  
    banksClient = context.banksClient;
    provider = new BankrunProvider(context);
    quartzProgram = new Program<Quartz>(QuartzIDL, QUARTZ_PROGRAM_ID, provider);

    const marginfiAccount = Keypair.generate();
    await initUser(
      quartzProgram, 
      banksClient,
      [user],
      {
        requiresMarginfiAccount: true,
        spendLimitPerTransaction: 1000_000_000,
        spendLimitPerTimeframe: 1000_000_000,
        extendSpendLimitPerTimeframeResetSlotAmount: 1000_000_000,
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
    await setupATA(context, USDC_MINT, user.publicKey, 10_000_000);
    await deposit(quartzProgram, banksClient, 10_000_000, DRIFT_MARKET_INDEX_USDC, {
      vault: vault,
      vaultSpl: getVaultSplPda(vault, USDC_MINT),
      owner: user.publicKey,
      ownerSpl: walletUsdc,
      splMint: USDC_MINT,
      driftUser: driftUser,
      driftUserStats: driftUserStats,
      driftState: driftState,
      spotMarketVault: usdcSpotMarketVault,
      tokenProgram: TOKEN_PROGRAM_ID,
      associatedTokenProgram: ASSOCIATED_PROGRAM_ID,
      driftProgram: DRIFT_PROGRAM_ID,
      systemProgram: SystemProgram.programId
    }, [
      toRemainingAccount(DRIFT_ORACLE_USDC, false, false),
      toRemainingAccount(usdcSpotMarket, true, false),
    ])
  }, TIMEOUT);

  test("Should spend", async () => {
    const amount = 5_000_000;
    const messageSentEventDataKeypair = Keypair.generate();

    console.log("Creating start ix");
    
    const ix_startSpend = await quartzProgram.methods
      .startSpend(new BN(amount))
      .accounts({
        vault: vault,
        owner: user.publicKey,
        spendCaller: spendCaller.publicKey,
        mule: getSpendMulePda(user.publicKey),
        usdcMint: USDC_MINT,
        driftUser: driftUser,
        driftUserStats: driftUserStats,
        driftState: driftState,
        spotMarketVault: usdcSpotMarketVault,
        driftSigner: DRIFT_SIGNER,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_PROGRAM_ID,
        driftProgram: DRIFT_PROGRAM_ID,
        instructions: SYSVAR_INSTRUCTIONS_PUBKEY,
        systemProgram: SystemProgram.programId,
      })
      .remainingAccounts([
        toRemainingAccount(DRIFT_ORACLE_USDC, false, false),
        toRemainingAccount(usdcSpotMarket, true, false),
      ])
      .instruction();

    console.log("Creating spend ix");

    const ix_completeSpend = await quartzProgram.methods
      .completeSpend()
      .accounts({
        vault: vault,
        owner: user.publicKey,
        spendCaller: spendCaller.publicKey,
        mule: getSpendMulePda(user.publicKey),
        usdcMint: USDC_MINT,
        bridgeRentPayer: bridgeRentPayer,
        senderAuthorityPda: senderAuthority,
        messageTransmitter: messageTransmitter,
        tokenMessenger: tokenMessenger,
        remoteTokenMessenger: remoteTokenMessenger,
        tokenMinter: tokenMinter,
        localToken: localToken,
        messageSentEventData: messageSentEventDataKeypair.publicKey,
        eventAuthority: eventAuthority,
        messageTransmitterProgram: MESSAGE_TRANSMITTER_PROGRAM_ID,
        tokenMessengerMinterProgram: TOKEN_MESSAGE_MINTER_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_PROGRAM_ID,
        instructions: SYSVAR_INSTRUCTIONS_PUBKEY,
        systemProgram: SystemProgram.programId,
      })
      .instruction();

    console.log("Processing instructions");
    
    const meta = await processTransaction(
      banksClient, 
      user.publicKey, 
      [ix_startSpend, ix_completeSpend],
      [user, messageSentEventDataKeypair]
    );
    console.log(meta.logMessages);
    
  }, TIMEOUT);
});

// TODO: Add more checks (for things that should fail)
