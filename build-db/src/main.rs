use log::{debug, info, trace};
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
    // LOGGER INIT
    env_logger::init();

    let args = cli::get_args();

    let mut db_conn =
        Connection::open_with_flags(&args[1], OpenFlags::SQLITE_OPEN_READ_WRITE).unwrap();
    let tx = db_conn.transaction().unwrap();
    debug!("Database connection established");

    //tx.execute("DELETE FROM measuring_value", params![]).unwrap();
    //tx.execute("DELETE FROM measuring_point", params![]).unwrap();
    //tx.execute("DELETE FROM measuring_time", params![]).unwrap();
    //tx.execute("DELETE FROM measurement", params![]).unwrap();

    let mut insert_measuring_value = tx
        .prepare(
            "INSERT INTO `measuring_value` (`measuring_point_id`, `block_element`, `phase`, `value`) VALUES (?, ?, ?, ?)",
        )
        .unwrap();

    let mut insert_measuring_point = tx
        .prepare(
            "INSERT INTO `measuring_point` (`block_id`, `measurement_id`, `sensor_id`) VALUES (?, ?, ?)",
        )
        .unwrap();

    let mut insert_measurement = tx
        .prepare("INSERT INTO `measurement` (`id`, `date`) VALUES (?, ?)")
        .unwrap();

    let measurement_date = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs() as u32;

    info!("INSERT measurements");
    // |  id | date |
    // | --- | ---- |
    // |   1 |  now |
    // |   2 |  now |
    // | ... |  now |
    // |   O |  now |
    (1..=O).for_each(|measurement_id| {
        trace!(
            "INSERT `measurement` (id: {}, date: {})",
            measurement_id,
            measurement_date
        );
        insert_measurement
            .execute(params![measurement_id as u32, measurement_date,])
            .unwrap();
    });

    info!("INSERT measuring_points");
    // |     id | measuring_id | block_id | sensor_id |
    // | ------ | ------------ | -------- | --------- |
    // |      1 |            1 |        1 |         1 |
    // |      2 |            1 |        1 |         2 |
    // |    ... |          ... |      ... |       ... |
    // |      P |            1 |        1 |         P |
    // |   P +1 |            1 |        2 |         1 |
    // |    ... |          ... |      ... |       ... |
    // |    M*P |            1 |        M |         P |
    // | M*P +1 |            2 |        1 |         1 |
    // |    ... |          ... |      ... |       ... |
    // |  O*M*P |            O |        M |         P |
    (1..=O).for_each(|measurement_id| {
        // for all measurements
        (1..=M).for_each(|block_id| {
            // for all block_elements
            (1..=P as u32).for_each(|sensor_id| {
                // for all sensors
                trace!(
                    "INSERT `measuring_point` (block_id: {}, measurement_id: {}, sensor_id: {})",
                    block_id,
                    measurement_id,
                    sensor_id
                );
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

    /// Calculates: `(2¹¹ - 1) × (1 + cos(i × 2π / N)) + 1 ∈ {1..2¹²-1}`
    fn cos_value_generator(i: usize) -> f64 {
        (MEAN - 1f64) * (1f64 + (X * i as f64).cos()) + 1f64
    }

    /// Calculates: `(2¹¹ - 1) × (1 + sin(i × 2π / N)) + 1 ∈ {1..2¹²-1}`
    fn sin_value_generator(i: usize) -> f64 {
        (MEAN - 1f64) * (1f64 + (X * i as f64).sin()) + 1f64
    }

    info!("INSERT measuring_values");
    // |        id | measuring_point_id | block_element | phase | value |
    // | --------- | ------------------ | ------------- | ----- | ----- |
    // |         1 |                  1 |             1 |     0 |  XXXX |
    // |         2 |                  1 |             1 |     1 |  XXXX |
    // |         3 |                  1 |             2 |     0 |  XXXX |
    // |         4 |                  1 |             2 |     1 |  XXXX |
    // |         5 |                  1 |             3 |     0 |  XXXX |
    // |       ... |                ... |               |   ... |   ... |
    // |       2*N |                  1 |             N |     1 |  XXXX |
    // |    2*N +1 |                  2 |             1 |     0 |  XXXX |
    // |       ... |                ... |           ... |   ... |   ... |
    // | 2*N*O*M*P |              O*M*P |             N |     1 |  XXXX |
    (1..=(O * M * P)).for_each(|measuring_point_id| {
        (1..=N).for_each(|block_element| {
            // for each measuring_point_id
            (0..=1).for_each(|phase| {
                // for each phase
                let value = phase * cos_value_generator(block_element) as u16 + (1 - phase) * sin_value_generator(block_element) as u16;
                trace!(
                    "INSERT `measuring_value` (measurement_point_id: {}, block_element: {}, phase: {}, value: {})",
                    measuring_point_id,
                    block_element,
                    phase,
                    value
                );
                insert_measuring_value
                    .execute(params![
                        measuring_point_id as u32,
                        block_element as u32,
                        phase,
                        value,
                    ])
                    .unwrap();
            });
        });
    });

    info!("Finished");

    // drop borrowed statements in order to drop Transaction tx
    drop(insert_measuring_value);
    drop(insert_measuring_point);
    drop(insert_measurement);

    tx.commit()
}
