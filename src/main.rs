use rusqlite::{Connection, Result, params, OpenFlags};
use config::File as ConfigFile;
use rustfft::FFTplanner;
use rustfft::num_complex::Complex32;
use rustfft::num_traits::Zero;

struct Config {
    block_size: usize,
    raw_db: String,
    training_db: String
}

fn main() -> Result<()> {
    const MEASUREMENT_ID: u32 = 1;

    let config = read_config("config").expect("Failed to load init config file");

    let mut data = get_data(&config.raw_db, MEASUREMENT_ID).unwrap();

    let mut fft: [Vec<Complex32>; 5] = Default::default();

    for sensor in 1..=5 {
        fft [sensor - 1] = calc_fft(&mut data[sensor - 1], config.block_size)?;
    }

    println!("FFT: {:#?}", fft);

    Ok(())
}

fn read_config(path: &str) -> Result<Config> {
    let mut settings = config::Config::default();
    settings.merge(ConfigFile::with_name(path)).expect("Failed to find init config file");

    Ok(Config {
        block_size: settings.get_int("block_size").expect("No block size configured") as usize,
        raw_db: settings.get_str("raw_db_host").expect("No database of the raw sensor data configured"),
        training_db: settings.get_str("training_db_host").expect("No database for the trainingsdata configured")
    })
}

fn get_data(db_path: &str, measurement_id: u32) -> Result<[Vec<Complex32>; 5]> {
    let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_WRITE).expect("Failed to open database connection");

    let mut stmt = conn.prepare(
        "SELECT I, Q FROM sensor_data
         WHERE measurement_id = ?1 AND sensor_id = ?2
         ORDER BY time_counter"
    ).expect("Failed preparing SELECT statement");

    let mut data: [Vec<Complex32>; 5] = Default::default();

    for sensor_id in 1..=5 {
        let measurements = stmt.query_map(
            params![measurement_id, sensor_id as u32],
            |row| -> Result<(u16,u16)> { Ok((row.get(0)?, row.get(1)?)) }
        )?.map(|m| m.unwrap()).map(|m| Complex32::new(m.0 as f32, m.1 as f32)).collect::<Vec<_>>();

        data[sensor_id - 1] = measurements;
    }

    Ok(data)
}

fn calc_fft(mut input: &mut Vec<Complex32>, block_size: usize) -> Result<Vec<Complex32>> {
    let mut output: Vec<Complex32> = vec![Zero::zero(); input.len()];

    let mut planner = FFTplanner::new(false);
    let fft = planner.plan_fft(block_size);
    fft.process_multi(&mut input, &mut output);

    Ok(output)
}
