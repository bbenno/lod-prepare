//! Window raw sensor and calculate FFT.

#![warn(missing_docs)]

use rusqlite::{Connection, MappedRows, OpenFlags, Result, Row, Statement, params};
use config::{FileSourceFile, ConfigError, Config, File as ConfigFile};
use rustfft::FFTplanner;
use rustfft::num_complex::Complex32;
use rustfft::num_traits::Zero;

const SENSOR_COUNT: usize = 5;

fn main() -> Result<()> {
    let config_file = ConfigFile::with_name("config");
    let config = extract_config_from_file(config_file).unwrap();

    let raw_db_conn = Connection::open_with_flags(config.get_str("raw_db_host").unwrap(), OpenFlags::SQLITE_OPEN_READ_WRITE).unwrap();
    let mut data = get_data(raw_db_conn).unwrap();

    let fft_data = calc_fft(&mut data, config.get("block_size").unwrap());

    println!("FFT: {:#?}", fft_data);

    Ok(())
}

fn extract_config_from_file(file: ConfigFile<FileSourceFile>) -> Result<Config, ConfigError> {
    let mut config = config::Config::default();
    config.merge(file)?;
    Ok(config)
}

fn get_data(db_conn: Connection) -> Result<[Vec<Complex32>; SENSOR_COUNT]> {
    let mut data: [Vec<Complex32>; SENSOR_COUNT] = Default::default();
    const SQL: &str = "SELECT I, Q FROM sensor_data
    WHERE measurement_id = ?1 AND sensor_id = ?2
    ORDER BY time_counter";
    let stmt = db_conn.prepare(SQL);
    let mut stmt = stmt.unwrap();
    let index_to_id = |idx: usize| -> u32 { idx as u32 + 1 };

    let iter = 0..SENSOR_COUNT;
    iter.for_each(|i| data[i] = get_sensor_data(&mut stmt, index_to_id(i)).unwrap());

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
where F: FnMut(&Row<'_>) -> Result<Complex32> {
    let vec = rows
        .map(|row| row.unwrap())
        .collect();
    Ok(vec)
}

fn convert_sql_row_to_complex(row: &Row<'_>) -> Result<Complex32> {
    let re: u16 = row.get(0)?;
    let im: u16 = row.get(1)?;
    Ok(Complex32::new(re as f32, im as f32))
}

fn calc_fft(data: &mut [Vec<Complex32>], block_size: usize) -> Result<[Vec<Complex32>; SENSOR_COUNT]> {
    let mut fft: [Vec<Complex32>; SENSOR_COUNT] = Default::default();

    let iter = data.iter_mut().map(|d| calc_sensor_fft(d, block_size));
    iter.enumerate().for_each(|(i,d)| fft[i] = d.unwrap() );
    Ok(fft)
}

fn calc_sensor_fft(input: &mut Vec<Complex32>, block_size: usize) -> Result<Vec<Complex32>> {
    let mut output: Vec<Complex32> = vec![Zero::zero(); input.len()];

    let mut planner = FFTplanner::new(false);
    let fft = planner.plan_fft(block_size);
    fft.process_multi(input, &mut output);

    Ok(output)
}
