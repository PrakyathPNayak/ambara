import { useCallback, useEffect, useMemo, useRef, useState } from 'react';

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
    sessionId: string;
    connectionStatus: 'connected' | 'disconnected' | 'reconnecting';
    setMessages: React.Dispatch<React.SetStateAction<ChatMessage[]>>;
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
    const [connectionStatus, setConnectionStatus] = useState<'connected' | 'disconnected' | 'reconnecting'>('disconnected');
    const wsRef = useRef<WebSocket | null>(null);
    const sessionId = useMemo(() => getSessionId(), []);

    useEffect(() => {
        localStorage.setItem(HISTORY_KEY, JSON.stringify(messages));
    }, [messages]);

    useEffect(() => {
        const ws = new WebSocket(`ws://localhost:8765/ws/${sessionId}`);
        wsRef.current = ws;

        ws.onopen = () => {
            setConnectionStatus('connected');
        };
        ws.onerror = () => {
            setConnectionStatus('reconnecting');
        };
        ws.onclose = () => {
            setConnectionStatus('disconnected');
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

        return () => {
            ws.close();
        };
    }, [sessionId]);

    const sendMessage = useCallback(async (text: string) => {
        if (!text.trim()) return;

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

        const response = await fetch('http://localhost:8765/chat', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ message: text, session_id: sessionId, context: [] }),
        });

        const payload = (await response.json()) as ChatResponse;
        const assistant: ChatMessage = {
            id: makeId('assistant'),
            role: 'assistant',
            content: payload.reply,
            timestamp: new Date().toISOString(),
            graph: payload.graph_generated ? payload.graph : null,
        };
        setMessages((prev) => [...prev, assistant]);
        setIsTyping(false);
    }, [sessionId]);

    return {
        sendMessage,
        messages,
        isTyping,
        sessionId,
        connectionStatus,
        setMessages,
    };
}
