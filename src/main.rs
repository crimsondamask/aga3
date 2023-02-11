use aga8::composition::Composition;
use aga8::detail::Detail;
use clap::Parser;
use std::time::Instant;
use tokio_modbus::prelude::{sync, *};
mod aga3;
use aga3::*;

/// A script for gas flow rate calculation according to AGA-3.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Modbus server IP address
    #[arg(short, long)]
    ip: String,

    /// Modbus server port. Defaults to 502.
    #[arg(short, long, default_value_t = 502)]
    port: u16,

    /// Start register. Defaults to 0.
    #[arg(long, default_value_t = 0)]
    register: u16,

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
}

fn main() {
    let args = Args::parse();

    let comp = Composition {
        methane: 0.9396,
        nitrogen: 0.02,
        carbon_dioxide: 0.04,
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

    let socket = format!("{}:{}", args.ip, args.port).parse().unwrap();
    let mut ctx = sync::tcp::connect(socket).unwrap();

    let buff = ctx.read_holding_registers(args.register, 10).unwrap();

    let pressure: f32 = {
        let data_32bit_rep = ((buff[0] as u32) << 16) | buff[1] as u32;
        let data_32_array = data_32bit_rep.to_ne_bytes();
        f32::from_ne_bytes(data_32_array)
    };
    let temperature: f32 = {
        let data_32bit_rep = ((buff[2] as u32) << 16) | buff[3] as u32;
        let data_32_array = data_32bit_rep.to_ne_bytes();
        f32::from_ne_bytes(data_32_array)
    };

    // println!("{}", pressure);
    // println!("{}", temperature);

    // let now = Instant::now();

    let mut aga8_test: Detail = Detail::new();
    aga8_test.set_composition(&comp).unwrap();
    aga8_test.p = args.pressure as f64 * 6.89476;
    aga8_test.t = ((args.temperature as f64) - 32.0) * 5.0 / 9.0 + 273.15;

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
        pressure_f: args.pressure,
        temperature_f: args.temperature,
        differential: args.differential,
        sg: args.sg,
        k: args.k,
        z_f,
        z_b,
    };

    let geometry = MeterGeometry {
        orifice_d: args.orifice,
        meter_d: args.bore,
    };
    // println!("Beta =            {}", &geometry.beta());
    // println!("E_v =            {}", &geometry.e_v());

    let aga3 = Aga3 {
        flowing_params: f_params,
        geometry,
    };

    // println!("x =            {}", &aga3.x_factor());
    // println!("Y =            {}", &aga3.y_factor());
    // println!("F mass =            {}", &aga3.mass_flow_factor());
    println!("Zf =            {}", &aga3.z_f());
    println!("Zb =            {}", &aga3.z_b());
    // println!("Sigma f =            {} lbm/ft3", &aga3.sigma_f());
    // println!("Sigma b =            {} lbm/ft3", &aga3.sigma_b());

    let q_v = aga3.q_v_b();
    println!("Q base =            {} ft3/hr", q_v);

    let q_v_metric = (q_v / 35.315) * 24.0;

    // let elapsed = now.elapsed();
    // println!("Elapsed: {:.2?}", elapsed);
    println!("Q base =            {} km3/day", q_v_metric / 1000.0);
    println!(
        "Q base =            {} km3/hr",
        q_v_metric / (24.0 * 1000.0)
    );
}
