import './ChatPanel.css';

interface GraphPreviewCardProps {
    graph: Record<string, unknown>;
    onLoadIntoCanvas: (graph: Record<string, unknown>) => void;
}

export function GraphPreviewCard({ graph, onLoadIntoCanvas }: GraphPreviewCardProps) {
    const nodes = Array.isArray((graph as { nodes?: unknown[] }).nodes) ? ((graph as { nodes?: unknown[] }).nodes as unknown[]) : [];
    const connections = Array.isArray((graph as { connections?: unknown[] }).connections)
        ? ((graph as { connections?: unknown[] }).connections as unknown[])
        : [];

    const nodeNames = nodes
        .slice(0, 5)
        .map((node) => (node as { filter_id?: string }).filter_id || 'node')
        .join(', ');

    const downloadJson = () => {
        const blob = new Blob([JSON.stringify(graph, null, 2)], { type: 'application/json' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = 'ambara-generated-graph.json';
        a.click();
        URL.revokeObjectURL(url);
    };

    return (
        <div className="graph-preview-card">
            <div className="graph-preview-title">Generated Graph</div>
            <div className="graph-preview-stats">{nodes.length} nodes, {connections.length} connections</div>
            <div className="graph-preview-nodes">{nodeNames}</div>
            <div className="graph-preview-actions">
                <button onClick={() => onLoadIntoCanvas(graph)}>Load into Canvas</button>
                <button onClick={downloadJson}>Download JSON</button>
            </div>
        </div>
    );
}
