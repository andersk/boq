use amq_protocol_types::FieldTable;
use amq_protocol_uri::{AMQPAuthority, AMQPQueryString, AMQPScheme, AMQPUri, AMQPUserInfo};
use anyhow::Result;
use futures_lite::StreamExt;
use lapin::message::Delivery;
use lapin::options::{BasicAckOptions, BasicConsumeOptions, BasicQosOptions, QueueDeclareOptions};
use lapin::{Channel, Connection, ConnectionProperties, Consumer};
use std::sync::Arc;

use crate::app_state::AppState;
use crate::notice::process_notice;

pub struct RabbitMQ {
    pub channel: Channel,
    consumer: Consumer,
}

impl RabbitMQ {
    pub async fn connect(
        host: String,
        username: String,
        password: String,
        notify_queue: &str,
    ) -> Result<RabbitMQ> {
        let port = 5672;
        tracing::debug!("connecting to queue on {username}@{host}:{port}");

        let connection = Connection::connect_uri(
            AMQPUri {
                scheme: AMQPScheme::AMQP,
                authority: AMQPAuthority {
                    userinfo: AMQPUserInfo { username, password },
                    host,
                    port,
                },
                vhost: "/".into(),
                query: AMQPQueryString::default(),
            },
            ConnectionProperties::default()
                .with_executor(tokio_executor_trait::Tokio::current())
                .with_reactor(tokio_reactor_trait::Tokio),
        )
        .await?;

        let channel = connection.create_channel().await?;
        channel.basic_qos(100, BasicQosOptions::default()).await?;

        let declare = {
            let channel = channel.clone();
            move |queue: &str| {
                let channel = channel.clone();
                let queue = queue.to_string();
                async move {
                    channel
                        .queue_declare(
                            &queue,
                            QueueDeclareOptions {
                                durable: true,
                                ..QueueDeclareOptions::default()
                            },
                            FieldTable::default(),
                        )
                        .await
                }
            }
        };
        let (notify_result, mobile_result, emails_result) = tokio::join!(
            declare(notify_queue),
            declare("missedmessage_mobile_notifications"),
            declare("missedmessage_emails"),
        );
        notify_result?;
        mobile_result?;
        emails_result?;

        let notify_consumer = channel
            .basic_consume(
                notify_queue,
                "consumer",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await?;

        tracing::debug!("listening on queue {notify_queue}");

        Ok(RabbitMQ {
            channel,
            consumer: notify_consumer,
        })
    }

    async fn handle_delivery(&mut self, state: &Arc<AppState>, delivery: Delivery) -> Result<()> {
        tracing::debug!(
            "RabbitMQ: delivery_tag={:?} exchange={:?} routing_key={:?} redelivered={:?} data={:?}",
            delivery.delivery_tag,
            delivery.exchange,
            delivery.routing_key,
            delivery.redelivered,
            String::from_utf8_lossy(&delivery.data),
        );
        process_notice(state, serde_json::from_slice(&delivery.data)?)?;
        self.channel
            .basic_ack(delivery.delivery_tag, BasicAckOptions::default())
            .await?;
        Ok(())
    }

    pub async fn run(mut self, state: Arc<AppState>) -> Result<()> {
        let mut shutdown_rx = state.shutdown_rx.clone();
        loop {
            tokio::select! {
                delivery = self.consumer.next() => {
                    let Some(delivery) = delivery else {
                        tracing::warn!("RabbitMQ connection closed");
                        break;
                    };
                    self.handle_delivery(&state, delivery?).await?;
                }
                _ = shutdown_rx.wait() => break,
            }
        }
        Ok(())
    }
}
