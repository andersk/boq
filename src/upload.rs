pub const MEDIUM_AVATAR_SIZE: i32 = 500;

pub fn get_avatar_url(hash_key: &str, medium: bool) -> String {
    // TODO/boq: Support S3
    let medium_suffix = if medium { "-medium" } else { "" };
    format!("/user_avatars/{hash_key}{medium_suffix}.png")
}
