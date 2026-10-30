//! # TemporalGraph
//!
//! Grafo de razonamiento efímero para inferencias relacionales al vuelo (Mejora 5).
use std::collections::{HashMap, HashSet};

/// Relación dirigida tipada entre dos entidades.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TemporalEdge {
    pub source: String,
    pub target: String,
    pub relation: String,
}

/// Grafo temporal de razonamiento construido dinámicamente a partir de premisas.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TemporalGraph {
    edges: Vec<TemporalEdge>,
    adj: HashMap<String, Vec<(String, String)>>,
    nodes: HashSet<String>,
}

impl TemporalGraph {
    /// Inicializa un grafo efímero vacío.
    pub fn new() -> Self {
        Self {
            edges: Vec::new(),
            adj: HashMap::new(),
            nodes: HashSet::new(),
        }
    }

    /// Inserta una relación dirigida `source --relation--> target`.
    pub fn add_relation(&mut self, source: impl Into<String>, relation: impl Into<String>, target: impl Into<String>) {
        let s = source.into();
        let r = relation.into();
        let t = target.into();
        self.nodes.insert(s.clone());
        self.nodes.insert(t.clone());
        self.adj.entry(s.clone()).or_default().push((t.clone(), r.clone()));
        self.edges.push(TemporalEdge { source: s, target: t, relation: r });
    }

    /// Verifica si existe una relación transitiva directa o indirecta bajo la misma relación.
    pub fn is_transitively_related(&self, start: &str, target: &str, expected_relation: &str) -> bool {
        if start == target { return true; }
        let mut visited = HashSet::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(start.to_string());
        visited.insert(start.to_string());

        while let Some(current) = queue.pop_front() {
            if let Some(neighbors) = self.adj.get(&current) {
                for (next_node, rel) in neighbors {
                    if rel == expected_relation {
                        if next_node == target {
                            return true;
                        }
                        if visited.insert(next_node.clone()) {
                            queue.push_back(next_node.clone());
                        }
                    }
                }
            }
        }
        false
    }

    /// Comprueba si existe un ciclo en las relaciones especificadas (ej. A > B y B > A, o A > B > C > A).
    pub fn has_cycle(&self, relation: &str) -> bool {
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut adj_rel: HashMap<String, Vec<String>> = HashMap::new();
        let mut rel_nodes = HashSet::new();

        for edge in &self.edges {
            if edge.relation == relation {
                rel_nodes.insert(edge.source.clone());
                rel_nodes.insert(edge.target.clone());
                *in_degree.entry(edge.target.clone()).or_default() += 1;
                in_degree.entry(edge.source.clone()).or_default();
                adj_rel.entry(edge.source.clone()).or_default().push(edge.target.clone());
            }
        }

        if rel_nodes.is_empty() { return false; }

        let mut queue = std::collections::VecDeque::new();
        for node in &rel_nodes {
            if in_degree.get(node).copied().unwrap_or(0) == 0 {
                queue.push_back(node.clone());
            }
        }

        let mut visited_count = 0;
        while let Some(u) = queue.pop_front() {
            visited_count += 1;
            if let Some(neighbors) = adj_rel.get(&u) {
                for v in neighbors {
                    if let Some(deg) = in_degree.get_mut(v) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push_back(v.clone());
                        }
                    }
                }
            }
        }

        visited_count != rel_nodes.len()
    }

    /// Encuentra la entidad extrema única. Si hay contradicción (ciclo) o múltiples cadenas desconectadas, retorna None.
    pub fn find_maximal(&self, relation: &str) -> Option<String> {
        if self.has_cycle(relation) {
            return None; // Contradicción matemática detectada
        }

        let mut incoming_count: HashMap<String, usize> = HashMap::new();
        let mut outgoing_count: HashMap<String, usize> = HashMap::new();

        for edge in &self.edges {
            if edge.relation == relation {
                *incoming_count.entry(edge.target.clone()).or_default() += 1;
                incoming_count.entry(edge.source.clone()).or_default();
                *outgoing_count.entry(edge.source.clone()).or_default() += 1;
            }
        }

        // Candidatos maximales: in_degree == 0 y out_degree > 0
        let maximals: Vec<String> = incoming_count
            .iter()
            .filter(|&(n, &deg)| deg == 0 && outgoing_count.get(n).copied().unwrap_or(0) > 0)
            .map(|(n, _)| n.clone())
            .collect();

        // Para ser un máximo global estricto bajo orden parcial, debe ser único
        // Y además debe poder alcanzar transitivamente a TODOS los demás nodos que participan en la relación
        if maximals.len() == 1 {
            let candidate = &maximals[0];
            for other in &self.nodes {
                if other != candidate && !self.is_transitively_related(candidate, other, relation) {
                    return None; // Evidencia insuficiente, dimensión mezclada o nodo incomparable
                }
            }
            Some(candidate.clone())
        } else {
            None // Múltiples máximos locales incomparables o ninguno
        }
    }

    /// Devuelve todos los nodos registrados.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Devuelve las aristas dirigidas registradas.
    pub fn edges(&self) -> &[TemporalEdge] {
        &self.edges
    }
}
