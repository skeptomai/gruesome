//! SQLite persistence. Replaces the DynamoDB single-table service.

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{FromRow, SqlitePool};
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{AppError, AppResult};

pub fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[derive(Clone)]
pub struct Db {
    pub pool: SqlitePool,
}

impl Db {
    /// Connect (creating the file if missing), enable foreign keys + WAL, and
    /// run migrations.
    pub async fn connect(database_url: &str) -> anyhow::Result<Self> {
        let opts = SqliteConnectOptions::from_str(database_url)?
            .create_if_missing(true)
            .foreign_keys(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(opts)
            .await?;

        sqlx::migrate!("./migrations").run(&pool).await?;
        Ok(Db { pool })
    }

    // ── Users ──────────────────────────────────────────────────────────────

    pub async fn create_user(&self, u: &User) -> AppResult<()> {
        sqlx::query(
            "INSERT INTO users (user_id, email, username, display_name, password_hash, role, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&u.user_id)
        .bind(&u.email)
        .bind(&u.username)
        .bind(&u.display_name)
        .bind(&u.password_hash)
        .bind(&u.role)
        .bind(u.created_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_user_by_id(&self, user_id: &str) -> AppResult<User> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE user_id = ?")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| AppError::NotFound("user".into()))
    }

    /// Login accepts either username or email (Cognito allowed both as aliases).
    pub async fn get_user_by_login(&self, login: &str) -> AppResult<Option<User>> {
        Ok(sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE username = ? COLLATE NOCASE OR email = ? COLLATE NOCASE",
        )
        .bind(login)
        .bind(login)
        .fetch_optional(&self.pool)
        .await?)
    }

    pub async fn username_exists(&self, username: &str) -> AppResult<bool> {
        let n: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE username = ? COLLATE NOCASE")
                .bind(username)
                .fetch_one(&self.pool)
                .await?;
        Ok(n > 0)
    }

    pub async fn email_exists(&self, email: &str) -> AppResult<bool> {
        let n: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE email = ? COLLATE NOCASE")
                .bind(email)
                .fetch_one(&self.pool)
                .await?;
        Ok(n > 0)
    }

    pub async fn update_password(&self, user_id: &str, password_hash: &str) -> AppResult<()> {
        sqlx::query("UPDATE users SET password_hash = ? WHERE user_id = ?")
            .bind(password_hash)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ── Password resets ────────────────────────────────────────────────────

    pub async fn upsert_password_reset(
        &self,
        user_id: &str,
        code_hash: &str,
        expires_at: i64,
    ) -> AppResult<()> {
        sqlx::query(
            "INSERT INTO password_resets (user_id, code_hash, expires_at) VALUES (?, ?, ?)
             ON CONFLICT(user_id) DO UPDATE SET code_hash = excluded.code_hash, expires_at = excluded.expires_at",
        )
        .bind(user_id)
        .bind(code_hash)
        .bind(expires_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Returns `(code_hash, expires_at)` if a reset exists for the user.
    pub async fn get_password_reset(&self, user_id: &str) -> AppResult<Option<(String, i64)>> {
        Ok(sqlx::query_as::<_, (String, i64)>(
            "SELECT code_hash, expires_at FROM password_resets WHERE user_id = ?",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?)
    }

    pub async fn delete_password_reset(&self, user_id: &str) -> AppResult<()> {
        sqlx::query("DELETE FROM password_resets WHERE user_id = ?")
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ── Games ──────────────────────────────────────────────────────────────

    /// Active (non-archived) games, player-facing order (display_order then age).
    pub async fn list_active_games(&self) -> AppResult<Vec<Game>> {
        Ok(sqlx::query_as::<_, Game>(
            "SELECT * FROM games WHERE archived = 0
             ORDER BY display_order IS NULL, display_order, created_at",
        )
        .fetch_all(&self.pool)
        .await?)
    }

    /// All games including archived (admin view).
    pub async fn list_all_games(&self) -> AppResult<Vec<Game>> {
        Ok(
            sqlx::query_as::<_, Game>("SELECT * FROM games ORDER BY created_at DESC")
                .fetch_all(&self.pool)
                .await?,
        )
    }

    pub async fn get_active_game(&self, game_id: &str) -> AppResult<Game> {
        sqlx::query_as::<_, Game>("SELECT * FROM games WHERE game_id = ? AND archived = 0")
            .bind(game_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| AppError::NotFound("game".into()))
    }

    pub async fn get_game_any(&self, game_id: &str) -> AppResult<Game> {
        sqlx::query_as::<_, Game>("SELECT * FROM games WHERE game_id = ?")
            .bind(game_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| AppError::NotFound("game".into()))
    }

    pub async fn insert_game(&self, g: &Game) -> AppResult<()> {
        sqlx::query(
            "INSERT INTO games
             (game_id, title, author, description, category, year, version, release, serial,
              checksum, file_size, s3_key, display_order, archived, created_at, updated_at)
             VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
        )
        .bind(&g.game_id)
        .bind(&g.title)
        .bind(&g.author)
        .bind(&g.description)
        .bind(&g.category)
        .bind(g.year)
        .bind(g.version)
        .bind(g.release)
        .bind(&g.serial)
        .bind(&g.checksum)
        .bind(g.file_size)
        .bind(&g.s3_key)
        .bind(g.display_order)
        .bind(g.archived)
        .bind(g.created_at)
        .bind(g.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(db) if db.is_unique_violation() => {
                AppError::Conflict("a game with that id already exists".into())
            }
            other => AppError::Internal(other.to_string()),
        })?;
        Ok(())
    }

    /// Update the admin-editable metadata fields.
    #[allow(clippy::too_many_arguments)]
    pub async fn update_game(
        &self,
        game_id: &str,
        title: &str,
        author: &str,
        description: &str,
        category: Option<&str>,
        year: Option<i64>,
        display_order: Option<i64>,
        updated_at: i64,
    ) -> AppResult<()> {
        let rows = sqlx::query(
            "UPDATE games SET title=?, author=?, description=?, category=?, year=?,
             display_order=?, updated_at=? WHERE game_id=?",
        )
        .bind(title)
        .bind(author)
        .bind(description)
        .bind(category)
        .bind(year)
        .bind(display_order)
        .bind(updated_at)
        .bind(game_id)
        .execute(&self.pool)
        .await?
        .rows_affected();
        if rows == 0 {
            return Err(AppError::NotFound("game".into()));
        }
        Ok(())
    }

    pub async fn archive_game(&self, game_id: &str, updated_at: i64) -> AppResult<()> {
        let rows = sqlx::query("UPDATE games SET archived=1, updated_at=? WHERE game_id=?")
            .bind(updated_at)
            .bind(game_id)
            .execute(&self.pool)
            .await?
            .rows_affected();
        if rows == 0 {
            return Err(AppError::NotFound("game".into()));
        }
        Ok(())
    }

    // ── Saves ──────────────────────────────────────────────────────────────

    pub async fn list_saves(&self, user_id: &str) -> AppResult<Vec<Save>> {
        Ok(sqlx::query_as::<_, Save>(
            "SELECT * FROM saves WHERE user_id = ? ORDER BY last_updated DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn list_saves_for_game(&self, user_id: &str, game_id: &str) -> AppResult<Vec<Save>> {
        Ok(sqlx::query_as::<_, Save>(
            "SELECT * FROM saves WHERE user_id = ? AND game_id = ? ORDER BY last_updated DESC",
        )
        .bind(user_id)
        .bind(game_id)
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn get_save(&self, user_id: &str, game_id: &str, save_name: &str) -> AppResult<Save> {
        sqlx::query_as::<_, Save>(
            "SELECT * FROM saves WHERE user_id = ? AND game_id = ? AND save_name = ?",
        )
        .bind(user_id)
        .bind(game_id)
        .bind(save_name)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("save".into()))
    }

    /// Insert or update a save's metadata (upload registers/refreshes it).
    pub async fn upsert_save(&self, s: &Save) -> AppResult<()> {
        sqlx::query(
            "INSERT INTO saves (user_id, game_id, save_name, s3_key, file_size, created_at, last_updated)
             VALUES (?,?,?,?,?,?,?)
             ON CONFLICT(user_id, game_id, save_name)
             DO UPDATE SET s3_key=excluded.s3_key, file_size=excluded.file_size, last_updated=excluded.last_updated",
        )
        .bind(&s.user_id)
        .bind(&s.game_id)
        .bind(&s.save_name)
        .bind(&s.s3_key)
        .bind(s.file_size)
        .bind(s.created_at)
        .bind(s.last_updated)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn delete_save(
        &self,
        user_id: &str,
        game_id: &str,
        save_name: &str,
    ) -> AppResult<()> {
        sqlx::query("DELETE FROM saves WHERE user_id = ? AND game_id = ? AND save_name = ?")
            .bind(user_id)
            .bind(game_id)
            .bind(save_name)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct Game {
    pub game_id: String,
    pub title: String,
    pub author: String,
    pub description: String,
    pub category: Option<String>,
    pub year: Option<i64>,
    pub version: i64,
    pub release: i64,
    pub serial: String,
    pub checksum: String,
    pub file_size: i64,
    pub s3_key: String,
    pub display_order: Option<i64>,
    pub archived: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, FromRow)]
pub struct Save {
    pub user_id: String,
    pub game_id: String,
    pub save_name: String,
    pub s3_key: String,
    pub file_size: i64,
    pub created_at: i64,
    pub last_updated: i64,
}

#[derive(Debug, Clone, FromRow)]
pub struct User {
    pub user_id: String,
    pub email: String,
    pub username: String,
    pub display_name: String,
    pub password_hash: String,
    pub role: Option<String>,
    pub created_at: i64,
}

impl User {
    pub fn is_admin(&self) -> bool {
        self.role.as_deref() == Some("admin")
    }
}
