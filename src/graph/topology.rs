//! Topological analysis and sorting of graphs.
//!
//! Provides algorithms for:
//! - Topological sorting (execution order)
//! - Parallel batch identification
//! - Dependency analysis

use crate::core::error::{GraphError, GraphResult, NodeId};
use crate::graph::structure::ProcessingGraph;
use std::collections::{HashMap, HashSet, VecDeque};

/// Analyzer for graph topology.
pub struct TopologyAnalyzer<'a> {
    graph: &'a ProcessingGraph,
}

impl<'a> TopologyAnalyzer<'a> {
    /// Create a new analyzer for the given graph.
    pub fn new(graph: &'a ProcessingGraph) -> Self {
        Self { graph }
    }

    /// Get the topological sort order (Kahn's algorithm).
    ///
    /// Returns nodes in an order where dependencies come before dependents.
    pub fn topological_sort(&self) -> GraphResult<Vec<NodeId>> {
        let mut in_degree: HashMap<NodeId, usize> = HashMap::new();
        let mut adjacency: HashMap<NodeId, Vec<NodeId>> = HashMap::new();

        // Initialize
        for node_id in self.graph.node_ids() {
            in_degree.insert(node_id, 0);
            adjacency.insert(node_id, Vec::new());
        }

        // Build adjacency list and count in-degrees
        for conn in self.graph.connections() {
            adjacency
                .get_mut(&conn.from.node_id)
                .unwrap()
                .push(conn.to.node_id);
            *in_degree.get_mut(&conn.to.node_id).unwrap() += 1;
        }

        // Start with nodes that have no incoming edges
        let mut queue: VecDeque<NodeId> = in_degree
            .iter()
            .filter(|(_, degree)| **degree == 0)
            .map(|(&id, _)| id)
            .collect();

        let mut result = Vec::with_capacity(self.graph.node_count());

        while let Some(node) = queue.pop_front() {
            result.push(node);

            for &neighbor in adjacency.get(&node).unwrap() {
                let degree = in_degree.get_mut(&neighbor).unwrap();
                *degree -= 1;
                if *degree == 0 {
                    queue.push_back(neighbor);
                }
            }
        }

        // If we haven't visited all nodes, there's a cycle
        if result.len() != self.graph.node_count() {
            let remaining: Vec<NodeId> = in_degree
                .iter()
                .filter(|(_, degree)| **degree > 0)
                .map(|(&id, _)| id)
                .collect();

            return Err(GraphError::CycleDetected { nodes: remaining });
        }

        Ok(result)
    }

    /// Group nodes into parallel execution batches.
    ///
    /// Nodes in the same batch can be executed in parallel because they
    /// don't depend on each other.
    pub fn parallel_batches(&self) -> GraphResult<Vec<Vec<NodeId>>> {
        let sorted = self.topological_sort()?;
        
        // Calculate the "depth" of each node (longest path from a source)
        let mut depth: HashMap<NodeId, usize> = HashMap::new();
        
        for &node_id in &sorted {
            let max_parent_depth = self
                .graph
                .connections_to(node_id)
                .filter_map(|conn| depth.get(&conn.from.node_id))
                .max()
                .copied()
                .unwrap_or(0);

            let node_depth = if self.graph.connections_to(node_id).next().is_none() {
                0 // Source node
            } else {
                max_parent_depth + 1
            };

            depth.insert(node_id, node_depth);
        }

        // Group by depth
        let max_depth = depth.values().max().copied().unwrap_or(0);
        let mut batches: Vec<Vec<NodeId>> = vec![Vec::new(); max_depth + 1];

        for (node_id, d) in depth {
            batches[d].push(node_id);
        }

        // Remove empty batches (shouldn't happen, but just in case)
        batches.retain(|batch| !batch.is_empty());

        Ok(batches)
    }

    /// Get the execution depth of a node.
    ///
    /// Depth 0 = source nodes (no dependencies)
    /// Higher depth = more dependencies
    pub fn node_depth(&self, node_id: NodeId) -> GraphResult<usize> {
        if !self.graph.has_node(node_id) {
            return Err(GraphError::NodeNotFound(node_id));
        }

        let mut max_depth = 0;
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back((node_id, 0));

        // BFS backwards to find longest path
        while let Some((current, depth)) = queue.pop_front() {
            if visited.insert(current) {
                for conn in self.graph.connections_to(current) {
                    queue.push_back((conn.from.node_id, depth + 1));
                    max_depth = max_depth.max(depth + 1);
                }
            }
        }

        Ok(max_depth)
    }

    /// Find all nodes that can be executed given a set of already-executed nodes.
    pub fn ready_to_execute(&self, executed: &HashSet<NodeId>) -> Vec<NodeId> {
        self.graph
            .node_ids()
            .filter(|&node_id| {
                // Not already executed
                !executed.contains(&node_id) &&
                // All dependencies are executed
                self.graph
                    .connections_to(node_id)
                    .all(|conn| executed.contains(&conn.from.node_id))
            })
            .collect()
    }

    /// Check if the graph has any cycles.
    pub fn has_cycle(&self) -> bool {
        self.topological_sort().is_err()
    }

    /// Get the critical path length (longest path through the graph).
    pub fn critical_path_length(&self) -> GraphResult<usize> {
        let batches = self.parallel_batches()?;
        Ok(batches.len())
    }

    /// Find all disconnected subgraphs.
    pub fn find_subgraphs(&self) -> Vec<HashSet<NodeId>> {
        let mut visited: HashSet<NodeId> = HashSet::new();
        let mut subgraphs = Vec::new();

        for node_id in self.graph.node_ids() {
            if !visited.contains(&node_id) {
                let subgraph = self.flood_fill(node_id);
                visited.extend(&subgraph);
                subgraphs.push(subgraph);
            }
        }

        subgraphs
    }

    /// Flood fill to find all connected nodes (ignoring edge direction).
    fn flood_fill(&self, start: NodeId) -> HashSet<NodeId> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(start);

        while let Some(current) = queue.pop_front() {
            if visited.insert(current) {
                // Add all connected nodes (both directions)
                for conn in self.graph.connections_from(current) {
                    queue.push_back(conn.to.node_id);
                }
                for conn in self.graph.connections_to(current) {
                    queue.push_back(conn.from.node_id);
                }
            }
        }

        visited
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::node::PassthroughNode;
    use crate::graph::structure::GraphNode;

    fn create_test_node() -> GraphNode {
        GraphNode::new(Box::new(PassthroughNode))
    }

    #[test]
    fn test_topological_sort() {
        let mut graph = ProcessingGraph::new();

        let node1 = graph.add_node(create_test_node());
        let node2 = graph.add_node(create_test_node());
        let node3 = graph.add_node(create_test_node());

        graph.connect(node1, "output", node2, "input").unwrap();
        graph.connect(node2, "output", node3, "input").unwrap();

        let analyzer = TopologyAnalyzer::new(&graph);
        let sorted = analyzer.topological_sort().unwrap();

        // node1 must come before node2, node2 before node3
        let pos1 = sorted.iter().position(|&n| n == node1).unwrap();
        let pos2 = sorted.iter().position(|&n| n == node2).unwrap();
        let pos3 = sorted.iter().position(|&n| n == node3).unwrap();

        assert!(pos1 < pos2);
        assert!(pos2 < pos3);
    }

    #[test]
    fn test_topological_sort_empty_graph() {
        // Empty graph contract: topological_sort returns Ok([]), and the
        // derived has_cycle() must report false (no cycles in nothing).
        let graph = ProcessingGraph::new();
        let analyzer = TopologyAnalyzer::new(&graph);

        let sorted = analyzer
            .topological_sort()
            .expect("empty graph must topologically sort to Ok");
        assert!(sorted.is_empty(), "empty graph sort must be empty: {sorted:?}");
        assert!(!analyzer.has_cycle(), "empty graph must not report a cycle");
    }

    #[test]
    fn test_topological_sort_single_node_no_connections() {
        // Single isolated node: in-degree 0, must appear exactly once.
        let mut graph = ProcessingGraph::new();
        let only = graph.add_node(create_test_node());

        let analyzer = TopologyAnalyzer::new(&graph);
        let sorted = analyzer.topological_sort().unwrap();

        assert_eq!(sorted, vec![only]);
        assert!(!analyzer.has_cycle());
    }

    #[test]
    fn test_topological_sort_disconnected_nodes() {
        // Two nodes with no edges between them: both must appear exactly
        // once. Order is not specified, so assert membership and length.
        let mut graph = ProcessingGraph::new();
        let a = graph.add_node(create_test_node());
        let b = graph.add_node(create_test_node());

        let analyzer = TopologyAnalyzer::new(&graph);
        let sorted = analyzer.topological_sort().unwrap();

        assert_eq!(sorted.len(), 2, "disconnected pair must yield exactly 2 entries: {sorted:?}");
        let set: HashSet<NodeId> = sorted.into_iter().collect();
        assert!(set.contains(&a));
        assert!(set.contains(&b));
        assert!(!analyzer.has_cycle());
    }

    #[test]
    fn test_parallel_batches() {
        let mut graph = ProcessingGraph::new();

        // Fan-out + isolated source pattern (PassthroughNode has only one
        // input port, so a true diamond merge isn't expressible here).
        //   A ──> B
        //   A ──> C
        //   D            (isolated source/leaf)
        //
        // Expected batches by depth:
        //   depth 0: {A, D}
        //   depth 1: {B, C}
        let a = graph.add_node(create_test_node());
        let b = graph.add_node(create_test_node());
        let c = graph.add_node(create_test_node());
        let d = graph.add_node(create_test_node());

        graph.connect(a, "output", b, "input").unwrap();
        graph.connect(a, "output", c, "input").unwrap();

        let analyzer = TopologyAnalyzer::new(&graph);
        let batches = analyzer.parallel_batches().unwrap();

        assert_eq!(batches.len(), 2, "fan-out + isolated source should yield 2 depth-batches, got {batches:?}");

        let batch0: HashSet<NodeId> = batches[0].iter().copied().collect();
        let batch1: HashSet<NodeId> = batches[1].iter().copied().collect();

        assert!(batch0.contains(&a), "depth-0 batch must contain source A");
        assert!(batch0.contains(&d), "depth-0 batch must contain isolated D");
        assert_eq!(batch0.len(), 2, "depth-0 batch should be exactly {{A, D}}: {batch0:?}");

        assert!(batch1.contains(&b), "depth-1 batch must contain B");
        assert!(batch1.contains(&c), "depth-1 batch must contain C");
        assert_eq!(batch1.len(), 2, "depth-1 batch should be exactly {{B, C}}: {batch1:?}");
    }

    #[test]
    fn test_ready_to_execute() {
        let mut graph = ProcessingGraph::new();

        let node1 = graph.add_node(create_test_node());
        let node2 = graph.add_node(create_test_node());
        let node3 = graph.add_node(create_test_node());

        graph.connect(node1, "output", node2, "input").unwrap();
        graph.connect(node2, "output", node3, "input").unwrap();

        let analyzer = TopologyAnalyzer::new(&graph);

        // Initially, only node1 should be ready
        let ready = analyzer.ready_to_execute(&HashSet::new());
        assert!(ready.contains(&node1));
        assert!(!ready.contains(&node2));
        assert!(!ready.contains(&node3));

        // After executing node1, node2 should be ready
        let mut executed = HashSet::new();
        executed.insert(node1);
        let ready = analyzer.ready_to_execute(&executed);
        assert!(ready.contains(&node2));
        assert!(!ready.contains(&node3));
    }

    #[test]
    fn test_find_subgraphs() {
        let mut graph = ProcessingGraph::new();

        // Two disconnected chains
        let a1 = graph.add_node(create_test_node());
        let a2 = graph.add_node(create_test_node());
        let b1 = graph.add_node(create_test_node());
        let b2 = graph.add_node(create_test_node());

        graph.connect(a1, "output", a2, "input").unwrap();
        graph.connect(b1, "output", b2, "input").unwrap();

        let analyzer = TopologyAnalyzer::new(&graph);
        let subgraphs = analyzer.find_subgraphs();

        assert_eq!(subgraphs.len(), 2);
    }
}
