use cosmwasm_std::{Addr, Uint128, Uint256};
use cw20::Cw20ReceiveMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    /// Reward Token address
    pub reward_token_addr: String,
    /// Lockable Token address
    pub lockable_token_addr: String,
    /// Timestamp when Contract will start accepting LP Token deposits
    pub init_timestamp: u64,
    /// Event duration in seconds
    pub event_duration: u32,
    /// Time duration for rewards is released sequentially from the time the event ends.
    pub reward_vesting_duration: u64,
    /// Min. no. of weeks allowed for lockup
    pub min_lock_duration: u16,
    /// Max. no. of weeks allowed for lockup
    pub max_lock_duration: u16,
    /// Min. lock period in seconds.
    pub lock_duration_second: Option<u32>,
    /// Lockdrop reward multiplier
    pub weekly_multiplier: u16,
    /// Lockdrop reward divider
    pub weekly_divider: u64,
    /// add total reward //2022-04-25
    pub total_reward: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),
    Claim { duration: u64 },
    Unlock { duration: u64 },
    UpdateReward { total_reward: Uint128 },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    /// Open a new user position or add to an existing position (Cw20ReceiveMsg)
    IncreaseLockup {
        duration: u64,
    },
    IncreaseReward {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    // LockdropInfo returns information of lockdrop event
    LockdropInfo {},
    // UserInfo returns the users lockdrop info as a json-encoded number
    UserInfo { addr: String },
    // Estimate returns the estimated rewards using provided amount and duration
    Estimate { amount: Uint128, duration: u64 },
}

/// We currently take no arguments for migrations
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LockdropInfoResponse {
    /// CTX Token address
    pub ctx_token_addr: Addr,
    /// CTX-UST Lp Token address
    pub lp_token_addr: Addr,

    /// Timestamp when Contract will start accepting LP Token staking
    pub event_start_second: u64,
    /// Timestamp when Contract will end accepting LP Token staking
    pub event_end_second: u64,

    /// Time duration for rewards is released sequentially from the time the event ends.
    pub reward_vesting_duration: u64,

    /// Min. no. of weeks allowed for lockup
    pub min_lock_duration: u64,
    /// Max. no. of weeks allowed for lockup
    pub max_lock_duration: u64,

    pub total_locked_lp_token: Uint128,
    /// Total lockdrop rewards to be distributed among the users
    pub total_lockdrop_reward: Uint128,
    /// Weighted score for locked LP token balance used to calculate CTX rewards a particular user can claim
    pub total_weighted_score: Uint256,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UserInfoResponse {
    /// Terraswap LP token
    pub lp_token_addr: Addr,
    /// Total CTX tokens user received as rewards for participation in the lockdrop
    pub lockdrop_total_locked_lp_token: Uint128,
    /// Total lockdrop rewards to be distributed among the users
    pub lockdrop_total_reward: Uint128,
    /// Total user's locked terraswap lp tokens
    pub user_total_locked_lp_token: Uint128,
    /// Total user's lockdrop rewards to be distributed
    pub user_total_reward: Uint128,
    /// Lockup positions
    pub lockup_infos: Vec<LockUpInfoResponse>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LockUpInfoResponse {
    pub duration: u64,
    /// Timestamp in second beyond which this position can be unlocked
    pub unlock_second: u64,
    /// Terraswap LP token locked by the user
    pub locked_lp_token: Uint128,
    /// total_weighted_score of the position
    pub weighted_score: Uint256,
    /// CTX tokens estimated|allocated as rewards for participation in the lockdrop
    pub total_reward: Uint128,
    /// CTX tokens that can be vested
    pub claimable: Uint128,
    /// CTX tokens received as rewards for participation in the lockdrop
    pub claimed: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct EstimatedRewardResponse {
    /// Total lockdrop rewards to be distributed among the users
    pub lockdrop_total_reward: Uint128,
    /// Estimated lockdrop rewards to be distributed to the user
    pub estimated_reward: Uint128,
}
