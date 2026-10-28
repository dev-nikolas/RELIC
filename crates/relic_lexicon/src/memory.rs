//! Proyección en RAM y verificación de integridad O(N) del Engrama JSONL. Menos de 180 líneas.

use std::collections::{HashMap, HashSet};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use relic_core::{ArithmeticOp, Complex, EpisodicMemory};
use crate::genesis::build_genesis_entries;
use crate::log::{EntryStatus, IntegrityError, LexiconPayload, LogEntry, GENESIS_PREV_HASH};
pub use crate::polysemy::{ClassicalPolysemyResult, PolysemousEntry};

#[derive(Debug, Clone)]
pub struct LexiconView {
    pub episodic: EpisodicMemory,
    pub op_aliases: HashMap<String, ArithmeticOp>,
    pub op_alias_weights: HashMap<String, (f64, u64)>,
    pub quantum_polysemy: HashMap<String, Vec<PolysemousEntry>>,
    pub phrase_macros: Vec<(String, String)>,
    pub entity_aliases: HashMap<String, String>,
    pub active_facts: HashMap<(String, String), (String, f64, u64)>,
    pub current_version: u64,
    pub head_hash: String,
}

impl Default for LexiconView {
    fn default() -> Self {
        Self {
            episodic: EpisodicMemory::new(), op_aliases: HashMap::new(), op_alias_weights: HashMap::new(),
            quantum_polysemy: HashMap::new(), phrase_macros: Vec::new(), entity_aliases: HashMap::new(),
            active_facts: HashMap::new(), current_version: 0, head_hash: GENESIS_PREV_HASH.to_string(),
        }
    }
}

impl LexiconView {
    pub fn resolve_op_alias(&self, word: &str) -> Option<ArithmeticOp> {
        self.op_aliases.get(&word.trim().to_lowercase()).copied()
    }
    pub fn apply_phrase_macros(&self, text: &str) -> String {
        let mut out = text.to_lowercase();
        for (phrase, expansion) in &self.phrase_macros {
            if out.contains(phrase.as_str()) { out = out.replace(phrase.as_str(), expansion.as_str()); }
        }
        out
    }
}

#[derive(Debug, Clone)]
pub struct EngramStore {
    entries: Vec<LogEntry>,
    active_view: LexiconView,
    backing_file: Option<PathBuf>,
}

impl Default for EngramStore { fn default() -> Self { Self::with_genesis() } }

impl EngramStore {
    pub fn new_empty() -> Self { Self { entries: Vec::new(), active_view: LexiconView::default(), backing_file: None } }
    pub fn with_genesis() -> Self {
        let entries = build_genesis_entries();
        Self { active_view: Self::rebuild_active_view(&entries).expect("Semilla génesis válida"), entries, backing_file: None }
    }
    pub fn from_jsonl_reader<R: BufRead>(reader: R) -> Result<Self, IntegrityError> {
        let mut entries = Vec::new();
        for (idx, line_res) in reader.lines().enumerate() {
            let line = line_res.map_err(|e| IntegrityError::IoError(e.to_string()))?;
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                entries.push(serde_json::from_str(trimmed).map_err(|e| IntegrityError::ParseError { line: idx + 1, details: e.to_string() })?);
            }
        }
        Ok(Self { active_view: Self::rebuild_active_view(&entries)?, entries, backing_file: None })
    }
    pub fn from_jsonl_str(content: &str) -> Result<Self, IntegrityError> { Self::from_jsonl_reader(content.as_bytes()) }
    pub fn open_or_init_file(path: impl AsRef<Path>) -> Result<Self, IntegrityError> {
        let path_buf = path.as_ref().to_path_buf();
        let mut store = if path_buf.exists() {
            Self::from_jsonl_reader(BufReader::new(File::open(&path_buf).map_err(|e| IntegrityError::IoError(e.to_string()))?))?
        } else {
            let s = Self::with_genesis();
            File::create(&path_buf).and_then(|mut f| f.write_all(s.to_jsonl_string().as_bytes())).map_err(|e| IntegrityError::IoError(e.to_string()))?;
            s
        };
        store.backing_file = Some(path_buf);
        Ok(store)
    }

    pub fn rebuild_view_at_version(entries: &[LogEntry], max_version: Option<u64>) -> Result<LexiconView, IntegrityError> {
        let (mut expected_prev, mut active_entries, mut last_ver) = (GENESIS_PREV_HASH.to_string(), Vec::with_capacity(entries.len()), 0);
        for (idx, entry) in entries.iter().enumerate() {
            entry.verify((idx + 1) as u64, &expected_prev)?;
            if max_version.is_some_and(|limit| entry.version > limit) { break; }
            expected_prev = entry.entry_hash.clone();
            last_ver = entry.version;
            match &entry.payload {
                LexiconPayload::Rollback { target_version, .. } => {
                    if *target_version >= entry.version {
                        return Err(IntegrityError::InvalidRollbackTarget { version: entry.version, target_version: *target_version });
                    }
                    active_entries.retain(|(v, _)| *v <= *target_version);
                }
                _ if entry.status != EntryStatus::Reverted => active_entries.push((entry.version, entry)),
                _ => {}
            }
        }

        let mut view = LexiconView { current_version: last_ver, head_hash: expected_prev, ..LexiconView::default() };
        let mut pruned_set: HashSet<(String, String)> = HashSet::new();

        for (_, entry) in &active_entries {
            match &entry.payload {
                LexiconPayload::GraphFact { from, relation, to, weight, usage_count } => {
                    let key = (from.clone(), to.clone());
                    pruned_set.remove(&key);
                    view.active_facts.insert(key, (relation.clone(), *weight, *usage_count));
                }
                LexiconPayload::EdgeReinforcement { from, to, new_weight, usage_count } => {
                    if let Some(f) = view.active_facts.get_mut(&(from.clone(), to.clone())) { f.1 = *new_weight; f.2 = *usage_count; }
                }
                LexiconPayload::ConsolidationCheckpoint { pruned_edges, .. } => {
                    for edge in pruned_edges { view.active_facts.remove(edge); pruned_set.insert(edge.clone()); }
                }
                LexiconPayload::OpAlias { alias_word, target_op, weight, usage_count } => {
                    let k = alias_word.trim().to_lowercase();
                    view.op_aliases.insert(k.clone(), *target_op);
                    view.op_alias_weights.insert(k, (*weight, *usage_count));
                }
                LexiconPayload::QuantumPolysemousAlias { alias_word, target_op, amplitude_r, phase_theta, context_word } => {
                    let k = alias_word.trim().to_lowercase();
                    let ctx = context_word.as_ref().map(|s| s.trim().to_lowercase()).filter(|s| !s.is_empty());
                    view.quantum_polysemy.entry(k).or_default().push(PolysemousEntry {
                        target_op: *target_op, amplitude: Complex::from_polar(*amplitude_r, *phase_theta), context_word: ctx,
                    });
                }
                LexiconPayload::PhraseMacro { phrase, expansion, .. } => {
                    let (p, e) = (phrase.trim().to_lowercase(), expansion.trim().to_lowercase());
                    view.phrase_macros.retain(|(existing, _)| existing != &p);
                    view.phrase_macros.push((p, e));
                }
                LexiconPayload::EntityAlias { alias, canonical, .. } => { view.entity_aliases.insert(alias.trim().to_lowercase(), canonical.clone()); }
                LexiconPayload::Rollback { .. } => {}
            }
        }
        view.phrase_macros.sort_by_key(|a| std::cmp::Reverse(a.0.len()));
        let mut sorted_facts: Vec<_> = view.active_facts.iter().collect();
        sorted_facts.sort_by(|a, b| a.0.cmp(b.0));
        for ((from, to), (_, weight, usage_count)) in sorted_facts {
            view.episodic.insert_edge_with_weight(from.clone(), to.clone(), *weight);
            view.episodic.set_edge_stats(from, to, *weight, *usage_count);
        }
        for (_, entry) in &active_entries {
            if let LexiconPayload::EntityAlias { alias, canonical, .. } = &entry.payload {
                view.episodic.register_entity_alias(alias.clone(), canonical.clone());
            }
        }
        Ok(view)
    }
    pub fn rebuild_active_view(entries: &[LogEntry]) -> Result<LexiconView, IntegrityError> { Self::rebuild_view_at_version(entries, None) }
    pub fn append_entry(&mut self, timestamp_ms: u64, status: EntryStatus, payload: LexiconPayload) -> Result<LogEntry, IntegrityError> {
        let entry = LogEntry::new_sealed(self.active_view.current_version + 1, timestamp_ms, self.active_view.head_hash.clone(), status, payload);
        let mut candidate_entries = self.entries.clone();
        candidate_entries.push(entry.clone());
        let new_view = Self::rebuild_active_view(&candidate_entries)?;
        if let Some(ref path) = self.backing_file {
            let line = serde_json::to_string(&entry).map_err(|e| IntegrityError::IoError(e.to_string()))?;
            writeln!(OpenOptions::new().create(true).append(true).open(path).map_err(|e| IntegrityError::IoError(e.to_string()))?, "{line}").map_err(|e| IntegrityError::IoError(e.to_string()))?;
        }
        self.entries = candidate_entries;
        self.active_view = new_view;
        Ok(entry)
    }
    pub fn to_jsonl_string(&self) -> String { self.entries.iter().filter_map(|e| serde_json::to_string(e).ok()).map(|l| format!("{l}\n")).collect() }
    pub fn entries(&self) -> &[LogEntry] { &self.entries }
    pub fn active_view(&self) -> &LexiconView { &self.active_view }
    pub fn active_view_mut(&mut self) -> &mut LexiconView { &mut self.active_view }
    pub fn current_version(&self) -> u64 { self.active_view.current_version }
    pub fn head_hash(&self) -> &str { &self.active_view.head_hash }
    pub fn detach_backing_file(&mut self) { self.backing_file = None; }
}
