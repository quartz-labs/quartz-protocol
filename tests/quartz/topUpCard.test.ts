import { BN, Program, web3 } from "@coral-xyz/anchor";
import { BankrunProvider } from "anchor-bankrun";
import { beforeAll, expect, test } from "@jest/globals";
import { ProgramTestContext, BanksClient, startAnchor } from "solana-bankrun";
import {
  Keypair,
  PublicKey,
  SystemProgram,
  Connection,
  LAMPORTS_PER_SOL
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
  QUARTZ_CALLER_BASE_ADDRESS
} from "../config/constants";
import config from "../config/config";
import { deposit, initUser, makeWrapSolIxs } from "../utils/instructions";
import { initDriftAccount } from "../utils/instructions";
import { getDriftSpotMarketVault, getDriftUserStats, getDriftState, getDriftUser, getVaultPda, getVaultSplPda, toRemainingAccount, getDriftSpotMarket, getSenderAuthority, getRemoteTokenMessenger, getTokenMessenger, getLocalToken, getMessageTransmitter, getTokenMinter, getEventAuthority, getBridgeRentPayer } from "../utils/accounts";

const TIMEOUT = 10_000;
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

    context = await startAnchor(
      "./",
      [
        { name: "drift", programId: DRIFT_PROGRAM_ID },
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
      ]
    );
  
    banksClient = context.banksClient;
    provider = new BankrunProvider(context);
    quartzProgram = new Program<Quartz>(QuartzIDL, QUARTZ_PROGRAM_ID, provider);

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

  test("Should top up card", async () => {
    const amount = 5_000_000;
    const messageSentEventDataKeypair = Keypair.generate();

    const ix_topUpCard = await quartzProgram.methods
      .topUpCard(new BN(amount))
      .accounts({
        vault: vault,
        vaultUsdc: getVaultSplPda(vault, USDC_MINT),
        owner: user.publicKey,
        usdcMint: USDC_MINT,
        driftUser: driftUser,
        driftUserStats: driftUserStats,
        driftState: driftState,
        spotMarketVault: usdcSpotMarketVault,
        driftSigner: DRIFT_SIGNER,
        driftProgram: DRIFT_PROGRAM_ID,
        providerBaseAddress: evmAddressToSolana(PROVIDER_BASE_ADDRESS),
        quartzCallerBaseAddress: evmAddressToSolana(QUARTZ_CALLER_BASE_ADDRESS),
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
        systemProgram: SystemProgram.programId,
      })
      .remainingAccounts([
        toRemainingAccount(DRIFT_ORACLE_USDC, false, false),
        toRemainingAccount(usdcSpotMarket, true, false),
      ])
      .instruction();
      
    const meta = await processTransaction(
      banksClient, 
      user.publicKey, 
      [ix_topUpCard],
      [user, messageSentEventDataKeypair]
    );
    console.log(meta.logMessages);
    
  }, TIMEOUT);
});

// TODO: Add more checks (for things that should fail)
