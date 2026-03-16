import { FormEvent, useMemo, useState } from 'react';
import { useChatApi } from '../../hooks/useChatApi';
import { GraphPreviewCard } from './GraphPreviewCard';
import './ChatPanel.css';

interface ChatPanelProps {
    onInsertGraph: (graph: Record<string, unknown>) => void;
}

export function ChatPanel({ onInsertGraph }: ChatPanelProps) {
    const { sendMessage, messages, isTyping, connectionStatus } = useChatApi();
    const [draft, setDraft] = useState('');

    const statusLabel = useMemo(() => {
        if (connectionStatus === 'connected') return 'Connected';
        if (connectionStatus === 'reconnecting') return 'Reconnecting';
        return 'Disconnected';
    }, [connectionStatus]);

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

            <div className="chat-messages">
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
                    <div className="chat-bubble chat-assistant">
                        <div className="typing-indicator">
                            <span />
                            <span />
                            <span />
                        </div>
                    </div>
                )}
            </div>

            <form className="chat-input-row" onSubmit={onSubmit}>
                <textarea
                    value={draft}
                    onChange={(event) => setDraft(event.target.value)}
                    placeholder="Describe the processing pipeline you want..."
                    rows={2}
                    onKeyDown={(event) => {
                        if (event.key === 'Enter' && !event.shiftKey) {
                            event.preventDefault();
                            void onSubmit(event);
                        }
                    }}
                />
                <button type="submit" disabled={!draft.trim()}>
                    Send
                </button>
            </form>
        </div>
    );
}
