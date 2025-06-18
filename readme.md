# Axum + Askama + HTMX

Create a `.env` file as described in `.env.sample`

Run Postgres and Redis
```bash
docker-compose up -d
```

Install `sqlx-cli`
```bash
cargo install sqlx-cli --features postgres
```

Set up the application database
```bash
sqlx db setup
```

Run the application
```bash
cargo run
```

If successful, the application should now run on port 8000.
