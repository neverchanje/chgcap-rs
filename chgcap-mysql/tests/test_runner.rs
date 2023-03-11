use std::collections::HashMap;

use anyhow::{anyhow, Result};
use chgcap_mysql::{MysqlSource, MysqlSourceConfigBuilder, MysqlTableEvent};
use futures::StreamExt;
use mysql_async::prelude::Query;
use mysql_async::{Conn, Pool};
use serde::Deserialize;

#[derive(Deserialize, PartialEq, Debug)]
struct TableData {
    create_table: String,
    inserts: Vec<String>,
    rows: Vec<String>,
}

async fn consume_cdc_events(expected_num: usize) -> Vec<MysqlTableEvent> {
    assert!(expected_num > 0);

    let cfg = MysqlSourceConfigBuilder::default()
        .hostname("127.0.0.1".into())
        .port(3306)
        .username("root".into())
        .password("123456".into())
        .database("mysql".into())
        .server_id(1)
        .build()
        .unwrap();

    let source = MysqlSource::new(cfg).await.unwrap();
    let cdc_stream = source.cdc_stream().await.unwrap();
    cdc_stream
        .map(|change| change.unwrap())
        .take(expected_num)
        .collect()
        .await
}

struct TestCase {
    _pool: Pool,
    conn: Conn,
    tables: HashMap<String, TableData>,
}

impl TestCase {
    async fn new(path: impl Into<String>) -> Self {
        let pool = mysql_async::Pool::new("mysql://root:123456@127.0.0.1:3306/mysql");
        let conn = pool.get_conn().await.unwrap();

        let path = path.into();
        let tables: HashMap<String, TableData> =
            serde_yaml::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();

        Self {
            _pool: pool,
            conn,
            tables,
        }
    }

    async fn run_inner(&mut self) -> Result<()> {
        let mut total_events = 0;
        for (_name, t) in self.tables.iter() {
            t.create_table.clone().ignore(&mut self.conn).await?;
            for insert in t.inserts.iter() {
                insert.ignore(&mut self.conn).await?;
            }
            total_events += t.rows.len();
        }

        let events = consume_cdc_events(total_events).await;
        let mut table_events: HashMap<String, Vec<String>> = HashMap::new();
        for e in events.iter() {
            let evs = table_events.entry(e.table_name.clone()).or_default();
            evs.extend(e.changes.iter().map(|ch| format!("{ch:?}")));
        }
        for (name, t) in self.tables.iter() {
            let evs = table_events.get(name).ok_or_else(|| {
                anyhow!("No event for table {name}. Received events: {table_events:?}")
            })?;
            assert_eq!(evs.len(), t.rows.len());
            for (i, row) in t.rows.iter().enumerate() {
                let ev = evs.get(i).unwrap();
                assert_eq!(ev, row);
            }
        }

        Ok(())
    }

    async fn run(&mut self) {
        let r = self.run_inner().await;
        self.teardown().await;
        if r.is_err() {
            panic!("Test failed: {:?}", r.unwrap_err());
        }
    }

    async fn teardown(&mut self) {
        // Clean up, no matter success or failure.
        for (table_name, _) in self.tables.iter() {
            format!("DROP TABLE {table_name}")
                .ignore(&mut self.conn)
                .await
                .unwrap();
        }
    }
}

async fn run_test(path: impl Into<String>) {
    TestCase::new(path).await.run().await;
}

#[tokio::test]
async fn test_float() {
    run_test("./tests/testdata/float_test.yaml").await
}
