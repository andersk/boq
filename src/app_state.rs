use std::sync::Mutex;

use crate::queues::Queues;
use crate::shutdown;

pub struct AppState {
    pub shared_secret: String,
    pub secret_key: String,
    pub shutdown_rx: shutdown::Receiver,
    pub db_pool: deadpool_postgres::Pool,
    pub queues: Mutex<Queues>,
    pub rabbitmq_channel: lapin::Channel,
}
