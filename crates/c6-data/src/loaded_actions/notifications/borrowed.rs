//! Borrow the existing notification backend for synchronous nested queries.
use super::{lifecycle, processing, Created};
use c6_core::{
    loaded_state::LoadedState, notifications::ComponentId, player_notifications_owner::ObjectId,
};

impl<B: lifecycle::Backend + ?Sized> lifecycle::Backend for &mut B {
    fn current_turn(&mut self, s: &mut LoadedState) -> Result<i32, lifecycle::Error> {
        (**self).current_turn(s)
    }
    fn report(
        &mut self,
        s: &mut LoadedState,
        o: ObjectId,
        r: lifecycle::Report,
    ) -> Result<(), lifecycle::Error> {
        (**self).report(s, o, r)
    }
    fn overridden_activate(
        &mut self,
        s: &mut LoadedState,
        o: ObjectId,
        a: bool,
    ) -> Result<(), lifecycle::Error> {
        (**self).overridden_activate(s, o, a)
    }
    fn overridden_on_add(
        &mut self,
        s: &mut LoadedState,
        o: ObjectId,
    ) -> Result<(), lifecycle::Error> {
        (**self).overridden_on_add(s, o)
    }
    fn overridden_supersede(
        &mut self,
        s: &mut LoadedState,
        r: ObjectId,
        c: ObjectId,
    ) -> Result<(), lifecycle::Error> {
        (**self).overridden_supersede(s, r, c)
    }
    fn deleting_destructor(
        &mut self,
        s: &mut LoadedState,
        o: ObjectId,
    ) -> Result<(), lifecycle::Error> {
        (**self).deleting_destructor(s, o)
    }
}
impl<B: processing::Backend + ?Sized> processing::Backend for &mut B {
    fn manager_initialized(&mut self, s: &mut LoadedState) -> Result<bool, processing::Error> {
        (**self).manager_initialized(s)
    }
    fn game_uninitializing(&mut self, s: &mut LoadedState) -> Result<bool, processing::Error> {
        (**self).game_uninitializing(s)
    }
    fn allocate(
        &mut self,
        s: &mut LoadedState,
        h: u32,
    ) -> Result<processing::Allocation, processing::Error> {
        (**self).allocate(s, h)
    }
    fn validate_duplicate_identity(
        &mut self,
        s: &mut LoadedState,
        c: &Created,
        t: ComponentId,
    ) -> Result<(), processing::Error> {
        (**self).validate_duplicate_identity(s, c, t)
    }
    fn release_pending(
        &mut self,
        s: &mut LoadedState,
        c: &mut Option<Created>,
    ) -> Result<(), processing::Error> {
        (**self).release_pending(s, c)
    }
    fn initialize_text(
        &mut self,
        s: &mut LoadedState,
        c: &mut Created,
    ) -> Result<(), processing::Error> {
        (**self).initialize_text(s, c)
    }
    fn manager_game_local_player(
        &mut self,
        s: &mut LoadedState,
    ) -> Result<Option<i32>, processing::Error> {
        (**self).manager_game_local_player(s)
    }
    fn blocking_changed(
        &mut self,
        s: &mut LoadedState,
        e: u32,
        o: i32,
        n: i32,
    ) -> Result<(), processing::Error> {
        (**self).blocking_changed(s, e, o, n)
    }
}
