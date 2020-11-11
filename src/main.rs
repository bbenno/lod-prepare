//! Window raw sensor and calculate FFT.

#![warn(missing_docs)]

use rusqlite::{params, Connection, OpenFlags, Result};
use rustfft::num_complex::Complex32;
use rustfft::num_traits::Zero;
use rustfft::FFTplanner;
use std::ops::RangeInclusive;

mod cli;

const SENSORS: RangeInclusive<u32> = 1..=5;
/// Sampling time for N measurements
const T: f64 = 52.39e-3;
/// Block size
const N: usize = 64;

/// FOR DEVELOPMENT PURPOSE ONLY
const MEASUREMENT_ID: u32 = 1;

/// SQL command to insert fft data
const INSERT_SQL: &str =
    "INSERT INTO `training_data` (measurement_id, block_id, sensor_id, frequency, value) VALUES (?, ?, ?, ?, ?)";
/// SQL query to select all raw sensor data
const SELECT_SQL: &str = "SELECT I, Q FROM `sensor_data`
    WHERE measurement_id = ? AND sensor_id = ?
    ORDER BY block_id, item_id";

fn main() -> Result<()> {
    let args = cli::get_args();

    // DB INIT
    let mut db_conn =
        Connection::open_with_flags(&args[1], OpenFlags::SQLITE_OPEN_READ_WRITE).unwrap();
    let tx = db_conn.transaction().unwrap();
    let mut insertion = tx.prepare(INSERT_SQL).unwrap();
    let mut selection = tx.prepare(SELECT_SQL).unwrap();

    // FFT INIT
    let mut planner = FFTplanner::new(false);
    let fft = planner.plan_fft(N);

    SENSORS.for_each(|sensor_id|
        || -> Result<Vec<Complex32>, &'static str> {
            // SELECT RAW SENSOR DATA FROM DATABASE
            let mut input = selection
                .query_map(
                    params![MEASUREMENT_ID, sensor_id],
                    |row| Ok(Complex32 {
                        re: row.get_unwrap::<usize, u16>(0) as f32,
                        im: row.get_unwrap::<usize, u16>(1) as f32,
                    })
                )
                .expect("database failure while querying input")
                .map(|row_result| row_result.unwrap())
                .collect::<Vec<Complex32>>();

            if input.len() % N == 0 {
                return Err("invalid data length");
            }

            // Calculate mean per chunk: mean = sum_(j=0)^N(x_j) / N
            let means = input
                .chunks_exact(N)
                .map(|c| c.iter().sum::<Complex32>() / (N as f32))
                .collect::<Vec<Complex32>>();

            // INPUT NORMALIZATION: x_i |-> x_i - mean
            input = input
                .chunks_exact(N)
                .enumerate()
                .map(|(i, c)|
                    c
                        .iter()
                        .map(|cc| cc - means[i])
                        .collect::<Vec<Complex32>>()
                )
                .flatten()
                .collect();

            let mut output: Vec<Complex32> = vec![Zero::zero(); input.len()];
            // CALCULATE FFT
            fft.process_multi(&mut input, &mut output);
            Ok(output
                .iter()
                // OUTPUT NORMALIZATION
                .map(|c| c * (1.0 / (input.len() as f32).sqrt()))
                .collect())
        }()
        .unwrap_or(Default::default())
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
                .fold((), |_, _| ())
        )
    );

    // drop borrowed statements in order to drop t
    drop(insertion);
    drop(selection);

    tx.commit()
}

fn f_idx_to_freq(idx: usize) -> f64 {
    (idx as f64) / T
}
