/*
 * Copyright (c) 2024, Circle Internet Financial LTD All Rights Reserved.
 *
 * SPDX-License-Identifier: Apache-2.0
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 * http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
 
 //! IsNonceUsed instruction handler

use {crate::state::UsedNonces, anchor_lang::prelude::*};

// Instruction accounts
#[derive(Accounts)]
pub struct IsNonceUsedContext<'info> {
    /// CHECK: Used nonces state
    /// Account will be explicitly loaded to avoid error when it's not initialized
    #[account()]
    pub used_nonces: UncheckedAccount<'info>,
}

// Instruction parameters
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct IsNonceUsedParams {
    pub nonce: u64,
}

// Instruction handler
pub fn is_nonce_used(
    ctx: Context<IsNonceUsedContext>,
    params: &IsNonceUsedParams
) -> Result<bool> {
    let account_info = ctx.accounts.used_nonces.to_account_info();
    
    if account_info.data_is_empty() {
        return Ok(false);
    }

    // Load the account data directly
    let used_nonces = UsedNonces::try_deserialize(&mut &**account_info.data.borrow())?;
    require_keys_eq!(*account_info.owner, crate::ID);
    
    Ok(used_nonces.is_nonce_used(params.nonce)?)
}
