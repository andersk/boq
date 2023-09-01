use sha1::{Digest, Sha1};

use crate::{
    avatar::AvatarSettings,
    types::{RealmId, UserId},
};

fn user_avatar_hash(uid: &str, avatar_salt: &str) -> String {
    // WARNING: If this method is changed, you may need to do a migration
    // similar to zerver/migrations/0060_move_avatars_to_be_uid_based.py .
    //
    // The salt probably doesn't serve any purpose now.  In the past we used a
    // hash of the email address, not the user ID, and we salted it in order to
    // make the hashing scheme different from Gravatar's.

    let mut hasher = Sha1::new();
    hasher.update(uid);
    hasher.update(avatar_salt);
    format!("{:x}", hasher.finalize())
}

pub fn user_avatar_path_from_ids(
    user_profile_id: UserId,
    realm_id: RealmId,
    settings: &AvatarSettings,
) -> String {
    let user_id_hash = user_avatar_hash(&user_profile_id.to_string(), &settings.avatar_salt);
    format!("{realm_id}/{user_id_hash}")
}
