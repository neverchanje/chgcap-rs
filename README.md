
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
| chgcap-mysql | MySQL | |
 

## Getting Started

### Installation

To install chgcap, use `cargo`:

`$ cargo install chgcap`

### Usage

To use chgcap, you must first configure a connector for the source database. Once the configuration is complete, you can start streaming the data using the API.

## Documentation

The full documentation can be found on [GitHub](https://github.com/
