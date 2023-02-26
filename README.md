# chgcap - Change-Data-Capture Connectors Library

chgcap is an open-source library for Change-Data-Capture (CDC). It is written in Rust and designed to be a drop-in replacement for Debezium. The motivation for this project is the difficulty of integrating Debezium with other languages. CDC should not be limited to the Java ecosystem. Ultimately, I would like it to be wrapped as a C library and make other language bindings based on it.

The initial focus of this project will be to support native CDC without relying on third parties such as Kafka Connect. This allows us to makes its usefulness more apparent. In addition, we will first focus on the Rust API and consider others if there are many issues requesting them.

## Features

It aims to provide all main features supported by Debezium, including:

- Ensures that **all data changes** are captured.
- Produces change events with a **very low delay** while avoiding increased CPU usage required for frequent polling. For example, for MySQL or PostgreSQL, the delay is in the millisecond range.
- Requires **no changes to your data model**, such as a "Last Updated" column.
- Can **capture deletes**.
- Can **capture old record state and additional metadata** such as transaction ID and causing query, depending on the database’s capabilities and configuration.
- Support for loading the **initial snapshot** before consuming the incremental data.

### Supported Databases

| Connector    | Database | Driver                                                         |
| ------------ | -------- | -------------------------------------------------------------- |
| chgcap-mysql | MySQL    | [mysql_async](https://docs.rs/mysql_async/latest/mysql_async/) |

## Getting Started

### Installation

To install chgcap, use `cargo`:

`$ cargo install chgcap`

### Usage

To use chgcap, you must first configure a connector for the source database. Once the configuration is complete, you can start streaming the data using the API.

## Documentation

The full documentation can be found on [GitHub](https://github.com/

## Credits

chgcap was inspired by and uses some code from the following open-source projects:

- [Rust MySQL CDC](https://github.com/rusuly/mysql_cdc)
- [Flink CDC Connectors](https://github.com/ververica/flink-cdc-connectors)

## License

chgcap is released under the [Apache 2.0 license](LICENSE).
