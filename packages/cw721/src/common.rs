use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ChainOwner {
    pub chain_type: String, // e.g. "eth", "nibiru"
    pub address: String,    // chain-specific address
}