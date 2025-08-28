![Funding Dashboard UI](image.png)

First, need to have PostgreSQL installed and a DB created 

Create an .env  file and add the DB URL to it 

sqlx database create 

sqlx migration run

Then it's good to go about backfilling the tables

To backfill the tables, the quick way is 

cargo run --bin sync

can also set the hours and specify the exchanges, which makes it easy to extend to a new exch later on 

# Backfill for the last 24 hours
cargo run --bin sync init --hours 24

# Backfill between specific timestamps
cargo run --bin sync init --between 1724544000000 1724630400000


would advise to only do sync so it backfills all of the tables


Target a single exchange: You can add the --exchange flag to any command to target a single exchange. Exchange names are case-insensitive.

# Backfill markets only for the paradex exchange
cargo run --bin sync markets --exchange paradex

# Backfill funding for the extended exchange for the last 168 hours
cargo run --bin sync funding --exchange extended --since-last 168


to use the backend once backfilled the db 

just do 

cargo run --bin backend


and in another terminal 

cd frontend

npm install
npm run dev

