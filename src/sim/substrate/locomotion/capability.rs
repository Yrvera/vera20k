//! Per-class locomotor capabilities that change native control flow.

use super::class::LocomotorClass;

/// Whether a live class implements the native piggyback capability.
///
/// Five active-YR classes expose it. The sixth native provider is DropPod,
/// which is deliberately absent because stock YR never selects it.
pub const fn piggyback_capable(class: LocomotorClass) -> bool {
    matches!(
        class,
        LocomotorClass::Drive
            | LocomotorClass::Ship
            | LocomotorClass::Walk
            | LocomotorClass::Jumpjet
            | LocomotorClass::Teleport
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn five_live_classes_are_piggyback_capable() {
        let capable: Vec<_> = LocomotorClass::ALL
            .into_iter()
            .filter(|class| piggyback_capable(*class))
            .collect();
        assert_eq!(
            capable,
            [
                LocomotorClass::Drive,
                LocomotorClass::Walk,
                LocomotorClass::Teleport,
                LocomotorClass::Ship,
                LocomotorClass::Jumpjet,
            ]
        );
    }
}
