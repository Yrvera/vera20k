//! The LogicClass active-object vector: the single authority on object order.
//!
//! Owns an insertion-ordered list of stable_ids. Tail-append on reveal,
//! order-preserving compacting remove on conceal, no sort. Membership itself is
//! tracked by a flag on each entity (see `GameEntity::in_logic_vector`); this type
//! owns only the order. Serializes transparently as its inner `Vec<u64>` so the
//! saved order is restored verbatim.
//!
//! Dependency rules: part of sim/ — depends only on std + serde.

/// Insertion-ordered, membership-gated active-object order.
#[derive(Debug, Default, Clone)]
pub struct LogicVector {
    order: Vec<u64>,
}

impl LogicVector {
    pub fn new() -> Self {
        Self { order: Vec::new() }
    }

    /// Tail-append. Caller guarantees `id` is not already present (the membership
    /// flag guard lives in `Simulation::register_live_object`).
    pub fn push(&mut self, id: u64) {
        self.order.push(id);
    }

    /// Order-preserving compacting remove. No-op if absent. Never swap-remove.
    pub fn remove(&mut self, id: u64) {
        self.order.retain(|&x| x != id);
    }

    /// The order verbatim — no sorted fallback, no filtering.
    pub fn snapshot(&self) -> Vec<u64> {
        self.order.clone()
    }

    /// Borrow the order for hashing / iteration.
    pub fn as_slice(&self) -> &[u64] {
        &self.order
    }

    pub fn len(&self) -> usize {
        self.order.len()
    }

    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }

    pub fn clear(&mut self) {
        self.order.clear();
    }

    /// Test-only: force a specific order (e.g. opposite stable-id order).
    #[cfg(test)]
    pub fn set_order_for_test(&mut self, order: Vec<u64>) {
        self.order = order;
    }
}

impl serde::Serialize for LogicVector {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.order.serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for LogicVector {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Self {
            order: Vec::<u64>::deserialize(deserializer)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_appends_to_tail_no_sort() {
        let mut v = LogicVector::new();
        v.push(5);
        v.push(1);
        v.push(3);
        assert_eq!(v.snapshot(), vec![5, 1, 3]); // insertion order, not sorted
    }

    #[test]
    fn unregister_preserves_order_compacting() {
        let mut v = LogicVector::new();
        v.push(10);
        v.push(20);
        v.push(30);
        v.remove(20);
        assert_eq!(v.snapshot(), vec![10, 30]); // left-shift, tail preserved
    }

    #[test]
    fn unregister_absent_id_is_safe() {
        let mut v = LogicVector::new();
        v.push(1);
        v.remove(99);
        assert_eq!(v.snapshot(), vec![1]);
    }

    #[test]
    fn snapshot_is_order_verbatim() {
        let mut v = LogicVector::new();
        v.push(7);
        v.push(2);
        assert_eq!(v.snapshot(), v.as_slice().to_vec());
    }

    #[test]
    fn serde_roundtrip_preserves_order() {
        let mut v = LogicVector::new();
        v.push(9);
        v.push(4);
        v.push(6);
        let bytes = bincode::serialize(&v).expect("serialize");
        let back: LogicVector = bincode::deserialize(&bytes).expect("deserialize");
        assert_eq!(back.snapshot(), vec![9, 4, 6]);
    }
}
