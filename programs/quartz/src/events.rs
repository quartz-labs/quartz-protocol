use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct CommonFields {
    pub slot: u64,
    pub unix_timestamp: i64,
    pub user: Pubkey,
}

impl CommonFields {
    pub fn new(clock: &Clock, user: Pubkey) -> Self {
        Self {
            slot: clock.slot,
            unix_timestamp: clock.unix_timestamp,
            user
        }
    }
}

#[event]
pub struct SpendLimitUpdatedEvent {
    pub common_fields: CommonFields,
    pub spend_limit_per_transaction: u64,
    pub spend_limit_per_timeframe: u64,
    pub remaining_spend_limit_per_timeframe: u64,
    pub next_timeframe_reset_timestamp: u64,
    pub timeframe_in_seconds: u64
}
