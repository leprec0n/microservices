# microservices

The following microservices exist:

- Account

They are build using Rust.

## Requirements

- Rust
- Docker

## Running

To run the project you can use `cargo watch -x 'run --bin {service}'`.

## Environment

The environment variables are located inside `example.env`, and should be copied to a `.env`.

Export all .env variables through `export $(cat .env | xargs)`.
