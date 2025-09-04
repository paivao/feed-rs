use futures::StreamExt;
use sqlx::postgres::PgArguments;
use sqlx::{Postgres, FromRow, PgPool, Row, Encode, Decode, types::Type, Error};
use sqlx::postgres::PgRow;
use sqlx::types::ipnetwork::IpNetwork;
use futures::stream::BoxStream;
use sqlx::types::chrono::{DateTime, Utc};

use crate::model::feed::{Feed};

const TABLE_NAMES: &'static [&'static str] = &[
    "ip_entries",
    "url_entries",
    "domain_entries"
];


#[derive(Default)]
struct BaseEntry {
    id: i64,
    enabled: bool,
    description: String,
    valid_until: Option<DateTime<Utc>>,
    //feed_id: i64,
}

#[derive(FromRow)]
struct IPEntry {
    inner: BaseEntry,
    value: IpNetwork,
}

pub trait TEntry where
    for <'r> Self: Sized + AsRef<BaseEntry> + AsMut<BaseEntry> + Unpin + FromRow<'r, PgRow> + Send,
    for <'q> Self::ValueType: Encode<'q, Postgres> + Type<Postgres> + Sync + Unpin + Decode<'q, Postgres> + Send  
{
    type ValueType;
    const INSERT_QUERY: &'static str = "INSERT INTO {} (value, enabled, feed_id, description, valid_until) VALUES $1, $2, $3, $4, $5 RETURNING id;";
    const FETCH_QUERY: &'static str = "SELECT value from {} WHERE feed_id = $1 AND enabled = TRUE AND (valid_until IS NULL OR valid_until >= NOW())";
    const GET_SOME_QUERY: &'static str = "SELECT (id, value, enabled, description, valid_until) from {} WHERE feed_id = $1";

    fn mut_value(&mut self) -> &mut Self::ValueType;
    fn value(&self) -> &Self::ValueType;
    fn new(value: Self::ValueType, inner: BaseEntry) -> Self;

    async fn insert(conn: &PgPool, feed: &Feed, value: Self::ValueType, description: Option<String>, valid_until: Option<DateTime<Utc>>) -> Result<Self, Error> {
        let descr = description.unwrap_or(String::new());
        let id:i64 = sqlx::query_scalar(Self::INSERT_QUERY)
            .bind(&value)
            .bind(true)
            .bind(feed.id)
            .bind(&descr)
            .bind(&valid_until)
            .fetch_one(conn).await?;
        Ok(Self::new(value, BaseEntry{id: id, enabled: true, description: descr, valid_until: valid_until/*, feed_id: feed.id*/}))
    }

    fn fetch_values<'q>(conn: &'q PgPool, feed: &Feed) -> BoxStream<'q, Result<Self::ValueType, Error>> where Self::ValueType: 'q
    {
        sqlx::query_scalar(Self::FETCH_QUERY).bind(feed.id).fetch(conn)
    }

    async fn fetch_some(conn: &PgPool, feed: &Feed, window: (i64, i64), enabled: Option<bool>, valid_until: Option<Option<DateTime<Utc>>>) -> Result<Vec<Self>, Error> {
        let mut builder = sqlx::QueryBuilder::new(Self::GET_SOME_QUERY);
        if let Some(enabled_cond) = enabled {
            builder.push(" AND enabled = ").push_bind(enabled_cond);
        }
        if let Some(valid_conf) = valid_until {
            builder.push(" AND valid_until");
            match valid_conf {
                Some(times) => builder.push(" >= ").push_bind(times),
                None => builder.push(" IS NULL")
            };
        }
        builder.build_query_as().fetch_all(conn).await
    }
}

