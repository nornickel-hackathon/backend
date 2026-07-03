//! Граф механизмов и generic-обход.
//!
//! `Graph` бандлит петрограф (узлы/рёбра) и таблицу claims по id: confidence
//! claim'ов нужна scoring-формулам (docs/SCORING.md), а в `GraphEdge` лежат
//! только id claim'ов. Это внутренняя структура ядра; платформа строит её из
//! `ExtractResponse` через [`Graph::build`].

use std::collections::{BTreeSet, HashMap, HashSet};

use contracts::{Claim, EdgeType, ExtractResponse, GraphEdge, GraphNode};
use petgraph::graph::{DiGraph, EdgeIndex, NodeIndex};
use petgraph::visit::EdgeRef;
use petgraph::Direction;

pub type GraphData = DiGraph<GraphNode, GraphEdge>;

/// Простой путь по графу: узлы (source-first .. kpi-last) и пройденные рёбра.
#[derive(Debug, Clone)]
pub struct Path {
    pub nodes: Vec<NodeIndex>,
    pub edges: Vec<EdgeIndex>,
}

pub struct Graph {
    graph: GraphData,
    claims: HashMap<String, Claim>,
    idx: HashMap<String, NodeIndex>,
}

impl Graph {
    /// Построить граф из `ExtractResponse`. Возвращает ошибку при дублях id и
    /// висящих ссылках рёбер — платформа маппит её в 422.
    pub fn build(extract: &ExtractResponse) -> Result<Graph, String> {
        let mut graph = DiGraph::new();
        let mut idx = HashMap::new();
        for node in &extract.entities {
            if idx.contains_key(&node.id) {
                return Err(format!("duplicate node id '{}'", node.id));
            }
            let i = graph.add_node(node.clone());
            idx.insert(node.id.clone(), i);
        }
        for edge in &extract.edges {
            let s = *idx.get(&edge.src).ok_or_else(|| {
                format!("edge '{}' src references unknown node '{}'", edge.id, edge.src)
            })?;
            let d = *idx.get(&edge.dst).ok_or_else(|| {
                format!("edge '{}' dst references unknown node '{}'", edge.id, edge.dst)
            })?;
            graph.add_edge(s, d, edge.clone());
        }
        let claims = extract
            .claims
            .iter()
            .map(|c| (c.id.clone(), c.clone()))
            .collect();
        Ok(Graph { graph, claims, idx })
    }

    /// Прямой доступ к петрографу (для операторов, обходящих рёбра напрямую).
    pub fn raw(&self) -> &GraphData {
        &self.graph
    }

    pub fn node(&self, id: &str) -> Option<&GraphNode> {
        self.idx.get(id).map(|i| &self.graph[*i])
    }

    pub fn index(&self, id: &str) -> Option<NodeIndex> {
        self.idx.get(id).copied()
    }

    pub fn weight(&self, i: NodeIndex) -> &GraphNode {
        &self.graph[i]
    }

    pub fn edge(&self, e: EdgeIndex) -> &GraphEdge {
        &self.graph[e]
    }

    pub fn claim(&self, id: &str) -> Option<&Claim> {
        self.claims.get(id)
    }

    /// Узлы с заданным тегом, отсортированы по id (детерминизм).
    pub fn nodes_with_tag(&self, tag: &str) -> Vec<NodeIndex> {
        let mut out: Vec<NodeIndex> = self
            .graph
            .node_indices()
            .filter(|i| self.graph[*i].has_tag(tag))
            .collect();
        out.sort_by(|a, b| self.graph[*a].id.cmp(&self.graph[*b].id));
        out
    }

    /// Incoming-рёбра заданного типа: пары (источник, ребро), сорт. по id ребра.
    pub fn incoming_edges_of_type(
        &self,
        node: NodeIndex,
        edge_type: EdgeType,
    ) -> Vec<(NodeIndex, EdgeIndex)> {
        let mut out: Vec<(NodeIndex, EdgeIndex)> = self
            .graph
            .edges_directed(node, Direction::Incoming)
            .filter(|er| er.weight().edge_type == edge_type)
            .map(|er| (er.source(), er.id()))
            .collect();
        out.sort_by(|a, b| self.graph[a.1].id.cmp(&self.graph[b.1].id));
        out
    }

    /// Уникальные id claim'ов на всех рёбрах (отсортированы — детерминизм).
    pub fn claims_on_edges(&self, edges: &[EdgeIndex]) -> Vec<String> {
        let mut set = BTreeSet::new();
        for e in edges {
            for c in &self.graph[*e].source_claims {
                set.insert(c.clone());
            }
        }
        set.into_iter().collect()
    }

    /// Есть ли у узла хоть одно инцидентное ребро с непустым `source_claims`.
    pub fn node_has_evidenced_edge(&self, node: NodeIndex) -> bool {
        let inc = self
            .graph
            .edges_directed(node, Direction::Incoming)
            .any(|er| !er.weight().source_claims.is_empty());
        let out = self
            .graph
            .edges_directed(node, Direction::Outgoing)
            .any(|er| !er.weight().source_claims.is_empty());
        inc || out
    }

    /// Все простые пути от `start` назад по incoming-рёбрам разрешённых типов
    /// до первого узла с тегом `until_tag`. Узлы в результате — source-first.
    pub fn enumerate_paths(
        &self,
        start: NodeIndex,
        allowed: &[EdgeType],
        until_tag: &str,
    ) -> Vec<Path> {
        let mut out = Vec::new();
        let mut nodes = vec![start];
        let mut edges = Vec::new();
        let mut visited = HashSet::new();
        visited.insert(start);
        self.dfs(start, allowed, until_tag, &mut visited, &mut nodes, &mut edges, &mut out);
        out
    }

    #[allow(clippy::too_many_arguments)]
    fn dfs(
        &self,
        current: NodeIndex,
        allowed: &[EdgeType],
        until_tag: &str,
        visited: &mut HashSet<NodeIndex>,
        nodes: &mut Vec<NodeIndex>,
        edges: &mut Vec<EdgeIndex>,
        out: &mut Vec<Path>,
    ) {
        if !edges.is_empty() && self.graph[current].has_tag(until_tag) {
            let mut n = nodes.clone();
            n.reverse();
            let mut e = edges.clone();
            e.reverse();
            out.push(Path { nodes: n, edges: e });
            return;
        }
        let mut incoming: Vec<(NodeIndex, EdgeIndex)> = self
            .graph
            .edges_directed(current, Direction::Incoming)
            .filter(|er| allowed.contains(&er.weight().edge_type))
            .map(|er| (er.source(), er.id()))
            .collect();
        incoming.sort_by(|a, b| self.graph[a.1].id.cmp(&self.graph[b.1].id));
        for (src, edge_id) in incoming {
            if visited.contains(&src) {
                continue;
            }
            visited.insert(src);
            nodes.push(src);
            edges.push(edge_id);
            self.dfs(src, allowed, until_tag, visited, nodes, edges, out);
            edges.pop();
            nodes.pop();
            visited.remove(&src);
        }
    }
}
