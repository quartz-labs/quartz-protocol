import dotenv from 'dotenv';
import { z } from 'zod';

dotenv.config();

const envSchema = z.object({
    RPC_URL: z.string().url()
});

const config = envSchema.parse(process.env);
export default config;
