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

## Session store

`docker run -p 6379:6379 --name leprecon-valkey valkey/valkey:7.2.5-alpine3.19`
