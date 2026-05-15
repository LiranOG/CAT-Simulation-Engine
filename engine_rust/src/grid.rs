// ============================================================================
// grid.rs - QuadTree spatial management.
// ============================================================================
// The spatial index allocates work in proportion to active civilization density.
// Empty regions are represented as empty leaf nodes. The current simulation
// rebuilds the tree once per tick, which is simpler and more reproducible than
// maintaining incremental updates for a sparse, frequently changing population.
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
    /// Maximum tree depth. Prevents infinite recursion on coincident agents.
    /// At depth D, the minimum cell dimension is width / 2^D. The default
    /// depth of 12 yields a minimum cell of about 0.244 units on a 1000-unit
    /// grid and keeps recursive stack depth bounded.
    pub max_depth: u32,
}

impl Default for GridConfig {
    /// Return the default square simulation domain and QuadTree limits.
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
    /// Construct an axis-aligned rectangle from inclusive minimum and maximum bounds.
    #[inline(always)]
    pub fn new(x_min: f64, y_min: f64, x_max: f64, y_max: f64) -> Self {
        Self {
            x_min,
            y_min,
            x_max,
            y_max,
        }
    }

    /// Return true when a point lies inside the inclusive rectangular bounds.
    #[inline(always)]
    pub fn contains_point(&self, x: f64, y: f64) -> bool {
        x >= self.x_min && x <= self.x_max && y >= self.y_min && y <= self.y_max
    }

    /// Return true when two axis-aligned boxes overlap on both axes.
    #[inline(always)]
    pub fn intersects(&self, other: &BoundingBox) -> bool {
        self.x_min <= other.x_max
            && self.x_max >= other.x_min
            && self.y_min <= other.y_max
            && self.y_max >= other.y_min
    }

    /// Return the geometric center used for deterministic quadrant splitting.
    #[inline(always)]
    pub fn center(&self) -> (f64, f64) {
        (
            (self.x_min + self.x_max) * 0.5,
            (self.y_min + self.y_max) * 0.5,
        )
    }

    /// Subdivide into four equal quadrants.
    ///
    /// Returns [SW, SE, NW, NE] in that order. The index mapping is stable
    /// across all callers; never reorder without updating `build_node`.
    pub fn subdivide(&self) -> [BoundingBox; 4] {
        let (cx, cy) = self.center();
        [
            BoundingBox::new(self.x_min, self.y_min, cx, cy), // SW
            BoundingBox::new(cx, self.y_min, self.x_max, cy), // SE
            BoundingBox::new(self.x_min, cy, cx, self.y_max), // NW
            BoundingBox::new(cx, cy, self.x_max, self.y_max), // NE
        ]
    }
}

/// Minimal agent reference stored in the tree: position + UUID only.
/// 32 bytes (align 8): Uuid=16B, x=8B, y=8B. Two refs per cache line.
#[derive(Debug, Clone)]
pub struct AgentRef {
    pub id: Uuid,
    pub x: f64,
    pub y: f64,
}

/// QuadTree node: either a leaf containing agents or an internal node
/// with up to four children. Empty nodes are represented as empty leaves,
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
        QuadNode::Leaf {
            bounds,
            agents: Vec::new(),
        }
    }

    /// Return the immutable spatial bounds represented by this node.
    pub fn bounds(&self) -> &BoundingBox {
        match self {
            QuadNode::Leaf { bounds, .. } => bounds,
            QuadNode::Internal { bounds, .. } => bounds,
        }
    }

    /// Return the number of active agents stored under this node.
    pub fn agent_count(&self) -> usize {
        match self {
            QuadNode::Leaf { agents, .. } => agents.len(),
            QuadNode::Internal { agent_count, .. } => *agent_count,
        }
    }
}

/// QuadTree: adaptive spatial indexing that allocates compute proportional to
/// civilization density. The void costs nothing.
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

    /// Rebuild the tree from a slice of agents, indexing only active ones.
    ///
    /// Called once per tick. Cheaper than incremental updates for the sparse
    /// civilisation densities this model targets.
    ///
    /// # Invariant assertion (debug builds)
    ///
    /// Verifies that the tree's total agent count matches the number of active
    /// agents supplied. A mismatch indicates a spatial-index desync that would
    /// silently corrupt range queries. Caught here before it propagates.
    pub fn rebuild(&mut self, agents: &[Agent]) {
        let bounds = BoundingBox::new(0.0, 0.0, self.config.width, self.config.height);

        let active_count = agents.iter().filter(|a| a.is_active()).count();

        let refs: Vec<AgentRef> = agents
            .iter()
            .filter(|a| a.is_active())
            .map(|a| AgentRef {
                id: a.id,
                x: a.position.0,
                y: a.position.1,
            })
            .collect();

        self.root = Self::build_node(bounds, refs, 0, &self.config);

        // Spatial index invariant: every active agent must be in the tree.
        debug_assert_eq!(
            self.root.agent_count(),
            active_count,
            "QuadTree desync: tree has {} agents, simulation has {} active.",
            self.root.agent_count(),
            active_count,
        );
    }

    /// Recursively build tree nodes.
    ///
    /// # Empty-bucket pruning (v2)
    ///
    /// Before calling `build_node` recursively on each quadrant, we check
    /// whether the bucket is empty. Empty quadrants become `QuadNode::empty`
    /// directly, with no recursive call, stack frame, or heap allocation for
    /// an empty Vec. This is important in the sparse-universe regime where
    /// large swaths of the grid are uninhabited.
    fn build_node(
        bounds: BoundingBox,
        agents: Vec<AgentRef>,
        depth: u32,
        config: &GridConfig,
    ) -> QuadNode {
        // Base case: fits in a leaf or depth limit reached.
        if agents.len() <= config.max_agents_per_node || depth >= config.max_depth {
            return QuadNode::Leaf { bounds, agents };
        }

        let quads = bounds.subdivide();
        let mut buckets: [Vec<AgentRef>; 4] = [vec![], vec![], vec![], vec![]];

        for agent in agents {
            // Agents on the exact boundary between quadrants go into the first
            // matching quadrant (SW). contains_point uses inclusive bounds on
            // all edges; the SW quadrant [x_min, cx] by [y_min, cy] wins ties.
            for (i, quad) in quads.iter().enumerate() {
                if quad.contains_point(agent.x, agent.y) {
                    buckets[i].push(agent);
                    break;
                }
            }
        }

        let total: usize = buckets.iter().map(|b| b.len()).sum();

        // Build each child. Empty buckets become empty leaves immediately:
        // no recursive call needed. This is the empty-bucket pruning fix.
        let children = [
            if buckets[0].is_empty() {
                QuadNode::empty(quads[0])
            } else {
                Self::build_node(quads[0], std::mem::take(&mut buckets[0]), depth + 1, config)
            },
            if buckets[1].is_empty() {
                QuadNode::empty(quads[1])
            } else {
                Self::build_node(quads[1], std::mem::take(&mut buckets[1]), depth + 1, config)
            },
            if buckets[2].is_empty() {
                QuadNode::empty(quads[2])
            } else {
                Self::build_node(quads[2], std::mem::take(&mut buckets[2]), depth + 1, config)
            },
            if buckets[3].is_empty() {
                QuadNode::empty(quads[3])
            } else {
                Self::build_node(quads[3], std::mem::take(&mut buckets[3]), depth + 1, config)
            },
        ];

        QuadNode::Internal {
            bounds,
            children: Box::new(children),
            agent_count: total,
        }
    }

    /// Range query: return IDs of all agents whose positions lie within `range`.
    /// Empty quadrants are skipped in O(1) via bounds intersection test.
    pub fn query_range(&self, range: &BoundingBox) -> Vec<Uuid> {
        let mut results = Vec::new();
        Self::query_node(&self.root, range, &mut results);
        results
    }

    /// Recursive implementation of rectangular range search.
    fn query_node(node: &QuadNode, range: &BoundingBox, results: &mut Vec<Uuid>) {
        if !node.bounds().intersects(range) {
            return; // Entire quadrant is outside range, so skip the subtree.
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

    /// Circular range query: return IDs of all agents within Euclidean distance
    /// `radius` of point `(cx, cy)`.
    ///
    /// # Two-phase filter (v2 fix)
    ///
    /// Phase 1: AABB pre-cull. Discard any quadrant whose bounding box does
    /// not intersect the query circle's circumscribed square. O(log N) with
    /// branch factor 4.
    ///
    /// Phase 2: exact distance check. For each candidate that survived Phase 1,
    /// compute squared Euclidean distance and compare against radius squared.
    /// This eliminates the corner overestimation of v1 (which returned agents
    /// up to 1.41 times the radius away from the center).
    ///
    /// # v1 bug
    ///
    /// The original implementation returned the AABB candidates directly and
    /// commented "AABB approximation is sufficient for influence-radius checks."
    /// This was incorrect because it returned agents in corners of the
    /// bounding square that were up to sqrt(2) times radius from the center, violating
    /// the circular-neighbourhood contract that any inter-agent influence system
    /// would reasonably assume.
    pub fn query_radius(&self, cx: f64, cy: f64, radius: f64) -> Vec<Uuid> {
        let bbox = BoundingBox::new(cx - radius, cy - radius, cx + radius, cy + radius);
        let radius_sq = radius * radius;

        let mut results = Vec::new();
        Self::query_node_radius(&self.root, &bbox, cx, cy, radius_sq, &mut results);
        results
    }

    /// Recursive implementation of exact circular radius search.
    fn query_node_radius(
        node: &QuadNode,
        bbox: &BoundingBox, // pre-computed circumscribed square
        cx: f64,
        cy: f64,
        radius_sq: f64,
        results: &mut Vec<Uuid>,
    ) {
        if !node.bounds().intersects(bbox) {
            return;
        }
        match node {
            QuadNode::Leaf { agents, .. } => {
                for agent in agents {
                    let dx = agent.x - cx;
                    let dy = agent.y - cy;
                    if dx * dx + dy * dy <= radius_sq {
                        results.push(agent.id);
                    }
                }
            }
            QuadNode::Internal { children, .. } => {
                for child in children.iter() {
                    Self::query_node_radius(child, bbox, cx, cy, radius_sq, results);
                }
            }
        }
    }

    /// Total number of active agents tracked in the tree.
    pub fn total_agents(&self) -> usize {
        self.root.agent_count()
    }

    /// Compute tree depth statistics for diagnostics.
    ///
    /// Returns (min_leaf_depth, max_leaf_depth, total_leaf_count).
    /// Log these at snapshot intervals to detect clustering pathologies
    /// that could degrade the QuadTree from O(K log N) to O(K * max_depth).
    pub fn depth_stats(&self) -> (u32, u32, usize) {
        Self::depth_walk(&self.root, 0)
    }

    /// Recursively accumulate minimum leaf depth, maximum leaf depth, and leaf count.
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

    // BoundingBox geometry tests.

    #[test]
    /// Verify inclusive point containment for rectangular bounds.
    fn test_bounding_box_contains() {
        let bb = BoundingBox::new(0.0, 0.0, 10.0, 10.0);
        assert!(bb.contains_point(5.0, 5.0));
        assert!(bb.contains_point(0.0, 0.0)); // inclusive lower bound
        assert!(bb.contains_point(10.0, 10.0)); // inclusive upper bound
        assert!(!bb.contains_point(10.1, 5.0));
        assert!(!bb.contains_point(-0.1, 5.0));
    }

    #[test]
    /// Verify intersection detection for overlapping and disjoint boxes.
    fn test_bounding_box_intersects() {
        let a = BoundingBox::new(0.0, 0.0, 10.0, 10.0);
        let b = BoundingBox::new(5.0, 5.0, 15.0, 15.0);
        let c = BoundingBox::new(11.0, 0.0, 20.0, 10.0); // no overlap
        assert!(a.intersects(&b));
        assert!(!a.intersects(&c));
    }

    #[test]
    /// Verify deterministic quadrant geometry.
    fn test_subdivide_geometry() {
        let bb = BoundingBox::new(0.0, 0.0, 100.0, 100.0);
        let quads = bb.subdivide();
        // SW
        assert_eq!(quads[0].x_min, 0.0);
        assert_eq!(quads[0].x_max, 50.0);
        assert_eq!(quads[0].y_max, 50.0);
        // NE
        assert_eq!(quads[3].x_min, 50.0);
        assert_eq!(quads[3].y_min, 50.0);
        assert_eq!(quads[3].x_max, 100.0);
        assert_eq!(quads[3].y_max, 100.0);
    }

    // Basic QuadTree behavior.

    #[test]
    /// Verify an empty tree contains one root leaf and no agents.
    fn test_empty_tree() {
        let config = GridConfig::default();
        let tree = QuadTree::new(&config);
        assert_eq!(tree.total_agents(), 0);
        let (min_d, max_d, leaves) = tree.depth_stats();
        assert_eq!(min_d, 0); // single empty root leaf
        assert_eq!(max_d, 0);
        assert_eq!(leaves, 1);
    }

    #[test]
    /// Verify rebuild and rectangular range query behavior.
    fn test_rebuild_and_range_query() {
        let config = GridConfig {
            width: 100.0,
            height: 100.0,
            max_agents_per_node: 2,
            max_depth: 8,
        };
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
        assert_eq!(found.len(), 1, "Only the agent at (10,10) is in the SW box");
    }

    // query_radius exact circular filter.

    #[test]
    /// Verify exact radius queries exclude square-corner false positives.
    fn test_query_radius_excludes_corners() {
        // Place agents at the four corners of a square of side 2r.
        // Only agents within radius r of the centre must be returned.
        // Corner agents are at distance r * sqrt(2), so they must be excluded.
        let config = GridConfig {
            width: 200.0,
            height: 200.0,
            max_agents_per_node: 1,
            max_depth: 8,
        };
        let mut tree = QuadTree::new(&config);
        let r = 10.0_f64;
        let cx = 100.0_f64;
        let cy = 100.0_f64;

        // Four agents exactly at the corners of the query square:
        // distance from center = r * sqrt(2), outside the circle.
        let corner_agents = vec![
            Agent::new((cx - r, cy - r), 0.1, 0.5, 0.3, 0.01, 0.001, 0.0, 0),
            Agent::new((cx + r, cy - r), 0.1, 0.5, 0.3, 0.01, 0.001, 0.0, 0),
            Agent::new((cx - r, cy + r), 0.1, 0.5, 0.3, 0.01, 0.001, 0.0, 0),
            Agent::new((cx + r, cy + r), 0.1, 0.5, 0.3, 0.01, 0.001, 0.0, 0),
        ];
        tree.rebuild(&corner_agents);

        let found = tree.query_radius(cx, cy, r);
        assert_eq!(
            found.len(),
            0,
            "Corner agents at distance r * sqrt(2) must not be returned by query_radius"
        );
    }

    #[test]
    /// Verify exact radius queries include points inside the circle.
    fn test_query_radius_includes_inner() {
        let config = GridConfig {
            width: 200.0,
            height: 200.0,
            max_agents_per_node: 1,
            max_depth: 8,
        };
        let mut tree = QuadTree::new(&config);
        let r = 10.0_f64;
        let cx = 100.0_f64;
        let cy = 100.0_f64;

        // One agent exactly at the centre (distance 0), one at 0.9r (inside),
        // one at 1.1r (outside).
        let agents = vec![
            Agent::new((cx, cy), 0.1, 0.5, 0.3, 0.01, 0.001, 0.0, 0), // d=0
            Agent::new((cx + 0.9 * r, cy), 0.1, 0.5, 0.3, 0.01, 0.001, 0.0, 0), // d=0.9r
            Agent::new((cx + 1.1 * r, cy), 0.1, 0.5, 0.3, 0.01, 0.001, 0.0, 0), // d=1.1r
        ];
        tree.rebuild(&agents);

        let found = tree.query_radius(cx, cy, r);
        assert_eq!(
            found.len(),
            2,
            "Expected 2 agents within radius: centre + 0.9r"
        );
    }

    // Degenerate spatial cases.

    #[test]
    /// Verify coincident agents terminate at max_depth without recursion failure.
    fn test_degenerate_coincident_agents() {
        // All agents at exactly the same position. The tree must recurse to
        // max_depth, return a single leaf with all agents, and not overflow
        // the stack or panic.
        let n = 50usize;
        let config = GridConfig {
            width: 100.0,
            height: 100.0,
            max_agents_per_node: 4,
            max_depth: 12,
        };
        let mut tree = QuadTree::new(&config);
        let agents: Vec<Agent> = (0..n)
            .map(|_| Agent::new((50.0, 50.0), 0.1, 0.5, 0.3, 0.01, 0.001, 0.0, 0))
            .collect();

        tree.rebuild(&agents); // Must not panic or overflow.
        assert_eq!(tree.total_agents(), n);

        // All agents are at (50, 50); a query at that exact point should
        // return all of them.
        let found = tree.query_radius(50.0, 50.0, 1.0);
        assert_eq!(found.len(), n, "All coincident agents must be queryable");
    }

    #[test]
    /// Verify clustered agents cannot force recursion beyond max_depth.
    fn test_depth_limit_prevents_infinite_recursion() {
        // Stress-test the depth cap with 1000 agents clustered in a tiny area.
        let config = GridConfig {
            width: 1000.0,
            height: 1000.0,
            max_agents_per_node: 2,
            max_depth: 8,
        };
        let mut tree = QuadTree::new(&config);
        // All agents within a 0.001 by 0.001 unit square, forcing max_depth.
        let agents: Vec<Agent> = (0..100)
            .map(|i| {
                let offset = i as f64 * 0.000_01;
                Agent::new(
                    (500.0 + offset, 500.0 + offset),
                    0.1,
                    0.5,
                    0.3,
                    0.01,
                    0.001,
                    0.0,
                    0,
                )
            })
            .collect();

        tree.rebuild(&agents);

        let (_, max_d, _) = tree.depth_stats();
        assert!(
            max_d <= config.max_depth,
            "Tree depth {} exceeded configured max_depth {}",
            max_d,
            config.max_depth
        );
    }

    #[test]
    /// Verify empty-bucket pruning avoids dense empty subtrees.
    fn test_empty_bucket_pruning() {
        // With only 1 agent in one quadrant, the other 3 quadrants should
        // be empty leaves without recursive build_node calls.
        let config = GridConfig {
            width: 100.0,
            height: 100.0,
            max_agents_per_node: 0,
            max_depth: 4,
        };
        let mut tree = QuadTree::new(&config);
        // max_agents_per_node = 0 forces subdivision of every non-empty node.
        let agents = vec![Agent::new((10.0, 10.0), 0.1, 0.5, 0.3, 0.01, 0.001, 0.0, 0)];
        tree.rebuild(&agents);
        assert_eq!(tree.total_agents(), 1);
        // Tree should have very few leaves despite max_depth=4 due to pruning.
        let (_, _, leaf_count) = tree.depth_stats();
        // Without pruning: 4^4 = 256 leaves at depth 4. With pruning: ~4 leaves.
        assert!(
            leaf_count < 20,
            "Empty-bucket pruning should dramatically reduce leaf count, got {}",
            leaf_count
        );
    }

    #[test]
    /// Verify depth diagnostics are internally consistent.
    fn test_depth_stats() {
        let config = GridConfig {
            width: 100.0,
            height: 100.0,
            max_agents_per_node: 1,
            max_depth: 4,
        };
        let mut tree = QuadTree::new(&config);
        let agents = vec![
            Agent::new((10.0, 10.0), 0.1, 0.5, 0.3, 0.01, 0.001, 0.0, 0),
            Agent::new((90.0, 90.0), 0.1, 0.5, 0.3, 0.01, 0.001, 0.0, 0),
        ];
        tree.rebuild(&agents);
        let (min_d, max_d, leaves) = tree.depth_stats();
        assert!(min_d <= max_d, "min_depth must be <= max_depth");
        assert!(
            leaves >= 2,
            "At least 2 leaf nodes for 2 spatially distinct agents"
        );
    }
}
