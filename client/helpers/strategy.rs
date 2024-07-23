/// Load balancing strategy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Strategy {
    Random,
    GreatestHeight,
}
