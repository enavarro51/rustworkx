// Licensed under the Apache License, Version 2.0 (the "License"); you may
// not use this file except in compliance with the License. You may obtain
// a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS, WITHOUT
// WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the
// License for the specific language governing permissions and limitations
// under the License.

use petgraph::data::{Build, Create};
use petgraph::visit::{Data, EdgeRef, IntoEdgeReferences, NodeIndexable, IntoEdges, IntoNodeIdentifiers};

use super::InvalidInputError;

/// Generate a binomial tree graph
///
/// Arguments:
///
/// * `order` - The order of the binomial tree.
/// * `weights` - A `Vec` of node weight objects. If the number of weights is
///     less than 2**order extra nodes with None will be appended.
/// * `default_node_weight` - A callable that will return the weight to use
///     for newly created nodes. This is ignored if `weights` is specified,
///     as the weights from that argument will be used instead.
/// * `default_edge_weight` - A callable that will return the weight object
///     to use for newly created edges.
/// * `bidirectional` - Whether edges are added bidirectionally, if set to
///     `true` then for any edge `(u, v)` an edge `(v, u)` will also be added.
///     If the graph is undirected this will result in a pallel edge.
///
/// # Example
/// ```rust
/// use rustworkx_core::petgraph;
/// use rustworkx_core::generators::binomial_tree_graph;
/// use rustworkx_core::petgraph::visit::EdgeRef;
///
/// let g: petgraph::graph::UnGraph<(), ()> = binomial_tree_graph(
///     Some(4),
///     None,
///     || {()},
///     || {()},
///     false
/// ).unwrap();
/// assert_eq!(
///     vec![(0, 1), (1, 2), (2, 3)],
///     g.edge_references()
///         .map(|edge| (edge.source().index(), edge.target().index()))
///         .collect::<Vec<(usize, usize)>>(),
/// )
/// ```
pub fn binomial_tree_graph<G, T, F, H, M>(
    order: u32,
    weights: Option<Vec<T>>,
    mut default_node_weight: F,
    mut default_edge_weight: H,
    bidirectional: bool,
) -> Result<G, InvalidInputError>
where
    G: Build + Create + Data<NodeWeight = T, EdgeWeight = M> + NodeIndexable + IntoEdges + IntoNodeIdentifiers,
    F: FnMut() -> T,
    H: FnMut() -> M,
    T: Clone,
{
    // if order >= MAX_ORDER {
    //     return Err(InvalidInputError {});
    // }
    let num_nodes = usize::pow(2, order);
    let num_edges = usize::pow(2, order) - 1;
    let mut graph = G::with_capacity(num_nodes, num_edges);

    for i in 0..num_nodes {
        match weights {
            Some(ref weights) => {
                if weights.len() > num_nodes {
                    return Err(InvalidInputError {});
                }
                if i < weights.len() {
                    graph.add_node(weights[i].clone())
                } else {
                    graph.add_node(default_node_weight())
                }
            }
            None => graph.add_node(default_node_weight()),
        };
    }

    fn find_edge<G>(graph: &mut G, source: usize, target: usize) -> bool
    where
        G: NodeIndexable + IntoEdgeReferences + IntoEdges + IntoNodeIdentifiers,
    {
        let mut found = false;
        for node in graph.node_identifiers() {
            for e in graph.edges(node) {
                if graph.to_index(e.source()) == source && graph.to_index(e.target()) == target {
                    found = true;
                    break;
                }
            }
        }
        found
    }
    let mut n = 1;
    let zero_index = 0;
    //let mut edge_map = HashSet<(usize, usize)>.with_capacity(num_edges);

    for _ in 0..order {
        let edges: Vec<(usize, usize)> = graph
            .edge_references()
            .map(|e| (graph.to_index(e.source()), graph.to_index(e.target())))
            .collect();

        for (source, target) in edges {
            let source_index = source + n;
            let target_index = target + n;

            if !find_edge(&mut graph, source_index, target_index) {
                graph.add_edge(
                    graph.from_index(source_index),
                    graph.from_index(target_index),
                    default_edge_weight(),
                );
            }
            if bidirectional {
                if !find_edge(&mut graph, target_index, source_index) {
                    graph.add_edge(
                        graph.from_index(target_index),
                        graph.from_index(source_index),
                        default_edge_weight(),
                    );
                }
            }
        }
        if !find_edge(&mut graph, zero_index, n) {
            graph.add_edge(
                graph.from_index(zero_index),
                graph.from_index(n),
                default_edge_weight(),
            );
        }
        if bidirectional {
            if !find_edge(&mut graph, n, zero_index) {
                graph.add_edge(
                    graph.from_index(n),
                    graph.from_index(zero_index),
                    default_edge_weight(),
                );
            }
        }
        n *= 2;
    }
    Ok(graph)
}

#[cfg(test)]
mod tests {
    use crate::generators::binomial_tree_graph;
    use crate::generators::InvalidInputError;
    use crate::petgraph;
    use crate::petgraph::visit::EdgeRef;

    #[test]
    fn test_with_weights() {
        let g: petgraph::graph::UnGraph<usize, ()> =
            binomial_tree_graph(None, Some(vec![0, 1, 2, 3]), || 4, || (), false).unwrap();
        assert_eq!(
            vec![(0, 1), (1, 2), (2, 3)],
            g.edge_references()
                .map(|edge| (edge.source().index(), edge.target().index()))
                .collect::<Vec<(usize, usize)>>(),
        );
        assert_eq!(
            vec![0, 1, 2, 3],
            g.node_weights().copied().collect::<Vec<usize>>(),
        );
    }

    #[test]
    fn test_bidirectional() {
        let g: petgraph::graph::DiGraph<(), ()> =
            binomial_tree_graph(Some(4), None, || (), || (), true).unwrap();
        assert_eq!(
            vec![(0, 1), (1, 0), (1, 2), (2, 1), (2, 3), (3, 2),],
            g.edge_references()
                .map(|edge| (edge.source().index(), edge.target().index()))
                .collect::<Vec<(usize, usize)>>(),
        );
    }

    #[test]
    fn test_error() {
        match binomial_tree_graph::<petgraph::graph::DiGraph<(), ()>, (), _, _, ()>(
            None,
            None,
            || (),
            || (),
            false,
        ) {
            Ok(_) => panic!("Returned a non-error"),
            Err(e) => assert_eq!(e, InvalidInputError),
        };
    }
}
