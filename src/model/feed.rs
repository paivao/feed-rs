use sqlx::{FromRow, Row, Type, PgPool, Error};

#[derive(FromRow)]
pub struct Feed {
    pub id: i64,
    pub name: String,
    #[sqlx(default)]
    pub description: String,
    pub is_public: bool,
    #[sqlx(rename = "type")]
    pub feed_type: FeedType,
    pub digest: Vec<u8>,
}

#[derive(Debug, Type)]
#[sqlx(type_name = "FeedType")]
#[sqlx(rename_all = "lowercase")]
pub enum FeedType {
    IP,
    URL,
    Domain
}

impl Feed {
    const INSERT_QUERY: &'static str = r#"INSERT INTO feeds (value, description, is_public, feed_type) VALUES ($1, $2, $3, $4) RETURNING id;"#;
    const SELECT_QUERY: &'static str = r#"SELECT id, name, description, is_public, digest, type as "feed_type: FeedType" FROM feeds WHERE name = $1 and is_public = $2"#;
    
    pub async fn insert(conn: &PgPool, name: String, feed_type: FeedType, description: Option<String>, is_public: Option<bool>) -> Result<Self, Error> {
        let descr = description.unwrap_or(String::new());
        let id: i64 = sqlx::query_scalar(Self::INSERT_QUERY)
            .bind(&name)
            .bind(&descr)
            .bind(is_public.unwrap_or(true))
            .bind(&feed_type)
            .fetch_one(conn).await?;
        Ok(Self{id: id, name: name, description: descr, is_public: is_public.unwrap_or(true), feed_type, digest: Vec::new()})
    }

    pub async fn list(conn: &PgPool) -> Result<(), Error> {
        let x: Feed = sqlx::query_as(r#"SELECT id, name, description, is_public, digest, type as "feed_type: FeedType" FROM feeds"#).fetch_one(conn).await?;
        Ok(())
    }

    pub async fn get(conn: &PgPool, name: &str, is_public: Option<bool>) -> Result<Self, Error> {
        sqlx::query_as(Self::SELECT_QUERY).bind(name).bind(is_public.unwrap_or(true)).fetch_one(conn).await
    }
}

