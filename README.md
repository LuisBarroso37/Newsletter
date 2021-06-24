# Newsletter

Email newsletter to which users can subscribe and receive notifications about new blog updates.

This project follows the book [Zero To Production In Rust](https://zero2prod.com) which is a great guide to backend development using Rust.

## Pre-requisite

You'll need to install:

- [Rust](https://www.rust-lang.org/tools/install)
- [Docker](https://docs.docker.com/get-docker/)

Launch a (migrated) Postgres database via Docker:

```bash
./scripts/init_db.sh
```

## How to build

Using `cargo`:

```bash
cargo build
```

## How to test

Using `cargo`:

```bash
cargo test
```
