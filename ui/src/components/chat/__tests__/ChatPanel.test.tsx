import { render, screen, fireEvent } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { ChatPanel } from '../ChatPanel';

vi.mock('../../../hooks/useChatApi', () => ({
    useChatApi: () => ({
        sendMessage: vi.fn(async () => { }),
        messages: [
            { id: '1', role: 'assistant', content: 'hello', timestamp: new Date().toISOString(), graph: null },
        ],
        isTyping: false,
        sessionId: 'sess-1',
        connectionStatus: 'connected',
        setMessages: vi.fn(),
    }),
}));

describe('ChatPanel', () => {
    it('renders without crashing', () => {
        render(<ChatPanel onInsertGraph={() => { }} />);
        expect(screen.getByText('Chat')).toBeTruthy();
    });

    it('sends message on submit', () => {
        render(<ChatPanel onInsertGraph={() => { }} />);
        fireEvent.change(screen.getByPlaceholderText(/Describe the processing pipeline/i), { target: { value: 'blur it' } });
        fireEvent.click(screen.getByText('Send'));
        expect(screen.getByText('hello')).toBeTruthy();
    });
});
