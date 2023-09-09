use std::collections::HashSet;

use serde::Serialize;

use crate::types::{MessageFlags, UserId};

#[derive(Clone, Debug, Serialize)]
pub struct UserMessageNotificationsData {
    #[serde(skip_serializing)]
    pub user_id: UserId,
    pub online_push_enabled: bool,
    pub dm_email_notify: bool,
    pub dm_push_notify: bool,
    pub mention_email_notify: bool,
    pub mention_push_notify: bool,
    pub topic_wildcard_mention_email_notify: bool,
    pub topic_wildcard_mention_push_notify: bool,
    pub stream_wildcard_mention_email_notify: bool,
    pub stream_wildcard_mention_push_notify: bool,
    pub stream_push_notify: bool,
    pub stream_email_notify: bool,
    pub followed_topic_push_notify: bool,
    pub followed_topic_email_notify: bool,
    pub topic_wildcard_mention_in_followed_topic_push_notify: bool,
    pub topic_wildcard_mention_in_followed_topic_email_notify: bool,
    pub stream_wildcard_mention_in_followed_topic_push_notify: bool,
    pub stream_wildcard_mention_in_followed_topic_email_notify: bool,
    pub sender_is_muted: bool,
    pub disable_external_notifications: bool,
}

pub struct UserIdSets {
    pub private_message: bool,
    pub disable_external_notifications: bool,
    pub online_push_user_ids: HashSet<UserId>,
    pub dm_mention_push_disabled_user_ids: HashSet<UserId>,
    pub dm_mention_email_disabled_user_ids: HashSet<UserId>,
    pub stream_push_user_ids: HashSet<UserId>,
    pub stream_email_user_ids: HashSet<UserId>,
    pub topic_wildcard_mention_user_ids: HashSet<UserId>,
    pub stream_wildcard_mention_user_ids: HashSet<UserId>,
    pub followed_topic_push_user_ids: HashSet<UserId>,
    pub followed_topic_email_user_ids: HashSet<UserId>,
    pub topic_wildcard_mention_in_followed_topic_user_ids: HashSet<UserId>,
    pub stream_wildcard_mention_in_followed_topic_user_ids: HashSet<UserId>,
    pub muted_sender_user_ids: HashSet<UserId>,
    pub all_bot_user_ids: HashSet<UserId>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationTrigger {
    // "direct_message" is for 1:1 direct messages as well as huddles
    DirectMessage,
    Mentioned,
    TopicWildcardMentioned,
    StreamWildcardMentioned,
    StreamPushNotify,
    StreamEmailNotify,
    FollowedTopicPushNotify,
    FollowedTopicEmailNotify,
    TopicWildcardMentionedInFollowedTopic,
    StreamWildcardMentionedInFollowedTopic,
}

impl UserMessageNotificationsData {
    pub fn from_user_id_sets(
        user_id: UserId,
        flags: &MessageFlags,
        &UserIdSets {
            private_message,
            disable_external_notifications,
            ref online_push_user_ids,
            ref dm_mention_push_disabled_user_ids,
            ref dm_mention_email_disabled_user_ids,
            ref stream_push_user_ids,
            ref stream_email_user_ids,
            ref topic_wildcard_mention_user_ids,
            ref stream_wildcard_mention_user_ids,
            ref followed_topic_push_user_ids,
            ref followed_topic_email_user_ids,
            ref topic_wildcard_mention_in_followed_topic_user_ids,
            ref stream_wildcard_mention_in_followed_topic_user_ids,
            ref muted_sender_user_ids,
            ref all_bot_user_ids,
        }: &UserIdSets,
    ) -> UserMessageNotificationsData {
        if all_bot_user_ids.contains(&user_id) {
            UserMessageNotificationsData {
                user_id,
                dm_email_notify: false,
                mention_email_notify: false,
                topic_wildcard_mention_email_notify: false,
                stream_wildcard_mention_email_notify: false,
                dm_push_notify: false,
                mention_push_notify: false,
                topic_wildcard_mention_push_notify: false,
                stream_wildcard_mention_push_notify: false,
                online_push_enabled: false,
                stream_push_notify: false,
                stream_email_notify: false,
                followed_topic_push_notify: false,
                followed_topic_email_notify: false,
                topic_wildcard_mention_in_followed_topic_push_notify: false,
                topic_wildcard_mention_in_followed_topic_email_notify: false,
                stream_wildcard_mention_in_followed_topic_push_notify: false,
                stream_wildcard_mention_in_followed_topic_email_notify: false,
                sender_is_muted: false,
                disable_external_notifications: false,
            }
        } else {
            // `stream_wildcard_mention_user_ids`,
            // `topic_wildcard_mention_user_ids`,
            // `stream_wildcard_mention_in_followed_topic_user_ids` and
            // `topic_wildcard_mention_in_followed_topic_user_ids` are those
            // user IDs for whom stream or topic wildcard mentions should obey
            // notification settings for personal mentions. Hence, it isn't an
            // independent notification setting && acts as a wrapper.
            let dm_email_notify =
                !dm_mention_email_disabled_user_ids.contains(&user_id) && private_message;
            let mention_email_notify = !dm_mention_email_disabled_user_ids.contains(&user_id)
                && flags.contains("mentioned");
            let topic_wildcard_mention_email_notify = topic_wildcard_mention_user_ids
                .contains(&user_id)
                && !dm_mention_email_disabled_user_ids.contains(&user_id)
                && flags.contains("wildcard_mentioned");
            let stream_wildcard_mention_email_notify = stream_wildcard_mention_user_ids
                .contains(&user_id)
                && !dm_mention_email_disabled_user_ids.contains(&user_id)
                && flags.contains("wildcard_mentioned");
            let topic_wildcard_mention_in_followed_topic_email_notify =
                topic_wildcard_mention_in_followed_topic_user_ids.contains(&user_id)
                    && !dm_mention_email_disabled_user_ids.contains(&user_id)
                    && flags.contains("wildcard_mentioned");
            let stream_wildcard_mention_in_followed_topic_email_notify =
                stream_wildcard_mention_in_followed_topic_user_ids.contains(&user_id)
                    && !dm_mention_email_disabled_user_ids.contains(&user_id)
                    && flags.contains("wildcard_mentioned");

            let dm_push_notify =
                !dm_mention_push_disabled_user_ids.contains(&user_id) && private_message;
            let mention_push_notify = !dm_mention_push_disabled_user_ids.contains(&user_id)
                && flags.contains("mentioned");
            let topic_wildcard_mention_push_notify = topic_wildcard_mention_user_ids
                .contains(&user_id)
                && !dm_mention_push_disabled_user_ids.contains(&user_id)
                && flags.contains("wildcard_mentioned");
            let stream_wildcard_mention_push_notify = stream_wildcard_mention_user_ids
                .contains(&user_id)
                && !dm_mention_push_disabled_user_ids.contains(&user_id)
                && flags.contains("wildcard_mentioned");
            let topic_wildcard_mention_in_followed_topic_push_notify =
                topic_wildcard_mention_in_followed_topic_user_ids.contains(&user_id)
                    && !dm_mention_push_disabled_user_ids.contains(&user_id)
                    && flags.contains("wildcard_mentioned");
            let stream_wildcard_mention_in_followed_topic_push_notify =
                stream_wildcard_mention_in_followed_topic_user_ids.contains(&user_id)
                    && !dm_mention_push_disabled_user_ids.contains(&user_id)
                    && flags.contains("wildcard_mentioned");

            UserMessageNotificationsData {
                user_id,
                dm_email_notify,
                mention_email_notify,
                topic_wildcard_mention_email_notify,
                stream_wildcard_mention_email_notify,
                dm_push_notify,
                mention_push_notify,
                topic_wildcard_mention_push_notify,
                stream_wildcard_mention_push_notify,
                online_push_enabled: online_push_user_ids.contains(&user_id),
                stream_push_notify: stream_push_user_ids.contains(&user_id),
                stream_email_notify: stream_email_user_ids.contains(&user_id),
                followed_topic_push_notify: followed_topic_push_user_ids.contains(&user_id),
                followed_topic_email_notify: followed_topic_email_user_ids.contains(&user_id),
                topic_wildcard_mention_in_followed_topic_push_notify,
                topic_wildcard_mention_in_followed_topic_email_notify,
                stream_wildcard_mention_in_followed_topic_push_notify,
                stream_wildcard_mention_in_followed_topic_email_notify,
                sender_is_muted: muted_sender_user_ids.contains(&user_id),
                disable_external_notifications,
            }
        }
    }

    // For these functions, acting_user_id is the user sent a message (or edited
    // a message) triggering the event for which we need to determine
    // notifiability.

    /// Common check for reasons not to trigger a notification that arex
    /// independent of users' notification settings and thus don't depend on
    /// what type of notification (email/push) it is.
    fn trivially_should_not_notify(&self, acting_user_id: UserId) -> bool {
        self.user_id == acting_user_id
            || self.sender_is_muted
            || self.disable_external_notifications
    }

    pub fn is_notifiable(&self, acting_user_id: UserId, idle: bool) -> bool {
        self.is_email_notifiable(acting_user_id, idle)
            || self.is_push_notifiable(acting_user_id, idle)
    }

    pub fn is_push_notifiable(&self, acting_user_id: UserId, idle: bool) -> bool {
        self.get_push_notification_trigger(acting_user_id, idle)
            .is_some()
    }

    pub fn get_push_notification_trigger(
        &self,
        acting_user_id: UserId,
        idle: bool,
    ) -> Option<NotificationTrigger> {
        if (!idle && !self.online_push_enabled) || self.trivially_should_not_notify(acting_user_id)
        {
            None
        }
        // The order here is important. If, for example, both
        // `mention_push_notify` and `stream_push_notify` are True, we
        // want to classify it as a mention, since that's more salient.
        else if self.dm_push_notify {
            Some(NotificationTrigger::DirectMessage)
        } else if self.mention_push_notify {
            Some(NotificationTrigger::Mentioned)
        } else if self.topic_wildcard_mention_in_followed_topic_push_notify {
            Some(NotificationTrigger::TopicWildcardMentionedInFollowedTopic)
        } else if self.stream_wildcard_mention_in_followed_topic_push_notify {
            Some(NotificationTrigger::StreamWildcardMentionedInFollowedTopic)
        } else if self.topic_wildcard_mention_push_notify {
            Some(NotificationTrigger::TopicWildcardMentioned)
        } else if self.stream_wildcard_mention_push_notify {
            Some(NotificationTrigger::StreamWildcardMentioned)
        } else if self.followed_topic_push_notify {
            Some(NotificationTrigger::FollowedTopicPushNotify)
        } else if self.stream_push_notify {
            Some(NotificationTrigger::StreamPushNotify)
        } else {
            None
        }
    }

    pub fn is_email_notifiable(&self, acting_user_id: UserId, idle: bool) -> bool {
        self.get_email_notification_trigger(acting_user_id, idle)
            .is_some()
    }

    pub fn get_email_notification_trigger(
        &self,
        acting_user_id: UserId,
        idle: bool,
    ) -> Option<NotificationTrigger> {
        if !idle || self.trivially_should_not_notify(acting_user_id) {
            None
        }
        // The order here is important. If, for example, both
        // `mention_email_notify` and `stream_email_notify` are True, we
        // want to classify it as a mention, since that's more salient.
        else if self.dm_email_notify {
            Some(NotificationTrigger::DirectMessage)
        } else if self.mention_email_notify {
            Some(NotificationTrigger::Mentioned)
        } else if self.topic_wildcard_mention_in_followed_topic_email_notify {
            Some(NotificationTrigger::TopicWildcardMentionedInFollowedTopic)
        } else if self.stream_wildcard_mention_in_followed_topic_email_notify {
            Some(NotificationTrigger::StreamWildcardMentionedInFollowedTopic)
        } else if self.topic_wildcard_mention_email_notify {
            Some(NotificationTrigger::TopicWildcardMentioned)
        } else if self.stream_wildcard_mention_email_notify {
            Some(NotificationTrigger::StreamWildcardMentioned)
        } else if self.followed_topic_email_notify {
            Some(NotificationTrigger::FollowedTopicEmailNotify)
        } else if self.stream_email_notify {
            Some(NotificationTrigger::StreamEmailNotify)
        } else {
            None
        }
    }
}
