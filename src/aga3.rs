use std::f32::consts::FRAC_PI_4;

pub const N3: f32 = 27.7070;
pub const NC: f32 = 323.279;
pub const MR_AIR: f32 = 28.9625;
pub const R_: f32 = 10.7316;
pub const N5: f32 = 459.67;
pub const BASE_P: f32 = 14.73;
pub const BASE_T: f32 = 60.0;
pub const C_D: f32 = 0.62;

pub struct FlowingParams {
    pub pressure_f: f32,
    pub temperature_f: f32,
    pub differential: f32,
    pub sg: f32,
    pub k: f32,
    pub z_f: f32,
    pub z_b: f32,
}

pub struct MeterGeometry {
    pub orifice_d: f32,
    pub meter_d: f32,
}
pub struct Aga3 {
    pub flowing_params: FlowingParams,
    pub geometry: MeterGeometry,
}

impl MeterGeometry {
    pub fn beta(&self) -> f32 {
        self.orifice_d / self.meter_d
    }

    pub fn e_v(&self) -> f32 {
        1.0 / (1.0 - self.beta().powi(4))
    }
}

impl Aga3 {
    pub fn x_factor(&self) -> f32 {
        self.flowing_params.differential / (N3 * self.flowing_params.pressure_f)
    }

    pub fn y_factor(&self) -> f32 {
        let y_p = (0.41 + 0.35 * self.geometry.beta().powi(4)) / self.flowing_params.k;

        let y_factor = 1.0 - (y_p * self.x_factor());

        y_factor
    }

    pub fn mass_flow_factor(&self) -> f32 {
        FRAC_PI_4 * NC * self.geometry.e_v() * (self.geometry.orifice_d.powi(2))
    }

    pub fn z_f(&self) -> f32 {
        self.flowing_params.z_f
    }

    pub fn z_b(&self) -> f32 {
        self.flowing_params.z_b
    }
    pub fn sigma_f(&self) -> f32 {
        let sigma_f = (self.flowing_params.pressure_f * MR_AIR * self.flowing_params.sg)
            / (self.z_f() * R_ * (self.flowing_params.temperature_f + N5));
        sigma_f
    }
    pub fn sigma_b(&self) -> f32 {
        let sigma_b =
            (BASE_P * MR_AIR * self.flowing_params.sg) / (self.z_b() * R_ * (BASE_T + N5));
        sigma_b
    }

    pub fn q_v_b(&self) -> f32 {
        let q_v = (self.mass_flow_factor()
            * self.y_factor()
            * C_D
            * (2.0 * self.flowing_params.differential * self.sigma_f()).sqrt())
            / self.sigma_b();

        q_v
    }
}
