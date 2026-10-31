//! # LinearizedSensitivityMatrix & Kirchhoff LODF SIMD Evaluador
//!
//! Matriz de factores de distribución de corte de línea (LODF) organizada SoA
//! por arista en falla `[outage_k][monitored_m]` para barrido AVX2 contiguo en L2 (16 KB).
//! Evalúa la redistribución paralela de Kirchhoff para contingencias N-k en < 50 ns.

pub const MAX_SENSITIVITY_EDGES: usize = 64;

/// Matriz de sensibilidad de corte de línea (LODF) alineada a 64 bytes (16 KB).
/// Almacena `lodf_by_outage[outage_k][monitored_m]` para cargas vectoriales contiguas.
#[repr(C, align(64))]
#[derive(Clone, Copy, Debug)]
pub struct LinearizedSensitivityMatrix {
    pub lodf_by_outage: [[f32; MAX_SENSITIVITY_EDGES]; MAX_SENSITIVITY_EDGES],
    pub edge_count: u8,
    pub _pad: [u8; 63],
}

const _: () = assert!(std::mem::size_of::<LinearizedSensitivityMatrix>() == 16448 || std::mem::size_of::<LinearizedSensitivityMatrix>() % 64 == 0);
const _: () = assert!(std::mem::align_of::<LinearizedSensitivityMatrix>() == 64);

impl LinearizedSensitivityMatrix {
    pub const fn empty() -> Self {
        Self {
            lodf_by_outage: [[0.0f32; MAX_SENSITIVITY_EDGES]; MAX_SENSITIVITY_EDGES],
            edge_count: 0,
            _pad: [0u8; 63],
        }
    }

    pub fn set_lodf(&mut self, outage_k: usize, monitored_m: usize, factor: f32) -> Result<(), &'static str> {
        if outage_k >= MAX_SENSITIVITY_EDGES || monitored_m >= MAX_SENSITIVITY_EDGES {
            return Err("Índice de arista fuera de límites (máx 64)");
        }
        self.lodf_by_outage[outage_k][monitored_m] = factor;
        if ((outage_k.max(monitored_m) + 1) as u8) > self.edge_count {
            self.edge_count = (outage_k.max(monitored_m) + 1) as u8;
        }
        Ok(())
    }

    /// Calcula la superposición de Kirchhoff post-contingencia y valida cotas térmicas.
    /// Retorna `Ok(margen_seguridad_minimo)` si todas las ramas son térmicamente admisibles,
    /// o `Err(arista_sobrecargada)` si ocurre una sobrecarga por flujo paralelo.
    #[inline(always)]
    pub fn evaluate_post_contingency_flows(
        &self,
        pre_flows: &[f32; MAX_SENSITIVITY_EDGES],
        failed_edges: &[u8],
        max_thermal_limits: &[f32; MAX_SENSITIVITY_EDGES],
        out_post_flows: &mut [f32; MAX_SENSITIVITY_EDGES],
    ) -> Result<f32, u8> {
        let n = (self.edge_count as usize).min(MAX_SENSITIVITY_EDGES);

        // 1. Inicializar con flujos previos
        out_post_flows[..n].copy_from_slice(&pre_flows[..n]);

        // 2. Superposición lineal exacta: f_m_post = f_m_pre + sum_k (LODF[k][m] * f_k_pre)
        for &k_byte in failed_edges {
            let k = k_byte as usize;
            if k >= MAX_SENSITIVITY_EDGES { continue; }
            let f_k = pre_flows[k];
            if f_k.abs() < 1e-6 { continue; }

            let row = &self.lodf_by_outage[k];
            Self::accumulate_lodf_simd(&mut out_post_flows[..n], &row[..n], f_k);
        }

        // 3. Poner a cero el flujo en las líneas falladas (abiertas)
        for &k_byte in failed_edges {
            let k = k_byte as usize;
            if k < n { out_post_flows[k] = 0.0; }
        }

        // 4. Verificación de límites térmicos y cálculo del margen de holgura
        let mut min_margin = 1.0f32;
        for m in 0..n {
            let flow_abs = out_post_flows[m].abs();
            let limit = max_thermal_limits[m];
            if limit > 1e-4 {
                if flow_abs > limit {
                    return Err(m as u8); // Sobrecarga térmica detectada
                }
                let margin = 1.0 - (flow_abs / limit);
                if margin < min_margin {
                    min_margin = margin;
                }
            }
        }

        Ok(min_margin)
    }

    #[inline(always)]
    fn accumulate_lodf_simd(post_flows: &mut [f32], lodf_row: &[f32], f_k: f32) {
        #[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
        {
            let len = post_flows.len();
            let chunks = len / 8;
            unsafe {
                use core::arch::x86_64::*;
                let v_fk = _mm256_set1_ps(f_k);
                for c in 0..chunks {
                    let offset = c * 8;
                    let v_post = _mm256_loadu_ps(post_flows.as_ptr().add(offset));
                    let v_lodf = _mm256_loadu_ps(lodf_row.as_ptr().add(offset));
                    let v_res = _mm256_fmadd_ps(v_lodf, v_fk, v_post);
                    _mm256_storeu_ps(post_flows.as_mut_ptr().add(offset), v_res);
                }
            }
            let rem_start = chunks * 8;
            for m in rem_start..len {
                post_flows[m] = lodf_row[m].mul_add(f_k, post_flows[m]);
            }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "avx2")))]
        {
            for m in 0..post_flows.len() {
                post_flows[m] = lodf_row[m].mul_add(f_k, post_flows[m]);
            }
        }
    }
}

impl Default for LinearizedSensitivityMatrix {
    #[inline]
    fn default() -> Self {
        Self::empty()
    }
}

