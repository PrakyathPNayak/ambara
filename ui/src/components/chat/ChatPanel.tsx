import { FormEvent, useEffect, useMemo, useRef, useState } from 'react';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import rehypeHighlight from 'rehype-highlight';
import { open } from '@tauri-apps/plugin-dialog';
import { useChatApi } from '../../hooks/useChatApi';
import { GraphPreviewCard } from './GraphPreviewCard';
import './ChatPanel.css';

const IMAGE_FILTERS = [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'bmp', 'tiff', 'tif', 'webp'] }];

interface ChatPanelProps {
    onInsertGraph: (graph: Record<string, unknown>) => void;
}

export function ChatPanel({ onInsertGraph }: ChatPanelProps) {
    const { sendMessage, messages, isTyping, connectionStatus, error, clearError } = useChatApi();
    const [draft, setDraft] = useState('');
    const [attachedImages, setAttachedImages] = useState<string[]>([]);
    const messagesEndRef = useRef<HTMLDivElement>(null);

    const statusLabel = useMemo(() => {
        if (connectionStatus === 'connected') return '● Connected';
        if (connectionStatus === 'reconnecting') return '↻ Reconnecting…';
        return '○ Disconnected';
    }, [connectionStatus]);

    useEffect(() => {
        const node = messagesEndRef.current;
        if (node && typeof node.scrollIntoView === 'function') {
            node.scrollIntoView({ behavior: 'smooth' });
        }
    }, [messages, isTyping]);

    const onSubmit = async (event: FormEvent) => {
        event.preventDefault();
        const message = draft;
        const images = [...attachedImages];
        setDraft('');
        setAttachedImages([]);
        await sendMessage(message, images);
    };

    const onAttachImage = async () => {
        try {
            const selected = await open({
                multiple: true,
                filters: IMAGE_FILTERS,
            });
            if (!selected) return;
            const paths = Array.isArray(selected) ? selected : [selected];
            setAttachedImages(prev => [...prev, ...paths]);
        } catch {
            // User cancelled or dialog unavailable
        }
    };

    const removeAttachment = (index: number) => {
        setAttachedImages(prev => prev.filter((_, i) => i !== index));
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
                        <div className="chat-content">
                            <ReactMarkdown remarkPlugins={[remarkGfm]} rehypePlugins={[rehypeHighlight]}>
                                {message.content}
                            </ReactMarkdown>
                        </div>
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
                {attachedImages.length > 0 && (
                    <div className="chat-attachments">
                        {attachedImages.map((path, i) => (
                            <span key={i} className="chat-attachment-chip">
                                📷 {path.split('/').pop()}
                                <button type="button" onClick={() => removeAttachment(i)} aria-label="Remove image">×</button>
                            </span>
                        ))}
                    </div>
                )}
                <div className="chat-input-controls">
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
                    <button type="button" className="attach-image-button" onClick={onAttachImage} aria-label="Attach image" title="Attach image">
                        📎
                    </button>
                    <button type="submit" disabled={!draft.trim()} aria-label="Send message">
                        Send
                    </button>
                </div>
            </form>
        </div>
    );
}
