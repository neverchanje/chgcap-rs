use std::collections::HashMap;
use std::time::Duration;

use anyhow::{anyhow, bail, Result};
use chgcap_mysql::{Event, EventData, Source, SourceConfigBuilder};
use mysql_async::prelude::Query;
use mysql_async::{Conn, Pool};
use serde::Deserialize;
use tokio::time::sleep;
use tokio_stream::StreamExt;

mod util;

#[derive(Deserialize, PartialEq, Debug)]
struct TableData {
    prepare: String,
    rows: Vec<String>,
}

async fn consume_cdc_events() -> Result<Vec<Event>> {
    let cfg = SourceConfigBuilder::default()
        .hostname("0.0.0.0".into())
        .port(3306)
        .username("root".into())
        .database("mysql".into())
        .server_id(1)
        .build()
        .unwrap();

    let source = Source::new(cfg).await.unwrap();
    let cdc_stream = source
        .cdc_stream()
        .await
        .unwrap()
        .timeout(Duration::from_secs(1));
    tokio::pin!(cdc_stream);

    let mut events: Vec<Event> = vec![];
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
        let pool = mysql_async::Pool::new("mysql://root@0.0.0.0:3306/mysql");
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
        sleep(Duration::from_secs(1)).await;

        let events = consume_cdc_events().await?;

        // Tables may be created multiple times. We use the latest.
        let tables_id: HashMap<String, u64> = events
            .iter()
            .map(|e| (e.table_name().clone(), e.table_id()))
            .collect();

        let mut table_events: HashMap<u64, Vec<String>> = HashMap::new();
        for e in events.iter() {
            let evs = table_events.entry(e.table_id()).or_default();
            match e.data() {
                EventData::DataChange(changes) => {
                    evs.extend(changes.iter().map(|ch| format!("{ch}")));
                }
                EventData::SchemaChange(_) => {
                    todo!()
                }
            }
        }
        for (name, t) in self.tables.iter() {
            let table_id = tables_id
                .get(name)
                .ok_or_else(|| anyhow!("No table id for table {name}."))?;
            let evs = table_events
                .get(table_id)
                .ok_or_else(|| anyhow!("No event for table {name}."))?;
            if evs.len() != t.rows.len() {
                bail!("Events count received ({}) for table '{name}'(id: {table_id}) mismatches with the expected ({}). Received:\n{evs:?}", evs.len(), t.rows.len());
            }
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
            format!("DROP TABLE IF EXISTS {table_name}")
                .ignore(&mut self.conn)
                .await
                .unwrap();
        }
    }
}

async fn run_test(path: impl Into<String>) {
    TestCase::new(path).await.run().await;
}

// docker run --name mysql -e MYSQL_ALLOW_EMPTY_PASSWORD=yes -p 3306:3306 -d mysql:8.1 --gtid_mode=ON --enforce_gtid_consistency=ON
// mysql -h 127.0.0.1 -P 3306 -u root -D mysql
#[tokio::test]
async fn test_cdc() {
    run_test("./tests/testdata/testdata.yaml").await;
}
