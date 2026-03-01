# Database Migrations

Govrix Platform shares the Scout OSS database schema.

Migrations are managed by the `govrix-scout-store` crate (Scout dependency).
Run Scout's migration runner on startup — Govrix server handles this automatically
when `DATABASE_URL` is set and a pool is established.

To run migrations manually:
```bash
cd /path/to/govrix-scout
sqlx migrate run
```
