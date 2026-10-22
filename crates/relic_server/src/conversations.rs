//! # relic_server::conversations
//!
//! Gestión persistente de sesiones conversacionales y generación de respuestas del asistente.
//! Menos de 180 líneas.

use std::time::{SystemTime, UNIX_EPOCH};
use serde_json::json;
use crate::engine::execute_query;
use crate::state::ServerState;
use crate::types::{ChatMessage, Conversation};

fn current_timestamp() -> (u64, String) {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
    (now, format!("2026-10-21T{:02}:{:02}:{:02}Z", (now / 3600000) % 24, (now / 60000) % 60, (now / 1000) % 60))
}

pub fn list_conversations(state: &ServerState) -> Vec<Conversation> {
    let convs = state.conversations.read().unwrap();
    let mut list: Vec<Conversation> = convs.values().cloned().collect();
    list.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    list
}

pub fn get_conversation(id: &str, state: &ServerState) -> Option<Conversation> {
    let convs = state.conversations.read().unwrap();
    convs.get(id).cloned()
}

pub fn create_conversation(title: Option<String>, state: &ServerState) -> Conversation {
    let (now_ms, now_str) = current_timestamp();
    let id = format!("conv_{}", now_ms);
    let title_str = title.unwrap_or_else(|| format!("Sesión RELIC #{}", now_ms % 10000));

    let conv = Conversation {
        id: id.clone(),
        title: title_str,
        created_at: now_str.clone(),
        updated_at: now_str,
        messages: Vec::new(),
        archive_ids: Vec::new(),
    };

    let mut map = state.conversations.write().unwrap();
    map.insert(id, conv.clone());
    conv
}

pub fn add_message_and_respond(
    conv_id: &str,
    user_content: &str,
    state: &ServerState,
) -> Result<ChatMessage, String> {
    let (now_ms, now_str) = current_timestamp();
    let user_msg = ChatMessage {
        id: format!("msg_{}_u", now_ms),
        role: "user".to_string(),
        content: user_content.to_string(),
        created_at: now_str.clone(),
        metadata: None,
    };

    let query_res = execute_query(user_content, state);

    let assistant_msg = ChatMessage {
        id: format!("msg_{}_a", now_ms + 1),
        role: "assistant".to_string(),
        content: query_res.text,
        created_at: now_str.clone(),
        metadata: Some(json!({
            "route": query_res.route,
            "status": query_res.status,
            "latency_ms": query_res.latency_ms,
            "rss_mb": query_res.rss_mb,
        })),
    };

    let mut map = state.conversations.write().unwrap();
    if let Some(conv) = map.get_mut(conv_id) {
        conv.messages.push(user_msg);
        conv.messages.push(assistant_msg.clone());
        conv.updated_at = now_str;
        Ok(assistant_msg)
    } else {
        Err(format!("Conversación '{}' no encontrada", conv_id))
    }
}
