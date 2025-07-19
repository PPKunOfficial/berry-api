use crate::database::Database;
use crate::models::user::{UpdateUserRequest, User};
use anyhow::{Context, Result};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use uuid::Uuid;

#[derive(Clone)]
pub struct UserRepository {
    db: Database,
}

impl UserRepository {
    pub fn new(db: Database) -> Self {
        UserRepository { db }
    }

    pub async fn create(
        &self,
        username: &str,
        email: &str,
        password: &str,
        full_name: Option<&str>,
    ) -> Result<User> {
        let password_hash = self.hash_password(password)?;

        let user = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (username, email, password_hash, full_name)
            VALUES ($1, $2, $3, $4)
            RETURNING id, username, email, password_hash, full_name, is_active, is_admin, created_at, updated_at, last_login_at
            "#,
        )
        .bind(username)
        .bind(email)
        .bind(password_hash)
        .bind(full_name)
        .fetch_one(self.db.get_pool())
        .await
        .context("Failed to create user")?;

        Ok(user)
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<User>> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT id, username, email, password_hash, full_name, is_active, is_admin, created_at, updated_at, last_login_at
            FROM users
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(self.db.get_pool())
        .await
        .context("Failed to find user by ID")?;

        Ok(user)
    }

    pub async fn find_by_username(&self, username: &str) -> Result<Option<User>> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT id, username, email, password_hash, full_name, is_active, is_admin, created_at, updated_at, last_login_at
            FROM users
            WHERE username = $1
            "#,
        )
        .bind(username)
        .fetch_optional(self.db.get_pool())
        .await
        .context("Failed to find user by username")?;

        Ok(user)
    }

    pub async fn find_by_email(&self, email: &str) -> Result<Option<User>> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT id, username, email, password_hash, full_name, is_active, is_admin, created_at, updated_at, last_login_at
            FROM users
            WHERE email = $1
            "#,
        )
        .bind(email)
        .fetch_optional(self.db.get_pool())
        .await
        .context("Failed to find user by email")?;

        Ok(user)
    }

    pub async fn update(&self, id: Uuid, updates: &UpdateUserRequest) -> Result<Option<User>> {
        let user = sqlx::query_as::<_, User>(
            r#"
            UPDATE users
            SET 
                email = COALESCE($2, email),
                full_name = COALESCE($3, full_name),
                is_active = COALESCE($4, is_active),
                updated_at = NOW()
            WHERE id = $1
            RETURNING id, username, email, password_hash, full_name, is_active, is_admin, created_at, updated_at, last_login_at
            "#,
        )
        .bind(id)
        .bind(updates.email.as_deref())
        .bind(updates.full_name.as_deref())
        .bind(updates.is_active)
        .fetch_optional(self.db.get_pool())
        .await
        .context("Failed to update user")?;

        Ok(user)
    }

    pub async fn update_last_login(&self, id: Uuid) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE users
            SET last_login_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(self.db.get_pool())
        .await
        .context("Failed to update last login")?;

        Ok(())
    }

    pub async fn delete(&self, id: Uuid) -> Result<bool> {
        let result = sqlx::query(
            r#"
            DELETE FROM users
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(self.db.get_pool())
        .await
        .context("Failed to delete user")?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn list(&self, limit: i64, offset: i64) -> Result<Vec<User>> {
        let users = sqlx::query_as::<_, User>(
            r#"
            SELECT id, username, email, password_hash, full_name, is_active, is_admin, created_at, updated_at, last_login_at
            FROM users
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(self.db.get_pool())
        .await
        .context("Failed to list users")?;

        Ok(users)
    }

    pub async fn verify_password(&self, username: &str, password: &str) -> Result<Option<User>> {
        let user = self.find_by_username(username).await?;

        if let Some(user) = &user {
            if self.verify_password_hash(password, &user.password_hash)? {
                return Ok(Some(user.clone()));
            }
        }

        Ok(None)
    }

    fn hash_password(&self, password: &str) -> Result<String> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();

        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| anyhow::anyhow!("Failed to hash password: {}", e))?
            .to_string();

        Ok(password_hash)
    }

    fn verify_password_hash(&self, password: &str, hash: &str) -> Result<bool> {
        let parsed_hash =
            PasswordHash::new(hash).map_err(|e| anyhow::anyhow!("Invalid password hash: {}", e))?;

        let argon2 = Argon2::default();

        Ok(argon2
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
    }
}
