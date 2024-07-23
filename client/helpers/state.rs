/// State of a node.
#[derive(Clone, Debug, PartialOrd, PartialEq, Eq, Ord)]
pub struct NodeState {
    pub height: u32,
    pub healthy: bool,
}

impl Default for NodeState {
    fn default() -> Self {
        NodeState {
            height: 0,
            healthy: true,
        }
    }
}
