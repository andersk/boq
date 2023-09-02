use anyhow::Result;
use lapin::options::BasicPublishOptions;
use lapin::BasicProperties;
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use serde_json::Value;

use crate::app_state::AppState;
use crate::avatar::get_avatar_field;
use crate::notification_data::{NotificationTrigger, UserIdSets, UserMessageNotificationsData};
use crate::queues::{Client, QueueId, Queues};
use crate::types::{MessageFlags, MessageId, RealmId, UserGroupId, UserId};

#[derive(Debug, Deserialize)]
struct MessageUser {
    id: UserId,
    #[serde(default)]
    flags: MessageFlags,
    #[serde(default)]
    mentioned_user_group_id: Option<UserId>,
}

#[derive(Debug, Deserialize)]
struct LegacyMessageUser {
    stream_push_notify: bool,
    #[serde(default)]
    stream_email_notify: bool,
    #[serde(default)]
    wildcard_mention_notify: bool,
    #[serde(default)]
    sender_is_muted: bool,
    #[serde(default)]
    online_push_enabled: bool,
    #[serde(default)]
    always_push_notify: bool,
    // We can calculate `mentioned` from the usermessage flags, so just remove
    // it
    // #[serde(default)]
    // mentioned: bool,
    #[serde(flatten)]
    user: MessageUser,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum MessageUsers {
    Users(Vec<MessageUser>),
    LegacyUsers(Vec<LegacyMessageUser>),
}

#[derive(Clone, Copy, Debug, Deserialize_repr, Eq, PartialEq, Serialize_repr)]
#[repr(u8)]
pub enum EmailAddressVisibility {
    Everyone = 1,
    Members = 2,
    Admins = 3,
    Nobody = 4,
    Moderators = 5,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ContentType {
    #[serde(rename = "text/html")]
    Html,
    #[serde(rename = "text/x-markdown")]
    Markdown,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UserDisplayRecipient {
    pub email: String,
    pub full_name: String,
    pub id: UserId,
    pub is_mirror_dummy: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum MessageRecipient {
    Private {
        display_recipient: Vec<UserDisplayRecipient>,
    },
    Stream {
        display_recipient: String,
        #[serde(alias = "topic")]
        subject: String,
    },
}

#[derive(Clone, Debug, Deserialize)]
pub struct WideMessage {
    #[serde(flatten)]
    attrs: HashMap<String, Value>,
    sender_email: String,
    sender_delivery_email: Option<String>,
    sender_id: UserId,
    id: MessageId,
    client: String,
    #[serde(skip)]
    avatar_url: (),
    sender_email_address_visibility: EmailAddressVisibility,
    sender_realm_id: RealmId,
    sender_avatar_source: String,
    sender_avatar_version: i32,
    #[serde(skip)]
    content_type: (),
    rendered_content: String,
    content: String,
    #[serde(flatten)]
    recipient: MessageRecipient,
    recipient_type: i16,
    recipient_type_id: i32,
    sender_is_mirror_dummy: bool,
    #[serde(skip)]
    invite_only_stream: (),
}

#[derive(Clone, Debug, Serialize)]
pub struct Message {
    #[serde(flatten)]
    pub attrs: HashMap<String, Value>,
    pub sender_email: String,
    pub sender_id: UserId,
    pub id: MessageId,
    pub client: String,
    pub avatar_url: Option<String>,
    pub content_type: ContentType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rendered_content: Option<String>,
    pub content: String,
    #[serde(flatten)]
    pub recipient: MessageRecipient,
    #[serde(skip_serializing_if = "is_false")]
    pub invite_only_stream: bool,
}

#[derive(Debug, Eq, Hash, PartialEq)]
struct MessageFlavor {
    apply_markdown: bool,
    client_gravatar: bool,
}

impl WideMessage {
    fn finalize_payload(
        &self,
        &MessageFlavor {
            apply_markdown,
            mut client_gravatar,
        }: &MessageFlavor,
        keep_rendered_content: bool,
        invite_only_stream: bool,
    ) -> Message {
        if self.sender_email_address_visibility != EmailAddressVisibility::Everyone {
            // If email address of the sender is only available to
            // administrators, clients cannot compute gravatars, so we force-set
            // it to false. If we plumbed the current user's role, we could
            // allow client_gravatar=True here if the current user's role has
            // access to the target user's email address.
            client_gravatar = false;
        }
        let avatar_url = get_avatar_field(
            self.sender_id,
            self.sender_realm_id,
            self.sender_delivery_email.as_ref().unwrap(),
            &self.sender_avatar_source,
            self.sender_avatar_version,
            false,
            client_gravatar,
        );
        let (content_type, content) = if apply_markdown {
            (ContentType::Html, &self.rendered_content)
        } else {
            (ContentType::Markdown, &self.content)
        };
        let rendered_content = keep_rendered_content.then(|| self.rendered_content.clone());
        Message {
            sender_email: self.sender_email.clone(),
            sender_id: self.sender_id,
            id: self.id,
            client: self.client.clone(),
            avatar_url,
            content_type,
            rendered_content,
            content: content.clone(),
            recipient: self.recipient.clone(),
            invite_only_stream,
            attrs: self.attrs.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct MessageEvent {
    sender_queue_id: Option<QueueId>,
    #[serde(default)]
    stream_name: Option<String>,
    #[serde(default)]
    invite_only: bool,
    #[serde(default)]
    realm_id: Option<RealmId>,
    local_id: Option<String>,

    #[serde(default)]
    presence_idle_user_ids: HashSet<UserId>,
    #[serde(default)]
    online_push_user_ids: HashSet<UserId>,
    // TODO/compatibility: Remove this alias when one can no longer
    // directly upgrade from 7.x to main.
    #[serde(default, alias = "pm_mention_push_disabled_user_ids")]
    dm_mention_push_disabled_user_ids: HashSet<UserId>,
    // TODO/compatibility: Remove this alias when one can no longer
    // directly upgrade from 7.x to main.
    #[serde(default, alias = "pm_mention_email_disabled_user_ids")]
    dm_mention_email_disabled_user_ids: HashSet<UserId>,
    #[serde(default)]
    stream_push_user_ids: HashSet<UserId>,
    #[serde(default)]
    stream_email_user_ids: HashSet<UserId>,
    #[serde(default)]
    topic_wildcard_mention_user_ids: HashSet<UserId>,
    // TODO/compatibility: Remove this alias when one can no longer directly
    // upgrade from 7.x to main.
    #[serde(default, alias = "wildcard_mention_user_ids")]
    stream_wildcard_mention_user_ids: HashSet<UserId>,
    #[serde(default)]
    followed_topic_push_user_ids: HashSet<UserId>,
    #[serde(default)]
    followed_topic_email_user_ids: HashSet<UserId>,
    #[serde(default)]
    topic_wildcard_mention_in_followed_topic_user_ids: HashSet<UserId>,
    #[serde(default)]
    stream_wildcard_mention_in_followed_topic_user_ids: HashSet<UserId>,
    #[serde(default)]
    muted_sender_user_ids: HashSet<UserId>,
    #[serde(default)]
    all_bot_user_ids: HashSet<UserId>,
    #[serde(default)]
    disable_external_notifications: bool,

    message_dict: WideMessage,
}

#[derive(Clone, Debug, Serialize)]
pub struct MessageUserInternalData {
    #[serde(flatten)]
    user_notifications_data: UserMessageNotificationsData,
    mentioned_user_group_id: Option<UserGroupId>,
    #[serde(flatten)]
    notified: Notified,
}

fn receiver_is_off_zulip(queues: &Queues, user_id: UserId) -> bool {
    !queues.for_user(user_id).is_some_and(|client_keys| {
        client_keys
            .iter()
            .any(|&client_key| queues.get(client_key).accepts_messages())
    })
}

fn is_false(b: &bool) -> bool {
    !b
}

#[derive(Clone, Debug, Default, Serialize)]
struct Notified {
    #[serde(skip_serializing_if = "is_false")]
    push_notified: bool,
    #[serde(skip_serializing_if = "is_false")]
    email_notified: bool,
}

#[derive(Debug, Serialize)]
struct OfflineNotice {
    user_profile_id: UserId,
    message_id: MessageId,
    trigger: NotificationTrigger,
    mentioned_user_group_id: Option<UserGroupId>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
enum OfflinePushNotice {
    Add(OfflineNotice),
    Remove {
        user_profile_id: UserId,
        message_ids: Vec<MessageId>,
    },
}

/// See https://zulip.readthedocs.io/en/latest/subsystems/notifications.html for
/// high-level design documentation.
fn maybe_enqueue_notifications(
    state: &Arc<AppState>,
    user_notifications_data: &UserMessageNotificationsData,
    acting_user_id: UserId,
    message_id: MessageId,
    mentioned_user_group_id: Option<UserGroupId>,
    idle: bool,
    already_notified: &Notified,
) -> Result<Notified> {
    let mut notified = Notified::default();

    if !already_notified.push_notified {
        if let Some(trigger) =
            user_notifications_data.get_push_notification_trigger(acting_user_id, idle)
        {
            let notice = OfflinePushNotice::Add(OfflineNotice {
                user_profile_id: user_notifications_data.user_id,
                message_id,
                trigger,
                mentioned_user_group_id,
            });
            let payload = serde_json::to_vec(&notice)?;
            let state = Arc::clone(state);
            tokio::spawn(async move {
                state
                    .rabbitmq_channel
                    .basic_publish(
                        "",
                        "missedmessage_mobile_notifications",
                        BasicPublishOptions::default(),
                        &payload,
                        BasicProperties::default().with_delivery_mode(2),
                    )
                    .await
            });
            notified.push_notified = true;
        }
    }

    // Send missed_message emails if a direct message or a mention. Eventually,
    // we'll add settings to allow email notifications to match the model of
    // push notifications above.
    if !already_notified.email_notified {
        if let Some(trigger) =
            user_notifications_data.get_email_notification_trigger(acting_user_id, idle)
        {
            let notice = OfflineNotice {
                user_profile_id: user_notifications_data.user_id,
                message_id,
                trigger,
                mentioned_user_group_id,
            };
            let payload = serde_json::to_vec(&notice)?;
            let state = Arc::clone(state);
            tokio::spawn(async move {
                state
                    .rabbitmq_channel
                    .basic_publish(
                        "",
                        "missedmessage_emails",
                        BasicPublishOptions::default(),
                        &payload,
                        BasicProperties::default().with_delivery_mode(2),
                    )
                    .await
            });
            notified.email_notified = true;
        }
    }

    Ok(notified)
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ClientEvent {
    Message {
        message: Arc<Message>,
        flags: MessageFlags,
        #[serde(skip_serializing_if = "Option::is_none")]
        internal_data: Option<MessageUserInternalData>,
        #[serde(skip_serializing_if = "Option::is_none")]
        local_message_id: Option<String>,
    },
}

fn enqueue_message_to_client(
    wide_message: &WideMessage,
    flavor_cache: &mut HashMap<MessageFlavor, Arc<Message>>,
    client: &mut Client,
    flags: &MessageFlags,
    is_sender: bool,
    internal_data: Option<&MessageUserInternalData>,
    invite_only: bool,
    local_id: Option<&String>,
) {
    if !client.accepts_messages() {
        // The actual check is the accepts_event() check below; this line is
        // just an optimization to avoid copying message data unnecessarily
        return;
    }

    let sending_client = &wide_message.client;
    let client_info = client.info();

    // Make sure Zephyr mirroring bots know whether stream is invite-only
    let invite_only_stream = client_info.client_type_name.contains("mirror") && invite_only;

    let flavor = MessageFlavor {
        apply_markdown: client_info.apply_markdown,
        client_gravatar: client_info.client_gravatar,
    };
    let message = Arc::clone(flavor_cache.entry(flavor).or_insert_with_key(|flavor| {
        Arc::new(wide_message.finalize_payload(flavor, false, invite_only_stream))
    }));

    let user_event = ClientEvent::Message {
        message,
        flags: flags.clone(),
        internal_data: internal_data.cloned(),
        local_message_id: if is_sender { local_id.cloned() } else { None },
    };

    if !client.accepts_event(&user_event) {
        return;
    }

    // The below prevents (Zephyr) mirroring loops.
    if sending_client.contains("mirror")
        && sending_client.to_lowercase() == client_info.client_type_name.to_lowercase()
    {
        return;
    }

    client.add_event(user_event);
}

/// See https://zulip.readthedocs.io/en/latest/subsystems/sending-messages.html
/// for high-level documentation on this subsystem.
fn process_message_event(
    state: &Arc<AppState>,
    mut event_template: MessageEvent,
    users: MessageUsers,
) -> Result<()> {
    let users = match users {
        MessageUsers::Users(users) => users,

        // do_send_messages used to send events with users in dict format, with
        // the dict containing the user_id and other data. We later trimmed down
        // the user data to only contain the user_id and the usermessage flags,
        // and put everything else in the event dict as lists. This block
        // handles any old-format events still in the queue during upgrade.
        //
        // TODO/compatibility: Remove this whole block once one can no longer
        // directly upgrade directly from 4.x to 5.0-dev.
        MessageUsers::LegacyUsers(legacy_users) => legacy_users
            .into_iter()
            .map(|legacy_user| {
                let user_id = legacy_user.user.id;
                if legacy_user.stream_push_notify {
                    event_template.stream_push_user_ids.insert(user_id);
                }
                if legacy_user.stream_email_notify {
                    event_template.stream_email_user_ids.insert(user_id);
                }
                if legacy_user.wildcard_mention_notify {
                    event_template
                        .stream_wildcard_mention_user_ids
                        .insert(user_id);
                }
                if legacy_user.sender_is_muted {
                    event_template.muted_sender_user_ids.insert(user_id);
                }

                // TODO/compatibility: Another translation code block for the
                // rename of `always_push_notify` to `online_push_enabled`.
                // Remove this when one can no longer directly upgrade from 4.x
                // to 5.0-dev.
                if legacy_user.online_push_enabled || legacy_user.always_push_notify {
                    event_template.online_push_user_ids.insert(user_id);
                }

                legacy_user.user
            })
            .collect(),
    };

    tracing::debug!("processing message event {event_template:?} {users:?}");

    let presence_idle_user_ids = event_template.presence_idle_user_ids;

    let mut wide_message = event_template.message_dict;

    // TODO/compatibility: Zulip servers that have message events in their event
    // queues and upgrade to the new version that expects sender_delivery_email
    // in these events will throw errors processing events. We can remove this
    // alias once we don't expect anyone to be directly upgrading from 2.0.x to
    // the latest Zulip.
    wide_message
        .sender_delivery_email
        .get_or_insert_with(|| wide_message.sender_email.clone());

    let sender_id = wide_message.sender_id;
    let message_id = wide_message.id;

    let user_id_sets = UserIdSets {
        private_message: matches!(wide_message.recipient, MessageRecipient::Private { .. }),
        disable_external_notifications: event_template.disable_external_notifications,
        online_push_user_ids: event_template.online_push_user_ids,
        dm_mention_push_disabled_user_ids: event_template.dm_mention_push_disabled_user_ids,
        dm_mention_email_disabled_user_ids: event_template.dm_mention_email_disabled_user_ids,
        stream_push_user_ids: event_template.stream_push_user_ids,
        stream_email_user_ids: event_template.stream_email_user_ids,
        topic_wildcard_mention_user_ids: event_template.topic_wildcard_mention_user_ids,
        stream_wildcard_mention_user_ids: event_template.stream_wildcard_mention_user_ids,
        followed_topic_push_user_ids: event_template.followed_topic_push_user_ids,
        followed_topic_email_user_ids: event_template.followed_topic_email_user_ids,
        topic_wildcard_mention_in_followed_topic_user_ids: event_template
            .topic_wildcard_mention_in_followed_topic_user_ids,
        stream_wildcard_mention_in_followed_topic_user_ids: event_template
            .stream_wildcard_mention_in_followed_topic_user_ids,
        muted_sender_user_ids: event_template.muted_sender_user_ids,
        all_bot_user_ids: event_template.all_bot_user_ids,
    };

    let mut flavor_cache = HashMap::new();
    let mut queues = state.queues.lock().unwrap();

    let processed_user_ids: HashSet<UserId> = users
        .into_iter()
        .map(|user_data| {
            let user_profile_id = user_data.id;
            let flags = &user_data.flags;
            let mentioned_user_group_id = user_data.mentioned_user_group_id;

            // If the recipient was offline and the message was a (1:1 or group)
            // direct message to them or they were @-notified potentially notify
            // more immediately
            let user_notifications_data = UserMessageNotificationsData::from_user_id_sets(
                user_profile_id,
                &flags,
                &user_id_sets,
            );

            // If the message isn't notifiable had the user been idle, then the user
            // shouldn't receive notifications even if they were online. In that
            // case we can avoid the more expensive `receiver_is_off_zulip` call,
            // and move on to process the next user.
            let notified = if user_notifications_data.is_notifiable(sender_id, true) {
                let idle = receiver_is_off_zulip(&queues, user_profile_id)
                    || presence_idle_user_ids.contains(&user_profile_id);
                maybe_enqueue_notifications(
                    state,
                    &user_notifications_data,
                    sender_id,
                    message_id,
                    mentioned_user_group_id,
                    idle,
                    &Notified {
                        push_notified: false,
                        email_notified: false,
                    },
                )?
            } else {
                Notified::default()
            };

            let internal_data = MessageUserInternalData {
                user_notifications_data,
                mentioned_user_group_id,
                notified,
            };

            if let Some(client_keys) = queues.for_user(user_profile_id) {
                for client_key in client_keys.clone() {
                    let client = queues.get_mut(client_key);
                    let is_sender = Some(client.queue_id) == event_template.sender_queue_id;
                    enqueue_message_to_client(
                        &wide_message,
                        &mut flavor_cache,
                        client,
                        &user_data.flags,
                        is_sender,
                        Some(&internal_data),
                        event_template.invite_only,
                        event_template.local_id.as_ref(),
                    );
                }
            }

            Ok(user_profile_id)
        })
        .collect::<Result<_>>()?;

    if event_template.stream_name.is_some() && !event_template.invite_only {
        if let Some(realm_id) = event_template.realm_id {
            if let Some(client_keys) = queues.for_realm_all_streams(realm_id) {
                for client_key in client_keys.clone() {
                    let client = queues.get_mut(client_key);

                    if processed_user_ids.contains(&client.info().user_profile_id) {
                        continue;
                    }

                    let is_sender = Some(client.queue_id) == event_template.sender_queue_id;
                    enqueue_message_to_client(
                        &wide_message,
                        &mut flavor_cache,
                        client,
                        &MessageFlags::new(),
                        is_sender,
                        None,
                        event_template.invite_only,
                        event_template.local_id.as_ref(),
                    );
                }
            }
        }
    }

    Ok(())
}

fn process_update_message_event(event: HashMap<String, Value>, users: &RawValue) {
    tracing::debug!("processing update_message event {event:?} {users:?}");
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyDeleteMessageUser {
    id: UserId,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum DeleteMessageUsers {
    Ids(Vec<UserId>),
    LegacyUsers(Vec<LegacyDeleteMessageUser>),
}

fn process_delete_message_event(event: HashMap<String, Value>, users: DeleteMessageUsers) {
    tracing::debug!("processing delete_message event {event:?} {users:?}");
}

fn process_presence_event(event: HashMap<String, Value>, users: Vec<UserId>) {
    tracing::debug!("processing presence event {event:?} {users:?}");
}

fn process_custom_profile_fields_event(event: HashMap<String, Value>, users: Vec<UserId>) {
    tracing::debug!("processing custom_profile_fields event {event:?} {users:?}");
}

/// This event may be generated to forward cleanup requests to the right shard.
fn process_cleanup_queue_event(event: HashMap<String, Value>, (user,): (UserId,)) {
    tracing::debug!("processing cleanup_queue event {event:?} {user:?}");
}

fn process_other_event(event: HashMap<String, Value>, users: Vec<UserId>) {
    tracing::debug!("processing event {event:?} {users:?}");
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    Message(MessageEvent),
    UpdateMessage(HashMap<String, Value>),
    DeleteMessage(HashMap<String, Value>),
    Presence(HashMap<String, Value>),
    CustomProfileFields(HashMap<String, Value>),
    CleanupQueue(HashMap<String, Value>),
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
pub struct Notice<'a> {
    #[serde(borrow)]
    event: &'a RawValue,
    #[serde(borrow)]
    users: &'a RawValue,
}

pub fn process_notice(state: &Arc<AppState>, notice: Notice) -> Result<()> {
    tracing::debug!("processing {notice:?}");

    let Notice { event, users } = notice;

    match serde_json::from_str::<Event>(event.get())? {
        Event::Message(event) => {
            process_message_event(state, event, serde_json::from_str(users.get())?)?
        }
        Event::UpdateMessage(event) => process_update_message_event(event, users),
        Event::DeleteMessage(event) => {
            process_delete_message_event(event, serde_json::from_str(users.get())?)
        }
        Event::Presence(event) => process_presence_event(event, serde_json::from_str(users.get())?),
        Event::CustomProfileFields(event) => {
            process_custom_profile_fields_event(event, serde_json::from_str(users.get())?)
        }
        Event::CleanupQueue(event) => {
            process_cleanup_queue_event(event, serde_json::from_str(users.get())?)
        }
        Event::Other => process_other_event(
            serde_json::from_str(event.get())?,
            serde_json::from_str(users.get())?,
        ),
    }

    Ok(())
}
