use aga8::composition::Composition;
use aga8::detail::Detail;
use chrono;
use clap::Parser;
use env_logger;
use log::info;
use std::thread;
use tokio_modbus::prelude::{sync, *};
mod aga3;
use aga3::*;
use std::io::{self, Write};

/// A script for gas flow rate calculation according to AGA-3.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Verbose. Prints rate values after each calculation.
    #[arg(short, long)]
    verbose: bool,

    /// If selected, values will be read from a Modbus block in the order: Pressure, temperature, diff, SG, orifice, linebore. Starting from the register selected by --register.
    #[arg(long)]
    modbus: bool,

    /// Modbus server IP address
    #[arg(short, long)]
    ip: String,

    /// Modbus server port. Defaults to 502.
    #[arg(short, long, default_value_t = 502)]
    port: u16,

    /// Start register for reading values. Defaults to 0.
    #[arg(long, default_value_t = 0)]
    register: u16,

    /// Start register for writing values. Defaults to 8.
    #[arg(long, default_value_t = 8)]
    write: u16,

    /// Orifice plate diameter
    #[arg(short, long)]
    orifice: f32,

    /// Meter internal diameter
    #[arg(short, long)]
    bore: f32,

    /// Flowing pressure
    #[arg(long)]
    pressure: f32,

    /// Flowing temperature
    #[arg(short, long)]
    temperature: f32,

    /// Flowing differential pressure
    #[arg(short, long)]
    differential: f32,

    /// Specific gravity
    #[arg(short, long)]
    sg: f32,

    /// Isentropic coefficient
    #[arg(long, default_value_t = 1.3)]
    k: f32,

    /// Compressibility factor

    #[arg(long)]
    zf: Option<f32>,

    /// Compressibility factor at base conditions
    #[arg(long, default_value_t = 1.0)]
    zb: f32,

    /// Scan rate in milliseconds. Defaults to 1000.
    #[arg(long, default_value_t = 1000)]
    scan: u64,
}

fn main() {
    env_logger::init();

    let args = Args::parse();

    info!(" [!] Parsed arguments.");

    let comp = Composition {
        methane: 0.9996,
        nitrogen: 0.00,
        carbon_dioxide: 0.00,
        ethane: 0.0,
        propane: 0.0,
        isobutane: 0.0,
        n_butane: 0.0,
        isopentane: 0.0,
        n_pentane: 0.0,
        hexane: 0.0,
        heptane: 0.0,
        octane: 0.0,
        nonane: 0.0,
        decane: 0.0,
        hydrogen: 0.0,
        oxygen: 0.0,
        carbon_monoxide: 0.0,
        water: 0.0,
        hydrogen_sulfide: 0.0004,
        helium: 0.0,
        argon: 0.0,
    };

    let mut aga8_test: Detail = Detail::new();

    let socket = format!("{}:{}", args.ip, args.port).parse().unwrap();
    let mut ctx = sync::tcp::connect(socket).unwrap();

    loop {
        let mut pressure: f32 = 0.0;
        let mut temperature: f32 = 0.0;
        let mut differential: f32 = 0.0;
        let mut sg: f32 = 0.0;
        let mut orifice: f32 = 0.0;
        let mut bore: f32 = 0.0;

        if args.modbus {
            let buff = ctx.read_holding_registers(args.register, 12).unwrap();
            pressure = u16_to_f32(buff[0], buff[1]);
            temperature = u16_to_f32(buff[2], buff[3]);
            differential = u16_to_f32(buff[4], buff[5]);
            sg = u16_to_f32(buff[6], buff[7]);
            orifice = u16_to_f32(buff[8], buff[9]);
            bore = u16_to_f32(buff[10], buff[11]);
        } else {
            pressure = args.pressure;
            temperature = args.temperature;
            differential = args.differential;
            sg = args.sg;
            orifice = args.orifice;
            bore = args.bore;
        }

        aga8_test.set_composition(&comp).unwrap();
        aga8_test.p = pressure as f64 * 6.89476;
        aga8_test.t = ((temperature as f64) - 32.0) * 5.0 / 9.0 + 273.15;

        aga8_test.density();
        aga8_test.properties();
        let z_f = match args.zf {
            Some(z_factor) => z_factor,
            None => aga8_test.z as f32,
        };

        aga8_test.p = BASE_P as f64 * 6.89476;
        aga8_test.t = ((BASE_T as f64) - 32.0) * 5.0 / 9.0 + 273.15;
        aga8_test.density();
        aga8_test.properties();

        let z_b = aga8_test.z as f32;

        let f_params: FlowingParams = FlowingParams {
            pressure_f: pressure,
            temperature_f: temperature,
            differential,
            sg,
            k: args.k,
            z_f,
            z_b,
        };

        let geometry = MeterGeometry {
            orifice_d: orifice,
            meter_d: bore,
        };

        let aga3 = Aga3 {
            flowing_params: f_params,
            geometry,
        };

        // println!("Zf =            {}", &aga3.z_f());
        // println!("Zb =            {}", &aga3.z_b());

        let q_v = aga3.q_v_b();

        let q_v_metric = (q_v / 35.315) * 24.0;
        let q_v_metric_h = q_v / 35.315;

        // println!("{:?}", f32_to_u16(q_v_metric));

        if args.modbus {
            let _ = ctx.write_multiple_registers(args.write, &f32_to_u16(q_v_metric));
        }

        thread::sleep(std::time::Duration::from_millis(args.scan));

        if args.verbose {
            let now = chrono::Local::now();
            io::stdout().flush().unwrap();
            print!(
                "\r[{}]    Qv = {} Sm3/h,    Qv = {} Sm3/d",
                now, q_v_metric_h, q_v_metric
            );
        }
    }
}

fn f32_to_u16(f_number: f32) -> [u16; 2] {
    let bits = f_number.to_bits();

    let first = ((bits >> 16) & 0xffff) as u16;
    let second = (bits & 0xffff) as u16;

    [first, second]
}

fn u16_to_f32(first: u16, second: u16) -> f32 {
    let data_32bit_rep = ((first as u32) << 16) | second as u32;
    let data_32_array = data_32bit_rep.to_ne_bytes();
    f32::from_ne_bytes(data_32_array)
}
