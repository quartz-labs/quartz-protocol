import * as anchor from "@coral-xyz/anchor";
import { BN, Program } from "@coral-xyz/anchor";
import { Quartz } from "../../target/types/quartz";
import { getVaultPda } from "../utils/accounts";
import { AddressLookupTableProgram, Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import fs from "fs";
import { TOKENS } from "../utils/tokens";

describe("my-project", () => {
    // Configure the client to use the local cluster.
    anchor.setProvider(anchor.AnchorProvider.env());

    let user: Keypair;
    let vaultPda: PublicKey;
    const program = anchor.workspace.Quartz as Program<Quartz>;

    const loadedKeyBytes = Uint8Array.from(
        JSON.parse(fs.readFileSync("./tests/quartz/testing-keypair.json", "utf8")),
    );

    user = Keypair.fromSecretKey(loadedKeyBytes);


    it("Is initialized!", async () => {
        // Verify balance
        const balance = await anchor.getProvider().connection.getBalance(user.publicKey);
        console.log("User balance:", balance);

        vaultPda = getVaultPda(user.publicKey);

        const slot = await anchor.getProvider().connection.getSlot() + 100;

        console.log('currentSlot:', slot);
        const slots = await anchor.getProvider().connection.getBlocks(slot - 200, undefined, "finalized");
        // if (slots.length < 100) {
        //   throw new Error(`Could find only ${slots.length} ${slots} on the main fork`);
        // }

        const [_ix, lookupTableAddress] = AddressLookupTableProgram.createLookupTable({
            authority: vaultPda,
            payer: vaultPda,
            recentSlot: slots[slots.length - 1],  // Use first slot from hashes
        });

        console.log("Lookup table address:", lookupTableAddress);

        const tx = await program.methods
            .initUser(new BN(0), new BN(slots[slots.length - 1]))
            .accounts({
                vault: vaultPda,
                owner: user.publicKey,
                systemProgram: SystemProgram.programId,
                lookupTable: lookupTableAddress,
                addressLookupTableProgram: new PublicKey("AddressLookupTab1e1111111111111111111111111"),
           })
           .remainingAccounts(Object.values(TOKENS).map(token => ({
            pubkey: token.mint,
            isWritable: false,
            isSigner: false,
           })))
            .signers([user]).rpc();

        console.log("Your transaction signature", tx);


    }, 1000000);
});