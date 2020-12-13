use clap::{crate_authors, crate_description, crate_version, App, Arg};
use log::{debug, info, trace};
use rusqlite::{params, Connection, OpenFlags, Result};
use std::{f64::consts::PI, time::SystemTime};

/// Block (element) size
const N: usize = 64;
/// BLock count
const M: usize = 16;
/// `2¹¹`
const MEAN: f64 = 2048f64;
/// `2π / N`
const X: f64 = 2.0 * PI / N as f64;

fn main() -> Result<()> {
    // LOGGER INIT
    env_logger::init();

    let opts = App::new("LOD Prepare")
        .about(crate_description!())
        .author(crate_authors!())
        .version(crate_version!())
        .args(&[
            Arg::from_usage("<database> 'Sets the database file to use'"),
            Arg::from_usage("-s, --sensors <SENSOR_COUNT> 'Sets count of sensors'")
                .case_insensitive(true)
                .default_value("5"),
            Arg::from_usage("-m, --measurements <MEASUREMENT_COUNT> 'Sets count of measurements'")
                .case_insensitive(true)
                .default_value("10"),
        ])
        .get_matches();

    let sensor_count: usize = opts
        .value_of("sensors")
        .expect("Failed to get value of 'sensors'")
        .parse()
        .expect("Failed to parse value of sensor=\"{}\" to number");
    let measurement_count: usize = opts
        .value_of("measurements")
        .expect("Failed to get value of 'measurements'")
        .parse()
        .expect("Failed to parse value of measurements to number");
    let db_name = opts
        .value_of("database")
        .expect("Failed to read line argument \"database\"");

    let mut db_conn =
        Connection::open_with_flags(db_name, OpenFlags::SQLITE_OPEN_READ_WRITE).unwrap();
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
    // O ≡ measurement_count
    //
    // |  id | date |
    // | --- | ---- |
    // |   1 |  now |
    // |   2 |  now |
    // | ... |  now |
    // |   O |  now |
    (1..=measurement_count).for_each(|measurement_id| {
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
    // O ≡ measurement_count
    //
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
    (1..=measurement_count).for_each(|measurement_id| {
        // for all measurements
        (1..=M).for_each(|block_id| {
            // for all block_elements
            (1..=sensor_count as u32).for_each(|sensor_id| {
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
    // O ≡ measurement_count
    //
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
    (1..=(measurement_count * M * sensor_count)).for_each(|measuring_point_id| {
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
