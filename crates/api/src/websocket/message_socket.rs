use actix::ActorContext;
use actix::{Actor, Addr, AsyncContext, Handler, Message as ActixMessage, StreamHandler};
use actix_web::{web, Error as ActixError, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use application::ports::{MessageEvent, RealtimePublisher};
use async_trait::async_trait;
use infrastructure::realtime::InMemoryRealtimePublisher;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use crate::AppState;

/// A serializable event delivered to WebSocket clients.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsEvent {
    MessageNew {
        message_id: Uuid,
        conversation_id: Uuid,
        sender_user_id: Uuid,
        sender_party_id: Option<Uuid>,
        recipient_type: String,
        recipient_user_id: Option<Uuid>,
        recipient_party_id: Option<Uuid>,
        recipient_deal_id: Option<Uuid>,
        recipient_room_id: Option<Uuid>,
        message_type: String,
        subject: Option<String>,
        content: String,
        reply_to_message_id: Option<Uuid>,
        #[serde(with = "time::serde::iso8601")]
        created_at: time::OffsetDateTime,
    },
    MessageUpdated {
        message_id: Uuid,
        conversation_id: Uuid,
        content: String,
        #[serde(with = "time::serde::iso8601")]
        edited_at: time::OffsetDateTime,
    },
    MessageDeleted {
        message_id: Uuid,
        conversation_id: Uuid,
    },
    MessageRead {
        message_id: Uuid,
        user_id: Uuid,
        party_id: Option<Uuid>,
        #[serde(with = "time::serde::iso8601")]
        read_at: time::OffsetDateTime,
    },
    MessageReaction {
        message_id: Uuid,
        user_id: Uuid,
        party_id: Option<Uuid>,
        reaction_type: String,
        total_likes: i64,
        total_dislikes: i64,
    },
    RoomDeleted {
        room_id: Uuid,
    },
}

impl From<MessageEvent> for WsEvent {
    fn from(event: MessageEvent) -> Self {
        match event {
            MessageEvent::MessageNew {
                message_id,
                conversation_id,
                sender_user_id,
                sender_party_id,
                recipient_type,
                recipient_user_id,
                recipient_party_id,
                recipient_deal_id,
                recipient_room_id,
                message_type,
                subject,
                content,
                reply_to_message_id,
                created_at,
            } => WsEvent::MessageNew {
                message_id,
                conversation_id,
                sender_user_id,
                sender_party_id,
                recipient_type: recipient_type.as_str().to_string(),
                recipient_user_id,
                recipient_party_id,
                recipient_deal_id,
                recipient_room_id,
                message_type: message_type.as_str().to_string(),
                subject,
                content,
                reply_to_message_id,
                created_at,
            },
            MessageEvent::MessageUpdated {
                message_id,
                conversation_id,
                content,
                edited_at,
            } => WsEvent::MessageUpdated {
                message_id,
                conversation_id,
                content,
                edited_at,
            },
            MessageEvent::MessageDeleted {
                message_id,
                conversation_id,
            } => WsEvent::MessageDeleted {
                message_id,
                conversation_id,
            },
            MessageEvent::MessageRead {
                message_id,
                user_id,
                party_id,
                read_at,
            } => WsEvent::MessageRead {
                message_id,
                user_id,
                party_id,
                read_at,
            },
            MessageEvent::MessageReaction {
                message_id,
                user_id,
                party_id,
                reaction_type,
                total_likes,
                total_dislikes,
            } => WsEvent::MessageReaction {
                message_id,
                user_id,
                party_id,
                reaction_type: reaction_type.as_str().to_string(),
                total_likes,
                total_dislikes,
            },
            MessageEvent::RoomDeleted { room_id } => WsEvent::RoomDeleted { room_id },
        }
    }
}

/// Internal message sent to a connected socket to push an event.
#[derive(ActixMessage, Clone)]
#[rtype(result = "()")]
pub struct PushEvent(pub WsEvent);

/// Registry of currently connected WebSocket sessions keyed by user id.
#[derive(Clone, Default)]
pub struct SessionRegistry {
    sessions: Arc<Mutex<HashMap<Uuid, Addr<MessageSocket>>>>,
}

impl SessionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&self, user_id: Uuid, addr: Addr<MessageSocket>) {
        self.sessions.lock().unwrap().insert(user_id, addr);
    }

    pub fn unregister(&self, user_id: Uuid) {
        self.sessions.lock().unwrap().remove(&user_id);
    }

    /// Broadcast an event to every connected session (MVP simplification).
    pub fn broadcast(&self, event: WsEvent) {
        let sessions = self.sessions.lock().unwrap();
        for addr in sessions.values() {
            addr.do_send(PushEvent(event.clone()));
        }
    }
}

/// A connected WebSocket client.
pub struct MessageSocket {
    user_id: Uuid,
    registry: SessionRegistry,
}

impl MessageSocket {
    pub fn new(user_id: Uuid, registry: SessionRegistry) -> Self {
        Self { user_id, registry }
    }
}

impl Actor for MessageSocket {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.registry.register(self.user_id, ctx.address());
    }

    fn stopping(&mut self, _: &mut Self::Context) -> actix::Running {
        self.registry.unregister(self.user_id);
        actix::Running::Stop
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for MessageSocket {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(bytes)) => ctx.pong(&bytes),
            Ok(ws::Message::Close(_)) => {
                self.registry.unregister(self.user_id);
                ctx.stop();
            }
            _ => {}
        }
    }
}

impl Handler<PushEvent> for MessageSocket {
    type Result = ();

    fn handle(&mut self, msg: PushEvent, ctx: &mut Self::Context) {
        if let Ok(text) = serde_json::to_string(&msg.0) {
            ctx.text(text);
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct WsAuthQuery {
    pub token: Option<String>,
}

/// WebSocket entrypoint. Validates a token from the query string or the
/// `Authorization` header and starts the actor.
pub async fn ws_handler(
    req: HttpRequest,
    stream: web::Payload,
    query: web::Query<WsAuthQuery>,
    state: web::Data<AppState>,
) -> Result<HttpResponse, ActixError> {
    let token = query.token.clone().or_else(|| {
        req.headers()
            .get(actix_web::http::header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "))
            .map(|s| s.to_string())
    });

    let token = match token {
        Some(t) => t,
        None => return Ok(HttpResponse::Unauthorized().finish()),
    };

    let ctx = match state.token_validator.verify(&token).await {
        Ok(c) => c,
        Err(_) => return Ok(HttpResponse::Unauthorized().finish()),
    };

    let socket = MessageSocket::new(ctx.user_id, state.websocket_registry.clone());
    ws::start(socket, &req, stream)
}

/// Real-time publisher that records events and forwards them to connected
/// WebSocket sessions.
#[derive(Clone)]
pub struct WebSocketPublisher {
    inner: Arc<InMemoryRealtimePublisher>,
    registry: SessionRegistry,
}

impl WebSocketPublisher {
    pub fn new(inner: Arc<InMemoryRealtimePublisher>, registry: SessionRegistry) -> Self {
        Self { inner, registry }
    }
}

#[async_trait]
impl RealtimePublisher for WebSocketPublisher {
    async fn publish(
        &self,
        event: MessageEvent,
    ) -> Result<(), application::errors::ApplicationError> {
        self.inner.publish(event.clone()).await?;
        self.registry.broadcast(event.into());
        Ok(())
    }
}
