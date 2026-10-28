//! Proyección en RAM y verificación de integridad O(N) del Engrama JSONL.
//! Menos de 180 líneas.

use std::collections::{HashMap, HashSet};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use relic_core::{ArithmeticOp, EpisodicMemory};
use crate::genesis::build_genesis_entries;
use crate::log::{EntryStatus, IntegrityError, LexiconPayload, LogEntry, GENESIS_PREV_HASH};

/// Vista proyectada en RAM del estado activo (sin entradas revertidas ni aristas podadas).
#[derive(Debug, Clone)]
pub struct LexiconView {
    pub episodic: EpisodicMemory,
    pub op_aliases: HashMap<String, ArithmeticOp>,
    pub op_alias_weights: HashMap<String, (f64, u64)>,
    pub phrase_macros: Vec<(String, String)>,
    pub entity_aliases: HashMap<String, String>,
    /// Mapa determinista `(from, to) -> (relation, weight, usage_count)` de hechos activos.
    pub active_facts: HashMap<(String, String), (String, f64, u64)>,
    pub current_version: u64,
    pub head_hash: String,
}

impl Default for LexiconView {
    fn default() -> Self {
        Self {
            episodic: EpisodicMemory::new(),
            op_aliases: HashMap::new(),
            op_alias_weights: HashMap::new(),
            phrase_macros: Vec::new(),
            entity_aliases: HashMap::new(),
            active_facts: HashMap::new(),
            current_version: 0,
            head_hash: GENESIS_PREV_HASH.to_string(),
        }
    }
}

impl LexiconView {
    /// Resuelve si una palabra corresponde a un alias de operación aritmética aprendido.
    pub fn resolve_op_alias(&self, word: &str) -> Option<ArithmeticOp> {
        let key = word.trim().to_lowercase();
        self.op_aliases.get(&key).copied()
    }

    /// Expande macros de frases compuestas aprendidas en orden de longitud descendente.
    pub fn apply_phrase_macros(&self, text: &str) -> String {
        let mut out = text.to_lowercase();
        for (phrase, expansion) in &self.phrase_macros {
            if out.contains(phrase.as_str()) {
                out = out.replace(phrase.as_str(), expansion.as_str());
            }
        }
        out
    }
}

/// Almacén transaccional del engrama con bitácora append-only y vista proyectada.
#[derive(Debug, Clone)]
pub struct EngramStore {
    entries: Vec<LogEntry>,
    active_view: LexiconView,
    backing_file: Option<PathBuf>,
}

impl Default for EngramStore {
    fn default() -> Self {
        Self::with_genesis()
    }
}

impl EngramStore {
    /// Crea un almacén completamente vacío (versión 0, hash génesis).
    pub fn new_empty() -> Self {
        Self {
            entries: Vec::new(),
            active_view: LexiconView::default(),
            backing_file: None,
        }
    }

    /// Crea un almacén inicializado con los 13 hechos de la semilla génesis verificada.
    pub fn with_genesis() -> Self {
        let entries = build_genesis_entries();
        let active_view = Self::rebuild_active_view(&entries)
            .expect("La semilla génesis debe ser criptográficamente válida");
        Self {
            entries,
            active_view,
            backing_file: None,
        }
    }

    /// Carga y verifica toda la cadena de hashes desde un lector `BufRead` en $O(N)$.
    pub fn from_jsonl_reader<R: BufRead>(reader: R) -> Result<Self, IntegrityError> {
        let mut entries = Vec::new();
        for (idx, line_res) in reader.lines().enumerate() {
            let line_num = idx + 1;
            let line = line_res.map_err(|e| IntegrityError::IoError(e.to_string()))?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let entry: LogEntry = serde_json::from_str(trimmed).map_err(|e| {
                IntegrityError::ParseError {
                    line: line_num,
                    details: e.to_string(),
                }
            })?;
            entries.push(entry);
        }
        let active_view = Self::rebuild_active_view(&entries)?;
        Ok(Self {
            entries,
            active_view,
            backing_file: None,
        })
    }

    /// Carga y verifica un documento JSONL desde un `&str`.
    pub fn from_jsonl_str(content: &str) -> Result<Self, IntegrityError> {
        Self::from_jsonl_reader(content.as_bytes())
    }

    /// Abre un archivo `.jsonl` existente verificando su integridad, o lo inicializa con génesis si no existe.
    pub fn open_or_init_file(path: impl AsRef<Path>) -> Result<Self, IntegrityError> {
        let path_buf = path.as_ref().to_path_buf();
        if path_buf.exists() {
            let file = File::open(&path_buf).map_err(|e| IntegrityError::IoError(e.to_string()))?;
            let mut store = Self::from_jsonl_reader(BufReader::new(file))?;
            store.backing_file = Some(path_buf);
            Ok(store)
        } else {
            let mut store = Self::with_genesis();
            let serialized = store.to_jsonl_string();
            let mut file = File::create(&path_buf).map_err(|e| IntegrityError::IoError(e.to_string()))?;
            file.write_all(serialized.as_bytes())
                .map_err(|e| IntegrityError::IoError(e.to_string()))?;
            store.backing_file = Some(path_buf);
            Ok(store)
        }
    }

    /// Reconstruye la vista activa hasta `max_version` (o toda la bitácora si `None`).
    pub fn rebuild_view_at_version(
        entries: &[LogEntry],
        max_version: Option<u64>,
    ) -> Result<LexiconView, IntegrityError> {
        let mut expected_prev = GENESIS_PREV_HASH.to_string();
        let mut active_entries: Vec<(u64, &LogEntry)> = Vec::with_capacity(entries.len());
        let mut last_ver = 0;

        for (idx, entry) in entries.iter().enumerate() {
            let expected_version = (idx + 1) as u64;
            entry.verify(expected_version, &expected_prev)?;
            if let Some(limit) = max_version {
                if entry.version > limit {
                    break;
                }
            }
            expected_prev = entry.entry_hash.clone();
            last_ver = entry.version;

            match &entry.payload {
                LexiconPayload::Rollback { target_version, .. } => {
                    if *target_version >= entry.version {
                        return Err(IntegrityError::InvalidRollbackTarget {
                            version: entry.version,
                            target_version: *target_version,
                        });
                    }
                    active_entries.retain(|(v, _)| *v <= *target_version);
                }
                _ => {
                    if entry.status != EntryStatus::Reverted {
                        active_entries.push((entry.version, entry));
                    }
                }
            }
        }

        let mut view = LexiconView {
            episodic: EpisodicMemory::new(),
            op_aliases: HashMap::new(),
            op_alias_weights: HashMap::new(),
            phrase_macros: Vec::new(),
            entity_aliases: HashMap::new(),
            active_facts: HashMap::new(),
            current_version: last_ver,
            head_hash: expected_prev,
        };

        let mut pruned_set: HashSet<(String, String)> = HashSet::new();

        for (_, entry) in &active_entries {
            match &entry.payload {
                LexiconPayload::GraphFact {
                    from,
                    relation,
                    to,
                    weight,
                    usage_count,
                } => {
                    let key = (from.clone(), to.clone());
                    pruned_set.remove(&key);
                    view.active_facts
                        .insert(key, (relation.clone(), *weight, *usage_count));
                }
                LexiconPayload::EdgeReinforcement {
                    from,
                    to,
                    new_weight,
                    usage_count,
                } => {
                    let key = (from.clone(), to.clone());
                    if let Some(fact) = view.active_facts.get_mut(&key) {
                        fact.1 = *new_weight;
                        fact.2 = *usage_count;
                    }
                }
                LexiconPayload::ConsolidationCheckpoint { pruned_edges, .. } => {
                    for edge in pruned_edges {
                        view.active_facts.remove(edge);
                        pruned_set.insert(edge.clone());
                    }
                }
                LexiconPayload::OpAlias {
                    alias_word,
                    target_op,
                    weight,
                    usage_count,
                } => {
                    let k = alias_word.trim().to_lowercase();
                    view.op_aliases.insert(k.clone(), *target_op);
                    view.op_alias_weights.insert(k, (*weight, *usage_count));
                }
                LexiconPayload::PhraseMacro {
                    phrase, expansion, ..
                } => {
                    let p = phrase.trim().to_lowercase();
                    let e = expansion.trim().to_lowercase();
                    view.phrase_macros.retain(|(existing, _)| existing != &p);
                    view.phrase_macros.push((p, e));
                }
                LexiconPayload::EntityAlias {
                    alias, canonical, ..
                } => {
                    let alias_clean = alias.trim().to_lowercase();
                    view.entity_aliases.insert(alias_clean, canonical.clone());
                }
                LexiconPayload::Rollback { .. } => {}
            }
        }

        // Ordenar phrase_macros de mayor a menor longitud para evitar solapamientos parciales
        view.phrase_macros.sort_by_key(|a| std::cmp::Reverse(a.0.len()));

        // Poblar EpisodicMemory determinísticamente con los hechos no podados
        let mut sorted_facts: Vec<_> = view.active_facts.iter().collect();
        sorted_facts.sort_by(|a, b| a.0.cmp(b.0));
        for ((from, to), (_, weight, usage_count)) in sorted_facts {
            view.episodic
                .insert_edge_with_weight(from.clone(), to.clone(), *weight);
            view.episodic
                .set_edge_stats(from, to, *weight, *usage_count);
        }

        // Registrar alias de entidad
        for (_, entry) in &active_entries {
            if let LexiconPayload::EntityAlias {
                alias, canonical, ..
            } = &entry.payload
            {
                view.episodic
                    .register_entity_alias(alias.clone(), canonical.clone());
            }
        }

        Ok(view)
    }

    /// Verifica toda la cadena de hashes SHA-256 desde génesis y reconstruye el estado activo en $O(N)$.
    pub fn rebuild_active_view(entries: &[LogEntry]) -> Result<LexiconView, IntegrityError> {
        Self::rebuild_view_at_version(entries, None)
    }

    /// Añade una nueva entrada sellada al final de la bitácora y actualiza la vista activa.
    pub fn append_entry(
        &mut self,
        timestamp_ms: u64,
        status: EntryStatus,
        payload: LexiconPayload,
    ) -> Result<LogEntry, IntegrityError> {
        let next_version = self.active_view.current_version + 1;
        let entry = LogEntry::new_sealed(
            next_version,
            timestamp_ms,
            self.active_view.head_hash.clone(),
            status,
            payload,
        );

        let mut candidate_entries = self.entries.clone();
        candidate_entries.push(entry.clone());
        let new_view = Self::rebuild_active_view(&candidate_entries)?;

        if let Some(ref path) = self.backing_file {
            let line = serde_json::to_string(&entry)
                .map_err(|e| IntegrityError::IoError(e.to_string()))?;
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .map_err(|e| IntegrityError::IoError(e.to_string()))?;
            writeln!(file, "{line}").map_err(|e| IntegrityError::IoError(e.to_string()))?;
        }

        self.entries = candidate_entries;
        self.active_view = new_view;
        Ok(entry)
    }

    pub fn to_jsonl_string(&self) -> String {
        let mut out = String::new();
        for entry in &self.entries {
            if let Ok(line) = serde_json::to_string(entry) {
                out.push_str(&line);
                out.push('\n');
            }
        }
        out
    }

    pub fn entries(&self) -> &[LogEntry] { &self.entries }
    pub fn active_view(&self) -> &LexiconView { &self.active_view }
    pub fn active_view_mut(&mut self) -> &mut LexiconView { &mut self.active_view }
    pub fn current_version(&self) -> u64 { self.active_view.current_version }
    pub fn head_hash(&self) -> &str { &self.active_view.head_hash }
    pub fn detach_backing_file(&mut self) { self.backing_file = None; }
}
