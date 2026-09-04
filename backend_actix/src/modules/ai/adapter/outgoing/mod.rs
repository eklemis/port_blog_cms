//! The Redis counter behind the quota.

mod usage_counter_redis;

pub use usage_counter_redis::RedisUsageCounter;
