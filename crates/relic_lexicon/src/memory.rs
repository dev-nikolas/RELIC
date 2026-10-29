//! Proyección en RAM y bitácora de histórico fuera de RAM con índice de offsets.
//! Menos de 180 líneas.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use crate::genesis::build_genesis_entries;
use crate::log::{EntryStatus, IntegrityError, LexiconPayload, LogEntry, GENESIS_PREV_HASH};
pub use crate::view::{ClassicalPolysemyResult, LexiconView, PolysemousEntry};

#[derive(Debug, Clone)]
pub struct EngramStore {
    entries: Vec<LogEntry>,
    entry_offsets: Vec<u64>,
    active_view: LexiconView,
    backing_file: Option<PathBuf>,
}

impl Default for EngramStore { fn default() -> Self { Self::with_genesis() } }

impl EngramStore {
    pub fn new_empty() -> Self { Self { entries: Vec::new(), entry_offsets: Vec::new(), active_view: LexiconView::default(), backing_file: None } }

    pub fn with_genesis() -> Self {
        let entries = build_genesis_entries();
        Self { active_view: Self::rebuild_active_view(&entries).expect("Génesis válida"), entries, entry_offsets: Vec::new(), backing_file: None }
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
        Ok(Self { active_view: Self::rebuild_active_view(&entries)?, entries, entry_offsets: Vec::new(), backing_file: None })
    }

    pub fn from_jsonl_str(content: &str) -> Result<Self, IntegrityError> { Self::from_jsonl_reader(content.as_bytes()) }

    pub fn open_or_init_file(path: impl AsRef<Path>) -> Result<Self, IntegrityError> {
        let path_buf = path.as_ref().to_path_buf();
        if path_buf.exists() {
            let file = File::open(&path_buf).map_err(|e| IntegrityError::IoError(e.to_string()))?;
            let mut reader = BufReader::new(file);
            let (mut offsets, mut entries, mut offset) = (Vec::new(), Vec::new(), 0u64);
            let mut line = String::new();
            while reader.read_line(&mut line).map_err(|e| IntegrityError::IoError(e.to_string()))? > 0 {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    offsets.push(offset);
                    entries.push(serde_json::from_str(trimmed).map_err(|e| IntegrityError::ParseError { line: offsets.len(), details: e.to_string() })?);
                }
                offset += line.len() as u64;
                line.clear();
            }
            let active_view = Self::rebuild_active_view(&entries)?;
            Ok(Self { entries: Vec::new(), entry_offsets: offsets, active_view, backing_file: Some(path_buf) })
        } else {
            let s = Self::with_genesis();
            File::create(&path_buf).and_then(|mut f| f.write_all(s.to_jsonl_string().as_bytes())).map_err(|e| IntegrityError::IoError(e.to_string()))?;
            Self::open_or_init_file(path_buf)
        }
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
        for (_, entry) in &active_entries { view.apply_payload(&entry.payload); }
        Ok(view)
    }

    pub fn rebuild_active_view(entries: &[LogEntry]) -> Result<LexiconView, IntegrityError> { Self::rebuild_view_at_version(entries, None) }

    pub fn rebuild_view_at(&self, max_ver: Option<u64>) -> Result<LexiconView, IntegrityError> {
        if let Some(ref path) = self.backing_file {
            let file = File::open(path).map_err(|e| IntegrityError::IoError(e.to_string()))?;
            let mut entries = Vec::new();
            for (idx, line_res) in BufReader::new(file).lines().enumerate() {
                let l = line_res.map_err(|e| IntegrityError::IoError(e.to_string()))?;
                if !l.trim().is_empty() {
                    entries.push(serde_json::from_str(l.trim()).map_err(|e| IntegrityError::ParseError { line: idx + 1, details: e.to_string() })?);
                }
            }
            Self::rebuild_view_at_version(&entries, max_ver)
        } else {
            Self::rebuild_view_at_version(&self.entries, max_ver)
        }
    }

    pub fn read_entry_at(&self, idx: usize) -> Result<LogEntry, IntegrityError> {
        if let Some(ref path) = self.backing_file {
            let &offset = self.entry_offsets.get(idx).ok_or_else(|| IntegrityError::IoError("Index out of bounds".to_string()))?;
            let mut file = File::open(path).map_err(|e| IntegrityError::IoError(e.to_string()))?;
            use std::io::Seek;
            file.seek(std::io::SeekFrom::Start(offset)).map_err(|e| IntegrityError::IoError(e.to_string()))?;
            let mut line = String::new();
            BufReader::new(file).read_line(&mut line).map_err(|e| IntegrityError::IoError(e.to_string()))?;
            serde_json::from_str(line.trim()).map_err(|e| IntegrityError::ParseError { line: idx + 1, details: e.to_string() })
        } else {
            self.entries.get(idx).cloned().ok_or_else(|| IntegrityError::IoError("Index out of bounds".to_string()))
        }
    }

    pub fn append_entry(&mut self, timestamp_ms: u64, status: EntryStatus, payload: LexiconPayload) -> Result<LogEntry, IntegrityError> {
        let entry = LogEntry::new_sealed(self.active_view.current_version + 1, timestamp_ms, self.active_view.head_hash.clone(), status, payload);
        let is_rebuild_needed = matches!(entry.payload, LexiconPayload::Rollback { .. } | LexiconPayload::ConsolidationCheckpoint { .. });

        if let Some(ref path) = self.backing_file {
            let line = serde_json::to_string(&entry).map_err(|e| IntegrityError::IoError(e.to_string()))?;
            let mut file = OpenOptions::new().create(true).append(true).open(path).map_err(|e| IntegrityError::IoError(e.to_string()))?;
            let offset = file.metadata().map_err(|e| IntegrityError::IoError(e.to_string()))?.len();
            writeln!(file, "{line}").map_err(|e| IntegrityError::IoError(e.to_string()))?;
            file.flush().map_err(|e| IntegrityError::IoError(e.to_string()))?;
            self.entry_offsets.push(offset);
            if is_rebuild_needed {
                self.active_view = self.rebuild_view_at(None)?;
            } else {
                self.active_view.current_version = entry.version;
                self.active_view.head_hash = entry.entry_hash.clone();
                if entry.status != EntryStatus::Reverted { self.active_view.apply_payload(&entry.payload); }
            }
        } else {
            if is_rebuild_needed {
                let mut candidate_entries = self.entries.clone();
                candidate_entries.push(entry.clone());
                self.active_view = Self::rebuild_active_view(&candidate_entries)?;
                self.entries = candidate_entries;
            } else {
                self.active_view.current_version = entry.version;
                self.active_view.head_hash = entry.entry_hash.clone();
                if entry.status != EntryStatus::Reverted { self.active_view.apply_payload(&entry.payload); }
                self.entries.push(entry.clone());
            }
        }
        Ok(entry)
    }

    pub fn rollback_to(&mut self, target_version: u64, timestamp_ms: u64, reason: impl Into<String>) -> Result<LogEntry, IntegrityError> {
        let payload = LexiconPayload::Rollback { target_version, reason: reason.into() };
        self.append_entry(timestamp_ms, EntryStatus::Reverted, payload)
    }
    pub fn to_jsonl_string(&self) -> String {
        if let Some(ref path) = self.backing_file {
            std::fs::read_to_string(path).unwrap_or_default()
        } else {
            self.entries.iter().filter_map(|e| serde_json::to_string(e).ok()).map(|l| format!("{l}\n")).collect()
        }
    }
    pub fn entries(&self) -> &[LogEntry] { &self.entries }
    pub fn entry_count(&self) -> usize { if self.backing_file.is_some() { self.entry_offsets.len() } else { self.entries.len() } }
    pub fn entry_offsets(&self) -> &[u64] { &self.entry_offsets }
    pub fn active_view(&self) -> &LexiconView { &self.active_view }
    pub fn active_view_mut(&mut self) -> &mut LexiconView { &mut self.active_view }
    pub fn current_version(&self) -> u64 { self.active_view.current_version }
    pub fn head_hash(&self) -> &str { &self.active_view.head_hash }
    pub fn detach_backing_file(&mut self) { self.backing_file = None; }
}
