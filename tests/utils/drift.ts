import { PublicKey } from "@solana/web3.js";
import { QUARTZ_PROGRAM_ID } from "./helpers";

export const getVault = (owner: PublicKey) => {
	const [vault] = PublicKey.findProgramAddressSync(
		[Buffer.from("vault"), owner.toBuffer()],
		new PublicKey(QUARTZ_PROGRAM_ID)
	)
	return vault;
}
