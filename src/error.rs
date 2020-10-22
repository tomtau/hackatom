use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("No data in ReceiveMsg")]
    NoData {},

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Only accepts tokens in the cw20_whitelist")]
    NotInWhitelist {},

    #[error("Clawback is not expired")]
    NotExpired {},

    #[error("Send some coins to create a clawback")]
    EmptyBalance {},

    #[error("Incoming clawback's backup and period should match the outgoing one")]
    ContractMismatch {},

    #[error("Clawback id already in use")]
    AlreadyInUse {},
}
