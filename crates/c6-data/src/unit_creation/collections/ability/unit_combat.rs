//! Concrete UnitCombat23434f0 + AdjustPlayerStrengthModifier23505f0 recipe.
//! The bonus is deferred to proposed combat and its ordinary subject requirements.
use super::*;
use c6_core::melee_attack::proposals::{
    collection_modifiers::{EffectFamily, Modifier},
    player_strength,
    registry::Participant,
};

pub(super) fn attach(
    owners: &mut Owners<'_>,
    context: u64,
    unit: UnitReference,
    instance: i32,
    recipe: &Recipe,
) -> Result<(), Error> {
    let receiver = (1u64 << 59) | u64::from(instance as u32);
    let effect = (1u64 << 58) | u64::from(instance as u32);
    let collection = (1u64 << 57) | u64::from(instance as u32);
    owners
        .effects
        .register_player_strength_effect(
            owners.state,
            effect,
            context,
            player_strength::Effect {
                parameters: player_strength::Parameters {
                    amount: recipe.amount,
                    stack_scale: 100,
                    maximum_property_adjustment: i32::MAX,
                    property: u32::MAX,
                    property_percentage_q8: 256,
                    game_turn_scale: 0,
                    divide_by_ever_alive_majors: false,
                },
                applied_total: 0,
            },
        )
        .map_err(Error::Query)?;
    let proposals = owners
        .state
        .combat_proposals_mut()
        .map_err(|_| Error::Missing("current UnitCombat proposals owner"))?;
    proposals
        .collection_modifiers_mut()
        .map_err(|_| Error::Missing("current combat collection modifiers"))?
        .register_current(Modifier {
            effect_family: EffectFamily::PlayerStrength,
            receiver,
            context,
            instance,
            control: common::Modifier {
                enabled: true,
                suppressed: false,
                owner_requirement_set: None,
            },
            definition: recipe.id.clone(),
            repeatable: recipe.repeatable,
            stack_limit: recipe.stack,
            subject_requirements: recipe.subject.clone(),
            effect,
            tracked: vec![],
            applied: vec![],
        })
        .map_err(|_| Error::Missing("duplicate ability combat modifier"))?;
    proposals
        .collections_mut()
        .map_err(|_| Error::Missing("current combat collections"))?
        .import_current(vec![(
            collection,
            collections::Collection {
                kind: Some(collections::Kind::Unit { unit }),
                added: c6_core::unit_moved_signal::ListenerRing::from_ordered(&[receiver])
                    .map_err(Error::Ring)?,
            },
        )])
        .map_err(|_| Error::Missing("duplicate ability combat collection"))?;
    proposals
        .subscribe_unit_combat(unit.component_id(), collection)
        .map_err(|_| Error::Missing("current UnitCombat signal map"))?;
    // GetItems23433e0 filters already-active proposals by the complete Unit ID.
    // Freshly initialized units have no relevant active combat. Retain the
    // registrations above before reporting a reached unsupported immediate apply.
    for (_, handle) in proposals.active() {
        let proposal = proposals
            .proposal(*handle)
            .map_err(|_| Error::Missing("current ability combat proposal"))?;
        if proposal.attacker == Participant::Unit(unit)
            || proposal.defender == Some(Participant::Unit(unit))
        {
            return Err(Error::Missing(
                "ability attachment to existing relevant combat",
            ));
        }
    }
    let object = owners
        .effects
        .object_mut(context)
        .map_err(Error::Query)?
        .unit_lifecycle
        .as_mut()
        .ok_or(Error::Missing("ability owner lifecycle"))?;
    object.owned_modifiers.push(instance);
    object
        .destroyed
        .subscribe(Callback::OwnerDestroyed(receiver))
        .map_err(Error::Ring)?;
    Ok(())
}
