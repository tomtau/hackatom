use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{
    Api, CanonicalAddr, Coin, Env, HumanAddr, Order, ReadonlyStorage, StdError, StdResult, Storage,
};
use cosmwasm_storage::{bucket, bucket_read, prefixed_read, Bucket, ReadonlyBucket};

use cw20::{Balance, Cw20Coin};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug, Default)]
pub struct GenericBalance {
    pub native: Vec<Coin>,
    pub cw20: Vec<Cw20Coin>,
}

impl GenericBalance {
    pub fn add_tokens(&mut self, add: Balance) {
        match add {
            Balance::Native(balance) => {
                for token in balance.0 {
                    let index = self.native.iter().enumerate().find_map(|(i, exist)| {
                        if exist.denom == token.denom {
                            Some(i)
                        } else {
                            None
                        }
                    });
                    match index {
                        Some(idx) => self.native[idx].amount += token.amount,
                        None => self.native.push(token),
                    }
                }
            }
            Balance::Cw20(token) => {
                let index = self.cw20.iter().enumerate().find_map(|(i, exist)| {
                    if exist.address == token.address {
                        Some(i)
                    } else {
                        None
                    }
                });
                match index {
                    Some(idx) => self.cw20[idx].amount += token.amount,
                    None => self.cw20.push(token),
                }
            }
        };
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug, Default)]
pub struct Clawback {
    /// the key that before "end_time" may transfer to Clawback
    /// (with the same "backup" and "clawback_period") or burn the tokens
    pub backup: CanonicalAddr,
    /// the receiver of tokens -- before "end_time", they may transfer only to "Clawback"
    /// with the same "backup" and "clawback_period";
    /// after "end_time", they may transfer anywhere
    pub holder: CanonicalAddr,
    /// end time (in seconds since epoch 00:00:00 UTC on 1 January 1970);
    /// when block time exceeds this value, the holder can transfer outside Clawback.
    pub end_time: u64,
    /// the duration of the clawback
    /// (end_time = block time + clawback_period)
    pub clawback_period: u64,
    /// Balance in Native and Cw20 tokens
    pub balance: GenericBalance,
    /// All possible contracts that we accept tokens from
    pub cw20_whitelist: Vec<CanonicalAddr>,
}

impl Clawback {
    pub fn is_expired(&self, env: &Env) -> bool {
        env.block.time > self.end_time
    }

    pub fn human_whitelist<A: Api>(&self, api: &A) -> StdResult<Vec<HumanAddr>> {
        self.cw20_whitelist
            .iter()
            .map(|h| api.human_address(h))
            .collect()
    }
}

pub const PREFIX_CLAWBACK: &[u8] = b"clawback";

pub fn clawbacks<S: Storage>(storage: &mut S) -> Bucket<S, Clawback> {
    bucket(storage, PREFIX_CLAWBACK)
}

pub fn clawbacks_read<S: ReadonlyStorage>(storage: &S) -> ReadonlyBucket<S, Clawback> {
    bucket_read(storage, PREFIX_CLAWBACK)
}

/// This returns the list of ids for all registered clawbacks
pub fn all_clawback_ids<S: ReadonlyStorage>(storage: &S) -> StdResult<Vec<String>> {
    prefixed_read(storage, PREFIX_CLAWBACK)
        .range(None, None, Order::Ascending)
        .map(|(k, _)| {
            String::from_utf8(k).map_err(|_| StdError::invalid_utf8("parsing clawback key"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    use cosmwasm_std::testing::MockStorage;
    use cosmwasm_std::Binary;

    #[test]
    fn no_clawback_ids() {
        let storage = MockStorage::new();
        let ids = all_clawback_ids(&storage).unwrap();
        assert_eq!(0, ids.len());
    }

    fn dummy_clawback() -> Clawback {
        Clawback {
            holder: CanonicalAddr(Binary(b"hold".to_vec())),
            backup: CanonicalAddr(Binary(b"back".to_vec())),
            ..Clawback::default()
        }
    }

    #[test]
    fn all_clawback_ids_in_order() {
        let mut storage = MockStorage::new();
        clawbacks(&mut storage)
            .save("lazy".as_bytes(), &dummy_clawback())
            .unwrap();
        clawbacks(&mut storage)
            .save("assign".as_bytes(), &dummy_clawback())
            .unwrap();
        clawbacks(&mut storage)
            .save("zen".as_bytes(), &dummy_clawback())
            .unwrap();

        let ids = all_clawback_ids(&storage).unwrap();
        assert_eq!(3, ids.len());
        assert_eq!(
            vec!["assign".to_string(), "lazy".to_string(), "zen".to_string()],
            ids
        )
    }
}
