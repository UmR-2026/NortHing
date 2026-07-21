import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export interface SessionMetaDto {
  id: string;
  name: string;
  updated_at: number;
}

export interface MessageDto {
  id: string;
  role: string;
  content: string;
  is_streaming: boolean;
}

export interface ChatChunk {
  session_id: string;
  text: string;
}

export interface ChatTurnState {
  session_id: string;
  turn_id?: string;
  state: "started" | "completed" | "failed" | "cancelled";
  error?: string;
  duration_ms?: number;
}

export interface ChatToolEvent {
  session_id: string;
  turn_id: string;
  call_id: string;
  phase: "started" | "completed";
  name: string;
  summary: string;
  detail?: string;
}

export async function createSession(): Promise<string> {
  return invoke<string>("create_session");
}

export async function listSessions(): Promise<SessionMetaDto[]> {
  return invoke<SessionMetaDto[]>("list_sessions");
}

export async function sendMessage(
  sessionId: string,
  text: string,
): Promise<void> {
  return invoke<void>("send_message", { sessionId, text });
}

export async function getMessages(
  sessionId: string,
): Promise<MessageDto[]> {
  return invoke<MessageDto[]>("get_messages", { sessionId });
}

export async function getOrCreateLatestSession(): Promise<string> {
  return invoke<string>("get_or_create_latest_session");
}

export async function stopStreaming(
  sessionId: string,
  turnId: string,
): Promise<void> {
  return invoke<void>("stop_streaming", { sessionId, turnId });
}

export async function getUiPrefs(): Promise<{ agent_name: string }> {
  return invoke<{ agent_name: string }>("get_ui_prefs");
}

export async function setUiPrefs(agentName: string): Promise<void> {
  return invoke<void>("set_ui_prefs", { agentName });
}

export function onChunk(
  handler: (payload: ChatChunk) => void,
): Promise<() => void> {
  return listen<ChatChunk>("chat-chunk", (event) => handler(event.payload));
}

export function onTurnState(
  handler: (payload: ChatTurnState) => void,
): Promise<() => void> {
  return listen<ChatTurnState>("chat-turn-state", (event) =>
    handler(event.payload),
  );
}

export function onToolEvent(
  handler: (payload: ChatToolEvent) => void,
): Promise<() => void> {
  return listen<ChatToolEvent>("chat-tool", (event) => handler(event.payload));
}
