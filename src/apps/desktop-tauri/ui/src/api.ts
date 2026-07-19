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
