use std::collections::HashMap;

use chgcap_mysql::{MysqlSource, MysqlSourceConfigBuilder, MysqlTableEvent};
use futures::StreamExt;
use mysql_async::prelude::Query;
use serde::Deserialize;

#[derive(Deserialize, PartialEq, Debug)]
struct Field {}

#[derive(Deserialize, PartialEq, Debug)]
struct TableData {
    create_table: String,
    inserts: Vec<String>,
    rows: Vec<Vec<Field>>,
}

async fn prepare_table(t: &TableData) {
    let pool = mysql_async::Pool::new("mysql://root:123456@127.0.0.1:3306");
    let mut conn = pool.get_conn().await.unwrap();

    t.create_table.clone().ignore(&mut conn).await.unwrap();
    for insert in t.inserts.iter() {
        insert.ignore(&mut conn).await.unwrap();
    }
}

async fn consume_cdc_events() -> Vec<MysqlTableEvent> {
    let cfg = MysqlSourceConfigBuilder::default()
        .hostname("127.0.0.1".into())
        .port(3306)
        .username("root".into())
        .password("123456".into())
        .database("mysql".into())
        .build()
        .unwrap();

    let source = MysqlSource::new(cfg).await.unwrap();
    let cdc_stream = source.cdc_stream().await.unwrap();
    cdc_stream.map(|change| change.unwrap()).collect().await
}

pub async fn run_test(path: impl Into<String>) {
    let path = path.into();
    let tables: HashMap<String, TableData> =
        serde_yaml::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    for (_name, t) in tables.iter() {
        prepare_table(t).await;
    }
    let events = consume_cdc_events().await;
    for (_, _event) in events.iter().enumerate() {}
}
