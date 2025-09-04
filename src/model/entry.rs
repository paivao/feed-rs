use sqlx::{Postgres, FromRow, PgPool, Encode, Decode, types::Type, Error};
use sqlx::types::ipnetwork::IpNetwork;
use futures::stream::BoxStream;
use sqlx::types::chrono::{DateTime, Utc};
use const_format::concatcp;

use crate::model::feed::{Feed};

const TABLE_NAMES: &'static [&'static str] = &[
    "ip_entries",
    "domain_entries",
    "url_entries",
];


#[derive(FromRow, Default)]
pub struct Entry<T, const N: usize> {
    id: i64,
    value: T,
    enabled: bool,
    description: Option<String>,
    valid_until: Option<DateTime<Utc>>,
    feed_id: i64
}

type IPEntry = Entry<IpNetwork, 0>;
type DomainEntry = Entry<String, 1>;
type UrlEntry = Entry<String, 2>;

impl<T, const N: usize> Entry<T, N>
    where
        for<'q> T: Encode<'q,Postgres> + Type<Postgres> + Sync + Unpin + Decode<'q, Postgres> + Send
{
    const TABLE_NAME: &str = TABLE_NAMES[N];
    
    pub async fn insert(conn: &PgPool, feed: &Feed, value: T, description: Option<String>, valid_until: Option<DateTime<Utc>>) -> Result<Self, Error> {
            let id = sqlx::query_scalar(concatcp!("INSERT INTO ", "a", " (value, enabled, feed_id, description, valid_until) VALUES $1, $2, $3, $4, $5 RETURNING id;"))
            .bind(&value)
            .bind(true)
            .bind(feed.id)
            .bind(&description)
            .bind(&valid_until)
            .fetch_one(conn).await?;
        Ok(Entry{ id: id, value: value, enabled: true, description: description, valid_until: valid_until, feed_id: feed.id })
    }
    pub fn fetch_values<'q, 'e>(conn: &'e PgPool, feed: &Feed) -> BoxStream<'e, Result<T, Error>> where T: 'e {
        static QUERY: &str = format!("SELECT value from {} WHERE feed_id = $1 AND enabled = TRUE AND (valid_until IS NULL OR valid_until >= NOW())", "AAA").as_str();
        let x = sqlx::query_scalar(QUERY).bind(feed.id);
        x.fetch(conn)
    }
}
