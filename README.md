
# chgcap - Change-Data-Capture Connectors Library

chgcap is an open-source library of connectors for Change-Data-Capture (CDC). It is written in Rust and designed to be easy to use and highly efficient.

## Features

- Easy to use API for streaming transaction logs from various databases.
- Support for loading the initial snapshot before consuming the incremental data.
- Flexible configuration options for setting up connectors.
- High performance and scalability.

### Supported Databases

| Connector | Database | Driver |
| --------- | -------- | ------ |
| chgcap-mysql | MySQL | [mysql_async](https://docs.rs/mysql_async/latest/mysql_async/) |

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
