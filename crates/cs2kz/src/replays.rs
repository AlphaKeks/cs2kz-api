use uuid::Uuid;

pub mod cleaner;

define_id_type! {
    /// A unique identifier for replays.
    pub struct ReplayId(Uuid);
}

impl ReplayId {
    pub(crate) fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }
}
