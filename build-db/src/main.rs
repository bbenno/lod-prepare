use rusqlite::{params, Connection, OpenFlags, Result};
use std::{f64::consts::PI, time::SystemTime};

#[path = "../../src/cli.rs"]
mod cli;

/// Block (element) size
const N: usize = 64;
/// BLock count
const M: usize = 16;
/// Measurements count
const O: usize = 10;
/// Sensor count
const P: usize = 5;
/// `2¹¹`
const MEAN: f64 = 2048f64;
/// `2π / N`
const X: f64 = 2.0 * PI / N as f64;

fn main() -> Result<()> {
    let args = cli::get_args();

    let mut db_conn =
        Connection::open_with_flags(&args[1], OpenFlags::SQLITE_OPEN_READ_WRITE).unwrap();
    let tx = db_conn.transaction().unwrap();

    //tx.execute("DELETE FROM measuring_value", params![]).unwrap();
    //tx.execute("DELETE FROM measuring_point", params![]).unwrap();
    //tx.execute("DELETE FROM measuring_time", params![]).unwrap();
    //tx.execute("DELETE FROM measurement", params![]).unwrap();

    let mut insert_measuring_value = tx
        .prepare(
            "INSERT INTO `measuring_value` (measuring_point_id, phase, value) VALUES (?, ?, ?)",
        )
        .unwrap();

    let mut insert_measuring_point = tx
        .prepare(
            "INSERT INTO `measuring_point` (block_id, measurement_id, sensor_id) VALUES (?, ?, ?)",
        )
        .unwrap();

    let mut insert_measuring_time = tx
        .prepare("INSERT INTO `measuring_time` (block, block_element) VALUES (?, ?)")
        .unwrap();

    let mut insert_measurement = tx
        .prepare("INSERT INTO `measurement` (id, date) VALUES (?, ?)")
        .unwrap();

    // INSERT measuring_times
    // |   id | block | block_element |
    // | ---- | ----- | ------------- |
    // |    1 |     0 |             0 |
    // |    2 |     0 |             1 |
    // |  ... |   ... |           ... |
    // |    N |     0 |           N-1 |
    // | N +1 |     1 |             0 |
    // |  ... |   ... |           ... |
    // |  N*M |   M-1 |           N-1 |
    (0..M).for_each(|block| {
        // for each block
        (0..N).for_each(|block_element| {
            // for each block element
            insert_measuring_time
                .execute(params![block as u32, block_element as u32])
                .unwrap();
        });
    });

    // INSERT measurements
    // |  id | ... |
    // | --- |
    // |   1 |
    // |   2 |
    // | ... |
    // |   O |
    (1..=O).for_each(|measurement_id| {
        insert_measurement
            .execute(params![
                measurement_id as u32,
                SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as u32,
            ])
            .unwrap();
    });

    // INSERT measuring_points
    // |       id | measuring_id | block_id | sensor_id |
    // | -------- | ------------ | -------- | --------- |
    // |        1 |            0 |        0 |         1 |
    // |        2 |            0 |        0 |         2 |
    // |      ... |          ... |      ... |       ... |
    // |        P |            0 |        0 |         P |
    // |     P +1 |            0 |        1 |         1 |
    // |      ... |          ... |      ... |       ... |
    // |    N*M*P |            0 |   N*M -1 |         P |
    // | N*M*P +1 |            1 |        0 |         1 |
    // |      ... |          ... |      ... |       ... |
    // |  O*N*M*P |            O |   N*M -1 |         P |
    (1..=O).for_each(|measurement_id| {
        // for all measurements
        (1..=(N * M)).for_each(|block_id| {
            // for all block_elements
            (1..=P as u32).for_each(|sensor_id| {
                // for all sensors
                insert_measuring_point
                    .execute(params![
                        block_id as u32,
                        measurement_id as u32,
                        sensor_id as u32,
                    ])
                    .unwrap();
            });
        });
    });

    /// Calculates: `2¹¹ × cos(i × 2π / N) + 2¹¹ ∈ {0..2¹²}`
    fn cos_value_generator(i: usize) -> f64 {
        MEAN + MEAN * (X * i as f64).cos()
    }

    /// Calculates: `2¹¹ × sin(i × 2π / N) + 2¹¹ ∈ {0..2¹²}`
    fn sin_value_generator(i: usize) -> f64 {
        MEAN + MEAN * (X * i as f64).sin()
    }

    // INSERT measuring_values
    // |            id | measurement_point_id | phase | value |
    // | ------------- | -------------------- | ----- | ----- |
    // |             0 |                    0 |     0 |  XXXX |
    // |             1 |                    0 |     1 |  XXXX |
    // |             2 |                    1 |     0 |  XXXX |
    // |             3 |                    1 |     1 |  XXXX |
    // |             4 |                    2 |     0 |  XXXX |
    // |           ... |                  ... |   ... |   ... |
    // | 2* O*N*M*P -1 |              O*N*M*P |     1 |  XXXX |
    (1..=(O * N * M * P)).for_each(|measuring_point_id| {
        // for each measuring_point_id
        (0..=1).for_each(|phase| {
            // for each phase
            insert_measuring_value
                .execute(params![
                    measuring_point_id as u32,
                    phase,
                    phase * cos_value_generator(measuring_point_id) as u16
                        + (1 - phase) * sin_value_generator(measuring_point_id) as u16,
                ])
                .unwrap();
        });
    });

    // drop borrowed statements in order to drop Transaction tx
    drop(insert_measuring_value);
    drop(insert_measuring_point);
    drop(insert_measuring_time);
    drop(insert_measurement);

    tx.commit()
}
