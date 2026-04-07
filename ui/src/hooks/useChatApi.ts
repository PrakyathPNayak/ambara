import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useSettingsStore } from '../store/settingsStore';

export type ChatRole = 'user' | 'assistant';

export interface ChatMessage {
    id: string;
    role: ChatRole;
    content: string;
    timestamp: string;
    graph?: Record<string, unknown> | null;
}

interface ChatResponse {
    reply: string;
    session_id: string;
    graph_generated: boolean;
    graph: Record<string, unknown> | null;
}

interface UseChatApiResult {
    sendMessage: (text: string) => Promise<void>;
    messages: ChatMessage[];
    isTyping: boolean;
    error: string | null;
    sessionId: string;
    connectionStatus: 'connected' | 'disconnected' | 'reconnecting';
    setMessages: React.Dispatch<React.SetStateAction<ChatMessage[]>>;
    clearError: () => void;
}

const SESSION_KEY = 'ambara.chat.sessionId';
const HISTORY_KEY = 'ambara.chat.history';

function makeId(prefix: string): string {
    return `${prefix}_${Math.random().toString(36).slice(2, 10)}`;
}

function getSessionId(): string {
    const existing = localStorage.getItem(SESSION_KEY);
    if (existing) return existing;
    const generated = makeId('sess');
    localStorage.setItem(SESSION_KEY, generated);
    return generated;
}

export function useChatApi(): UseChatApiResult {
    const apiUrl = useSettingsStore((state) => state.settings.apiUrl) || 'http://localhost:8765';
    const wsUrl = apiUrl.replace(/^http/, 'ws');
    const [messages, setMessages] = useState<ChatMessage[]>(() => {
        const raw = localStorage.getItem(HISTORY_KEY);
        if (!raw) return [];
        try {
            return JSON.parse(raw) as ChatMessage[];
        } catch {
            return [];
        }
    });
    const [isTyping, setIsTyping] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [connectionStatus, setConnectionStatus] = useState<'connected' | 'disconnected' | 'reconnecting'>('disconnected');
    const wsRef = useRef<WebSocket | null>(null);
    const sessionId = useMemo(() => getSessionId(), []);

    useEffect(() => {
        localStorage.setItem(HISTORY_KEY, JSON.stringify(messages));
    }, [messages]);

    const retryCountRef = useRef(0);
    const retryTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
    const mountedRef = useRef(true);

    useEffect(() => {
        mountedRef.current = true;
        const MAX_RETRIES = 10;
        const BASE_DELAY_MS = 1000;
        const MAX_DELAY_MS = 30_000;

        function connect() {
            if (!mountedRef.current) return;

            const ws = new WebSocket(`${wsUrl}/ws/${sessionId}`);
            wsRef.current = ws;

            ws.onopen = () => {
                retryCountRef.current = 0;
                setConnectionStatus('connected');
            };

            ws.onerror = () => {
                // onclose will fire after onerror; reconnection handled there
            };

            ws.onclose = () => {
                if (!mountedRef.current) return;
                wsRef.current = null;

                if (retryCountRef.current < MAX_RETRIES) {
                    setConnectionStatus('reconnecting');
                    const delay = Math.min(BASE_DELAY_MS * 2 ** retryCountRef.current, MAX_DELAY_MS);
                    retryCountRef.current += 1;
                    retryTimerRef.current = setTimeout(connect, delay);
                } else {
                    setConnectionStatus('disconnected');
                }
            };

            ws.onmessage = (event) => {
                const payload = JSON.parse(event.data) as { type: string; content?: string; graph?: Record<string, unknown> | null; graph_generated?: boolean };
                if (payload.type === 'token') {
                    setMessages((prev) => {
                        const last = prev[prev.length - 1];
                        if (last?.role === 'assistant' && last.id.startsWith('stream_')) {
                            const updated = [...prev];
                            updated[updated.length - 1] = { ...last, content: `${last.content}${payload.content ?? ''}` };
                            return updated;
                        }
                        return [
                            ...prev,
                            {
                                id: `stream_${makeId('msg')}`,
                                role: 'assistant',
                                content: payload.content ?? '',
                                timestamp: new Date().toISOString(),
                                graph: null,
                            },
                        ];
                    });
                }
                if (payload.type === 'done') {
                    setIsTyping(false);
                    if (payload.graph_generated && payload.graph) {
                        setMessages((prev) => {
                            const updated = [...prev];
                            const idx = updated.length - 1;
                            if (idx >= 0) {
                                updated[idx] = { ...updated[idx], graph: payload.graph };
                            }
                            return updated;
                        });
                    }
                }
            };
        }

        connect();

        return () => {
            mountedRef.current = false;
            if (retryTimerRef.current) clearTimeout(retryTimerRef.current);
            wsRef.current?.close();
        };
    }, [sessionId, wsUrl]);

    const sendMessage = useCallback(async (text: string) => {
        if (!text.trim()) return;

        setError(null);
        const userMsg: ChatMessage = {
            id: makeId('user'),
            role: 'user',
            content: text,
            timestamp: new Date().toISOString(),
        };
        setMessages((prev) => [...prev, userMsg]);
        setIsTyping(true);

        if (wsRef.current && wsRef.current.readyState === WebSocket.OPEN) {
            wsRef.current.send(text);
            return;
        }

        try {
            const response = await fetch(`${apiUrl}/chat`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ message: text, session_id: sessionId, context: [] }),
            });

            if (!response.ok) {
                const errorText = await response.text().catch(() => 'Unknown server error');
                throw new Error(`Server error (${response.status}): ${errorText}`);
            }

            const payload = (await response.json()) as ChatResponse;
            const assistant: ChatMessage = {
                id: makeId('assistant'),
                role: 'assistant',
                content: payload.reply,
                timestamp: new Date().toISOString(),
                graph: payload.graph_generated ? payload.graph : null,
            };
            setMessages((prev) => [...prev, assistant]);
        } catch (err) {
            const message = err instanceof Error ? err.message : 'Failed to send message';
            setError(message);
            setMessages((prev) => [
                ...prev,
                {
                    id: makeId('error'),
                    role: 'assistant',
                    content: `⚠️ ${message}. Please try again.`,
                    timestamp: new Date().toISOString(),
                    graph: null,
                },
            ]);
        } finally {
            setIsTyping(false);
        }
    }, [sessionId, apiUrl]);

    const clearError = useCallback(() => setError(null), []);

    return {
        sendMessage,
        messages,
        isTyping,
        error,
        sessionId,
        connectionStatus,
        setMessages,
        clearError,
    };
}
