import { BanksClient, Clock, ProgramTestContext } from "solana-bankrun";
import { PublicKey, TransactionMessage, VersionedTransaction, TransactionInstruction, Connection, AddressLookupTableAccount, AccountInfo, AddressLookupTableState } from "@solana/web3.js";
import { QuoteResponse } from "@jup-ag/api";
import { AccountMeta } from "@jup-ag/api";
import { MarketIndex } from "../config/tokens";
import { TOKENS } from "../config/tokens";
import { PYTH_ORACLE_PROGRAM_ID } from "../config/constants";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { getAssociatedTokenAddressSync } from "@solana/spl-token";
import { AccountLayout } from "@solana/spl-token";
import { ACCOUNT_SIZE } from "@solana/spl-token";
import { AddressLookupTableProgram } from "@solana/web3.js";

export const advanceBySlots = async (
  context: ProgramTestContext,
  slots: bigint
) => {
  const currentClock = await context.banksClient.getClock();
  context.setClock(
    new Clock(
      currentClock.slot + slots,
      currentClock.epochStartTimestamp,
      currentClock.epoch,
      currentClock.leaderScheduleEpoch,
      50n
    )
  );
};

export const processTransaction = async (
  banksClient: BanksClient,
  payer: PublicKey,
  instructions: TransactionInstruction[],
) => {
  const latestBlockhash = await banksClient.getLatestBlockhash();
  const messageV0 = new TransactionMessage({
      payerKey: payer,
      recentBlockhash: latestBlockhash[0],
      instructions: instructions,
  }).compileToV0Message();

  const tx = new VersionedTransaction(messageV0);
  const meta = await banksClient.processTransaction(tx);
  return meta;
};

export async function getJupiterSwapIx(
  walletPubkey: PublicKey, 
  connection: Connection, 
  quoteResponse: QuoteResponse
): Promise<{ 
  ix_jupiterSwap: TransactionInstruction, 
  jupiterLookupTables: AddressLookupTableAccount[] 
}> {
    const instructions: any = await (
        await fetch('https://quote-api.jup.ag/v6/swap-instructions', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify({
                quoteResponse,
                userPublicKey: walletPubkey.toBase58(),
                useCompression: true,
            })
        })
    ).json();

    if (instructions.error) {
        throw new Error(`Failed to get swap instructions: ${instructions.error}`);
    }
    const { swapInstruction, addressLookupTableAddresses } = instructions;

    const getAddressLookupTableAccounts = async (
        keys: string[]
      ): Promise<AddressLookupTableAccount[]> => {
        const addressLookupTableAccountInfos =
          await connection.getMultipleAccountsInfo(
            keys.map((key) => new PublicKey(key))
          );
      
        return addressLookupTableAccountInfos.reduce((acc: AddressLookupTableAccount[], accountInfo: AccountInfo<Buffer> | null, index: number) => {
          const addressLookupTableAddress = keys[index];
          if (accountInfo) {
            if (addressLookupTableAddress === undefined) throw new Error("Address lookup table address is undefined");
            const addressLookupTableAccount = new AddressLookupTableAccount({
              key: new PublicKey(addressLookupTableAddress),
              state: AddressLookupTableAccount.deserialize(accountInfo.data),
            });
            acc.push(addressLookupTableAccount);
          }
      
          return acc;
        }, new Array<AddressLookupTableAccount>());
    };

    const addressLookupTableAccounts: AddressLookupTableAccount[] = [];
    addressLookupTableAccounts.push(
        ...(await getAddressLookupTableAccounts(addressLookupTableAddresses ?? []))
    );

    const ix_jupiterSwap =  new TransactionInstruction({
        programId: new PublicKey(swapInstruction.programId),
        keys: swapInstruction.accounts.map((key: AccountMeta) => ({
          pubkey: new PublicKey(key.pubkey),
          isSigner: key.isSigner,
          isWritable: key.isWritable,
        })),
        data: Buffer.from(swapInstruction.data, "base64"),
    });

    return {
        ix_jupiterSwap,
        jupiterLookupTables: addressLookupTableAccounts,
    };
}

export function getPythOracle(marketIndex: MarketIndex) {
  const priceFeedId = TOKENS[marketIndex].pythPriceFeedId;

  const shardId = 0;
  let priceFeedIdBuffer: Buffer;
  if (priceFeedId.startsWith("0x")) {
      priceFeedIdBuffer = Buffer.from(priceFeedId.slice(2), "hex");
  } else {
      priceFeedIdBuffer = Buffer.from(priceFeedId, "hex");
  }
  if (priceFeedIdBuffer.length !== 32) {
      throw new Error("Feed ID should be 32 bytes long");
  }
  const shardBuffer = Buffer.alloc(2);
  shardBuffer.writeUint16LE(shardId, 0);
  return PublicKey.findProgramAddressSync([shardBuffer, priceFeedIdBuffer], PYTH_ORACLE_PROGRAM_ID)[0];
}

export async function setupATA(
  context: ProgramTestContext,
  mint: PublicKey,
  owner: PublicKey,
  amount: number
): Promise<PublicKey> {
  const tokenAccData = Buffer.alloc(ACCOUNT_SIZE);
  AccountLayout.encode(
    {
      mint: mint,
      owner,
      amount: BigInt(amount),
      delegateOption: 0,
      delegate: PublicKey.default,
      delegatedAmount: BigInt(0),
      state: 1,
      isNativeOption: 0,
      isNative: BigInt(0),
      closeAuthorityOption: 0,
      closeAuthority: PublicKey.default,
    },
    tokenAccData,
  );

  const ata = getAssociatedTokenAddressSync(mint, owner, true);
  const ataAccountInfo = {
    lamports: 1_000_000_000,
    data: tokenAccData,
    owner: TOKEN_PROGRAM_ID,
    executable: false,
  };

  context.setAccount(ata, ataAccountInfo);
  return ata;
}

export async function setupAddressLookupTable(
  connection: Connection,
  banksClient: BanksClient,
  context: ProgramTestContext,
  authority: PublicKey,
  addressLookupTable: PublicKey
) {
  const mainnetLookupTable = await connection.getAddressLookupTable(addressLookupTable).then(value => value.value);
  const addresses = mainnetLookupTable.state.addresses;
  if (addresses.length <= 0) throw new Error("Address lookup table is empty");

  const currentSlot = await banksClient.getSlot();
  await context.warpToSlot(currentSlot + BigInt(100));
  const newSlot = await banksClient.getSlot();

  const [ix_initLookupTable, lookupTable] = AddressLookupTableProgram.createLookupTable({
    authority: authority,
    payer: authority,
    recentSlot: newSlot - BigInt(1)
  });

  const CHUNK_SIZE = 20;
  const addressChunks = [];
  for (let i = 0; i < addresses.length; i += CHUNK_SIZE) {
    addressChunks.push(addresses.slice(i, i + CHUNK_SIZE));
  }

  const ixs_extendLookupTable: TransactionInstruction[] = addressChunks.map(chunk => 
    AddressLookupTableProgram.extendLookupTable({
      authority: authority,
      payer: authority,
      lookupTable: lookupTable,
      addresses: chunk
    })
  );

  const meta = await processTransaction(banksClient, authority, [ix_initLookupTable, ...ixs_extendLookupTable]);
  await context.warpToSlot(newSlot + BigInt(10));

  return {
    meta,
    lookupTable
  };
}

export async function fetchAddressLookupTable(
  banksClient: BanksClient,
  addressLookupTable: PublicKey
): Promise<AddressLookupTableAccount> {
  const lookupTableAccount = await banksClient.getAccount(addressLookupTable);
  if (!lookupTableAccount) throw new Error("Address lookup table not found");

  const state = AddressLookupTableAccount.deserialize(lookupTableAccount.data);
  if (!state) throw new Error("Address lookup table state not found");
  
  return new AddressLookupTableAccount({
    key: addressLookupTable,
    state: {
      deactivationSlot: state.deactivationSlot,
      lastExtendedSlot: state.lastExtendedSlot,
      lastExtendedSlotStartIndex: state.lastExtendedSlotStartIndex,
      authority: state.authority,
      addresses: state.addresses.map(address => new PublicKey(address.toBytes())),
    }
  });
}
