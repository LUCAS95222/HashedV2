use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Status: 400, msg: {msg:?}")]
    BadRequest { msg: String },

    #[error("Status: 501, msg: {msg:?}")]
    NotImplemented { msg: String },

    #[error("Custom Error val: {val:?}")]
    CustomError { val: String },
}
