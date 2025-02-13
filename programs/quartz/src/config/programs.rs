use anchor_lang::prelude::*;

#[derive(Clone)]
pub struct AddressLookupTableProgram;

impl anchor_lang::Id for AddressLookupTableProgram {
    fn id() -> Pubkey {
        "AddressLookupTab1e1111111111111111111111111".parse().unwrap()
    }
}