#!/usr/bin/sh

DB_PATH=../measurements.db

# recreate new, empty database
rm -f ${DB_PATH} && \
sqlite3 ${DB_PATH} < ${DB_PATH%.*}.sql && \
# fill with dummy data
cargo run -p "build-db" ${DB_PATH} && \
# print dumym data
sqlite3 -readonly -box ${DB_PATH} "SELECT * FROM measuring_value"
