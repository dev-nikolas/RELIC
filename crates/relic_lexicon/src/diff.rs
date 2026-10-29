//! Motor de inspección y Diff Estructurado entre versiones del Engrama (`relic_lexicon::diff`).
//! Incluye exportadores deterministas a Mermaid y Graphviz DOT. Menos de 180 líneas.

use serde::{Deserialize, Serialize};
use crate::log::{IntegrityError, LexiconPayload, LogEntry};
use crate::memory::EngramStore;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WeightDelta {
    pub from: String,
    pub to: String,
    pub weight_before: f64,
    pub weight_after: f64,
    pub delta: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RevertedEntrySummary {
    pub version: u64,
    pub summary: String,
    pub reverted_by_version: u64,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VersionDiff {
    pub v_from: u64,
    pub v_to: u64,
    pub added_facts: Vec<(String, String, String, f64)>,
    pub removed_facts: Vec<(String, String, String)>,
    pub retained_facts: Vec<(String, String, String, f64)>,
    pub added_aliases: Vec<(String, String)>,
    pub reverted_entries: Vec<RevertedEntrySummary>,
    pub weight_deltas: Vec<WeightDelta>,
}

impl EngramStore {
    pub fn diff_versions(&self, v_from: u64, v_to: u64) -> Result<VersionDiff, IntegrityError> {
        let (view_from, view_to) = (self.rebuild_view_at(Some(v_from))?, self.rebuild_view_at(Some(v_to))?);
        let entries = if self.entries().is_empty() {
            let mut es = Vec::new();
            for idx in 0..self.entry_count() { es.push(self.read_entry_at(idx)?); }
            es
        } else {
            self.entries().to_vec()
        };
        diff_from_views_and_entries(&entries, v_from, v_to, &view_from, &view_to)
    }
}

pub fn diff_versions(entries: &[LogEntry], v_from: u64, v_to: u64) -> Result<VersionDiff, IntegrityError> {
    let view_from = EngramStore::rebuild_view_at_version(entries, Some(v_from))?;
    let view_to = EngramStore::rebuild_view_at_version(entries, Some(v_to))?;
    diff_from_views_and_entries(entries, v_from, v_to, &view_from, &view_to)
}

fn diff_from_views_and_entries(entries: &[LogEntry], v_from: u64, v_to: u64, view_from: &crate::view::LexiconView, view_to: &crate::view::LexiconView) -> Result<VersionDiff, IntegrityError> {

    let (mut added_facts, mut retained_facts, mut weight_deltas) = (Vec::new(), Vec::new(), Vec::new());
    for ((from, to), (rel, w_to, _)) in &view_to.active_facts {
        if let Some((_, w_from, _)) = view_from.active_facts.get(&(from.clone(), to.clone())) {
            retained_facts.push((from.clone(), rel.clone(), to.clone(), *w_to));
            if (*w_to - *w_from).abs() > 1e-9 {
                weight_deltas.push(WeightDelta {
                    from: from.clone(), to: to.clone(), weight_before: *w_from, weight_after: *w_to, delta: *w_to - *w_from,
                });
            }
        } else {
            added_facts.push((from.clone(), rel.clone(), to.clone(), *w_to));
        }
    }
    added_facts.sort_by(|a, b| (a.0.as_str(), a.2.as_str()).cmp(&(b.0.as_str(), b.2.as_str())));
    retained_facts.sort_by(|a, b| (a.0.as_str(), a.2.as_str()).cmp(&(b.0.as_str(), b.2.as_str())));
    weight_deltas.sort_by(|a, b| (a.from.as_str(), a.to.as_str()).cmp(&(b.from.as_str(), b.to.as_str())));

    let mut removed_facts: Vec<_> = view_from.active_facts.iter()
        .filter(|((f, t), _)| !view_to.active_facts.contains_key(&(f.clone(), t.clone())))
        .map(|((f, t), (rel, _, _))| (f.clone(), rel.clone(), t.clone()))
        .collect();

    let mut added_aliases = Vec::new();
    for (alias, op) in &view_to.op_aliases {
        if !view_from.op_aliases.contains_key(alias) { added_aliases.push((alias.clone(), format!("OpAlias({op:?})"))); }
    }
    for (alias, canon) in &view_to.entity_aliases {
        if !view_from.entity_aliases.contains_key(alias) { added_aliases.push((alias.clone(), format!("EntityAlias({canon})"))); }
    }
    for (phrase, exp) in &view_to.phrase_macros {
        if !view_from.phrase_macros.iter().any(|(p, _)| p == phrase) { added_aliases.push((phrase.clone(), format!("PhraseMacro({exp})"))); }
    }
    added_aliases.sort();

    let mut reverted_entries = Vec::new();
    for entry in entries.iter().filter(|e| e.version > v_from && e.version <= v_to) {
        if let LexiconPayload::Rollback { target_version, reason } = &entry.payload {
            for target_e in entries.iter().filter(|e| e.version > *target_version && e.version < entry.version) {
                let summary = match &target_e.payload {
                    LexiconPayload::GraphFact { from, relation, to, .. } => {
                        if !view_to.active_facts.contains_key(&(from.clone(), to.clone()))
                            && !removed_facts.iter().any(|(f, _, t)| f == from && t == to)
                        {
                            removed_facts.push((from.clone(), relation.clone(), to.clone()));
                        }
                        format!("GraphFact({from} -[{relation}]-> {to})")
                    }
                    LexiconPayload::OpAlias { alias_word, target_op, .. } => format!("OpAlias('{alias_word}' -> {target_op:?})"),
                    LexiconPayload::PhraseMacro { phrase, expansion, .. } => format!("PhraseMacro('{phrase}' -> '{expansion}')"),
                    LexiconPayload::EntityAlias { alias, canonical, .. } => format!("EntityAlias('{alias}' -> '{canonical}')"),
                    _ => continue,
                };
                reverted_entries.push(RevertedEntrySummary {
                    version: target_e.version, summary, reverted_by_version: entry.version, reason: reason.clone(),
                });
            }
        }
    }
    removed_facts.sort();

    Ok(VersionDiff { v_from, v_to, added_facts, removed_facts, retained_facts, added_aliases, reverted_entries, weight_deltas })
}

pub fn to_mermaid_diff(diff: &VersionDiff) -> String {
    let mut lines = vec![
        "flowchart LR".to_string(),
        format!("    %% Engram Diff v{} -> v{}", diff.v_from, diff.v_to),
        "    classDef adopted stroke:#16a34a,stroke-width:2px,color:#15803d;".to_string(),
        "    classDef reverted stroke:#dc2626,stroke-width:2px,stroke-dasharray: 5 5,color:#b91c1c;".to_string(),
    ];
    let (mut edge_idx, mut green_edges, mut red_edges) = (0usize, Vec::new(), Vec::new());
    for (from, rel, to, w) in &diff.retained_facts {
        lines.push(format!("    \"{from}\" -->|\"{rel} (w={w:.2})\"| \"{to}\""));
        edge_idx += 1;
    }
    for (from, rel, to, w) in &diff.added_facts {
        lines.push(format!("    \"{from}\" == \"+ {rel} (w={w:.2})\" ==> \"{to}\""));
        green_edges.push(edge_idx);
        edge_idx += 1;
    }
    for (from, rel, to) in &diff.removed_facts {
        lines.push(format!("    \"{from}\" -. \"REVERTED: {rel}\" .-> \"{to}\""));
        red_edges.push(edge_idx);
        edge_idx += 1;
    }
    for (alias, target) in &diff.added_aliases {
        lines.push(format!("    \"alias:{alias}\"[\"+ Alias: {alias} => {target}\"]:::adopted"));
    }
    for rev in &diff.reverted_entries {
        lines.push(format!("    \"scar_v{}\"[\"SCAR v{} (rev by v{}): {}\"]:::reverted", rev.version, rev.version, rev.reverted_by_version, rev.summary));
    }
    for idx in green_edges { lines.push(format!("    linkStyle {idx} stroke:#16a34a,stroke-width:3px;")); }
    for idx in red_edges { lines.push(format!("    linkStyle {idx} stroke:#dc2626,stroke-width:2px,stroke-dasharray:5;")); }
    lines.join("\n")
}

pub fn to_dot_diff(diff: &VersionDiff) -> String {
    let mut lines = vec![format!("digraph EngramDiff_v{}_to_v{} {{", diff.v_from, diff.v_to), "    rankdir=LR;".to_string()];
    for (from, rel, to, w) in &diff.retained_facts {
        lines.push(format!("    \"{from}\" -> \"{to}\" [label=\"{rel} (w={w:.2})\", color=\"#475569\"];"));
    }
    for (from, rel, to, w) in &diff.added_facts {
        lines.push(format!("    \"{from}\" -> \"{to}\" [label=\"+ {rel} (w={w:.2})\", color=\"#16a34a\", penwidth=2.5];"));
    }
    for (from, rel, to) in &diff.removed_facts {
        lines.push(format!("    \"{from}\" -> \"{to}\" [label=\"REVERTED: {rel}\", color=\"#dc2626\", style=\"dashed\"];"));
    }
    for (alias, target) in &diff.added_aliases {
        lines.push(format!("    \"alias:{alias}\" [label=\"+ {alias} => {target}\", shape=box, color=\"#16a34a\"];"));
    }
    for rev in &diff.reverted_entries {
        lines.push(format!("    \"scar_v{}\" [label=\"SCAR v{}: {}\", shape=note, color=\"#dc2626\", style=\"dashed\"];", rev.version, rev.version, rev.summary));
    }
    lines.push("}".to_string());
    lines.join("\n")
}
