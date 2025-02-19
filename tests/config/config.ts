import { Keypair } from '@solana/web3.js';
import bs58 from 'bs58';
import dotenv from 'dotenv';
import { z } from 'zod';

dotenv.config();

const envSchema = z.object({
    RPC_URL: z.string().url(),
    RENT_RECLAIMER: z.string()
        .transform((val) => {
            try {
                return Keypair.fromSecretKey(bs58.decode(val));
            } catch (error) {
                throw new Error(`Invalid RENT_RECLAIMER: ${val}`);
            }
        }),
    SPEND_CALLER: z.string()
        .transform((val) => {
            try {
                return Keypair.fromSecretKey(bs58.decode(val));
            } catch (error) {
                throw new Error(`Invalid SPEND_CALLER: ${val}`);
            }
        }),
});

const config = envSchema.parse(process.env);
export default config;
