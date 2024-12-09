import { getVault, QUARTZ_PROGRAM_ID, RPC_URL, toRemainingAccount, USDC_MINT } from "../../utils/helpers"
import { createAssociatedTokenAccountInstruction, getAssociatedTokenAddress, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { ASSOCIATED_TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { DRIFT_MARKET_INDEX_SOL, DRIFT_MARKET_INDEX_USDC, DRIFT_ORACLE_1, DRIFT_ORACLE_2, DRIFT_PROGRAM_ID, DRIFT_SIGNER, DRIFT_SPOT_MARKET_SOL, DRIFT_SPOT_MARKET_USDC, getDriftUserStats } from "../../utils/drift"
import { getDriftUser } from "../../utils/drift";
import { getVaultSpl, WSOL_MINT } from "../../utils/helpers";
import { PublicKey, Keypair, SystemProgram, LAMPORTS_PER_SOL, SYSVAR_INSTRUCTIONS_PUBKEY, Connection, TransactionMessage, VersionedTransaction } from "@solana/web3.js";
import { BN, Program } from "@coral-xyz/anchor";
import BigNumber from "bignumber.js";
import { BanksClient, startAnchor } from "solana-bankrun";
import { BankrunProvider } from "anchor-bankrun";
import { Quartz, IDL as QuartzIDL } from "../../../target/types/quartz";

export const setupTestEnvironment = async () => {
    const user = Keypair.generate();
    const connection = new Connection(RPC_URL);
    const accountInfo = await connection.getAccountInfo(new PublicKey("5zpq7DvB6UdFFvpmBPspGPNfUGoBRRCE2HHg5u3gxcsN"));
    const solSpotMarketVaultAccountInfo = await connection.getAccountInfo(new PublicKey("DfYCNezifxAEsQbAJ1b3j6PX3JVBe8fu11KBhxsbw5d2"));
    const usdcSpotMarketVaultAccountInfo = await connection.getAccountInfo(new PublicKey("GXWqPpjQpdz7KZw9p7f5PX2eGxHAhvpNXiviFkAB8zXg"));
    const solSpotMarketAccountInfo = await connection.getAccountInfo(DRIFT_SPOT_MARKET_SOL);
    const usdcSpotMarketAccountInfo = await connection.getAccountInfo(DRIFT_SPOT_MARKET_USDC);
    const oracle1AccountInfo = await connection.getAccountInfo(DRIFT_ORACLE_1);
    const oracle2AccountInfo = await connection.getAccountInfo(DRIFT_ORACLE_2);
    const driftSignerAccountInfo = await connection.getAccountInfo(DRIFT_SIGNER);
    const usdcMintAccountInfo = await connection.getAccountInfo(USDC_MINT);


    const context = await startAnchor("./", [{ name: "drift", programId: DRIFT_PROGRAM_ID }],
        [
            {
                address: user.publicKey,
                info: {
                    lamports: 1_000_000_000_000_000,
                    data: Buffer.alloc(0),
                    owner: SystemProgram.programId,
                    executable: false,
                },
            },
            //drift authority
            {
                address: new PublicKey("rxEaSMXqKx9GvYY8rrZB1SG5CQUXTfnXbZSaceaaPzA"),
                info: {
                    lamports: 1_000_000_000,
                    data: Buffer.alloc(0),
                    owner: new PublicKey("6JjHXLheGSNvvexgzMthEcgjkcirDrGduc3HAKB2P1v2"),
                    executable: false,
                }
            },
            //drift state
            {
                address: new PublicKey("5zpq7DvB6UdFFvpmBPspGPNfUGoBRRCE2HHg5u3gxcsN"),
                info: accountInfo
            },
            // Drift Sol spot market vault
            {
                address: new PublicKey("DfYCNezifxAEsQbAJ1b3j6PX3JVBe8fu11KBhxsbw5d2"),
                info: solSpotMarketVaultAccountInfo
            },
            // Drift USDC spot market vault
            {
                address: new PublicKey("GXWqPpjQpdz7KZw9p7f5PX2eGxHAhvpNXiviFkAB8zXg"),
                info: usdcSpotMarketVaultAccountInfo
            },
            {
                address: DRIFT_SPOT_MARKET_SOL,
                info: solSpotMarketAccountInfo
            },
            {
                address: DRIFT_SPOT_MARKET_USDC,
                info: usdcSpotMarketAccountInfo
            },
            {
                address: DRIFT_ORACLE_1,
                info: oracle1AccountInfo
            },
            {
                address: DRIFT_SIGNER,
                info: driftSignerAccountInfo
            },
            {
                address: DRIFT_ORACLE_2,
                info: oracle2AccountInfo
            },
            {
                address: USDC_MINT,
                info: usdcMintAccountInfo
            }
        ]
    );

    const banksClient = context.banksClient;
    const provider = new BankrunProvider(context);

    const quartzProgram = new Program<Quartz>(
        QuartzIDL,
        QUARTZ_PROGRAM_ID,
        provider,
    );

    const vaultPda = getVault(user.publicKey);

    return { user, context, banksClient, quartzProgram, vaultPda };
}


// export async function executeAutoRepay(vault: PublicKey, owner: PublicKey, loanAmountBaseUnits: number, wallet: Keypair, program: Program<Quartz>): Promise<string> {
//     if (!this.program || !this.wallet || !this.walletWSol) throw new Error("AutoRepayBot is not initialized");

//     const walletWSol = await getAssociatedTokenAddress(WSOL_MINT, wallet.publicKey);

//     const oix_createWSolAtaPromise = createAssociatedTokenAccountInstruction(
//         wallet.publicKey,
//         walletWSol,
//         wallet.publicKey,
//         WSOL_MINT
//     )

//     const vaultWsol = getVaultSpl(vault, WSOL_MINT);
//     const vaultUsdc = getVaultSpl(vault, USDC_MINT);
//     const driftUser = getDriftUser(vault);
//     const driftUserStats = getDriftUserStats(vault);

//     //const jupiterQuotePromise = getJupiterSwapQuote(WSOL_MINT, USDC_MINT, loanAmountBaseUnits);
//     const jupiterQuotePromise = "x"

//     const preLoanBalancePromise = this.connection.getTokenAccountBalance(this.walletWSol!).then(res => res.value.amount);

//     const autoRepayDepositPromise = program.methods
//         .autoRepayDeposit(DRIFT_MARKET_INDEX_USDC)
//         .accounts({
//             vault: vault,
//             vaultSpl: vaultUsdc,
//             owner: owner,
//             caller: this.wallet.publicKey,
//             callerSpl: this.walletUsdc,
//             splMint: USDC_MINT,
//             driftUser: driftUser,
//             driftUserStats: driftUserStats,
//             driftState: this.driftState,
//             spotMarketVault: this.driftSpotMarketUsdc,
//             tokenProgram: TOKEN_PROGRAM_ID,
//             associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
//             driftProgram: DRIFT_PROGRAM_ID,
//             systemProgram: SystemProgram.programId,
//             instructions: SYSVAR_INSTRUCTIONS_PUBKEY,
//         })
//         .remainingAccounts([
//             toRemainingAccount(DRIFT_ORACLE_2, false, false),
//             toRemainingAccount(DRIFT_ORACLE_1, false, false),
//             toRemainingAccount(DRIFT_SPOT_MARKET_SOL, true, false),
//             toRemainingAccount(DRIFT_SPOT_MARKET_USDC, true, false)
//         ])
//         .instruction();

//     const autoRepayWithdrawPromise = program.methods
//         .autoRepayWithdraw(DRIFT_MARKET_INDEX_SOL)
//         .accounts({
//             vault: vault,
//             vaultSpl: vaultWsol,
//             owner: owner,
//             caller: this.wallet.publicKey,
//             callerSpl: this.walletWSol,
//             splMint: WSOL_MINT,
//             driftUser: driftUser,
//             driftUserStats: driftUserStats,
//             driftState: this.driftState,
//             spotMarketVault: this.driftSpotMarketSol,
//             driftSigner: DRIFT_SIGNER,
//             tokenProgram: TOKEN_PROGRAM_ID,
//             driftProgram: DRIFT_PROGRAM_ID,
//             systemProgram: SystemProgram.programId,
//             depositPriceUpdate: this.usdcUsdPriceFeedAccount,
//             withdrawPriceUpdate: this.solUsdPriceFeedAccount,
//             instructions: SYSVAR_INSTRUCTIONS_PUBKEY,
//         })
//         .remainingAccounts([
//             toRemainingAccount(DRIFT_ORACLE_2, false, false),
//             toRemainingAccount(DRIFT_ORACLE_1, false, false),
//             toRemainingAccount(DRIFT_SPOT_MARKET_SOL, true, false),
//             toRemainingAccount(DRIFT_SPOT_MARKET_USDC, false, false)
//         ])
//         .instruction();

//     const [preLoanBalance, jupiterQuote] = await Promise.all([preLoanBalancePromise, jupiterQuotePromise]);
//     //const jupiterSwapPromise = getJupiterSwapIx(this.wallet.publicKey, this.connection, jupiterQuote);
//     const jupiterSwapPromise = "x"

//     const amountLamports = Number(jupiterQuote.inAmount);
//     const amountLamportsWithSlippage = Math.floor(amountLamports * (1.01));
//     const walletWsolBalance = Number(preLoanBalance) + amountLamportsWithSlippage;

//     const autoRepayStartPromise = program.methods
//         .autoRepayStart(new BN(walletWsolBalance))
//         .accounts({
//             caller: this.wallet.publicKey,
//             callerWithdrawSpl: this.walletWSol,
//             withdrawMint: WSOL_MINT,
//             vault: vault,
//             vaultWithdrawSpl: vaultWsol,
//             owner: owner,
//             tokenProgram: TOKEN_PROGRAM_ID,
//             associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
//             systemProgram: SystemProgram.programId,
//             instructions: SYSVAR_INSTRUCTIONS_PUBKEY,
//         })
//         .instruction();

//     const [
//         oix_createWSolAta,
//         ix_autoRepayStart,
//         jupiterSwap,
//         ix_autoRepayDeposit,
//         ix_autoRepayWithdraw
//     ] = await Promise.all([oix_createWSolAtaPromise, autoRepayStartPromise, jupiterSwapPromise, autoRepayDepositPromise, autoRepayWithdrawPromise]);
//     const { ix_jupiterSwap, jupiterLookupTables } = jupiterSwap;

//     const amountSolUi = new BigNumber(amountLamportsWithSlippage).div(LAMPORTS_PER_SOL);
//     const { flashloanTx } = await this.marginfiAccount!.makeLoopTx(
//         amountSolUi,
//         amountSolUi,
//         this.wSolBank!,
//         this.wSolBank!,
//         [...oix_createWSolAta, ix_autoRepayStart, ix_jupiterSwap, ix_autoRepayDeposit, ix_autoRepayWithdraw],
//         [this.quartzLookupTable!, ...jupiterLookupTables],
//         0.002,
//         false
//     );

//     const signedTx = await this.wallet.signTransaction(flashloanTx);
//     const signature = await this.connection.sendRawTransaction(signedTx.serialize());
//     return signature;
// }