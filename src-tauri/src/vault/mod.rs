//! Master-password "vault" (P3-T07).
//!
//! Scope of the current implementation:
//!   - Hash the master password with Argon2id and store the encoded hash in
//!     the existing `settings` KV table (key `master_password_hash`).
//!   - Provide set / change / disable / verify operations.
//!
//! Note on disk encryption: the OS keychain already encrypts every saved
//! password (P3-T06). The master password adds an *application-level* lock
//! preventing UI access. A future iteration may derive a DEK from the
//! master password and re-wrap keychain values for end-to-end protection.
//!
//! Choice of parameters: we use `Argon2::default()` which is the OWASP
//! recommended baseline (m=19 MiB, t=2, p=1). On modern desktop hardware a
//! single verify takes ~50 ms — fast enough for an unlock prompt but slow
//! enough to deter offline brute force on the dumped hash.

use anyhow::{Context, Result};
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use serde_json::Value;

use crate::store::{settings as settings_dao, DbPool};

/// Settings key storing the encoded Argon2id PHC string.
const HASH_KEY: &str = "master_password_hash";

/// PIN length bounds. 4 minimum is the minimum considered sane; 12 max keeps
/// the input dialog readable. The KDF (Argon2id ~50 ms verify) raises the
/// brute-force cost above what a 4-digit PIN's entropy would suggest, but a
/// 6+ digit PIN is still strongly recommended.
const PIN_MIN_LEN: usize = 4;
const PIN_MAX_LEN: usize = 12;

/// Validate that `pin` is a digits-only string of the right length. Returns
/// a clean error message that the front surfaces in the dialog.
fn validate_pin(pin: &str) -> Result<()> {
    let len = pin.chars().count();
    if !(PIN_MIN_LEN..=PIN_MAX_LEN).contains(&len) {
        return Err(anyhow::anyhow!(
            "le PIN doit faire entre {PIN_MIN_LEN} et {PIN_MAX_LEN} chiffres"
        ));
    }
    if !pin.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow::anyhow!("le PIN ne doit contenir que des chiffres"));
    }
    Ok(())
}

/// Hash `password` with a fresh random salt and return the PHC-encoded string.
fn hash(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("argon2 hash: {e}"))?;
    Ok(hash.to_string())
}

/// Verify `password` against an Argon2id encoded hash.
fn verify(password: &str, encoded: &str) -> Result<bool> {
    let parsed = PasswordHash::new(encoded).map_err(|e| anyhow::anyhow!("parse hash: {e}"))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

/// Return the stored encoded hash, or `None` if no master password is set.
async fn stored_hash(pool: &DbPool) -> Result<Option<String>> {
    let val = settings_dao::get(pool, HASH_KEY)
        .await
        .context("read hash")?;
    Ok(val.and_then(|v| v.as_str().map(|s| s.to_string())))
}

/// `true` when a master password is currently configured.
pub async fn has_pin(pool: &DbPool) -> Result<bool> {
    Ok(stored_hash(pool).await?.is_some())
}

/// Verify `password` against the stored hash. Returns `Ok(false)` both when
/// the password is wrong and when no master password is configured — the
/// front uses [`has_pin`] beforehand to disambiguate.
pub async fn verify_pin(pool: &DbPool, password: &str) -> Result<bool> {
    let Some(stored) = stored_hash(pool).await? else {
        return Ok(false);
    };
    verify(password, &stored)
}

/// Set the PIN for the first time. Refuses to overwrite an existing
/// hash — call [`change_pin`] for that.
pub async fn set_pin(pool: &DbPool, new_pin: &str) -> Result<()> {
    if has_pin(pool).await? {
        return Err(anyhow::anyhow!(
            "un PIN est déjà défini — utiliser change_pin"
        ));
    }
    validate_pin(new_pin)?;
    let encoded = hash(new_pin)?;
    settings_dao::set(pool, HASH_KEY, &Value::String(encoded)).await?;
    Ok(())
}

/// Rotate the PIN. Verifies the current one before storing the new.
pub async fn change_pin(pool: &DbPool, current_pin: &str, new_pin: &str) -> Result<()> {
    if !verify_pin(pool, current_pin).await? {
        return Err(anyhow::anyhow!("PIN actuel incorrect"));
    }
    validate_pin(new_pin)?;
    let encoded = hash(new_pin)?;
    settings_dao::set(pool, HASH_KEY, &Value::String(encoded)).await?;
    Ok(())
}

/// Disable the PIN. Requires verifying the current one.
pub async fn disable_pin(pool: &DbPool, current_pin: &str) -> Result<()> {
    if !verify_pin(pool, current_pin).await? {
        return Err(anyhow::anyhow!("PIN actuel incorrect"));
    }
    settings_dao::set(pool, HASH_KEY, &Value::Null).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::init_pool;

    async fn fresh_pool() -> DbPool {
        let tmp = tempfile::Builder::new()
            .suffix(".sqlite")
            .tempfile()
            .expect("tmp");
        let path = tmp.path().to_path_buf();
        drop(tmp);
        init_pool(&path).await.expect("pool")
    }

    #[tokio::test]
    async fn happy_path() {
        let pool = fresh_pool().await;
        assert!(!has_pin(&pool).await.unwrap());
        assert!(!verify_pin(&pool, "1234").await.unwrap());

        set_pin(&pool, "123456").await.unwrap();
        assert!(has_pin(&pool).await.unwrap());
        assert!(verify_pin(&pool, "123456").await.unwrap());
        assert!(!verify_pin(&pool, "999999").await.unwrap());
    }

    #[tokio::test]
    async fn cant_set_twice() {
        let pool = fresh_pool().await;
        set_pin(&pool, "1111").await.unwrap();
        assert!(set_pin(&pool, "2222").await.is_err());
    }

    #[tokio::test]
    async fn change_requires_current() {
        let pool = fresh_pool().await;
        set_pin(&pool, "1234").await.unwrap();
        assert!(change_pin(&pool, "9999", "5678").await.is_err());
        change_pin(&pool, "1234", "5678").await.unwrap();
        assert!(!verify_pin(&pool, "1234").await.unwrap());
        assert!(verify_pin(&pool, "5678").await.unwrap());
    }

    #[tokio::test]
    async fn disable_clears_hash() {
        let pool = fresh_pool().await;
        set_pin(&pool, "1234").await.unwrap();
        assert!(disable_pin(&pool, "9999").await.is_err());
        disable_pin(&pool, "1234").await.unwrap();
        assert!(!has_pin(&pool).await.unwrap());
    }

    #[tokio::test]
    async fn rejects_non_numeric_or_wrong_length() {
        let pool = fresh_pool().await;
        // Too short
        assert!(set_pin(&pool, "123").await.is_err());
        // Too long
        assert!(set_pin(&pool, "1234567890123").await.is_err());
        // Letters
        assert!(set_pin(&pool, "12a4").await.is_err());
        // OK
        set_pin(&pool, "1234").await.unwrap();
    }
}
