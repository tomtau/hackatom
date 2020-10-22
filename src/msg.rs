use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Api, CanonicalAddr, Coin, HumanAddr, StdResult};

use cw20::{Balance, Cw20CoinHuman, Cw20ReceiveMsg};

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct InitMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    Create(CreateMsg),
    /// Adds all sent native tokens to the contract
    TopUp {
        id: String,
    },
    /// Sends all tokens to the holder (after end time).
    Withdraw {
        /// id is a human-readable name for the clawback from create
        id: String,
    },
    /// Updates the end time with the extra clawback_period
    Refresh {
        /// id is a human-readable name for the clawback from create
        id: String,
    },
    /// Destroys the tokens
    Burn {
        /// id is a human-readable name for the clawback from create
        id: String,
    },
    /// Transfer is only allowed between the clawbacks with the same
    /// "backup", "clawback_period" and "cw20_whitelist"
    ClawbackTransfer {
        /// id is a human-readable name for the clawback from create
        from_id: String,
        /// id is a human-readable name for the clawback from create
        to_id: String,
        /// the amount of the token(s) to transfer
        amount: Balance,
    },
    /// This accepts a properly-encoded ReceiveMsg from a cw20 contract
    Receive(Cw20ReceiveMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReceiveMsg {
    Create(CreateMsg),
    /// Adds all sent native tokens to the contract
    TopUp {
        id: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CreateMsg {
    /// id is a human-readable name for the clawback to use later
    /// 3-20 bytes of utf-8 text
    pub id: String,
    /// the key that before "end_time" may transfer to Clawback
    /// (with the same "backup" and "clawback_period") or burn the tokens
    pub backup: HumanAddr,
    /// the receiver of tokens -- before "end_time", they may transfer only to "Clawback"
    /// with the same "backup" and "clawback_period";
    /// after "end_time", they may transfer anywhere
    pub holder: HumanAddr,
    /// the duration of the clawback
    /// (end_time = block time + clawback_period)
    pub clawback_period: u64,
    /// Besides any possible tokens sent with the CreateMsg, this is a list of all cw20 token addresses
    /// that are accepted by the clawback during a top-up. This is required to avoid a DoS attack by topping-up
    /// with an invalid cw20 contract. See https://github.com/CosmWasm/cosmwasm-plus/issues/19
    pub cw20_whitelist: Option<Vec<HumanAddr>>,
}

impl CreateMsg {
    pub fn canonical_whitelist<A: Api>(&self, api: &A) -> StdResult<Vec<CanonicalAddr>> {
        match self.cw20_whitelist.as_ref() {
            Some(v) => v.iter().map(|h| api.canonical_address(h)).collect(),
            None => Ok(vec![]),
        }
    }
}

pub fn is_valid_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    if bytes.len() < 3 || bytes.len() > 20 {
        return false;
    }
    true
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Show all open clawbacks. Return type is ListResponse.
    List {},
    /// Returns the details of the named clawback, error if not created
    /// Return type: DetailsResponse.
    Details { id: String },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ListResponse {
    /// list all registered ids
    pub clawbacks: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct DetailsResponse {
    /// id of this clawback
    pub id: String,
    /// the key that before "end_time" may transfer to Clawback
    /// (with the same "backup" and "clawback_period") or burn the tokens
    pub backup: HumanAddr,
    /// the receiver of tokens -- before "end_time", they may transfer only to "Clawback"
    /// with the same "backup" and "clawback_period";
    /// after "end_time", they may transfer anywhere
    pub holder: HumanAddr,
    /// end time (in seconds since epoch 00:00:00 UTC on 1 January 1970);
    /// when block time exceeds this value, the holder can transfer outside Clawback.
    pub end_time: u64,
    /// the duration of the clawback
    /// (end_time = block time + clawback_period)
    pub clawback_period: u64,
    /// Balance in native tokens
    pub native_balance: Vec<Coin>,
    /// Balance in cw20 tokens
    pub cw20_balance: Vec<Cw20CoinHuman>,
    /// Whitelisted cw20 tokens
    pub cw20_whitelist: Vec<HumanAddr>,
}
