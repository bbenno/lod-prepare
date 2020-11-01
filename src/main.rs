use rusqlite::{Connection, Result, params, OpenFlags};
use config::File as ConfigFile;

#[derive(Debug)]
struct Measurement {
    i: i32,
    q: i32,
}

struct Config {
    block_size: usize,
    db_host: String,
}

fn main() -> Result<()> {
    const MEASUREMENT_ID: u32 = 1;

    let config = read_config("config").expect("Failed to load init config file");

    let data = get_data(&config.db_host, MEASUREMENT_ID).unwrap();

    for sensor in 1..=5 {
        println!("Sensor {}", sensor);

        for chunk in data[sensor - 1].chunks(config.block_size).into_iter() {
            calc_fft(chunk);
        }
    }

    Ok(())
}

fn read_config(path: &str) -> Result<Config> {
    let mut settings = config::Config::default();
    settings.merge(ConfigFile::with_name(path)).expect("Failed to find init config file");

    Ok(Config {
        block_size: settings.get_int("block_size").expect("No block size configured") as usize,
        db_host: settings.get_str("database_host").expect("No database host configured"),
    })
}

fn get_data(db_path: &str, measurement_id: u32) -> Result<[Vec<Measurement>; 5]> {
    let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_WRITE).expect("Failed to open database connection");

    let mut stmt = conn.prepare(
        "SELECT I, Q FROM sensor_data
         WHERE measurement_id = ?1 AND sensor_id = ?2
         ORDER BY time_counter"
    ).expect("Failed preparing SELECT statement");

    let mut data: [Vec<Measurement>; 5] = Default::default();

    for sensor_id in 1..=5 {
        let measurements = stmt.query_map(params![measurement_id, sensor_id as u32], |row| {
            Ok(Measurement {
                i: row.get(0)?,
                q: row.get(1)?,
            })
        })?.map(|m| m.unwrap()).collect::<Vec<_>>();

        data[sensor_id - 1] = measurements;
    }

    Ok(data)
}

fn calc_fft(chunk: &[Measurement]) {
    println!("Chunk {:?}", chunk);
}
