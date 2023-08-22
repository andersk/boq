use crate::types::{RealmId, UserId};

pub fn get_avatar_field(
    user_id: UserId,
    realm_id: RealmId,
    email: &str,
    avatar_source: &str,
    avatar_version: i32,
    medium: bool,
    client_gravatar: bool,
) -> Option<String> {
    // TODO/boq: get_avatar_field
    None
}
