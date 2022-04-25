use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Uint128, Uint256};
use cw_storage_plus::{Item, Map, U64Key};

pub const STATE: Item<State> = Item::new("state");
pub const CONFIG: Item<Config> = Item::new("config");

/// Key consists of an user address and a duration
pub const LOCKUP_INFO: Map<(&Addr, U64Key), LockupInfo> = Map::new("user_lockup_position");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub total_locked_lp_token: Uint128,
    /// Weighted score for locked LP token balance used to calculate CTX rewards a particular user can claim
    pub total_weighted_score: Uint256,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    /// Account which can update the config
    pub owner: Addr,

    /// Reward Token address
    pub reward_token_addr: Addr,
    /// Lockable Token address
    pub lockable_token_addr: Addr,

    /// Timestamp when Contract will start accepting LP Token staking
    pub event_start_second: u64,
    /// Timestamp when Contract will end accepting LP Token staking
    pub event_end_second: u64,

    /// Time duration for rewards is released sequentially from the time the event ends.
    pub reward_vesting_duration: u64,

    /// Min. no. of lock duration allowed for lockup
    pub min_lock_duration: u64,
    /// Max. no. of lock duration allowed for lockup
    pub max_lock_duration: u64,
    /// Min. lock duration in seconds.  
    pub lock_duration_second: u64,

    /// Lockdrop Reward multiplier
    pub weekly_multiplier: u64,
    /// Lockdrop reward divider
    pub weekly_divider: u64,

    /// Total lockdrop rewards to be distributed among the users
    pub lockdrop_reward: Uint128,

    /// add total rewards   //2022-04-25
    pub total_reward: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LockupInfo {
    /// Terraswap LP tokens locked by the user
    pub locked_lp_token: Uint128,
    /// total_weighted_score of the position
    pub weighted_score: Uint256,
    /// CTX tokens received as rewards for participation in the lockdrop
    pub claimed: Uint128,
    /// Timestamp beyond which this position can be unlocked
    pub unlock_timestamp: u64,
}
