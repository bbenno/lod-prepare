use std::f64::consts::PI;

use rusqlite::{params, Connection, OpenFlags, Result};

#[path = "../../src/cli.rs"]
mod cli;

const MEASUREMENT_ID: u32 = 1;
const SENSOR_ID: u32 = 1;
/// Block size
const N: usize = 64;

// MEAN = 2u32.pow(11)
const MEAN: f64 = 2048f64;

const X: f64 = 2.0 * PI / N as f64;

const INSERT_SQL: &str = "INSERT INTO measured_values (measurement_id, block_id, sensor_id, item_id, phase, value) VALUES (?, ?, ?, ?, ?, ?)";
const CLEAR_SQL: &str = "DELETE FROM measured_values";

fn main() -> Result<()> {
    let args = cli::get_args();

    let mut db_conn =
        Connection::open_with_flags(&args[1], OpenFlags::SQLITE_OPEN_READ_WRITE).unwrap();
    let tx = db_conn.transaction().unwrap();

    tx.execute(CLEAR_SQL, params![]).unwrap();
    tx.execute(
        "INSERT INTO measurements (id) VALUES (?)",
        params![MEASUREMENT_ID],
    )
    .unwrap();

    /// Calculates: `2¹¹ × i × cos(2π / N) + 2¹¹ ∈ {0..2¹²}`
    fn cos_value_generator(i: usize) -> f64 {
        MEAN + MEAN * (X * i as f64).cos()
    }

    fn sin_value_generator(i: usize) -> f64 {
        MEAN + MEAN * (X * i as f64).sin()
    }

    (1..=2)
        .for_each(|block_id|
            (0..N)
                .for_each(|item_id|
                    (0..=1)
                        .map(|phase|
                            // measurement_id, block_id, sensor_id, item_id, phase, value
                            tx.execute(
                                INSERT_SQL,
                                params![MEASUREMENT_ID, block_id, SENSOR_ID, item_id as u32, phase, phase * cos_value_generator(item_id) as u16 + (1-phase) * sin_value_generator(item_id) as u16]
                            )
                            .unwrap()
                        )
                        .fold((),|_, _| ())
                )
        );

    tx.commit()
}
