use crate::config::Config;
use argon2::password_hash::{
    rand_core::{OsRng, RngCore},
    PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
};
use argon2::Argon2;
use chrono::{Duration, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum UserError {
    #[error("user not found")]
    NotFound,
    #[error("username already exists")]
    Duplicate,
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("user disabled")]
    Disabled,
    #[error("forbidden")]
    Forbidden,
    #[error("{0}")]
    Other(String),
    #[error("db error: {0}")]
    Db(#[from] rusqlite::Error),
}

#[derive(Debug, Clone, Serialize)]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    pub role: String,
    pub disabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    #[serde(default = "default_role")]
    pub role: String,
}

fn default_role() -> String {
    "operator".into()
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub expires_at: String,
    pub user: UserInfo,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChangePasswordRequest {
    pub old_password: String,
    pub new_password: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SetDisabledRequest {
    pub disabled: bool,
}

#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user_id: Option<String>,
    pub username: String,
    pub role: String,
    pub is_service: bool,
    /// Present for session logins; used by logout.
    pub session_token: Option<String>,
}

impl AuthContext {
    pub fn is_admin(&self) -> bool {
        self.is_service || self.role == "admin"
    }
}

pub struct UserStore {
    #[allow(dead_code)]
    db_path: PathBuf,
    conn: Mutex<Connection>,
    session_ttl_hours: i64,
}

impl UserStore {
    pub fn open(db_path: &Path, config: &Config) -> anyhow::Result<Self> {
        let conn = Connection::open(db_path)?;
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY NOT NULL,
                username TEXT NOT NULL UNIQUE,
                password_hash TEXT NOT NULL,
                role TEXT NOT NULL DEFAULT 'admin',
                disabled INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS sessions (
                token TEXT PRIMARY KEY NOT NULL,
                user_id TEXT NOT NULL,
                expires_at TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS user_prefs (
                user_id TEXT NOT NULL,
                pref_key TEXT NOT NULL,
                value TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                PRIMARY KEY (user_id, pref_key)
            );
            ",
        )?;

        let store = Self {
            db_path: db_path.to_path_buf(),
            conn: Mutex::new(conn),
            session_ttl_hours: config.session_ttl_hours as i64,
        };
        store.ensure_seed_admin(config)?;
        Ok(store)
    }

    fn with_conn<T>(
        &self,
        f: impl FnOnce(&Connection) -> Result<T, UserError>,
    ) -> Result<T, UserError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| UserError::Other("db lock poisoned".into()))?;
        f(&conn)
    }

    fn hash_password(password: &str) -> Result<String, UserError> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        argon2
            .hash_password(password.as_bytes(), &salt)
            .map(|h| h.to_string())
            .map_err(|e| UserError::Other(e.to_string()))
    }

    fn verify_password(password: &str, hash: &str) -> Result<bool, UserError> {
        let parsed =
            PasswordHash::new(hash).map_err(|e| UserError::Other(e.to_string()))?;
        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok())
    }

    fn ensure_seed_admin(&self, config: &Config) -> anyhow::Result<()> {
        let count: i64 = self
            .with_conn(|conn| {
                conn.query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0))
                    .map_err(UserError::from)
            })
            .map_err(|e| anyhow::anyhow!(e))?;
        if count > 0 {
            return Ok(());
        }
        let username = config.seed_username.trim();
        let password = config.seed_password.trim();
        if username.is_empty() || password.is_empty() {
            anyhow::bail!("seed_username / seed_password required when no users exist");
        }
        self.create_user(CreateUserRequest {
            username: username.to_string(),
            password: password.to_string(),
            role: "admin".into(),
        })
        .map_err(|e| anyhow::anyhow!(e))?;
        tracing::info!(username = %username, "seed admin user created");
        Ok(())
    }

    fn row_to_user(row: &rusqlite::Row<'_>) -> rusqlite::Result<UserInfo> {
        Ok(UserInfo {
            id: row.get(0)?,
            username: row.get(1)?,
            role: row.get(2)?,
            disabled: row.get::<_, i64>(3)? != 0,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
        })
    }

    pub fn list_users(&self) -> Result<Vec<UserInfo>, UserError> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, username, role, disabled, created_at, updated_at
                 FROM users ORDER BY username ASC",
            )?;
            let rows = stmt.query_map([], Self::row_to_user)?;
            let mut out = Vec::new();
            for r in rows {
                out.push(r?);
            }
            Ok(out)
        })
    }

    pub fn get_user(&self, id: &str) -> Result<UserInfo, UserError> {
        self.with_conn(|conn| {
            conn.query_row(
                "SELECT id, username, role, disabled, created_at, updated_at
                 FROM users WHERE id = ?1",
                params![id],
                Self::row_to_user,
            )
            .optional()?
            .ok_or(UserError::NotFound)
        })
    }

    pub fn create_user(&self, req: CreateUserRequest) -> Result<UserInfo, UserError> {
        let username = req.username.trim().to_string();
        if username.is_empty() {
            return Err(UserError::Other("username is required".into()));
        }
        if req.password.len() < 6 {
            return Err(UserError::Other("password must be at least 6 characters".into()));
        }
        let role = match req.role.trim() {
            "admin" | "operator" => req.role.trim().to_string(),
            _ => return Err(UserError::Other("role must be admin or operator".into())),
        };
        let now = Utc::now().to_rfc3339();
        let id = Uuid::new_v4().to_string();
        let hash = Self::hash_password(&req.password)?;
        self.with_conn(|conn| {
            match conn.execute(
                "INSERT INTO users (id, username, password_hash, role, disabled, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, 0, ?5, ?6)",
                params![id, username, hash, role, now, now],
            ) {
                Ok(_) => Ok(()),
                Err(rusqlite::Error::SqliteFailure(err, _))
                    if err.code == rusqlite::ErrorCode::ConstraintViolation =>
                {
                    Err(UserError::Duplicate)
                }
                Err(e) => Err(UserError::Db(e)),
            }
        })?;
        self.get_user(&id)
    }

    pub fn set_disabled(&self, id: &str, disabled: bool) -> Result<UserInfo, UserError> {
        let now = Utc::now().to_rfc3339();
        self.with_conn(|conn| {
            let n = conn.execute(
                "UPDATE users SET disabled = ?1, updated_at = ?2 WHERE id = ?3",
                params![if disabled { 1 } else { 0 }, now, id],
            )?;
            if n == 0 {
                return Err(UserError::NotFound);
            }
            Ok(())
        })?;
        if disabled {
            let _ = self.revoke_user_sessions(id);
        }
        self.get_user(id)
    }

    pub fn login(&self, req: LoginRequest) -> Result<LoginResponse, UserError> {
        let username = req.username.trim().to_string();
        let (id, hash, _role, disabled): (String, String, String, i64) = self.with_conn(|conn| {
            conn.query_row(
                "SELECT id, password_hash, role, disabled FROM users WHERE username = ?1",
                params![username],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .optional()?
            .ok_or(UserError::InvalidCredentials)
        })?;
        if disabled != 0 {
            return Err(UserError::Disabled);
        }
        if !Self::verify_password(&req.password, &hash)? {
            return Err(UserError::InvalidCredentials);
        }
        let token = random_token();
        let now = Utc::now();
        let expires = now + Duration::hours(self.session_ttl_hours.max(1));
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO sessions (token, user_id, expires_at, created_at) VALUES (?1, ?2, ?3, ?4)",
                params![
                    token,
                    id,
                    expires.to_rfc3339(),
                    now.to_rfc3339()
                ],
            )?;
            Ok(())
        })?;
        let user = self.get_user(&id)?;
        Ok(LoginResponse {
            token,
            expires_at: expires.to_rfc3339(),
            user,
        })
    }

    pub fn logout(&self, token: &str) -> Result<(), UserError> {
        self.with_conn(|conn| {
            conn.execute("DELETE FROM sessions WHERE token = ?1", params![token])?;
            Ok(())
        })
    }

    pub fn revoke_user_sessions(&self, user_id: &str) -> Result<(), UserError> {
        self.with_conn(|conn| {
            conn.execute("DELETE FROM sessions WHERE user_id = ?1", params![user_id])?;
            Ok(())
        })
    }

    pub fn resolve_session(&self, token: &str) -> Result<AuthContext, UserError> {
        let now = Utc::now().to_rfc3339();
        let (user_id, expires_at): (String, String) = self.with_conn(|conn| {
            conn.query_row(
                "SELECT user_id, expires_at FROM sessions WHERE token = ?1",
                params![token],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?
            .ok_or(UserError::InvalidCredentials)
        })?;
        if expires_at < now {
            let _ = self.logout(token);
            return Err(UserError::InvalidCredentials);
        }
        let user = self.get_user(&user_id)?;
        if user.disabled {
            return Err(UserError::Disabled);
        }
        Ok(AuthContext {
            user_id: Some(user.id),
            username: user.username,
            role: user.role,
            is_service: false,
            session_token: Some(token.to_string()),
        })
    }

    pub fn change_password(
        &self,
        user_id: &str,
        req: ChangePasswordRequest,
    ) -> Result<(), UserError> {
        if req.new_password.len() < 6 {
            return Err(UserError::Other("password must be at least 6 characters".into()));
        }
        let hash: String = self.with_conn(|conn| {
            conn.query_row(
                "SELECT password_hash FROM users WHERE id = ?1",
                params![user_id],
                |r| r.get(0),
            )
            .optional()?
            .ok_or(UserError::NotFound)
        })?;
        if !Self::verify_password(&req.old_password, &hash)? {
            return Err(UserError::InvalidCredentials);
        }
        let new_hash = Self::hash_password(&req.new_password)?;
        let now = Utc::now().to_rfc3339();
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE users SET password_hash = ?1, updated_at = ?2 WHERE id = ?3",
                params![new_hash, now, user_id],
            )?;
            Ok(())
        })?;
        // Force re-login after password change.
        self.revoke_user_sessions(user_id)?;
        Ok(())
    }

    pub fn get_pref(&self, user_id: &str, pref_key: &str) -> Result<Option<String>, UserError> {
        self.with_conn(|conn| {
            conn.query_row(
                "SELECT value FROM user_prefs WHERE user_id = ?1 AND pref_key = ?2",
                params![user_id, pref_key],
                |r| r.get(0),
            )
            .optional()
            .map_err(UserError::from)
        })
    }

    pub fn set_pref(&self, user_id: &str, pref_key: &str, value: &str) -> Result<(), UserError> {
        let now = Utc::now().to_rfc3339();
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO user_prefs (user_id, pref_key, value, updated_at)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(user_id, pref_key) DO UPDATE SET
                   value = excluded.value,
                   updated_at = excluded.updated_at",
                params![user_id, pref_key, value, now],
            )?;
            Ok(())
        })
    }
}

fn random_token() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
