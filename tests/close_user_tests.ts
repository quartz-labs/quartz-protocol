import * as anchor from "@coral-xyz/anchor";
import { Program, AnchorError } from "@coral-xyz/anchor";
import { FundsProgram } from "../target/types/funds_program";
import { Keypair, SystemProgram, LAMPORTS_PER_SOL, Transaction, PublicKey } from "@solana/web3.js";
import { createMint, getOrCreateAssociatedTokenAccount, TOKEN_PROGRAM_ID, mintTo, ASSOCIATED_TOKEN_PROGRAM_ID } from "@solana/spl-token";
import dotenv from 'dotenv';
const fs = require("fs");
import path from "path";
import { assert, expect } from "chai";
import { setupTests } from "./setup_tests";
dotenv.config();


describe("close_user tests", () => {
  let testSetup: Awaited<ReturnType<typeof setupTests>>;

  before(async () => {
    testSetup = await setupTests();
  });

  // TOOD - Implement once adding mobile app
  // it("close_user with user as init_payer", async () => {
  //   const { program, otherKeypairVaultPda, otherKeypairVaultUsdcPda, otherBackupKeypair, otherUserKeypair, testUsdcMint } = testSetup;

  //   await program.methods
  //     .initUser()
  //     .accounts({
  //       // @ts-ignore - Causing an issue in Cursor IDE
  //       vault: otherKeypairVaultPda,
  //       vaultUsdc: otherKeypairVaultUsdcPda,
  //       payer: otherBackupKeypair.publicKey,
  //       backup: otherBackupKeypair.publicKey,
  //       user: otherUserKeypair.publicKey,
  //       usdcMint: testUsdcMint,
  //       tokenProgram: TOKEN_PROGRAM_ID,
  //       systemProgram: SystemProgram.programId,
  //     })
  //     .signers([otherBackupKeypair])
  //     .rpc();
    
  //   const account = await program.account.vault.fetch(otherKeypairVaultPda);
  //   expect(account.backup.equals(otherBackupKeypair.publicKey)).to.be.true;
  //   expect(account.user.equals(otherUserKeypair.publicKey)).to.be.true;
  //   expect(account.initPayer.equals(otherBackupKeypair.publicKey)).to.be.true;

  //   await program.methods
  //     .closeUser()
  //     .accounts({
  //       // @ts-ignore - Causing an issue in Cursor IDE
  //       vault: otherKeypairVaultPda,
  //       initPayer: otherBackupKeypair.publicKey,
  //       backup: otherBackupKeypair.publicKey,
  //       user: otherUserKeypair.publicKey
  //     })
  //     .signers([otherUserKeypair])
  //     .rpc();
  // });

  // it("close_user incorrect init_payer", async () => {
  //   const { program, vaultPda, backupKeypair, userKeypair, otherUserKeypair } = testSetup;

  //   const desiredErrorCode = "InvalidInitPayer";

  //   try {
  //     await program.methods
  //       .closeUser()
  //       .accounts({
  //         // @ts-ignore - Causing an issue in Curosr IDE
  //         vault: vaultPda,
  //         initPayer: otherUserKeypair.publicKey,
  //         backup: backupKeypair.publicKey,
  //         user: userKeypair.publicKey,
  //       })  
  //       .signers([userKeypair])
  //       .rpc();

  //     assert.fail(0, 1, "close_user instruction call should have failed");
  //   } catch (err) {
  //     expect(err).to.be.instanceOf(AnchorError);
  //     expect((err as AnchorError).error.errorCode.code).to.equal(desiredErrorCode);
  //   }
  // });


  it("close_user incorrect signature", async () => {
    const { program, vaultPda, ownerKeypair, quartzManagerKeypair } = testSetup;
    const desiredErrorMessage = "Missing signature"

    try {
      await program.methods
        .closeUser()
        .accounts({
          // @ts-ignore - Causing an issue in Cursor IDE
          vault: vaultPda,
          owner: ownerKeypair.publicKey
        })
        .signers([quartzManagerKeypair])
        .rpc();

      assert.fail("close_user instruction should have failed");
    } catch (err) {
      expect(err).to.be.instanceOf(Error);
      expect(err.message).to.include(desiredErrorMessage);
    }
  });


  it("close_user incorrect owner", async () => {
    const { program, vaultPda, ownerKeypair, otherOwnerKeypair, quartzManagerKeypair } = testSetup;
    const desiredErrorMessage = "Missing signature"

    try {
      await program.methods
        .closeUser()
        .accounts({
          // @ts-ignore - Causing an issue in Cursor IDE
          vault: vaultPda,
          owner: otherOwnerKeypair.publicKey
        })
        .signers([quartzManagerKeypair])
        .rpc();

      assert.fail("close_user instruction should have failed");
    } catch (err) {
      expect(err).to.be.instanceOf(Error);
      expect(err.message).to.include(desiredErrorMessage);
    }
  });


  // TODO - Implement after adding mobile app
  // it("close_user backup signature", async () => {
  //   const { program, vaultPda, backupKeypair, userKeypair, quartzManagerKeypair } = testSetup;
  //   const desiredErrorMessage = "unknown signer"

  //   try {
  //     await program.methods
  //       .closeUser()
  //       .accounts({
  //         // @ts-ignore - Causing an issue in Cursor IDE
  //         vault: vaultPda,
  //         initPayer: quartzManagerKeypair.publicKey,
  //         backup: backupKeypair.publicKey,
  //         user: userKeypair.publicKey
  //       })
  //       .signers([backupKeypair])
  //       .rpc();

  //     assert.fail("close_user instruction should have failed");
  //   } catch (err) {
  //     expect(err).to.be.instanceOf(Error);
  //     expect(err.message).to.include(desiredErrorMessage);
  //   }
  // });


  // it("close_user after change to new user", async () => {
  //   const { program, testUsdcKeypair, quartzManagerKeypair, testUsdcMint } = testSetup;
  //   const testBackupKeypair = Keypair.generate();
  //   const testUserKeypair = Keypair.generate();
  //   const testNewUserKeypair = Keypair.generate();
  //   const [testKeypairVaultPda] = anchor.web3.PublicKey.findProgramAddressSync(
  //     [Buffer.from("vault"), testBackupKeypair.publicKey.toBuffer()],
  //     program.programId
  //   );
  //   const [testKeypairVaultUsdcPda] = anchor.web3.PublicKey.findProgramAddressSync(
  //     [Buffer.from("vault"), testBackupKeypair.publicKey.toBuffer(), testUsdcKeypair.publicKey.toBuffer()],
  //     program.programId
  //   );

  //   // Init account
  //   await program.methods
  //     .initUser()
  //     .accounts({
  //       // @ts-ignore - Causing an issue in Cursor IDE
  //       vault: testKeypairVaultPda,
  //       vaultUsdc: testKeypairVaultUsdcPda,
  //       payer: quartzManagerKeypair.publicKey,
  //       backup: testBackupKeypair.publicKey,
  //       user: testUserKeypair.publicKey,
  //       usdcMint: testUsdcMint,
  //       tokenProgram: TOKEN_PROGRAM_ID,
  //       systemProgram: SystemProgram.programId,
  //     })
  //     .signers([quartzManagerKeypair])
  //     .rpc();
  //   let account = await program.account.vault.fetch(testKeypairVaultPda);
  //   expect(account.backup.equals(testBackupKeypair.publicKey)).to.be.true;
  //   expect(account.user.equals(testUserKeypair.publicKey)).to.be.true;
  //   expect(account.initPayer.equals(quartzManagerKeypair.publicKey)).to.be.true;

  //   // Change user
  //   await program.methods
  //     .changeUser()
  //     .accounts({
  //       // @ts-ignore - Causing an issue in Cursor IDE
  //       vault: testKeypairVaultPda,
  //       backup: testBackupKeypair.publicKey,
  //       newUser: testNewUserKeypair.publicKey,
  //     })
  //     .signers([testBackupKeypair])
  //     .rpc();
  //   account = await program.account.vault.fetch(testKeypairVaultPda);
  //   expect(account.backup.equals(testBackupKeypair.publicKey)).to.be.true;
  //   expect(account.user.equals(testNewUserKeypair.publicKey)).to.be.true;
  //   expect(account.initPayer.equals(quartzManagerKeypair.publicKey)).to.be.true;

  //   // Close account with new user
  //   await program.methods
  //     .closeUser()
  //     .accounts({
  //       // @ts-ignore - Causing an issue in Cursor IDE
  //       vault: testKeypairVaultPda,
  //       initPayer: quartzManagerKeypair.publicKey,
  //       backup: testBackupKeypair.publicKey,
  //       user: testNewUserKeypair.publicKey
  //     })
  //     .signers([testNewUserKeypair])
  //     .rpc();
  // });


  it("close_user", async () => {
    const { program, vaultPda, ownerKeypair, quartzManagerKeypair } = testSetup;

    await program.methods
      .closeUser()
      .accounts({
        // @ts-ignore - Causing an issue in Cursor IDE
        vault: vaultPda,
        owner: ownerKeypair.publicKey
      })
      .signers([ownerKeypair])
      .rpc();
  });
});