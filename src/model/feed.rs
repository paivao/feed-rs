use sqlx::{FromRow, Row, Type, PgPool, Error};
use rocket::{serde::{Deserialize, Serialize}};

#[derive(FromRow)]
pub struct Feed {
    pub id: i64,
    pub name: String,
    #[sqlx(default)]
    pub description: Option<String>,
    pub is_public: bool,
    #[sqlx(rename = "type")]
    pub feed_type: FeedType,
    pub digest: Box<[u8]>
}

#[derive(Serialize, Deserialize, Debug, Type)]
#[serde(crate = "rocket::serde")]
#[sqlx(type_name = "FeedType")]
#[sqlx(rename_all = "lowercase")]
pub enum FeedType {
    IP,
    URL,
    Domain
}

impl FeedType {
    pub fn table_name(&self) -> &'static str {
        match self {
            FeedType::IP => "ip_entries",
            FeedType::URL => "url_entries",
            FeedType::Domain => "domain_entries"
        }
    }
}

impl Feed {
    pub async fn insert(&mut self, conn: &PgPool) -> Result<(), Error> {
        self.id = ::sqlx::query(r#"INSERT INTO feeds (value, description, is_public, feed_type) VALUES ($1, $2, $3, $4) RETURNING id;"#)
            .bind(&self.name)
            .bind(&self.description)
            .bind(&self.is_public)
            .bind(&self.feed_type)
            .fetch_one(conn).await?.get(0);
        Ok(())
    }

    pub async fn fetch(conn: &PgPool) -> Result<(), Error> {
        let x = sqlx::query_as!(Feed, r#"SELECT id, name, description, is_public, digest, type as "feed_type: FeedType" FROM feeds"#).fetch_one(conn).await?;
        Ok(())
    }
}

