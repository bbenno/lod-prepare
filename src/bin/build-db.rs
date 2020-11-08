use rusqlite::{params, Connection, OpenFlags, Result};

const MEASUREMENT_ID: u32 = 1;
const SENSOR_ID: u32 = 1;
/// Block size
const N: usize = 64;
const DB_PATH: &str = "../measurements.db";

const INSERT_SQL: &str = "INSERT INTO measured_values (measurement_id, block_id, sensor_id, item_id, phase, value) VALUES (?, ?, ?, ?, ?, ?)";
const CLEAR_SQL: &str = "DELETE FROM measured_values";

fn main() -> Result<()> {
    let mut db_conn =
        Connection::open_with_flags(DB_PATH, OpenFlags::SQLITE_OPEN_READ_WRITE).unwrap();
    let tx = db_conn.transaction().unwrap();

    tx.execute(CLEAR_SQL, params![]).unwrap();
    tx.execute(
        "INSERT INTO measurements (id) VALUES (?)",
        params![MEASUREMENT_ID],
    )
    .unwrap();

    /// Calculates: `2¹¹ × i × cos(2π / N) + 2¹¹ ∈ {0..2¹²}`
    fn value_generator(i: usize) -> f64 {
        let x = 2.0 * std::f64::consts::PI / N as f64;
        2.0f64.powi(11) * (x * i as f64).cos() + 2.0f64.powi(11)
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
                                params![MEASUREMENT_ID, block_id, SENSOR_ID, item_id as u32, phase, phase * value_generator(item_id) as u16]
                            )
                            .unwrap()
                        )
                        .fold((),|_, _| ())
                )
        );

    tx.commit()
}
