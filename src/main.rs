use rusqlite::{Connection, Result, NO_PARAMS};

#[derive(Debug)]
struct Value {
    measurement_id: i32,
    time_counter: i32,
    sensor_id: i32,
    phase: i32,
    value: i32,
}

fn main() -> Result<()> {
    let conn = Connection::open("measurements.sqlite")?;

    let mut stmt = conn.prepare(
        "SELECT measurement_id, time_counter, sensor_id, phase, value FROM measured_values"
    )?;

    let values = stmt.query_map(NO_PARAMS, |row| {
        Ok(Value {
            measurement_id: row.get(0)?,
            time_counter: row.get(1)?,
            sensor_id: row.get(2)?,
            phase: row.get(3)?,
            value: row.get(4)?,
        })
    })?;

    for value in values {
        println!("{:?}", value);
    }

    Ok(())
}
