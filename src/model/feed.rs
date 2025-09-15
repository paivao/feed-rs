use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type, PgPool, Error};

#[derive(FromRow, Serialize, Deserialize)]
pub struct Feed {
    id: i64,
    pub name: String,
    #[sqlx(default)]
    pub description: String,
    #[sqlx(rename = "type")]
    pub feed_type: FeedType,
    pub digest: Vec<u8>,
}

#[derive(Deserialize)]
pub struct InsertFeedData {
    pub name: String,
    pub feed_type: FeedType,
    pub description: Option<String>
}

#[derive(Deserialize)]
pub struct UpdateFeedData {
    pub description: Option<String>
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[sqlx(type_name = "FeedType")]
#[sqlx(rename_all = "lowercase")]
pub enum FeedType {
    IP,
    URL,
    Domain
}

impl Feed {
    const INSERT_QUERY: &'static str = r#"INSERT INTO feeds (value, description, feed_type) VALUES ($1, $2, $3, $4) RETURNING id;"#;
    const GET_QUERY: &'static str = r#"SELECT id, name, description, digest, type as "feed_type: FeedType" FROM feeds WHERE name = $1"#;
    const GET_ID_QUERY: &'static str = r#"SELECT id, name, description, digest, type as "feed_type: FeedType" FROM feeds WHERE id = $1"#;
    const LIST_QUERY: &'static str = r#"SELECT id, name, description, digest, type as "feed_type: FeedType" FROM feeds"#;
    const LIST_SOME_QUERY: &'static str = r#"SELECT id, name, description, digest, type as "feed_type: FeedType" FROM feeds LIMIT $1 OFFSET $2"#;
    const UPDATE_DIGEST_QUERY: &'static str = r#"UPDATE feeds SET digest = $1 WHERE id = $2"#;
    
    pub async fn insert(conn: &PgPool, data: InsertFeedData) -> Result<Self, Error> {
        let descr = data.description.unwrap_or(String::new());
        let id: i64 = sqlx::query_scalar(Self::INSERT_QUERY)
            .bind(&data.name)
            .bind(&descr)
            .bind(&data.feed_type)
            .fetch_one(conn).await?;
        Ok(Self{id: id, name: data.name, description: descr, feed_type: data.feed_type, digest: Vec::new()})
    }

    pub async fn list(conn: &PgPool, window: Option<super::Window>) -> Result<Vec<Self>, Error> {
        if let Some(window) = window {
            sqlx::query_as(Self::LIST_SOME_QUERY).bind(window.size).bind(window.pos * window.size).fetch_all(conn).await
        } else {
            sqlx::query_as(Self::LIST_QUERY).fetch_all(conn).await
        }
    }

    pub async fn get(conn: &PgPool, name: &str) -> Result<Self, Error> {
        sqlx::query_as(Self::GET_QUERY).bind(name).fetch_one(conn).await
    }

    pub async fn get_by_id(conn: &PgPool, id: i64) -> Result<Self, Error> {
        sqlx::query_as(Self::GET_ID_QUERY).bind(id).fetch_one(conn).await
    }

    pub async fn update_digest(&self, conn: &PgPool) -> Result<(), Error> {
        sqlx::query(Self::UPDATE_DIGEST_QUERY).bind(&self.digest).bind(self.id).execute(conn).await?;
        Ok(())
    }
}

