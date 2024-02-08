# chgcap - Change-Data-Capture Connectors Library

Chgcap is an open-source library for Change-Data-Capture (CDC) written in Rust. It provides an alternative to Debezium, which is mostly limited to the Java ecosystem. With Chgcap, developers can more easily build custom replicas for their RDBMS. Use cases include creating real-time MySQL caches or real-time OLAP engines for Postgres.

We initially focus on the Rust API, but will consider other language bindings if there are many requests for them.

**WARNING:** Chgap is currently in its early development phase. When it reaches the beta stage, I will publish a beta version on crates.io. My initial objective is to create a DuckDB+MySQL CDC demo, which will showcase how to create a MySQL replica with OLAP functionality. If this is something you are interested in, welcome to follow us for updates.

## Features

It aims to provide all main features supported by Debezium, including:

- Ensures that **all data changes** are captured.
- Produces change events with a **very low delay** while avoiding increased CPU usage required for frequent polling. For example, for MySQL or PostgreSQL, the delay is in the millisecond range.
- Requires **no changes to your data model**, such as a "Last Updated" column.
- Can **capture deletes**.
- Can **capture old record state and additional metadata** such as transaction ID and causing query, depending on the database’s capabilities and configuration.
- Support for loading the **initial snapshot** before consuming the incremental data.

### Supported Databases

| Connector    | Databases | Driver                                                         |
| ------------ | --------- | -------------------------------------------------------------- |
| chgcap-mysql | MySQL     | [mysql_async](https://docs.rs/mysql_async/latest/mysql_async/) |

## Getting Started

### Installation

To install chgcap, use `cargo`:

```sh
cargo install chgcap
```

### Usage

To use chgcap, you must first configure a connector for the source database. Once the configuration is complete, you can start streaming the data using the API.

## Documentation

The full documentation can be found on https://github.com/neverchanje/chgcap-rs

## Credits

chgcap was inspired by and uses some code from the following open-source projects:

- [Flink CDC Connectors](https://github.com/ververica/flink-cdc-connectors)
- [Debezium](https://github.com/debezium/debezium)

## License

chgcap is released under the [Apache 2.0 license](LICENSE).
