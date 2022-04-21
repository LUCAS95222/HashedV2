use cosmwasm_std::{StdResult, Uint128};
use cw20::{Cw20ReceiveMsg, Denom};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use chrono::{NaiveDate, NaiveDateTime, Datelike, Duration};
use std::convert::TryInto;

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct InstantiateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),

    //////////////////////////
    /// Creator Operations ///
    //////////////////////////
    RegisterVestingAccount {
        master_address: Option<String>, // if given, the vesting account can be unregistered
        addresses: Vec<String>,
        vesting_key: String,
        vesting_schedule: VestingSchedule,
    },
    /// only available when master_address was set
    DeregisterVestingAccount {
        addresses: Vec<String>,
        vesting_key: String,
        vested_token_recipient: Option<String>,
        left_vesting_token_recipient: Option<String>,
    },

    /////////////////////////////////
    /// VestingAccount Operations ///
    /////////////////////////////////
    Claim {
        vesting_keys: Vec<String>,
        recipient: Option<String>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    /// Register vesting account with token transfer
    RegisterVestingAccount {
        master_address: Option<String>, // if given, the vesting account can be unregistered
        addresses: Vec<String>,
        vesting_key: String,
        vesting_schedule: VestingSchedule,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    VestingAccount {
        address: String,
        start_after: Option<Denom>,
        limit: Option<u32>,
    },
}

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Debug)]
pub struct VestingAccountResponse {
    pub address: String,
    pub vestings: Vec<VestingData>,
}

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Debug)]
pub struct VestingData {
    pub master_address: Option<String>,
    pub vesting_key: String,
    pub vesting_denom: Denom,
    pub vesting_amount: Uint128,
    pub vested_amount: Uint128,
    pub vesting_schedule: VestingSchedule,
    pub claimable_amount: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Condition {
    pub style: String,
    pub hour: Option<u32>,
    pub weekday: Option<u32>,
    pub day: Option<u32>,
    pub month: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum VestingSchedule {
    /// LinearVesting is used to vest tokens linearly during a time period.
    /// The total_amount will be vested during this period.
    LinearVesting {
        start_time: String,      // vesting start time in second unit
        end_time: String,        // vesting end time in second unit
        vesting_amount: Uint128, // total vesting amount
    },
    /// PeriodicVesting is used to vest tokens
    /// at regular intervals for a specific period.
    /// To minimize calculation error,
    /// (end_time - start_time) should be multiple of vesting_interval
    /// deposit_amount = amount * ((end_time - start_time) / vesting_interval + 1)
    PeriodicVesting {
        start_time: String,       // vesting start time in second unit
        end_time: String,         // vesting end time in second unit
        vesting_interval: String, // vesting interval in second unit
        amount: Uint128,          // the amount will be vested in a interval
    },
    /// ConditionalVesting is used to vest tokens at conditional dates.
    ConditionalVesting {
        start_time: String,       // vesting start time in second unit
        end_time: String,         // vesting end time in second unit
        amount: Uint128,          // the amount will be vested at condition
        condition: Condition,
    },
}

impl VestingSchedule {
    pub fn vested_count(&self, block_time: u64) -> Uint128 {
        match self {
            VestingSchedule::LinearVesting {
                start_time: _,
                end_time: _,
                vesting_amount: _,
            } => {
                return Uint128::zero();
            }
            VestingSchedule::PeriodicVesting {
                start_time,
                end_time,
                vesting_interval,
                amount: _,
            } => {
                let start_time = start_time.parse::<u64>().unwrap();
                let end_time = end_time.parse::<u64>().unwrap();
                let vesting_interval = vesting_interval.parse::<u64>().unwrap();

                if block_time < start_time {
                    return Uint128::zero();
                }

                if block_time >= end_time {
                    let num_interval = 1 + (end_time - start_time) / vesting_interval;
                    return Uint128::from(num_interval);
                }

                let passed_interval = 1 + (block_time - start_time) / vesting_interval;
                return Uint128::from(passed_interval);
            }
            VestingSchedule::ConditionalVesting {
                start_time,
                end_time,
                amount: _,
                condition,
            } => {
                let start_time = start_time.parse::<u64>().unwrap();
                let end_time = end_time.parse::<u64>().unwrap();

                if block_time < start_time {
                    return Uint128::zero();
                }

                let c_hour:u32;
                match condition.hour {
                    None => c_hour = 0,
                    Some(i) => c_hour = i,
                }
                let c_wday:u32;
                match condition.weekday {
                    None => c_wday = 0,
                    Some(i) => c_wday = i,
                }
                let c_day:u32;
                match condition.day {
                    None => c_day = 1,
                    Some(i) => c_day = i,
                }
                let c_month:u32;
                match condition.month {
                    None => c_month = 1,
                    Some(i) => c_month = i,
                }

                let mut passed = 0u32;
                let start_dt = NaiveDateTime::from_timestamp(start_time.try_into().unwrap(), 0);
                let end_dt = if block_time > end_time {
                    NaiveDateTime::from_timestamp(end_time.try_into().unwrap(), 0)
                } else {
                    NaiveDateTime::from_timestamp(block_time.try_into().unwrap(), 0)
                };
                let mut cur_year = start_dt.year();
                let mut cur_month = start_dt.month();
                let cur_wday = start_dt.weekday().num_days_from_sunday();
                let cur_day = start_dt.day();
                let mut cur_dt:NaiveDateTime;

                match condition.style.as_str() {
                    "daily" => {
                        cur_dt = NaiveDate::from_ymd(cur_year, cur_month, cur_day).and_hms(c_hour, 0, 0);
                        let one_day = Duration::days(1);
                        while cur_dt < start_dt {
                            cur_dt += one_day;
                        }
                        while cur_dt <= end_dt {
                            passed += 1;
                            cur_dt += one_day;
                        }
                    },
                    "weekly" => {
                        cur_dt = NaiveDate::from_ymd(cur_year, cur_month, cur_day).and_hms(c_hour, 0, 0);
                        if c_wday > cur_wday {
                            cur_dt += Duration::days((c_wday - cur_wday).into());
                        } else {
                            cur_dt -= Duration::days((cur_wday - c_wday).into());
                        }
                        let one_week = Duration::days(7);
                        while cur_dt < start_dt {
                            cur_dt += one_week;
                        }
                        while cur_dt <= end_dt {
                            passed += 1;
                            cur_dt += one_week;
                        }
                    },
                    "monthly" => {
                        cur_dt = NaiveDate::from_ymd(cur_year, cur_month, c_day).and_hms(c_hour, 0, 0);
                        while cur_dt < start_dt {
                            cur_month += 1;
                            if cur_month > 12 {
                                cur_month = 1;
                                cur_year += 1;
                            }
                            cur_dt = NaiveDate::from_ymd(cur_year, cur_month, c_day).and_hms(c_hour, 0, 0);
                        }
                        while cur_dt <= end_dt {
                            passed += 1;
                            cur_month += 1;
                            if cur_month > 12 {
                                cur_month = 1;
                                cur_year += 1;
                            }
                            cur_dt = NaiveDate::from_ymd(cur_year, cur_month, c_day).and_hms(c_hour, 0, 0);
                        }
                    },
                    "yearly" => {
                        cur_dt = NaiveDate::from_ymd(cur_year, c_month, c_day).and_hms(c_hour, 0, 0);
                        while cur_dt < start_dt {
                            cur_year += 1;
                            cur_dt = NaiveDate::from_ymd(cur_year, c_month, c_day).and_hms(c_hour, 0, 0);
                        }
                        while cur_dt <= end_dt {
                            passed += 1;
                            cur_year += 1;
                            cur_dt = NaiveDate::from_ymd(cur_year, c_month, c_day).and_hms(c_hour, 0, 0);
                        }
                    },
                    _ => {
                        return Uint128::zero();
                    },
                }

                return Uint128::from(passed);
            }
        }
    }
    pub fn vested_amount(&self, block_time: u64) -> StdResult<Uint128> {
        match self {
            VestingSchedule::LinearVesting {
                start_time,
                end_time,
                vesting_amount,
            } => {
                let start_time = start_time.parse::<u64>().unwrap();
                let end_time = end_time.parse::<u64>().unwrap();

                if block_time <= start_time {
                    return Ok(Uint128::zero());
                }

                if block_time >= end_time {
                    return Ok(*vesting_amount);
                }

                let vested_token = vesting_amount
                    .checked_mul(Uint128::from(block_time - start_time))?
                    .checked_div(Uint128::from(end_time - start_time))?;

                Ok(vested_token)
            }
            VestingSchedule::PeriodicVesting {
                start_time: _,
                end_time: _,
                vesting_interval: _,
                amount,
            } => {
                let passed_count = self.vested_count(block_time);

                if passed_count.is_zero() {
                    return Ok(Uint128::zero());
                }
                Ok(amount.checked_mul(passed_count)?)
            }
            VestingSchedule::ConditionalVesting {
                start_time: _,
                end_time: _,
                amount,
                condition: _,
            } => {
                let passed_count = self.vested_count(block_time);

                if passed_count.is_zero() {
                    return Ok(Uint128::zero());
                }
                Ok(amount.checked_mul(passed_count)?)
            }
        }
    }
}

#[test]
fn linear_vesting_vested_amount() {
    let schedule = VestingSchedule::LinearVesting {
        start_time: "100".to_string(),
        end_time: "110".to_string(),
        vesting_amount: Uint128::new(1000000u128),
    };

    assert_eq!(schedule.vested_amount(100).unwrap(), Uint128::zero());
    assert_eq!(
        schedule.vested_amount(105).unwrap(),
        Uint128::new(500000u128)
    );
    assert_eq!(
        schedule.vested_amount(110).unwrap(),
        Uint128::new(1000000u128)
    );
    assert_eq!(
        schedule.vested_amount(115).unwrap(),
        Uint128::new(1000000u128)
    );
}

#[test]
fn periodic_vesting_vested_amount() {
    let schedule = VestingSchedule::PeriodicVesting {
        start_time: "105".to_string(),
        end_time: "110".to_string(),
        vesting_interval: "5".to_string(),
        amount: Uint128::new(500000u128),
    };

    assert_eq!(schedule.vested_amount(100).unwrap(), Uint128::zero());
    assert_eq!(
        schedule.vested_amount(105).unwrap(),
        Uint128::new(500000u128)
    );
    assert_eq!(
        schedule.vested_amount(110).unwrap(),
        Uint128::new(1000000u128)
    );
    assert_eq!(
        schedule.vested_amount(115).unwrap(),
        Uint128::new(1000000u128)
    );
}

#[test]
fn conditional_vesting_vested_amount() {
    let schedule = VestingSchedule::ConditionalVesting {
        start_time: "105".to_string(),
        end_time: "172900".to_string(),
        amount: Uint128::new(500000u128),
        condition: Condition {
            style: "daily".to_string(),
            hour: Some(10),
            weekday: None,
            day: None,
            month: None,
        }
    };
    assert_eq!(schedule.vested_amount(100).unwrap(), Uint128::zero());
    // 1970-01-01 09:59:00 = zero
    assert_eq!( 
        schedule.vested_amount(35940).unwrap(),
        Uint128::zero()
    );
    // 1970-01-01 10:00:01 = one time
    assert_eq!( 
        schedule.vested_amount(36001).unwrap(),
        Uint128::new(500000u128)
    );
    // 1970-01-02 09:59:59 = one time
    assert_eq!( 
        schedule.vested_amount(122399).unwrap(),
        Uint128::new(500000u128)
    );
    // 1970-01-02 10:00:01 = two times
    assert_eq!(
        schedule.vested_amount(122401).unwrap(),
        Uint128::new(1000000u128)
    );

    let schedule = VestingSchedule::ConditionalVesting {
        start_time: "105".to_string(),
        end_time: "1814400".to_string(),
        amount: Uint128::new(500000u128),
        condition: Condition {
            style: "weekly".to_string(),
            hour: Some(0),
            weekday: Some(3), // Wed
            day: None,
            month: None,
        }
    };
    assert_eq!(schedule.vested_amount(100).unwrap(), Uint128::zero());
    // 1970-01-06 00:00:00 = Tue = zero
    assert_eq!( 
        schedule.vested_amount(432000).unwrap(),
        Uint128::zero()
    );
    // 1970-01-07 00:00:00 = Wed = one time
    assert_eq!( 
        schedule.vested_amount(518400).unwrap(),
        Uint128::new(500000u128)
    );
    // 1970-01-11 00:00:00 = Sun = one time
    assert_eq!( 
        schedule.vested_amount(864000).unwrap(),
        Uint128::new(500000u128)
    );
    // 1970-01-15 00:00:00 = Thu = two times
    assert_eq!(
        schedule.vested_amount(1209600).unwrap(),
        Uint128::new(1000000u128)
    );
    // 1970-01-22 00:00:01 = all times
    assert_eq!(
        schedule.vested_amount(1814401).unwrap(),
        Uint128::new(1500000u128)
    );
}
