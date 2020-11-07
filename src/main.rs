//! Window raw sensor and calculate FFT.

#![warn(missing_docs)]

use std::sync::Arc;
use rusqlite::{Connection, MappedRows, OpenFlags, Result, Row, Statement, params};
use config::{FileSourceFile, ConfigError, Config, File as ConfigFile};
use rustfft::{FFT, FFTplanner};
use rustfft::num_complex::Complex32;
use rustfft::num_traits::Zero;

type SensorData = (u32, Vec<Complex32>);

const SENSOR_COUNT: u32 = 5;

fn main() -> Result<()> {
    let config_file = ConfigFile::with_name("config");
    let config = extract_config_from_file(config_file).unwrap();

    let raw_db_conn = Connection::open_with_flags(config.get_str("raw_db_host").unwrap(), OpenFlags::SQLITE_OPEN_READ_WRITE).unwrap();
    let mut data = get_data(&raw_db_conn).unwrap();

    let fft_data = calc_fft(&mut data, config.get("block_size").unwrap()).unwrap();

    println!("FFT: {:#?}", fft_data);

    let training_db_conn = raw_db_conn;
    save_data(&training_db_conn, fft_data).unwrap();

    Ok(())
}

fn extract_config_from_file(file: ConfigFile<FileSourceFile>) -> Result<Config, ConfigError> {
    let mut config = config::Config::default();
    config.merge(file)?;
    Ok(config)
}

fn get_data(db_conn: &Connection) -> Result<Vec<SensorData>> {
    const SQL: &str = "SELECT I, Q FROM sensor_data
    WHERE measurement_id = ?1 AND sensor_id = ?2
    ORDER BY block_id, item_id";
    let mut stmt = db_conn.prepare(SQL).unwrap();
    let data: Vec<_> = (1..=SENSOR_COUNT)
        .filter_map(|i|
            get_sensor_data(&mut stmt, i)
                .map(|vec| (i, vec))
                .ok()
        )
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
where F: FnMut(&Row) -> Result<Complex32> {
    let vec = rows
        .map(|row| row.unwrap())
        .collect();
    Ok(vec)
}

fn convert_sql_row_to_complex(row: &Row) -> Result<Complex32> {
    let re: u16 = row.get(0)?;
    let im: u16 = row.get(1)?;
    Ok(Complex32::new(re as f32, im as f32))
}

fn calc_fft(data: &mut Vec<SensorData>, block_size: usize) -> Result<Vec<SensorData>> {
    let mut planner = FFTplanner::new(false);
    let fft = planner.plan_fft(block_size);

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

fn save_data(db_conn: &Connection, data: Vec<SensorData>) -> Result<u32> {
    const MEASUREMENT_ID: u32 = 1;
    const SQL: &str = "INSERT INTO `training_data` (measurement_id, block_id, sensor_id, frequency, value) VALUES (?1, ?2, ?3, ?4, ?5)";
    let mut stmt = db_conn.prepare(SQL).unwrap();

    let c = data.chunks_exact(64)
        .enumerate()
        .map(|(block_id, block)|
        // Iteration over blocks
            block.into_iter()
                .map(|(sensor_id,v)|
                // Iteration over sensors
                    v.iter()
                        .enumerate()
                        .map(|(freq, val)|
                        // Iteration over values
                            stmt.execute(params![MEASUREMENT_ID, block_id as u32, sensor_id, freq as u32 * 10, 1])
                                .unwrap() as u32
                        )
                        .sum::<u32>()
                )
                .sum::<u32>()
        )
        .sum::<u32>();

    Ok(c)
}
