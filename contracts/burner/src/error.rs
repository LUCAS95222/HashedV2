use cosmwasm_std::{Response, StdError};
use thiserror::Error;

pub type ContractResult = core::result::Result<Response, ContractError>;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Status: 400, message: {message:?}")]
    BadRequest { message: String },

    #[error("Status: 500, message: {message:?}")]
    InternalServerError { message: String },

    #[error("Status: {status:?}, message {message:?}")]
    CustomError { status: u16, message: String },
}
