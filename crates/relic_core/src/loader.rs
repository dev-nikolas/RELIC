//! Cargador zero-copy de pesos en formato Safetensors con memmap2.
//! Menos de 180 lineas.

use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use memmap2::Mmap;
use serde::Deserialize;
use crate::model::RelicModel;

#[derive(Deserialize, Debug)]
struct TensorEntry {
    dtype: String,
    #[allow(dead_code)]
    shape: Vec<usize>,
    data_offsets: [usize; 2],
}

pub struct SafetensorsLoader {
    mmap: Mmap,
    header: HashMap<String, TensorEntry>,
    data_base_offset: usize,
}

impl SafetensorsLoader {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let file = File::open(path).map_err(|e| format!("Error abriendo archivo: {e}"))?;
        let mmap = unsafe { Mmap::map(&file).map_err(|e| format!("Error mapeando memoria: {e}"))? };

        if mmap.len() < 8 {
            return Err("Archivo Safetensors demasiado pequeno".into());
        }

        let header_len = u64::from_le_bytes(mmap[0..8].try_into().unwrap()) as usize;
        let data_base_offset = 8 + header_len;
        if mmap.len() < data_base_offset {
            return Err("Encabezado de Safetensors excede longitud de archivo".into());
        }

        let header_bytes = &mmap[8..data_base_offset];
        let header: HashMap<String, TensorEntry> = serde_json::from_slice(header_bytes)
            .map_err(|e| format!("Error parseando JSON de Safetensors: {e}"))?;

        Ok(Self {
            mmap,
            header,
            data_base_offset,
        })
    }

    /// Lee un tensor como vector de f32 (soporta F32 y conversion desde F16).
    pub fn get_tensor_f32(&self, name: &str) -> Result<Vec<f32>, String> {
        let entry = self
            .header
            .get(name)
            .ok_or_else(|| format!("Tensor '{name}' no encontrado en Safetensors"))?;

        let start = self.data_base_offset + entry.data_offsets[0];
        let end = self.data_base_offset + entry.data_offsets[1];
        let raw_bytes = &self.mmap[start..end];

        match entry.dtype.as_str() {
            "F32" => {
                let num_floats = raw_bytes.len() / 4;
                let mut data = Vec::with_capacity(num_floats);
                for chunk in raw_bytes.chunks_exact(4) {
                    data.push(f32::from_le_bytes(chunk.try_into().unwrap()));
                }
                Ok(data)
            }
            "F16" => {
                let num_floats = raw_bytes.len() / 2;
                let mut data = Vec::with_capacity(num_floats);
                for chunk in raw_bytes.chunks_exact(2) {
                    let bits = u16::from_le_bytes(chunk.try_into().unwrap());
                    data.push(half_to_f32(bits));
                }
                Ok(data)
            }
            other => Err(format!("Dtype no soportado: {other}")),
        }
    }

    /// Carga todos los pesos del Safetensors dentro de una instancia de RelicModel.
    pub fn load_into_model(&self, model: &mut RelicModel) -> Result<(), String> {
        model.embed_tokens = self.get_tensor_f32("model.embed_tokens.weight")?;
        model.norm.weight = self.get_tensor_f32("model.norm.weight")?;
        model.lm_head = self.get_tensor_f32("lm_head.weight")?;

        for i in 0..model.config.num_hidden_layers {
            let p = format!("model.layers.{i}");
            let layer = &mut model.layers[i];

            layer.input_layernorm.weight = self.get_tensor_f32(&format!("{p}.input_layernorm.weight"))?;
            layer.post_attention_layernorm.weight = self.get_tensor_f32(&format!("{p}.post_attention_layernorm.weight"))?;

            layer.self_attn.q_proj = self.get_tensor_f32(&format!("{p}.self_attn.q_proj.weight"))?;
            layer.self_attn.k_proj = self.get_tensor_f32(&format!("{p}.self_attn.k_proj.weight"))?;
            layer.self_attn.v_proj = self.get_tensor_f32(&format!("{p}.self_attn.v_proj.weight"))?;
            layer.self_attn.o_proj = self.get_tensor_f32(&format!("{p}.self_attn.o_proj.weight"))?;
            layer.self_attn.q_norm.weight = self.get_tensor_f32(&format!("{p}.self_attn.q_norm.weight"))?;
            layer.self_attn.k_norm.weight = self.get_tensor_f32(&format!("{p}.self_attn.k_norm.weight"))?;

            layer.mlp.gate_proj = self.get_tensor_f32(&format!("{p}.mlp.gate_proj.weight"))?;
            layer.mlp.up_proj = self.get_tensor_f32(&format!("{p}.mlp.up_proj.weight"))?;
            layer.mlp.down_proj = self.get_tensor_f32(&format!("{p}.mlp.down_proj.weight"))?;
        }
        Ok(())
    }
}

/// Conversion determinista IEEE 754 de float16 (half) a float32.
fn half_to_f32(h: u16) -> f32 {
    let sign = ((h >> 15) & 1) as u32;
    let exp = ((h >> 10) & 0x1f) as u32;
    let mant = (h & 0x3ff) as u32;

    if exp == 0 {
        if mant == 0 {
            f32::from_bits(sign << 31)
        } else {
            let mut m = mant;
            let mut e = 0;
            while (m & 0x400) == 0 {
                m <<= 1;
                e += 1;
            }
            let f_exp = (127 - 15 + 1 - e) << 23;
            let f_mant = (m & 0x3ff) << 13;
            f32::from_bits((sign << 31) | f_exp | f_mant)
        }
    } else if exp == 31 {
        let f_exp = 0xff << 23;
        let f_mant = mant << 13;
        f32::from_bits((sign << 31) | f_exp | f_mant)
    } else {
        let f_exp = (exp + 127 - 15) << 23;
        let f_mant = mant << 13;
        f32::from_bits((sign << 31) | f_exp | f_mant)
    }
}
