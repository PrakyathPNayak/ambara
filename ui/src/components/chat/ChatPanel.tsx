import { FormEvent, useEffect, useMemo, useRef, useState } from 'react';
import { useChatApi } from '../../hooks/useChatApi';
import { GraphPreviewCard } from './GraphPreviewCard';
import './ChatPanel.css';

interface ChatPanelProps {
    onInsertGraph: (graph: Record<string, unknown>) => void;
}

export function ChatPanel({ onInsertGraph }: ChatPanelProps) {
    const { sendMessage, messages, isTyping, connectionStatus, error, clearError } = useChatApi();
    const [draft, setDraft] = useState('');
    const messagesEndRef = useRef<HTMLDivElement>(null);

    const statusLabel = useMemo(() => {
        if (connectionStatus === 'connected') return '● Connected';
        if (connectionStatus === 'reconnecting') return '↻ Reconnecting…';
        return '○ Disconnected';
    }, [connectionStatus]);

    useEffect(() => {
        messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
    }, [messages, isTyping]);

    const onSubmit = async (event: FormEvent) => {
        event.preventDefault();
        const message = draft;
        setDraft('');
        await sendMessage(message);
    };

    return (
        <div className="chat-panel">
            <div className="chat-header">
                <h3>Chat</h3>
                <span className={`chat-status chat-status-${connectionStatus}`}>{statusLabel}</span>
            </div>

            <div className="chat-messages" role="log" aria-live="polite" aria-label="Chat messages">
                {messages.length === 0 && !isTyping && (
                    <div className="chat-empty-state">
                        <p><strong>Welcome to Ambara Chat</strong></p>
                        <p>Describe an image processing pipeline and I'll build it for you. Try:</p>
                        <ul>
                            <li>"Load an image, apply blur, and save"</li>
                            <li>"What filters are available for color adjustment?"</li>
                            <li>"Build a pipeline to resize and sharpen images in batch"</li>
                        </ul>
                    </div>
                )}
                {messages.map((message) => (
                    <div key={message.id} className={`chat-bubble chat-${message.role}`}>
                        <div className="chat-content">{message.content}</div>
                        <div className="chat-meta">{new Date(message.timestamp).toLocaleTimeString()}</div>
                        {message.graph && (
                            <div className="chat-graph-wrapper">
                                <GraphPreviewCard graph={message.graph} onLoadIntoCanvas={onInsertGraph} />
                                <button className="insert-graph-button" onClick={() => onInsertGraph(message.graph as Record<string, unknown>)}>
                                    Insert Graph
                                </button>
                            </div>
                        )}
                    </div>
                ))}
                {isTyping && (
                    <div className="chat-bubble chat-assistant" aria-label="Assistant is typing">
                        <div className="typing-indicator" role="status">
                            <span />
                            <span />
                            <span />
                            <span className="sr-only">Assistant is typing…</span>
                        </div>
                    </div>
                )}
                <div ref={messagesEndRef} />
            </div>

            {error && (
                <div className="chat-error-bar" role="alert">
                    <span>Connection error — messages may not be delivered.</span>
                    <button onClick={clearError} type="button">Dismiss</button>
                </div>
            )}

            <form className="chat-input-row" onSubmit={onSubmit}>
                <textarea
                    value={draft}
                    onChange={(event) => setDraft(event.target.value)}
                    placeholder="Describe the processing pipeline you want..."
                    rows={2}
                    aria-label="Message input"
                    onKeyDown={(event) => {
                        if (event.key === 'Enter' && !event.shiftKey) {
                            event.preventDefault();
                            void onSubmit(event);
                        }
                    }}
                />
                <button type="submit" disabled={!draft.trim()} aria-label="Send message">
                    Send
                </button>
            </form>
        </div>
    );
}
