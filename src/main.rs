//! Window raw sensor and calculate FFT.

#![warn(missing_docs)]

use rusqlite::{params, Connection, MappedRows, OpenFlags, Result, Row, Statement};
use rustfft::num_complex::Complex32;
use rustfft::num_traits::Zero;
use rustfft::{FFTplanner, FFT};
use std::sync::Arc;

type SensorData = (u32, Vec<Complex32>);

const SENSOR_COUNT: u32 = 5;
/// Sampling time for N measurements
const T: f64 = 52.39e-3;
/// Block size
const N: usize = 64;

const DB_PATH: &str = "measurements.db";

const MEASUREMENT_ID: u32 = 1;

const INSERT_SQL: &str =
    "INSERT INTO `training_data` (measurement_id, block_id, sensor_id, frequency, value)
     VALUES (?1, ?2, ?3, ?4, ?5)";
const SELECT_SQL: &str = "SELECT I, Q FROM `sensor_data`
    WHERE measurement_id = ?1 AND sensor_id = ?2
    ORDER BY block_id, item_id";

fn main() -> Result<()> {
    let db_conn = Connection::open_with_flags(DB_PATH, OpenFlags::SQLITE_OPEN_READ_WRITE).unwrap();
    let mut data = get_data(&db_conn).unwrap();

    let fft_data = calc_fft(&mut data).unwrap();

    println!("FFT: {:#?}", fft_data);

    let inserted_rows = save_data(db_conn.prepare(INSERT_SQL).unwrap(), &fft_data).unwrap();
    println!("Inserted {} rows", inserted_rows);

    Ok(())
}

fn get_data(db_conn: &Connection) -> Result<Vec<SensorData>> {
    let mut stmt = db_conn.prepare(SELECT_SQL).unwrap();
    let data: Vec<_> = (1..=SENSOR_COUNT)
        .filter_map(|i| get_sensor_data(&mut stmt, i).map(|vec| (i, vec)).ok())
        .collect();

    Ok(data)
}

fn get_sensor_data(stmt: &mut Statement, sensor_id: u32) -> Result<Vec<Complex32>> {
    // For developing purposes temporary only measuring values of measurement 1 are considered.
    const MEASUREMENT_ID: u32 = 1;

    let mapped_rows = stmt
        .query_map(
            params![MEASUREMENT_ID, sensor_id],
            convert_sql_row_to_complex,
        )
        .unwrap();
    convert_rows_to_vec(mapped_rows)
}

fn convert_rows_to_vec<F>(rows: MappedRows<F>) -> Result<Vec<Complex32>>
where
    F: FnMut(&Row) -> Result<Complex32>,
{
    let vec = rows.map(|row| row.unwrap()).collect();
    Ok(vec)
}

fn convert_sql_row_to_complex(row: &Row) -> Result<Complex32> {
    let re: u16 = row.get(0)?;
    let im: u16 = row.get(1)?;
    Ok(Complex32::new(re as f32, im as f32))
}

fn calc_fft(data: &mut Vec<SensorData>) -> Result<Vec<SensorData>> {
    let mut planner = FFTplanner::new(false);
    let fft = planner.plan_fft(N);

    let fft_data = data
        .iter_mut()
        .map(|(i, v)| (*i, calc_sensor_fft(&fft, v).unwrap()))
        .collect();

    Ok(fft_data)
}

fn calc_sensor_fft(fft: &Arc<dyn FFT<f32>>, input: &mut Vec<Complex32>) -> Result<Vec<Complex32>> {
    let mut output: Vec<Complex32> = vec![Zero::zero(); input.len()];
    fft.process_multi(input, &mut output);
    Ok(output)
}

fn save_data(mut stmt: Statement, data: &Vec<SensorData>) -> Result<u32> {
    fn f_idx_to_freq(idx: usize) -> f64 {
        (idx as f64) / T
    };

    let c = data
        .iter()
        .map(|(sensor_id, v)|
        // Iteration over sensors
            v
                .chunks_exact(N)
                .enumerate()
                .map(|(block_id, block)|
                // Iteration over blocks
                    block
                        .iter()
                        .enumerate()
                        .map(|(freq_idx, val)|
                        // Iteration over values
                            stmt
                                .execute(params![MEASUREMENT_ID, block_id as u32, sensor_id, f_idx_to_freq(freq_idx), 1])
                                .unwrap() as u32
                    )
                    .sum::<u32>()
            )
            .sum::<u32>()
    )
    .sum::<u32>();

    Ok(c)
}
