//! AllianceCombats current cached-player predicate and AddItem continuation.
//! No alliance membership is inferred from names or current diplomacy state.
use super::{collections, registry};
#[derive(Debug)]
pub enum Error<E> {
    Context(E),
    Proposal(registry::Error),
    MissingCollection(u64),
    MissingKind(u64),
    WrongKind(u64),
    Collection(collections::Error<E>),
}
type ContextError<C> = <C as registry::Context>::Error;

/// HasPlayer23416d0, for the typed proposal constructor's disjoint member
/// variants. Current attacker owner is tested before reading defender owner.
pub fn has_player<C: registry::Context>(
    c: &mut C,
    handle: registry::Handle,
    player: i32,
) -> Result<bool, Error<C::Error>> {
    let p = c
        .runtime()
        .map_err(Error::Context)?
        .proposal(handle)
        .map_err(Error::Proposal)?;
    let (attacker, defender) = (p.attacker, p.defender);
    if c.owner(attacker).map_err(Error::Context)? == player {
        return Ok(true);
    }
    match defender {
        Some(defender) => Ok(c.owner(defender).map_err(Error::Context)? == player),
        None => Ok(false),
    }
}

/// ObjectRelevant2399950, then OnCombatProposed23999b0. Receiver+60 and+64
/// are source-defined cached players, not an invitation to recompute alliances.
pub fn on_combat_proposed<C>(
    c: &mut C,
    collection: u64,
    handle: registry::Handle,
) -> Result<(), Error<ContextError<C>>>
where
    C: registry::Context + collections::Context<Error = ContextError<C>>,
{
    let kind = c
        .collections()
        .map_err(Error::Context)?
        .collection(collection)
        .ok_or(Error::MissingCollection(collection))?
        .kind
        .ok_or(Error::MissingKind(collection))?;
    let collections::Kind::Alliance {
        first_player,
        second_player,
    } = kind
    else {
        return Err(Error::WrongKind(collection));
    };
    if (first_player != -1 && has_player(c, handle, first_player)?)
        || (second_player != -1 && has_player(c, handle, second_player)?)
    {
        collections::add_item(c, collection, handle).map_err(Error::Collection)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{map::UnitReference, unit::ComponentId, unit_moved_signal::ListenerRing};
    struct Test {
        owner: registry::Runtime,
        callbacks: Vec<u64>,
        reads: Vec<i32>,
        fail: bool,
    }
    fn unit(owner: i16, id: u32) -> UnitReference {
        UnitReference::from_component_id(ComponentId {
            owner,
            component_type: 1,
            id,
        })
    }
    impl registry::Context for Test {
        type Error = &'static str;
        fn runtime(&self) -> Result<&registry::Runtime, Self::Error> {
            Ok(&self.owner)
        }
        fn runtime_mut(&mut self) -> Result<&mut registry::Runtime, Self::Error> {
            Ok(&mut self.owner)
        }
        fn owner(&mut self, p: registry::Participant) -> Result<i32, Self::Error> {
            let registry::Participant::Unit(u) = p else {
                unreachable!()
            };
            let owner = i32::from(u.component_id().owner);
            self.reads.push(owner);
            Ok(owner)
        }
        fn unit_id(&mut self, u: UnitReference) -> Result<ComponentId, Self::Error> {
            Ok(u.component_id())
        }
        fn callback(&mut self, _: u64, _: registry::Handle) -> Result<(), Self::Error> {
            unreachable!()
        }
    }
    impl collections::Context for Test {
        type Error = &'static str;
        fn collections(&self) -> Result<&collections::Runtime, Self::Error> {
            Ok(self.owner.collections().unwrap())
        }
        fn collections_mut(&mut self) -> Result<&mut collections::Runtime, Self::Error> {
            Ok(self.owner.collections_mut().unwrap())
        }
        fn item_changed(
            &mut self,
            receiver: u64,
            p: &mut registry::Handle,
            added: bool,
        ) -> Result<(), Self::Error> {
            assert!(added);
            self.callbacks.push(receiver);
            self.owner
                .proposal_mut(*p)
                .unwrap()
                .defender_strength_modifier += 3;
            if self.fail {
                Err("modifier")
            } else {
                Ok(())
            }
        }
    }
    fn current(pairs: &[(i32, i32)]) -> (Test, registry::Handle) {
        let collections = collections::Runtime::from_initial(
            pairs
                .iter()
                .enumerate()
                .map(|(i, &(a, b))| {
                    (
                        i as u64,
                        collections::Collection {
                            kind: Some(collections::Kind::Alliance {
                                first_player: a,
                                second_player: b,
                            }),
                            added: ListenerRing::from_ordered(&[i as u64]).unwrap(),
                        },
                    )
                })
                .collect(),
        )
        .unwrap();
        let mut owner = registry::Runtime::from_initial(
            7832,
            vec![],
            Some(ListenerRing::from_ordered(&[]).unwrap()),
            Some(vec![]),
            Some(vec![]),
        )
        .unwrap()
        .with_collections(collections)
        .unwrap();
        let (_, h) = owner
            .register(registry::Proposal::melee_units(
                unit(12, 7),
                unit(62, 8),
                None,
            ))
            .unwrap();
        (
            Test {
                owner,
                callbacks: vec![],
                reads: vec![],
                fail: false,
            },
            h,
        )
    }
    #[test]
    fn ten_current_cached_pairs_reach_only_the_two_matching_collections() {
        let pairs = [
            (4, 6),
            (7, 9),
            (0, 7),
            (8, 10),
            (15, 16),
            (3, 12),
            (3, 14),
            (4, 12),
            (9, 14),
            (8, 16),
        ];
        let (mut c, h) = current(&pairs);
        for i in 0..10 {
            on_combat_proposed(&mut c, i, h).unwrap();
        }
        assert_eq!(c.callbacks, [5, 7]);
        assert_eq!(c.owner.proposal(h).unwrap().defender_strength_modifier, 6);
    }
    #[test]
    fn first_player_match_skips_second_has_player_and_minus_one_skips_both() {
        let (mut c, h) = current(&[(12, 62), (-1, -1)]);
        on_combat_proposed(&mut c, 0, h).unwrap();
        assert_eq!(c.reads, [12]);
        c.reads.clear();
        on_combat_proposed(&mut c, 1, h).unwrap();
        assert!(c.reads.is_empty());
        assert_eq!(c.callbacks, [0]);
    }
    #[test]
    fn reached_modifier_error_retains_the_real_proposal_mutation() {
        let (mut c, h) = current(&[(3, 12)]);
        c.fail = true;
        assert!(matches!(
            on_combat_proposed(&mut c, 0, h),
            Err(Error::Collection(collections::Error::Context("modifier")))
        ));
        assert_eq!(c.owner.proposal(h).unwrap().defender_strength_modifier, 3);
    }
}
