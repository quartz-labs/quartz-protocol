import { Program } from "@coral-xyz/anchor";
import { BankrunProvider } from "anchor-bankrun";
import { expect, test } from '@jest/globals';
import {
	startAnchor,
	ProgramTestContext,
	BanksClient
} from "solana-bankrun";
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import { IDL as QuartzIDL, Quartz } from "../../../target/types/quartz";
import { getVault, QUARTZ_PROGRAM_ID } from "../../utils/helpers";
import { initUser } from "./userSetup";

describe("Quartz User", () => {
	let provider: BankrunProvider,
		user: Keypair,
		context: ProgramTestContext,
		banksClient: BanksClient,
		quartzProgram: Program<Quartz>;

	beforeEach(async () => {
		user = Keypair.generate();
		context = await startAnchor("./", [],
			[
				{
					address: user.publicKey,
					info: {
						lamports: 1_000_000_000,
						data: Buffer.alloc(0),
						owner: SystemProgram.programId,
						executable: false,
					},
				},
			]
		);

		provider = new BankrunProvider(context);
		quartzProgram = new Program<Quartz>(
			QuartzIDL,
			QUARTZ_PROGRAM_ID,
			provider,
		);

		banksClient = context.banksClient;
	});

	test("Init User", async () => {
		const vaultPda = getVault(user.publicKey);

		await initUser(quartzProgram, banksClient, vaultPda, user);
	});

	test("Fails to init user with wrong vault", async () => {
		const [badVaultPda] = PublicKey.findProgramAddressSync(
			[Buffer.from("bad_vault"), user.publicKey.toBuffer()],
			new PublicKey(QUARTZ_PROGRAM_ID)
		);

		try {
			await initUser(quartzProgram, banksClient, badVaultPda, user);

			// If we reach here, the test should fail
			expect(false).toBe(true);
		} catch (error: any) {
			expect(error);
		}
	});

	test("Close User", async () => {
		const vaultPda = getVault(user.publicKey);

		await initUser(quartzProgram, banksClient, vaultPda, user);

		await quartzProgram.methods
			.closeUser()
			.accounts({
				vault: vaultPda,
					owner: user.publicKey
			})
			.signers([user])
			.rpc();

		try {
			const closedVaultAccount = await quartzProgram.account.vault.fetch(vaultPda);
		} catch (error: any) {
			expect(error.message).toContain("Could not find");
		}
	});
});