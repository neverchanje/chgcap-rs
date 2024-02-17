use env_logger::Target;
use indexmap::IndexMap;
use log::info;
use std::time::Duration;

use anyhow::{bail, Result};
use chgcap_mysql::{Event, EventData, Source, SourceConfigBuilder};
use chgcap_mysql_test_utils::mysql_container::Mysql;
use mysql_async::prelude::Query;
use mysql_async::{Conn, Pool};
use serde::Deserialize;
use testcontainers::clients::Cli;
use testcontainers::Container;
use tokio::time::sleep;
use tokio_stream::StreamExt;

lazy_static::lazy_static! {
    pub static ref DOCKER: Cli = Cli::default();
    pub static ref MYSQL_CONTAINER: Container<'static, Mysql> = DOCKER.run(Mysql::default());
    pub static ref LOGGER: () = env_logger::builder().filter_level(log::LevelFilter::Info).target(Target::Stdout).init();
}

#[derive(Deserialize, PartialEq, Debug)]
struct TableData {
    comment: Option<String>,
    prepare: String,
    rows: String,
    ddls: Option<String>,
}

/// This function behaves as a user of the chgcap. It consumes and collects all CDC events into a list.
async fn consume_cdc_events() -> Result<Vec<Event>> {
    let cfg = SourceConfigBuilder::default()
        .hostname("0.0.0.0".into())
        .port(MYSQL_CONTAINER.get_host_port_ipv4(3306))
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

/// The template structure of `single_table_cdc.yaml`.
struct TestSuite {
    _pool: Pool,
    conn: Conn,

    // `Indexmap` can preserve the order of insertion.
    tables: IndexMap<String, TableData>,
    table_events: IndexMap<String, Vec<String>>,
}

impl TestSuite {
    async fn load(path: impl Into<String>) -> Self {
        let pool = mysql_async::Pool::new(
            format!(
                "mysql://root@0.0.0.0:{}/mysql",
                MYSQL_CONTAINER.get_host_port_ipv4(3306)
            )
            .as_str(),
        );
        let conn = pool.get_conn().await.unwrap();

        let tables: IndexMap<String, TableData> =
            serde_yaml::from_str(&std::fs::read_to_string(path.into()).unwrap()).unwrap();

        Self {
            _pool: pool,
            conn,
            tables,
            table_events: IndexMap::new(),
        }
    }

    async fn write_data_and_collect_events(&mut self) -> Result<()> {
        // Run preparation queries into database.
        for (_name, t) in self.tables.iter() {
            t.prepare.clone().ignore(&mut self.conn).await?;
            info!("Run queries: {}", t.prepare);
        }
        sleep(Duration::from_secs(1)).await;

        let events = consume_cdc_events().await?;

        // Tables may be created multiple times. We use the latest.
        let table_ids: IndexMap<String, u64> = events
            .iter()
            .map(|e| (e.table_name().clone(), e.table_id()))
            .collect();

        let mut table_events = IndexMap::<u64, Vec<String>>::new();
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

        self.table_events = table_ids
            .iter()
            .filter_map(|(table_name, table_id)| {
                table_events
                    .get(table_id)
                    .map(|events| (table_name.clone(), events.clone()))
            })
            .collect();

        Ok(())
    }

    async fn check_inner(&mut self) -> Result<()> {
        self.write_data_and_collect_events().await?;

        for (table_data, events) in self.tables.iter().filter_map(|(key, table_data)| {
            self.table_events
                .get(key)
                .map(|events| (table_data, events))
        }) {
            check_cdc_rows_eq(&table_data.rows, events)?;
        }

        Ok(())
    }

    async fn check(&mut self) {
        let r = self.check_inner().await;
        self.teardown().await;
        if r.is_err() {
            panic!("Test failed: {:?}", r.unwrap_err());
        }
    }

    async fn fix(&mut self) {
        self.write_data_and_collect_events().await.unwrap();

        let mut tables = serde_yaml::Mapping::new();

        for (key, table_data, events) in self.tables.iter().filter_map(|(key, table_data)| {
            self.table_events
                .get(key)
                .map(|events| (key, table_data, events))
        }) {
            let mut table = serde_yaml::Mapping::new();
            if let Some(comment) = &table_data.comment {
                table.insert("comment".into(), comment.clone().into());
            }
            table.insert("prepare".into(), table_data.prepare.clone().into());
            table.insert("rows".into(), events.join("\n").into());

            tables.insert(key.clone().into(), serde_yaml::Value::Mapping(table));
        }

        let yaml = serde_yaml::to_string(&tables).unwrap();
        std::fs::write("./tests/testdata/single_table_cdc.yaml", yaml).unwrap();
    }

    async fn teardown(&mut self) {
        for (table_name, _) in self.tables.iter() {
            format!("DROP TABLE IF EXISTS {table_name}")
                .ignore(&mut self.conn)
                .await
                .unwrap();
        }
    }
}

fn check_cdc_rows_eq(expected: &str, actual: &[String]) -> anyhow::Result<()> {
    let expected = expected.trim().to_string();
    let actual = actual.join("\n").trim().to_string();
    if expected != actual {
        bail!("Rows mismatched.\nExpected:\n{expected}\nActual:\n{actual}")
    }
    Ok(())
}

#[tokio::test]
async fn test_single_table_cdc() {
    let mut t = TestSuite::load("./tests/testdata/single_table_cdc.yaml").await;
    t.check().await;
    t.teardown().await;
}

#[tokio::test]
async fn fix_single_table_cdc() {
    let mut t = TestSuite::load("./tests/testdata/single_table_cdc.yaml").await;
    t.fix().await;
    t.teardown().await;
}
