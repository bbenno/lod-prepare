use rusqlite::{Connection, Result, params};

#[derive(Debug)]
struct Measurement {
    i: i32,
    q: i32,
}

fn main() -> Result<()> {
    let measurement_id = 1;
    let block_size = 64;
    let conn = Connection::open("measurements.db")?;

    let mut stmt = conn.prepare(
        "SELECT I, Q FROM sensor_data
         WHERE measurement_id = ?1 AND sensor_id = ?2
         ORDER BY time_counter"
    ).unwrap();

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
