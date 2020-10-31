use rusqlite::{Connection, Result, params, OpenFlags};
use config::File as ConfigFile;
use config::Config;

#[derive(Debug)]
struct Measurement {
    i: i32,
    q: i32,
}

fn main() -> Result<()> {
    let mut settings = Config::default();
    settings
        .merge(ConfigFile::with_name("config"))
        .expect("Failed to load init config file");

    let measurement_id = 1;
    let block_size = settings.get_int("block_size").expect("No block size configured") as usize;
    let path = settings.get_str("database_host").expect("No database host configured");
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_WRITE).expect("Failed to open database connection");

    let mut stmt = conn.prepare(
        "SELECT I, Q FROM sensor_data
         WHERE measurement_id = ?1 AND sensor_id = ?2
         ORDER BY time_counter"
    ).expect("Failed preparing SELECT statement");

    for sensor_id in 1..=5 {
        let measurements = stmt.query_map(params![measurement_id, sensor_id], |row| {
            Ok(Measurement {
                i: row.get(0)?,
                q: row.get(1)?,
            })
        })?.collect::<Vec<_>>();

        let m_chunks = measurements.chunks(block_size);

        println!("Sensor {}", sensor_id);
        for measurement_block in m_chunks {
            println!("{:?}", measurement_block);
        }
    }

    Ok(())
}
