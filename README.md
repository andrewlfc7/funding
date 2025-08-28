
![Funding Dashboard UI](image.png)


# Prerequisites
- **PostgreSQL**.
- **Rust**.
- **Node.js and npm**.



# Setup Instructions
1. Database Configuration

1. Install PostgreSQL and create a database.

2. Create a `.env` file in the project root with your database URL:

```env
DATABASE_URL=postgresql://user:password@localhost:5432/your_database
```

Run the following commands to set up the database:

```bash
sqlx database create
sqlx migrate run
```


### 2. Backfilling the Database

To populate the database tables, use the `sync` binary. You can backfill all tables or target specific exchanges and time ranges.

```bash
Quick Backfill (All Tables)
cargo run --bin sync

Backfill for the Last 24 Hours
cargo run --bin sync init --hours 24

Backfill Between Specific Timestamps
Use Unix timestamps (in milliseconds) to specify a range:

cargo run --bin sync init --between 1724544000000 1724630400000
```


Target a Single Exchange
Add the `--exchange` flag to target a specific exchange (case-insensitive).


Backfill markets for the paradex exchange:
```bash
cargo run --bin sync markets --exchange paradex


Backfill funding for the extended exchange for the last 168 hours:

cargo run --bin sync funding --exchange extended --since-last 168
```


**Recommendation**: Use `cargo run --bin sync` to backfill all tables unless specific data is needed.

# 3. Running the Backend
Start the backend after backfilling the database:

```bash
cargo run --bin backend
```

# 4. Running the Frontend
In a separate terminal, navigate to the `frontend` directory and run:
```bash
cd frontend
npm install
npm run dev
```



Extending to New Exchanges
To add support for a new exchange:
- Use the `--exchange` flag with the new exchange name in sync commands.
- Ensure the exchange is supported in the backend configuration.




# Notes
- Exchange names are case-insensitive.
- Verify the `.env` file has the correct database URL.
- Refer to the project documentation or open an issue for support.




# Contributing
1. Fork the repository.
2. Create a new branch (`git checkout -b feature-name`).
3. Commit your changes (`git commit -m "Add feature"`).
4. Push to the branch (`git push origin feature-name`).
5. Open a pull request.


