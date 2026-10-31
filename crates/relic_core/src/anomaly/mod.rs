//! Módulo de Detección de Anomalías Causal Seis Ojos en RELIC.

pub mod channel;
pub mod cusum;
pub mod ecdf;
pub mod echo_state;
pub mod peak_decay;
pub mod refractory;
pub mod spatial_bypass;
pub mod topology_coupling;

pub use channel::{SixEyesChannel, TelemetryAssessment};
pub use cusum::TelemetryDriftMonitor;
pub use ecdf::EcdfWindow;
pub use echo_state::{EchoStatePredictor, EMBED_DIM, RESERVOIR_SIZE};
pub use peak_decay::StreamingPeakDecay;
pub use refractory::RefractorySuppressor;
pub use spatial_bypass::SpatialSpikeBypass;
pub use topology_coupling::{
    NodeSpatialNeighborhood, QuiescentCalibrator, ResidualSprtMartingale,
    TopologicalResidualEngine, DEFAULT_TOLERANCE_RATIO, MAX_BOUNDED_NEIGHBORS,
    MAX_SPATIAL_NEIGHBORS, WALD_THRESHOLD_ALPHA_1E6,
};

pub type ReservoirWeights = (
    [[f64; EMBED_DIM]; RESERVOIR_SIZE],
    [[f64; RESERVOIR_SIZE]; RESERVOIR_SIZE],
);

/// Carga estática de matrices W_in y W_res desde el archivo binario exportado.
#[allow(clippy::needless_range_loop)]
pub fn load_weights_from_bytes(bytes: &[u8]) -> Result<ReservoirWeights, &'static str> {
    const WIN_LEN: usize = RESERVOIR_SIZE * EMBED_DIM; // 1024 floats = 8192 bytes
    const WRES_LEN: usize = RESERVOIR_SIZE * RESERVOIR_SIZE; // 4096 floats = 32768 bytes
    const MIN_BYTES: usize = (WIN_LEN + WRES_LEN) * 8; // 40960 bytes

    if bytes.len() < MIN_BYTES {
        return Err("Buffer de pesos demasiado corto");
    }

    let mut win = [[0.0; EMBED_DIM]; RESERVOIR_SIZE];
    let mut wres = [[0.0; RESERVOIR_SIZE]; RESERVOIR_SIZE];

    let mut offset = 0;
    for i in 0..RESERVOIR_SIZE {
        for j in 0..EMBED_DIM {
            if offset + 8 > bytes.len() {
                return Err("Buffer truncado en W_in");
            }
            let chunk: [u8; 8] = match bytes[offset..offset + 8].try_into() {
                Ok(arr) => arr,
                Err(_) => return Err("Fallo de conversión de chunk en W_in"),
            };
            win[i][j] = f64::from_le_bytes(chunk);
            offset += 8;
        }
    }

    for i in 0..RESERVOIR_SIZE {
        for j in 0..RESERVOIR_SIZE {
            if offset + 8 > bytes.len() {
                return Err("Buffer truncado en W_res");
            }
            let chunk: [u8; 8] = match bytes[offset..offset + 8].try_into() {
                Ok(arr) => arr,
                Err(_) => return Err("Fallo de conversión de chunk en W_res"),
            };
            wres[i][j] = f64::from_le_bytes(chunk);
            offset += 8;
        }
    }

    Ok((win, wres))
}

pub static WEIGHTS_SEED42_BYTES: &[u8] = include_bytes!("data/weights_seed42.bin");
pub static WEIGHTS_SEED7_BYTES: &[u8] = include_bytes!("data/weights_seed7.bin");
pub static WEIGHTS_SEED99_BYTES: &[u8] = include_bytes!("data/weights_seed99.bin");

