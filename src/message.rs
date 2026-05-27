use crate::types::{Pos, ResourceKind};

pub enum Message {
    ResourceDiscovered {
        pos: Pos,
        kind: ResourceKind,
        quantity: u32,
    },
    ResourceCollected {
        pos: Pos,
        kind: ResourceKind,
        amount: u32,
    },
    ResourceDepleted {
        pos: Pos,
    },
}
