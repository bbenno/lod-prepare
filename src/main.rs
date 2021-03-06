//! Window raw sensor and calculate FFT.

#![warn(missing_docs)]

use clap::{crate_authors, crate_description, crate_version, App, ArgGroup};
use log::{debug, error, info, trace};
use rusqlite::{params, Connection, OpenFlags, Result, NO_PARAMS};
use rustfft::{num_complex::Complex32, FftPlanner};
use std::f32::consts::PI;

/// Sampling time for N measurements
const T: f64 = 52.39e-3;
/// Block size
const N: usize = 64;

type Window = fn(u32) -> f32;

struct SensorValue {
    id: u32,
    value: Complex32,
}

fn main() -> Result<()> {
    // LOGGER INIT
    env_logger::init();

    let opts = App::new("LOD Prepare")
        .about(crate_description!())
        .author(crate_authors!())
        .version(crate_version!())
        .args_from_usage(
            "<database>                           'Sets the database file to use'
             [hamming]         --hamming          'Sets hamming window function'
             [blackman]        --blackman         'Sets blackman window function'
             [blackman-harris] --blackman-harris  'Sets blackman-harris window function'",
        )
        .group(
            ArgGroup::with_name("window function")
                .args(&["hamming", "blackman", "blackman-harris"])
                .required(false),
        )
        .get_matches();

    // Only one option is present due to the mutual exclusion of the ArgGroup.
    let window: Window = match opts {
        _ if opts.is_present("hamming") => hamming,
        _ if opts.is_present("blackman") => blackman,
        _ if opts.is_present("blackman-harris") => blackman_harris,
        _ => dirichlet,
    };
    let db_name = opts
        .value_of("database")
        .expect("Failed to read line argument \"database\"");

    // DB INIT
    let mut db_conn =
        Connection::open_with_flags(db_name, OpenFlags::SQLITE_OPEN_READ_WRITE).unwrap();
    let tx = db_conn.transaction().unwrap();
    let mut insertion = tx
        .prepare("INSERT INTO `training_value` (`measuring_point_id`, `frequency`, `value`) VALUES (?, ?, ?)")
        .unwrap();
    let mut selection = tx
        .prepare("SELECT `measuring_point_id`, `I`, `Q` FROM `sensor_value` ORDER BY `measuring_point_id`")
        .unwrap();

    // DELETE previous training data
    tx.execute("DELETE FROM `training_value`", NO_PARAMS)
        .expect("Failed deleting the previous training data");

    // FFT INIT
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(N);

    || -> Result<Vec<SensorValue>, &'static str> {
        info!("Read `measured_values` from database");
        // SELECT RAW SENSOR DATA FROM DATABASE
        let input_values = selection
            .query_map(
                params![],
                |row| Ok(SensorValue {
                    id: row.get_unwrap(0),
                    value: Complex32 {
                        re: row.get_unwrap::<usize, u16>(1) as f32,
                        im: row.get_unwrap::<usize, u16>(2) as f32,
                    },
                })
            )
            .expect("database failure while querying input")
            .map(|row_result| row_result.unwrap())
            .collect::<Vec<SensorValue>>();

        let mut buffer = input_values
            .iter()
            .map(|sv| sv.value)
            .collect::<Vec<Complex32>>();

        debug!("{} buffer values", buffer.len());
        trace!("Buffer: {:#?}", buffer);

        if buffer.len() % N != 0 {
            error!("Count of buffer values has to be multiple of {}. Got: {}", N, buffer.len());
            return Err("invalid data length");
        }

        // Calculate mean per chunk
        //   mean = ∑ᴺᵢ₌₁(xᵢ) / N
        let means = buffer
            .chunks_exact(N)
            .map(|c| c.iter().sum::<Complex32>() / (N as f32))
            .collect::<Vec<Complex32>>();
        trace!("Means (per chunk): {:?}", means);

        // INPUT NORMALIZATION: Remove DC offset
        //   xᵢ ↦ xᵢ - mean
        buffer = buffer
            .chunks_exact(N)
            .enumerate()
            .map(|(i, c)|
                c
                    .iter()
                    .map(|cc| cc - means[i])
                    .enumerate()
                    .map(|(i,c)| window(i as u32) * c)
                    .collect::<Vec<Complex32>>()
            )
            .flatten()
            .collect();

        // CALCULATE FFT
        fft.process(&mut buffer);

        // OUTPUT NORMALIZATION
        //   yᵢ ↦ yᵢ / √N
        let output_values = buffer
            .iter()
            .map(|c| c * (N as f32).sqrt().recip())
            .zip(input_values.iter().map(|d| d.id))
            .map(|(c, i)| SensorValue { id: i, value: c})
            .collect::<Vec<SensorValue>>();

        if buffer.is_empty() {
            info!("no FFT input")
        }

        Ok(output_values)
    }()
    .unwrap_or_default()
    // DB INSERTION
    .chunks_exact(N)
    .for_each(|block|
    // Iteration over blocks
        block
            .iter()
            .enumerate()
            .for_each(|(freq_idx, sv)| {
            // Iteration over values
                insertion.execute(params![sv.id, f_idx_to_freq(freq_idx), sv.value.norm() as f64]).unwrap();
            })
    );

    // Drop borrowed statements in order to drop Transaction tx
    drop(insertion);
    drop(selection);

    tx.commit()
}

fn f_idx_to_freq(idx: usize) -> f64 {
    let n: i32 = N as i32;
    let i: i32 = idx as i32;

    (((i + n / 2) % n) - (n / 2)) as f64 / T
}

/// Dirichlet window
///
/// 𝑤(𝑛) = 1,   𝑛 = 0,…,𝑁-1
///
/// # Arguments
///
/// * `n` - index of current input signal in window of width N
///
/// # Example
///
/// ```
/// input
///     .chunks_exact(N)
///     .map(|(index, chunk)| {
///         chunk
///             .iter()
///             .enumerate()
///             .map(|(index, value)| dirichlet(index as u32) * value)
///             .collect()
///     });
/// ```
fn dirichlet(_n: u32) -> f32 {
    1f32
}

/// Blackman window with α = 0.16
///
/// 𝑤(𝑛) = 𝛼₀ − 𝛼₁ × 𝑐𝑜𝑠(2𝜋𝑛 / (𝑁-1)) + 𝛼₂ × 𝑐𝑜𝑠(4𝜋𝑛 / (𝑁-1)),   𝑛 = 0,…,𝑁-1
///
/// * 𝛼₀ = 0.5 × (1 - 𝛼)
/// * 𝛼₁ = 0.5
/// * 𝛼₂ = 0.5 × 𝛼
///
/// # Arguments
///
/// * `n` - index of current input signal in window of width N
///
/// # Example
///
/// ```
/// input
///     .chunks_exact(N)
///     .map(|(index, chunk)| {
///         chunk
///             .iter()
///             .enumerate()
///             .map(|(index, value)| blackman(index as u32) * value)
///             .collect()
///     });
/// ```
fn blackman(n: u32) -> f32 {
    const A: f32 = 0.16;
    const A0: f32 = (1f32 - A) / 2f32;
    const A1: f32 = 0.5f32;
    const A2: f32 = A / 2f32;

    A0 - A1 * ((2f32 * PI * n as f32) / (N - 1) as f32).cos()
        + A2 * ((4f32 * PI * n as f32) / (N - 1) as f32).cos()
}

/// Blackman-Harris      window with α = 0.16
///
/// 𝑤(𝑛) = 𝛼₀ − 𝛼₁ × 𝑐𝑜𝑠(2𝜋𝑛 / (𝑁-1)) + 𝛼₂ × 𝑐𝑜𝑠(4𝜋𝑛 / (𝑁-1)) - 𝛼₃ × 𝑐𝑜𝑠(6𝜋𝑛 / (𝑁-1)),   𝑛 = 0,…,𝑁-1
///
/// * 𝛼₀ = 0.35875
/// * 𝛼₁ = 0.48829
/// * 𝛼₂ = 0.14128
/// * 𝛼₃ = 0.01168
///
/// # Arguments
///
/// * `n` - index of current input signal in window of width N
///
/// # Example
///
/// ```
/// input
///     .chunks_exact(N)
///     .map(|(index, chunk)| {
///         chunk
///             .iter()
///             .enumerate()
///             .map(|(index, value)| blackman_harris(index as u32) * value)
///             .collect()
///     });
/// ```
fn blackman_harris(n: u32) -> f32 {
    const A0: f32 = 0.35875f32;
    const A1: f32 = 0.48829f32;
    const A2: f32 = 0.14128f32;
    const A3: f32 = 0.01168f32;

    A0 - A1 * ((2f32 * PI * n as f32) / (N - 1) as f32).cos()
        + A2 * ((4f32 * PI * n as f32) / (N - 1) as f32).cos()
        - A3 * ((6f32 * PI * n as f32) / (N - 1) as f32).cos()
}

/// Hamming window
///
/// 𝑤(𝑛) = 𝛼 − 𝛽 × 𝑐𝑜𝑠(2𝜋𝑛 / (𝑁-1)),   𝑛 = 0,…,𝑁-1
///
/// * 𝛼 = 25 / 46
/// * 𝛽 = 1 - 𝛼
///
/// # Arguments
///
/// * `n` - index of current input signal in window of width N
///
/// # Example
///
/// ```
/// input
///     .chunks_exact(N)
///     .map(|(index, chunk)| {
///         chunk
///             .iter()
///             .enumerate()
///             .map(|(index, value)| hamming(index as u32) * value)
///             .collect()
///     });
/// ```
fn hamming(n: u32) -> f32 {
    const A: f32 = 25f32 / 46f32;
    const B: f32 = 1f32 - A;

    A - B * ((2f32 * PI * n as f32) / (N - 1) as f32).cos()
}
