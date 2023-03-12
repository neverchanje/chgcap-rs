use std::collections::HashMap;
use std::time::Duration;

use anyhow::{anyhow, Result};
use chgcap_mysql::{MysqlSource, MysqlSourceConfigBuilder, MysqlTableEvent};
use mysql_async::prelude::Query;
use mysql_async::{Conn, Pool};
use serde::Deserialize;
use tokio_stream::StreamExt;

mod util;

#[derive(Deserialize, PartialEq, Debug)]
struct TableData {
    prepare: String,
    rows: Vec<String>,
}

async fn consume_cdc_events() -> Result<Vec<MysqlTableEvent>> {
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
    let cdc_stream = source
        .cdc_stream()
        .await
        .unwrap()
        .timeout(Duration::from_secs(1));
    tokio::pin!(cdc_stream);

    let mut events: Vec<MysqlTableEvent> = vec![];
    while let Ok(Some(c)) = cdc_stream.try_next().await {
        events.push(c?);
    }
    Ok(events)
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

        let tables: HashMap<String, TableData> =
            serde_yaml::from_str(&std::fs::read_to_string(path.into()).unwrap()).unwrap();

        Self {
            _pool: pool,
            conn,
            tables,
        }
    }

    async fn run_inner(&mut self) -> Result<()> {
        for (_name, t) in self.tables.iter() {
            t.prepare.clone().ignore(&mut self.conn).await?;
        }

        let events = consume_cdc_events().await?;

        // Tables may be created multiple times. We use the latest.
        let tables_id: HashMap<String, u64> = events
            .iter()
            .map(|e| (e.table_name.clone(), e.table_id))
            .collect();

        let mut table_events: HashMap<u64, Vec<String>> = HashMap::new();
        for e in events.iter() {
            let evs = table_events.entry(e.table_id).or_default();
            evs.extend(e.changes.iter().map(|ch| format!("{ch}")));
        }
        for (name, t) in self.tables.iter() {
            let table_id = tables_id
                .get(name)
                .ok_or_else(|| anyhow!("No table id for table {name}."))?;
            let evs = table_events
                .get(table_id)
                .ok_or_else(|| anyhow!("No event for table {name}."))?;
            assert_eq!(evs.len(), t.rows.len());
            for (i, row) in t.rows.iter().enumerate() {
                let ev = evs.get(i).unwrap();
                ensure_eq!(ev, row);
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

// docker run --name mysql -d -p 3306:3306 -e MYSQL_ROOT_PASSWORD=123456 mysql/mysql-server:8.0
#[tokio::test]
async fn test_cdc() {
    run_test("./tests/testdata/testdata.yaml").await
}
