use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use std::vec;

use cosmwasm_std::{Addr, BlockInfo, CustomMsg, StdResult, Storage, Uint128};

use cw721::{
    Bid, ContractInfoResponse, Cw721, Expiration, LongTermRental, Rental, Sell, ShortTermRental,
};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, MultiIndex};

pub const BRIDGE_WALLET: &str = "nibiru_bridge_address";
pub struct Cw721Contract<'a, T, C, E, Q>
where
    T: Serialize + DeserializeOwned + Clone,
    Q: CustomMsg,
    E: CustomMsg,
{
    pub contract_info: Item<'a, ContractInfoResponse>,
    pub token_count: Item<'a, u64>,
    pub fee: Item<'a, u64>,
    pub balances: Map<'a, &'a str, Uint128>,
    /// Stored as (granter, operator) giving operator full control over granter's account
    pub operators: Map<'a, (&'a String, &'a String), Expiration>,
    pub tokens: IndexedMap<'a, &'a str, TokenInfo<T>, TokenIndexes<'a, T>>,

    pub(crate) _custom_response: PhantomData<C>,
    pub(crate) _custom_query: PhantomData<Q>,
    pub(crate) _custom_execute: PhantomData<E>,
}

// This is a signal, the implementations are in other files
impl<'a, T, C, E, Q> Cw721<T, C> for Cw721Contract<'a, T, C, E, Q>
where
    T: Serialize + DeserializeOwned + Clone,
    C: CustomMsg,
    E: CustomMsg,
    Q: CustomMsg,
{
}

impl<T, C, E, Q> Default for Cw721Contract<'static, T, C, E, Q>
where
    T: Serialize + DeserializeOwned + Clone,
    E: CustomMsg,
    Q: CustomMsg,
{
    fn default() -> Self {
        Self::new(
            "nft_info",
            "num_tokens",
            "fee",
            "balances",
            "operators",
            "tokens",
            "tokens__owner",
        )
    }
}

impl<'a, T, C, E, Q> Cw721Contract<'a, T, C, E, Q>
where
    T: Serialize + DeserializeOwned + Clone,
    E: CustomMsg,
    Q: CustomMsg,
{
    fn new(
        contract_key: &'a str,
        token_count_key: &'a str,
        fee_key: &'a str,
        balance_key: &'a str,
        operator_key: &'a str,
        tokens_key: &'a str,
        tokens_owner_key: &'a str,
    ) -> Self {
        let indexes = TokenIndexes {
            owner: MultiIndex::new(token_owner_idx, tokens_key, tokens_owner_key),
        };
        Self {
            contract_info: Item::new(contract_key),
            token_count: Item::new(token_count_key),
            fee: Item::new(fee_key),
            operators: Map::new(operator_key),
            balances: Map::new(balance_key),
            tokens: IndexedMap::new(tokens_key, indexes),
            _custom_response: PhantomData,
            _custom_execute: PhantomData,
            _custom_query: PhantomData,
        }
    }

    pub fn get_fee(&self, storage: &dyn Storage) -> StdResult<u64> {
        Ok(self.fee.may_load(storage)?.unwrap_or_default())
    }

    pub fn set_fee(&self, storage: &mut dyn Storage, fee: u64) -> StdResult<u64> {
        self.fee.save(storage, &fee)?;
        Ok(fee)
    }

    pub fn token_count(&self, storage: &dyn Storage) -> StdResult<u64> {
        Ok(self.token_count.may_load(storage)?.unwrap_or_default())
    }

    pub fn get_balance(&self, storage: &dyn Storage, denom: String) -> StdResult<Uint128> {
        Ok(self.balances.may_load(storage, &denom)?.unwrap_or_default())
    }

    pub fn increase_balance(
        &self,
        storage: &mut dyn Storage,
        denom: String,
        amount: Uint128,
    ) -> StdResult<Uint128> {
        let mut balance = self.balances.may_load(storage, &denom)?.unwrap_or_default();
        balance += amount;
        self.balances.save(storage, &denom, &balance)?;
        Ok(balance)
    }

    pub fn decrease_balance(
        &self,
        storage: &mut dyn Storage,
        denom: String,
        amount: Uint128,
    ) -> StdResult<Uint128> {
        let mut balance = self.balances.may_load(storage, &denom)?.unwrap_or_default();
        balance -= amount;
        self.balances.save(storage, &denom, &balance)?;
        Ok(balance)
    }

    // pub fn decrease_balance(
    //     &self,
    //     storage: &mut dyn Storage,
    //     denom: String,
    //     amount: Uint128,
    // ) -> StdResult<Uint128> {
    //     let balance = self.balances.may_load(storage, &denom)?.unwrap_or_default();
    //     let new_balance = balance.checked_sub(amount)
    //         .map_err(|_| StdError::overflow(OverflowError::new(OverflowOperation::Sub, balance, amount)))?;
    //     self.balances.save(storage, &denom, &new_balance)?;
    //     Ok(new_balance)
    // }

    pub fn increment_tokens(&self, storage: &mut dyn Storage) -> StdResult<u64> {
        let val = self.token_count(storage)? + 1;
        self.token_count.save(storage, &val)?;
        Ok(val)
    }

    pub fn decrement_tokens(&self, storage: &mut dyn Storage) -> StdResult<u64> {
        let val = self.token_count(storage)? - 1;
        self.token_count.save(storage, &val)?;
        Ok(val)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Owner {
    pub chain_type: String,
    pub address: String,
}

impl Owner {
    pub fn validate_sender(&self, sender: &Addr) -> bool {
        match self.chain_type.as_str() {
            "nibiru" => self.address == sender.to_string(),
            _ => sender.to_string() == BRIDGE_WALLET,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TokenInfo<T> {
    /// The owner of the newly minted NFT
    pub owner: Owner,
    /// Approvals are stored here, as we clear them all upon transfer and cannot accumulate much
    pub approvals: Vec<Approval>,

    pub longterm_rental: LongTermRental,

    pub shortterm_rental: ShortTermRental,
    pub rentals: Vec<Rental>,
    pub bids: Vec<Bid>,
    pub sell: Sell,

    pub token_uri: Option<String>,

    pub extension: T,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Approval {
    /// Account that can transfer/send the token
    pub spender: Addr,
    /// When the Approval expires (maybe Expiration::never)
    pub expires: Expiration,
}

impl Approval {
    pub fn is_expired(&self, block: &BlockInfo) -> bool {
        self.expires.is_expired(block)
    }
}

pub struct TokenIndexes<'a, T>
where
    T: Serialize + DeserializeOwned + Clone,
{
    pub owner: MultiIndex<'a, String, TokenInfo<T>, String>,
}

impl<'a, T> IndexList<TokenInfo<T>> for TokenIndexes<'a, T>
where
    T: Serialize + DeserializeOwned + Clone,
{
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<TokenInfo<T>>> + '_> {
        let v: Vec<&dyn Index<TokenInfo<T>>> = vec![&self.owner];
        Box::new(v.into_iter())
    }
}

pub fn token_owner_idx<T>(_pk: &[u8], d: &TokenInfo<T>) -> String {
    return d.owner.address.clone();
}
