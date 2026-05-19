use crate::error::ApiError;
use crate::utils::{self, create_password};
use serde::Deserialize;
use sqlx::{PgPool, Row, prelude::FromRow};

#[derive(Deserialize, Default)]
pub struct Group {
    pub id: i64,
    pub name: String,
}

#[derive(FromRow, Deserialize)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub password_hash: String,
    #[sqlx(default)]
    pub groups: Vec<Group>,
}

impl User {
    /// Create a new user in the database
    pub async fn create(
        pool: &PgPool,
        name: &str,
        email: &str,
        password: &str,
    ) -> Result<Self, ApiError> {
        //let password_hash = format!("{:x}", md5::compute(password.as_bytes()));
        let password_hash = create_password(password)?;

        let row = sqlx::query(
            "INSERT INTO users (name, email, password_hash) VALUES ($1, $2, $3) RETURNING id, name, email, password_hash"
        )
        .bind(name)
        .bind(email)
        .bind(&password_hash)
        .fetch_one(pool)
        .await?;

        Ok(User {
            id: row.get("id"),
            name: row.get("name"),
            email: row.get("email"),
            password_hash: row.get("password_hash"),
            groups: Vec::new(),
        })
    }

    /// Get a user by ID
    pub async fn get_by_id(pool: &PgPool, id: i64) -> Result<Option<Self>, sqlx::Error> {
        let row = sqlx::query("SELECT id, name, email, password_hash FROM users WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await?;

        Ok(row.map(|r| User {
            id: r.get("id"),
            name: r.get("name"),
            email: r.get("email"),
            password_hash: r.get("password_hash"),
            groups: Vec::new(),
        }))
    }

    /// Get a user by email
    pub async fn get_by_email(pool: &PgPool, email: &str) -> Result<Option<Self>, sqlx::Error> {
        let row = sqlx::query("SELECT id, name, email, password_hash FROM users WHERE email = $1")
            .bind(email)
            .fetch_optional(pool)
            .await?;

        Ok(row.map(|r| User {
            id: r.get("id"),
            name: r.get("name"),
            email: r.get("email"),
            password_hash: r.get("password_hash"),
            groups: Vec::new(),
        }))
    }

    /// Get a user by username
    pub async fn get_by_name(pool: &PgPool, name: &str) -> Result<Option<Self>, sqlx::Error> {
        let row = sqlx::query("SELECT id, name, email, password_hash FROM users WHERE name = $1")
            .bind(name)
            .fetch_optional(pool)
            .await?;

        Ok(row.map(|r| User {
            id: r.get("id"),
            name: r.get("name"),
            email: r.get("email"),
            password_hash: r.get("password_hash"),
            groups: Vec::new(),
        }))
    }

    /// Update user information
    pub async fn update(
        pool: &PgPool,
        id: i64,
        name: Option<&str>,
        email: Option<&str>,
    ) -> Result<Option<Self>, sqlx::Error> {
        let mut query_str = String::from("UPDATE users SET ");
        let mut bindings = vec![];
        let mut param_count = 1;

        if let Some(n) = name {
            query_str.push_str(&format!("name = ${} ", param_count));
            bindings.push(n.to_string());
            param_count += 1;
        }

        if let Some(e) = email {
            if param_count > 2 {
                query_str.push_str(", ");
            }
            query_str.push_str(&format!("email = ${} ", param_count));
            bindings.push(e.to_string());
            param_count += 1;
        }

        if bindings.is_empty() {
            return Self::get_by_id(pool, id).await;
        }

        query_str.push_str(&format!(
            "WHERE id = ${} RETURNING id, name, email, password_hash",
            param_count
        ));

        let mut query = sqlx::query(&query_str);
        for binding in bindings {
            query = query.bind(binding);
        }
        query = query.bind(id);

        let row = query.fetch_optional(pool).await?;

        Ok(row.map(|r| User {
            id: r.get("id"),
            name: r.get("name"),
            email: r.get("email"),
            password_hash: r.get("password_hash"),
            groups: Vec::new(),
        }))
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
    pub async fn verify_credentials(
        pool: &PgPool,
        username_or_email: &str,
        password: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        // Try to find user by email or name
        let user = sqlx::query(
            "SELECT id, name, email, password_hash FROM users WHERE email = $1 OR name = $1",
        )
        .bind(username_or_email)
        .fetch_optional(pool)
        .await?;

        if let Some(row) = user {
            let stored_hash: String = row.get("password_hash");
            let provided_hash = format!("{:x}", md5::compute(password.as_bytes()));

            if stored_hash == provided_hash {
                return Ok(Some(User {
                    id: row.get("id"),
                    name: row.get("name"),
                    email: row.get("email"),
                    password_hash: row.get("password_hash"),
                    groups: Vec::new(),
                }));
            }
        }

        Ok(None)
    }

    /// List all users (with optional pagination)
    pub async fn list_all(
        pool: &PgPool,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Self>, sqlx::Error> {
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
                password_hash: r.get("password_hash"),
                groups: Vec::new(),
            })
            .collect())
    }
}
