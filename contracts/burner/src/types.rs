use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrationReq {
    pub asset: String,
    pub amount: Option<String>,
    pub nft_id: Option<String>,
    pub to: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Copy)]
#[serde(rename_all = "snake_case")]
pub enum TokenType {
    Cw20,
    Cw721,
}

impl fmt::Display for TokenType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TokenType::Cw20 => write!(f, "cw20"),
            TokenType::Cw721 => write!(f, "cw721"),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Created,
    Swapped,
    PaidBack,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TokenInfo {
    pub addr: String,
    pub token_type: TokenType,
}

pub enum TxResultStatusCode {
    Success = 0,
}

#[cfg(test)]
mod tests {
    use crate::types::TokenType;
    #[test]
    fn test_display() {
        assert_eq!(format!("{}", TokenType::Cw20), "cw20");
        assert_eq!(format!("{}", TokenType::Cw721), "cw721");
    }
}
