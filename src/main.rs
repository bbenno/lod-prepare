//! Window raw sensor and calculate FFT.

#![warn(missing_docs)]

use rusqlite::{params, Connection, OpenFlags, Result};
use rustfft::num_complex::Complex32;
use rustfft::num_traits::Zero;
use rustfft::{FFTplanner, FFT};
use std::{ops::RangeInclusive, sync::Arc};

const SENSORS: RangeInclusive<u32> = 1..=5;
/// Sampling time for N measurements
const T: f64 = 52.39e-3;
/// Block size
const N: usize = 64;

const DB_PATH: &str = "measurements.db";

/// FOR DEVELOPMENT PURPOSE ONLY
const MEASUREMENT_ID: u32 = 1;

/// SQL command to insert fft data
const INSERT_SQL: &str =
    "INSERT INTO `training_data` (measurement_id, block_id, sensor_id, frequency, value)
     VALUES (?1, ?2, ?3, ?4, ?5)";
/// SQL query to select all raw sensor data
const SELECT_SQL: &str = "SELECT I, Q FROM `sensor_data`
    WHERE measurement_id = ?1 AND sensor_id = ?2
    ORDER BY block_id, item_id";

fn main() -> Result<()> {
    // DB INIT
    let mut db_conn =
        Connection::open_with_flags(DB_PATH, OpenFlags::SQLITE_OPEN_READ_WRITE).unwrap();
    let tx = db_conn.transaction().unwrap();
    let mut insertion = tx.prepare(INSERT_SQL).unwrap();
    let mut selection = tx.prepare(SELECT_SQL).unwrap();

    // FFT INIT
    let mut planner = FFTplanner::new(false);
    let fft = planner.plan_fft(N);

    SENSORS.for_each(|sensor_id|
        calc_fft_for_sensordata(&fft,
            // SELECT RAW SENSOR DATA FROM DATABASE
            &mut selection
                .query_map(
                    params![MEASUREMENT_ID, sensor_id],
                    |row| Ok(Complex32 {
                        re: row.get_unwrap::<usize, u16>(0) as f32,
                        im: row.get_unwrap::<usize, u16>(1) as f32,
                    })
                ).unwrap()
                .map(|row| row.unwrap())
                .collect()
        ).unwrap()
        // DB INSERTION
        .chunks_exact(N)
        .enumerate()
        .for_each(|(block_id, block)|
        // Iteration over blocks
            block
                .iter()
                .enumerate()
                .map(|(freq_idx, val)|
                // Iteration over values
                    insertion
                        .execute(params![MEASUREMENT_ID, block_id as u32, sensor_id, f_idx_to_freq(freq_idx), 1]).unwrap()
                )
                .fold((), |t, _| t)
        )
    );

    // drop borrowed statements in order to drop t
    drop(insertion);
    drop(selection);

    tx.commit()
}

fn calc_fft_for_sensordata(
    fft: &Arc<dyn FFT<f32>>,
    input: &mut Vec<Complex32>,
) -> Result<Vec<Complex32>> {
    let mut output: Vec<Complex32> = vec![Zero::zero(); input.len()];
    fft.process_multi(input, &mut output);
    Ok(output)
}
fn f_idx_to_freq(idx: usize) -> f64 {
    (idx as f64) / T
}
