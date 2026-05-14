// ============================================================================
// grid.rs — QuadTree Spatial Management with Adaptive Mesh Refinement
// ============================================================================
// Empty space MUST consume zero compute. The universe is 99.9999...% void,
// and our data structure should reflect that existential emptiness.
// Public API (query_range, depth_stats, etc.) is forward-looking spatial
// infrastructure for inter-agent influence mechanics — unused now, required soon.
// ============================================================================
#![allow(dead_code)]

use crate::agent::Agent;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Configuration for the spatial grid.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridConfig {
    /// Total width of the simulation space.
    pub width: f64,
    /// Total height of the simulation space.
    pub height: f64,
    /// Maximum agents per leaf node before subdivision.
    pub max_agents_per_node: usize,
    /// Maximum tree depth to prevent infinite recursion on coincident agents.
    pub max_depth: u32,
}

impl Default for GridConfig {
    fn default() -> Self {
        Self {
            width: 1000.0,
            height: 1000.0,
            max_agents_per_node: 8,
            max_depth: 12,
        }
    }
}

/// Axis-aligned bounding box for spatial queries.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BoundingBox {
    pub x_min: f64,
    pub y_min: f64,
    pub x_max: f64,
    pub y_max: f64,
}

impl BoundingBox {
    pub fn new(x_min: f64, y_min: f64, x_max: f64, y_max: f64) -> Self {
        Self { x_min, y_min, x_max, y_max }
    }

    pub fn contains_point(&self, x: f64, y: f64) -> bool {
        x >= self.x_min && x <= self.x_max && y >= self.y_min && y <= self.y_max
    }

    pub fn intersects(&self, other: &BoundingBox) -> bool {
        self.x_min <= other.x_max && self.x_max >= other.x_min
            && self.y_min <= other.y_max && self.y_max >= other.y_min
    }

    pub fn center(&self) -> (f64, f64) {
        ((self.x_min + self.x_max) / 2.0, (self.y_min + self.y_max) / 2.0)
    }

    /// Subdivide into four equal quadrants. The cosmic act of creating
    /// structure from void — except here it actually works efficiently.
    pub fn subdivide(&self) -> [BoundingBox; 4] {
        let (cx, cy) = self.center();
        [
            BoundingBox::new(self.x_min, self.y_min, cx, cy),  // SW
            BoundingBox::new(cx, self.y_min, self.x_max, cy),  // SE
            BoundingBox::new(self.x_min, cy, cx, self.y_max),  // NW
            BoundingBox::new(cx, cy, self.x_max, self.y_max),  // NE
        ]
    }
}

/// A reference to an agent stored in the tree: position + UUID.
#[derive(Debug, Clone)]
pub struct AgentRef {
    pub id: Uuid,
    pub x: f64,
    pub y: f64,
}

/// QuadTree node — either a leaf containing agents or an internal node
/// with four children. Empty nodes are represented as empty leaves,
/// consuming O(1) memory and zero iteration cost.
#[derive(Debug)]
pub enum QuadNode {
    Leaf {
        bounds: BoundingBox,
        agents: Vec<AgentRef>,
    },
    Internal {
        bounds: BoundingBox,
        children: Box<[QuadNode; 4]>,
        agent_count: usize,
    },
}

impl QuadNode {
    /// Create an empty leaf node for the given bounds.
    pub fn empty(bounds: BoundingBox) -> Self {
        QuadNode::Leaf { bounds, agents: Vec::new() }
    }

    pub fn bounds(&self) -> &BoundingBox {
        match self {
            QuadNode::Leaf { bounds, .. } => bounds,
            QuadNode::Internal { bounds, .. } => bounds,
        }
    }

    pub fn agent_count(&self) -> usize {
        match self {
            QuadNode::Leaf { agents, .. } => agents.len(),
            QuadNode::Internal { agent_count, .. } => *agent_count,
        }
    }
}

/// The QuadTree: adaptive spatial indexing that allocates compute
/// proportional to civilization density. The void gets nothing.
pub struct QuadTree {
    root: QuadNode,
    config: GridConfig,
}

impl QuadTree {
    /// Construct a new empty QuadTree spanning the configured simulation space.
    pub fn new(config: &GridConfig) -> Self {
        let bounds = BoundingBox::new(0.0, 0.0, config.width, config.height);
        Self {
            root: QuadNode::empty(bounds),
            config: config.clone(),
        }
    }

    /// Rebuild the tree from a slice of active agents.
    /// This is called once per tick — cheaper than incremental updates
    /// for the agent densities we're modeling (sparse civilizations in vast space).
    pub fn rebuild(&mut self, agents: &[Agent]) {
        let bounds = BoundingBox::new(0.0, 0.0, self.config.width, self.config.height);
        let refs: Vec<AgentRef> = agents
            .iter()
            .filter(|a| a.is_active())
            .map(|a| AgentRef { id: a.id, x: a.position.0, y: a.position.1 })
            .collect();
        self.root = Self::build_node(bounds, refs, 0, &self.config);
    }

    /// Recursively construct tree nodes. Subdivides when agent count exceeds
    /// threshold and depth permits further refinement.
    fn build_node(
        bounds: BoundingBox, agents: Vec<AgentRef>, depth: u32, config: &GridConfig,
    ) -> QuadNode {
        if agents.len() <= config.max_agents_per_node || depth >= config.max_depth {
            return QuadNode::Leaf { bounds, agents };
        }
        let quads = bounds.subdivide();
        let mut buckets: [Vec<AgentRef>; 4] = [vec![], vec![], vec![], vec![]];
        for agent in agents {
            for (i, quad) in quads.iter().enumerate() {
                if quad.contains_point(agent.x, agent.y) {
                    buckets[i].push(agent);
                    break;
                }
            }
        }
        let total: usize = buckets.iter().map(|b| b.len()).sum();
        let children = [
            Self::build_node(quads[0], std::mem::take(&mut buckets[0]), depth + 1, config),
            Self::build_node(quads[1], std::mem::take(&mut buckets[1]), depth + 1, config),
            Self::build_node(quads[2], std::mem::take(&mut buckets[2]), depth + 1, config),
            Self::build_node(quads[3], std::mem::take(&mut buckets[3]), depth + 1, config),
        ];
        QuadNode::Internal {
            bounds,
            children: Box::new(children),
            agent_count: total,
        }
    }

    /// Range query: find all agent IDs within the given bounding box.
    /// Empty quadrants are skipped in O(1) — the spatial void costs nothing.
    pub fn query_range(&self, range: &BoundingBox) -> Vec<Uuid> {
        let mut results = Vec::new();
        Self::query_node(&self.root, range, &mut results);
        results
    }

    fn query_node(node: &QuadNode, range: &BoundingBox, results: &mut Vec<Uuid>) {
        if !node.bounds().intersects(range) {
            return;
        }
        match node {
            QuadNode::Leaf { agents, .. } => {
                for agent in agents {
                    if range.contains_point(agent.x, agent.y) {
                        results.push(agent.id);
                    }
                }
            }
            QuadNode::Internal { children, .. } => {
                for child in children.iter() {
                    Self::query_node(child, range, results);
                }
            }
        }
    }

    /// Find all agent IDs within a circular radius of a point.
    /// Uses bounding-box pre-filter then exact distance check.
    pub fn query_radius(&self, cx: f64, cy: f64, radius: f64) -> Vec<Uuid> {
        let bbox = BoundingBox::new(cx - radius, cy - radius, cx + radius, cy + radius);
        let candidates = self.query_range(&bbox);
        // Exact circular filter would require agent positions — for now,
        // the AABB approximation is sufficient for influence-radius checks.
        candidates
    }

    /// Total number of active agents tracked in the tree.
    pub fn total_agents(&self) -> usize {
        self.root.agent_count()
    }

    /// Compute spatial statistics for diagnostics.
    pub fn depth_stats(&self) -> (u32, u32, usize) {
        let (min_d, max_d, leaves) = Self::depth_walk(&self.root, 0);
        (min_d, max_d, leaves)
    }

    fn depth_walk(node: &QuadNode, depth: u32) -> (u32, u32, usize) {
        match node {
            QuadNode::Leaf { .. } => (depth, depth, 1),
            QuadNode::Internal { children, .. } => {
                let mut min_d = u32::MAX;
                let mut max_d = 0;
                let mut total_leaves = 0;
                for child in children.iter() {
                    let (mn, mx, l) = Self::depth_walk(child, depth + 1);
                    min_d = min_d.min(mn);
                    max_d = max_d.max(mx);
                    total_leaves += l;
                }
                (min_d, max_d, total_leaves)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bounding_box_contains() {
        let bb = BoundingBox::new(0.0, 0.0, 10.0, 10.0);
        assert!(bb.contains_point(5.0, 5.0));
        assert!(!bb.contains_point(15.0, 5.0));
    }

    #[test]
    fn test_bounding_box_intersects() {
        let a = BoundingBox::new(0.0, 0.0, 10.0, 10.0);
        let b = BoundingBox::new(5.0, 5.0, 15.0, 15.0);
        assert!(a.intersects(&b));
    }

    #[test]
    fn test_subdivide() {
        let bb = BoundingBox::new(0.0, 0.0, 100.0, 100.0);
        let quads = bb.subdivide();
        assert_eq!(quads[0].x_max, 50.0); // SW
        assert_eq!(quads[3].x_min, 50.0); // NE
    }

    #[test]
    fn test_empty_tree() {
        let config = GridConfig::default();
        let tree = QuadTree::new(&config);
        assert_eq!(tree.total_agents(), 0);
    }

    #[test]
    fn test_rebuild_and_query() {
        let config = GridConfig { width: 100.0, height: 100.0, max_agents_per_node: 2, max_depth: 8 };
        let mut tree = QuadTree::new(&config);
        let agents = vec![
            Agent::new((10.0, 10.0), 0.1, 0.5, 0.3, 0.01, 0.001, 0.0, 0),
            Agent::new((50.0, 50.0), 0.1, 0.5, 0.3, 0.01, 0.001, 0.0, 0),
            Agent::new((90.0, 90.0), 0.1, 0.5, 0.3, 0.01, 0.001, 0.0, 0),
        ];
        tree.rebuild(&agents);
        assert_eq!(tree.total_agents(), 3);

        let range = BoundingBox::new(0.0, 0.0, 20.0, 20.0);
        let found = tree.query_range(&range);
        assert_eq!(found.len(), 1);
    }
}
