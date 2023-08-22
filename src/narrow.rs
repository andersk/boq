use serde::Deserialize;

use crate::notice::{Message, MessageRecipient};
use crate::types::MessageFlags;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Operator {
    Stream,
    Topic,
    Sender,
    Is,
}

pub type Narrow = Vec<(Operator, String)>;

const RESOLVED_TOPIC_PREFIX: &str = "✔ ";

pub fn matches_narrow(message: &Message, flags: &MessageFlags, narrow: &Narrow) -> bool {
    // TODO: Eventually handle negated narrow terms.
    narrow.iter().all(|(operator, operand)| match operator {
        Operator::Stream => match &message.recipient {
            MessageRecipient::Stream {
                display_recipient, ..
            } => operand.to_lowercase() == display_recipient.to_lowercase(),
            _ => false,
        },
        Operator::Topic => match &message.recipient {
            MessageRecipient::Stream { subject, .. } => {
                operand.to_lowercase() == subject.to_lowercase()
            }
            _ => false,
        },
        Operator::Sender => operand.to_lowercase() == message.sender_email.to_lowercase(),
        Operator::Is => match operand.as_str() {
            "dm" | "private" => matches!(message.recipient, MessageRecipient::Private { .. }),
            "starred" => flags.contains("starred"),
            "unread" => !flags.contains("read"),
            "alerted" | "mentioned" => flags.contains("mentioned"),
            "resolved" => match &message.recipient {
                MessageRecipient::Stream { subject, .. } => {
                    subject.starts_with(RESOLVED_TOPIC_PREFIX)
                }
                _ => false,
            },
            _ => true,
        },
    })
}
