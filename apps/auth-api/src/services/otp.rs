use crate::errors::AppError;
use crate::services::tokens::hash_token;
use deadpool_redis::Pool;

pub async fn store(pool: &Pool, key: &str, code: &str, ttl: u64) -> Result<(), AppError> {
    let mut connection = pool
        .get()
        .await
        .map_err(|error| AppError::Redis(error.to_string()))?;
    let ttl =
        i64::try_from(ttl).map_err(|_| AppError::Validation("Invalid OTP lifetime".to_string()))?;
    let _: () = redis::pipe()
        .atomic()
        .del(key)
        .hset(key, "digest", hash_token(code))
        .hset(key, "attempts", 0)
        .expire(key, ttl)
        .query_async(&mut connection)
        .await
        .map_err(|error| AppError::Redis(error.to_string()))?;
    Ok(())
}

pub async fn consume(pool: &Pool, key: &str, code: &str) -> Result<bool, AppError> {
    let mut connection = pool
        .get()
        .await
        .map_err(|error| AppError::Redis(error.to_string()))?;
    let outcome: i64 = redis::Script::new(r#"
local expected = redis.call('HGET', KEYS[1], 'digest')
if not expected then return 0 end
local attempts = redis.call('HINCRBY', KEYS[1], 'attempts', 1)
if attempts > 5 then redis.call('DEL', KEYS[1]); return 0 end
local actual = ARGV[1]
local difference = bit.bxor(string.len(expected), string.len(actual))
for i = 1, string.len(expected) do
    difference = bit.bor(difference, bit.bxor(string.byte(expected, i), string.byte(actual, i) or 0))
end
if difference == 0 then redis.call('DEL', KEYS[1]); return 1 end
if attempts >= 5 then redis.call('DEL', KEYS[1]) end
return 0
"#).key(key).arg(hash_token(code.trim())).invoke_async(&mut connection).await
        .map_err(|error| AppError::Redis(error.to_string()))?;
    Ok(outcome == 1)
}
