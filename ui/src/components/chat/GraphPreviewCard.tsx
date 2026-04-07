import './ChatPanel.css';

interface GraphNode {
    id: string;
    filter_id: string;
    parameters?: Record<string, unknown>;
}

interface GraphConnection {
    from_node: string;
    to_node: string;
    from_port: string;
    to_port: string;
}

interface GraphPreviewCardProps {
    graph: Record<string, unknown>;
    onLoadIntoCanvas: (graph: Record<string, unknown>) => void;
}

function buildFlowSummary(nodes: GraphNode[], connections: GraphConnection[]): string {
    if (nodes.length === 0) return 'Empty graph';
    if (nodes.length === 1) return nodes[0].filter_id;

    // Build adjacency: node_id -> list of next node_ids (in connection order)
    const outgoing = new Map<string, string[]>();
    const incoming = new Set<string>();
    for (const conn of connections) {
        const list = outgoing.get(conn.from_node) ?? [];
        list.push(conn.to_node);
        outgoing.set(conn.from_node, list);
        incoming.add(conn.to_node);
    }

    // Find root nodes (no incoming connections)
    const roots = nodes.filter((n) => !incoming.has(n.id));
    if (roots.length === 0) return nodes.map((n) => n.filter_id).join(', ');

    // Walk the graph from first root to build a linear flow
    const visited = new Set<string>();
    const flow: string[] = [];
    const nodeMap = new Map(nodes.map((n) => [n.id, n.filter_id]));

    let current = roots[0].id;
    while (current && !visited.has(current)) {
        visited.add(current);
        flow.push(nodeMap.get(current) ?? current);
        const nexts = outgoing.get(current);
        current = nexts?.[0] ?? '';
    }

    // If there are unvisited nodes (branches), note them
    const unvisited = nodes.filter((n) => !visited.has(n.id));
    let suffix = '';
    if (unvisited.length > 0) {
        suffix = ` (+${unvisited.length} branch node${unvisited.length > 1 ? 's' : ''})`;
    }

    return flow.join(' → ') + suffix;
}

export function GraphPreviewCard({ graph, onLoadIntoCanvas }: GraphPreviewCardProps) {
    const nodes = (Array.isArray(graph.nodes) ? graph.nodes : []) as GraphNode[];
    const connections = (Array.isArray(graph.connections) ? graph.connections : []) as GraphConnection[];

    const flowSummary = buildFlowSummary(nodes, connections);

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
            <div className="graph-preview-flow">{flowSummary}</div>
            <div className="graph-preview-actions">
                <button onClick={() => onLoadIntoCanvas(graph)}>Load into Canvas</button>
                <button onClick={downloadJson}>Download JSON</button>
            </div>
        </div>
    );
}
