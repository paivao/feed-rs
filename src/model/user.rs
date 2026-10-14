use crate::error::ApiError;
use crate::utils::{create_password, verify_password};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row, prelude::FromRow};

#[derive(Deserialize, Serialize, Default, Clone)]
pub struct Group {
    pub id: i32,
    pub name: String,
}

#[derive(FromRow, Deserialize, Serialize, Clone)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
    #[sqlx(default)]
    pub groups: Vec<Group>,
}

impl User {
    const INSERT_QUERY: &'static str = r#"INSERT INTO users (name, email, password_hash) VALUES ($1, $2, $3) RETURNING id, name, email"#;
    const GET_BY_ID_QUERY: &'static str = r#"SELECT id, name, email FROM users WHERE id = $1"#;
    const GET_BY_EMAIL_QUERY: &'static str =
        r#"SELECT id, name, email FROM users WHERE email = $1"#;
    const GET_BY_NAME_QUERY: &'static str = r#"SELECT id, name, email FROM users WHERE name = $1"#;
    const UPDATE_QUERY: &'static str = r#"UPDATE users SET name = $1, email = $2 WHERE id = $3"#;
    /// Create a new user in the database
    pub async fn create(
        pool: &PgPool,
        name: &str,
        email: &str,
        password: &str,
    ) -> Result<Self, ApiError> {
        //let password_hash = format!("{:x}", md5::compute(password.as_bytes()));
        let password_hash = create_password(password)?;

        let row = sqlx::query(Self::INSERT_QUERY)
            .bind(name)
            .bind(email)
            .bind(&password_hash)
            .fetch_one(pool)
            .await?;

        Ok(User {
            id: row.get("id"),
            name: row.get("name"),
            email: row.get("email"),
            groups: Vec::new(),
        })
    }

    /// Get a user by ID
    pub async fn get_by_id(pool: &PgPool, id: i64) -> sqlx::Result<Self> {
        let row = sqlx::query(Self::GET_BY_ID_QUERY)
            .bind(id)
            .fetch_one(pool)
            .await?;

        Ok(User {
            id: row.get("id"),
            name: row.get("name"),
            email: row.get("email"),
            groups: Vec::new(),
        })
    }

    /// Get a user by email
    pub async fn get_by_email(pool: &PgPool, email: &str) -> sqlx::Result<Self> {
        let row = sqlx::query(Self::GET_BY_EMAIL_QUERY)
            .bind(email)
            .fetch_one(pool)
            .await?;

        Ok(User {
            id: row.get("id"),
            name: row.get("name"),
            email: row.get("email"),
            // TODO: implement group fetching
            groups: Vec::new(),
        })
    }

    /// Get a user by username
    pub async fn get_by_name(pool: &PgPool, name: &str) -> sqlx::Result<Self> {
        let row = sqlx::query(Self::GET_BY_NAME_QUERY)
            .bind(name)
            .fetch_one(pool)
            .await?;

        Ok(User {
            id: row.get("id"),
            name: row.get("name"),
            email: row.get("email"),
            // TODO: implement group fetching
            groups: Vec::new(),
        })
    }

    /// Update user information
    pub async fn update(&self, pool: &PgPool) -> sqlx::Result<()> {
        let row = sqlx::query(Self::UPDATE_QUERY)
            .bind(&self.name)
            .bind(&self.email)
            .bind(self.id)
            .execute(pool)
            .await?;

        return if row.rows_affected() != 1 {
            Err(sqlx::Error::RowNotFound)
        } else {
            Ok(())
        };
    }

    /// Delete a user by ID
    pub async fn delete(pool: &PgPool, id: i64) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Verify user credentials (login check)
    /// Returns Some(User) if credentials are valid, None otherwise
    pub async fn verify_credentials(pool: &PgPool, username: &str, password: &str) -> Option<Self> {
        // Try to find user by email or name
        let user = sqlx::query("SELECT id, name, email, password_hash FROM users WHERE name = $1")
            .bind(username)
            .fetch_one(pool)
            .await;

        let stored_hash: String = if let Ok(_user) = &user {
            _user.get("password_hash")
        } else {
            String::new()
        };

        // Runs it even when user is None (anti-timing attack)
        if verify_password(&stored_hash, password) {
            match user {
                Ok(user) => Some(User {
                    id: user.get("id"),
                    name: user.get("name"),
                    email: user.get("email"),
                    groups: Vec::new(),
                }),
                Err(e) => {
                    crate::app_debug!("Verify credentials: database error: {}", e);
                    None
                }
            }
        } else {
            None
        }
    }

    /// List all users (with optional pagination)
    pub async fn list_all(pool: &PgPool, limit: i64, offset: i64) -> sqlx::Result<Vec<Self>> {
        let rows = sqlx::query(
            "SELECT id, name, email, password_hash FROM users ORDER BY id LIMIT $1 OFFSET $2",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| User {
                id: r.get("id"),
                name: r.get("name"),
                email: r.get("email"),
                groups: Vec::new(),
            })
            .collect())
    }
}
