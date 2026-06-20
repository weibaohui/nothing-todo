import { useMemo } from 'react';
import dagre from 'dagre';
import type { LoopStepDto } from '@/types/loop';

interface FlowGraphProps {
  steps: LoopStepDto[];
  selectedStepId: number | null;
  tracedStepIds?: number[];
  tracedSequenceMap?: Record<number, number>;
  onSelectStep: (step: LoopStepDto) => void;
  onAddStep: () => void;
}

const NODE_WIDTH = 180;
const NODE_HEIGHT = 80;
const RANK_SEP = 60;
const NODE_SEP = 30;

type EdgeType = 'success-next' | 'success-goto' | 'fail-skip' | 'fail-goto' | 'fail-break' | 'end';

interface LayoutEdge {
  from: string;
  to: string;
  label: string;
  type: EdgeType;
  fromId: number;
  toId: number;
}

interface LayoutNode {
  id: number;
  x: number;
  y: number;
  step: LoopStepDto;
}

const EDGE_STYLES: Record<EdgeType, { color: string; dash: string; labelColor: string }> = {
  'success-next': { color: '#94a3b8', dash: '', labelColor: '#94a3b8' },
  'success-goto': { color: '#22c55e', dash: '', labelColor: '#16a34a' },
  'fail-skip':    { color: '#f97316', dash: '5,3', labelColor: '#ea580c' },
  'fail-goto':    { color: '#ef4444', dash: '', labelColor: '#dc2626' },
  'fail-break':   { color: '#ef4444', dash: '', labelColor: '#dc2626' },
  'end':          { color: '#94a3b8', dash: '', labelColor: '#94a3b8' },
};

function classifyEdge(
  _step: LoopStepDto,
  _allSteps: LoopStepDto[],
  policy: string,
  _gotoId: number | null,
  isSuccess: boolean,
): EdgeType {
  if (policy === 'end') return 'end';
  if (isSuccess) {
    if (policy === 'goto') return 'success-goto';
    return 'success-next';
  }
  // failure edges
  switch (policy) {
    case 'skip': return 'fail-skip';
    case 'goto': return 'fail-goto';
    case 'break': return 'fail-break';
    default: return 'fail-break';
  }
}

function resolveTargetStep(
  step: LoopStepDto,
  allSteps: LoopStepDto[],
  policy: string,
  gotoId: number | null,
): number | undefined {
  if (policy === 'next' || policy === 'skip') {
    const idx = allSteps.findIndex(s => s.id === step.id);
    if (idx >= 0 && idx < allSteps.length - 1) {
      return allSteps[idx + 1].id;
    }
    return undefined;
  }
  if (policy === 'goto' && gotoId != null) {
    return gotoId;
  }
  return undefined;
}

function useFlowLayout(steps: LoopStepDto[]) {
  return useMemo(() => {
    if (steps.length === 0) return { nodes: [], edges: [] as LayoutEdge[], width: 0, height: 0 };

    const g = new dagre.graphlib.Graph();
    g.setGraph({ rankdir: 'LR', ranksep: RANK_SEP, nodesep: NODE_SEP, marginx: 20, marginy: 20 });
    g.setDefaultEdgeLabel(() => ({}));

    // Add nodes
    for (const step of steps) {
      g.setNode(String(step.id), { width: NODE_WIDTH, height: NODE_HEIGHT });
    }

    // Build edges
    const layoutEdges: LayoutEdge[] = [];
    for (const step of steps) {
      // Success edge
      const successType = classifyEdge(step, steps, step.on_success, step.success_goto_step_id, true);
      const successTarget = resolveTargetStep(step, steps, step.on_success, step.success_goto_step_id);
      if (successTarget != null) {
        g.setEdge(String(step.id), String(successTarget));
        layoutEdges.push({
          from: String(step.id), to: String(successTarget),
          label: step.on_success === 'goto' ? `✅→${steps.find(s => s.id === successTarget)?.name || successTarget}` : '',
          type: successType, fromId: step.id, toId: successTarget,
        });
      }

      // Failure edge (only if different from success edge)
      if (step.min_rating != null && step.on_rating_fail !== step.on_success) {
        const failType = classifyEdge(step, steps, step.on_rating_fail, step.fail_goto_step_id, false);
        const failTarget = resolveTargetStep(step, steps, step.on_rating_fail, step.fail_goto_step_id);
        if (failTarget != null) {
          g.setEdge(String(step.id), String(failTarget));
          layoutEdges.push({
            from: String(step.id), to: String(failTarget),
            label: step.on_rating_fail === 'goto' ? `❌→${steps.find(s => s.id === failTarget)?.name || failTarget}` : 
                    step.on_rating_fail === 'skip' ? '失败→继续' : '',
            type: failType, fromId: step.id, toId: failTarget,
          });
        }
      }
    }

    dagre.layout(g);

    const nodes: LayoutNode[] = steps.map(step => {
      const pos = g.node(String(step.id));
      return {
        id: step.id,
        x: pos.x - NODE_WIDTH / 2,
        y: pos.y - NODE_HEIGHT / 2,
        step,
      };
    });

    const graphWidth = g.graph().width || 0;
    const graphHeight = g.graph().height || 0;

    return { nodes, edges: layoutEdges, width: graphWidth + 40, height: graphHeight + 40 };
  }, [steps]);
}

export function LoopFlowGraph({ steps, selectedStepId, tracedStepIds = [], tracedSequenceMap: _tracedSequenceMap = {}, onSelectStep, onAddStep }: FlowGraphProps) {
  const { nodes, edges, width, height } = useFlowLayout(steps);

  // 判断节点是否在轨迹中
  const isTraced = (id: number) => tracedStepIds.length === 0 || tracedStepIds.includes(id);
  const traceIndex = (id: number) => tracedStepIds.indexOf(id);

  // 判断边是否在轨迹中（源和目标都在 tracedStepIds 中且连续）
  const isEdgeTraced = (fromId: number, toId: number) => {
    if (tracedStepIds.length === 0) return true;
    const fi = tracedStepIds.indexOf(fromId);
    const ti = tracedStepIds.indexOf(toId);
    return fi >= 0 && ti >= 0 && ti === fi + 1;
  };

  if (steps.length === 0) {
    return (
      <div
        onClick={onAddStep}
        role="button"
        tabIndex={0}
        onKeyDown={(e) => { if (e.key === 'Enter') onAddStep(); }}
        style={{
          display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center',
          minHeight: 160, width: '100%',
          border: '2px dashed var(--color-border, #e2e8f0)',
          borderRadius: 12, cursor: 'pointer',
          color: 'var(--color-text-tertiary, #94a3b8)',
          fontSize: 13, gap: 8,
          transition: 'border-color 200ms, color 200ms',
        }}
        onMouseEnter={(e) => { e.currentTarget.style.borderColor = '#0891b2'; e.currentTarget.style.color = '#0891b2'; }}
        onMouseLeave={(e) => { e.currentTarget.style.borderColor = '#e2e8f0'; e.currentTarget.style.color = '#94a3b8'; }}
      >
        <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
          <line x1="12" y1="5" x2="12" y2="19" /><line x1="5" y1="12" x2="19" y2="12" />
        </svg>
        <span>暂无执行环节，点击添加</span>
      </div>
    );
  }

  return (
    <div style={{ overflowX: 'auto', overflowY: 'hidden', padding: '12px 0', minHeight: 160 }}>
      <svg width={width} height={height} style={{ display: 'block' }}>
        {/* Edges */}
        {edges.map((edge, i) => {
          const style = EDGE_STYLES[edge.type] || EDGE_STYLES['success-next'];
          const traced = isEdgeTraced(edge.fromId, edge.toId);
          const hasTrace = tracedStepIds.length > 0;
          const opacity = hasTrace ? (traced ? 1 : 0.15) : 1;
          return (
            <g key={`edge-${i}`} opacity={opacity}>
              {/* Arrow line */}
              <defs>
                <marker
                  id={`arrow-${i}`}
                  viewBox="0 0 10 10" refX="10" refY="5"
                  markerWidth="6" markerHeight="6" orient="auto"
                >
                  <path d="M 0 0 L 10 5 L 0 10 z" fill={style.color} />
                </marker>
              </defs>
              <path
                d={buildEdgePath(edge, nodes)}
                fill="none"
                stroke={style.color}
                strokeWidth={hasTrace && traced ? 3 : 1.5}
                strokeDasharray={style.dash || undefined}
                markerEnd={`url(#arrow-${i})`}
              />
              {/* Label */}
              {edge.label && (
                <text
                  x={getEdgeMidX(edge, nodes)}
                  y={getEdgeMidY(edge, nodes) - 6}
                  textAnchor="middle"
                  fontSize={10}
                  fill={style.labelColor}
                  style={{ fontFamily: 'monospace' }}
                >
                  {edge.label}
                </text>
              )}
            </g>
          );
        })}

        {/* Nodes */}
        {nodes.map(node => {
          const traced = isTraced(node.id);
          const hasTrace = tracedStepIds.length > 0;
          const opacity = hasTrace ? (traced ? 1 : 0.3) : 1;
          const ti = traceIndex(node.id);
          const isSelected = selectedStepId === node.id;
          return (
          <g
            key={`node-${node.id}`}
            onClick={() => onSelectStep(node.step)}
            style={{ cursor: 'pointer' }}
            opacity={opacity}
          >
            <rect
              x={node.x} y={node.y}
              width={NODE_WIDTH} height={NODE_HEIGHT}
              rx={8} ry={8}
              fill={isSelected ? '#f0f9ff' : '#ffffff'}
              stroke={isSelected ? '#0891b2' : (hasTrace && traced ? '#22c55e' : '#e2e8f0')}
              strokeWidth={isSelected ? 2 : (hasTrace && traced ? 2 : 1)}
            />
            {/* Traced step index badge */}
            {hasTrace && traced && ti >= 0 && (
              <rect
                x={node.x + NODE_WIDTH - 28} y={node.y + NODE_HEIGHT - 18}
                width={22} height={14} rx={4}
                fill="#22c55e"
              />
            )}
            {hasTrace && traced && ti >= 0 && (
              <text
                x={node.x + NODE_WIDTH - 17} y={node.y + NODE_HEIGHT - 7}
                textAnchor="middle" fontSize={8} fontWeight={700}
                fill="#ffffff"
                style={{ fontFamily: 'monospace' }}
              >
                {ti + 1}
              </text>
            )}
            {/* Status dot */}
            <circle
              cx={node.x + NODE_WIDTH - 10} cy={node.y + 10} r={4}
              fill={node.step.enabled ? '#22c55e' : '#94a3b8'}
            />
            {/* Index badge */}
            <rect
              x={node.x - 12} y={node.y + NODE_HEIGHT / 2 - 10}
              width={20} height={20} rx={10}
              fill={isSelected ? '#0891b2' : '#f1f5f9'}
            />
            <text
              x={node.x - 2} y={node.y + NODE_HEIGHT / 2 + 4}
              textAnchor="middle" fontSize={11} fontWeight={700}
              fill={isSelected ? '#ffffff' : '#64748b'}
              style={{ fontFamily: 'monospace' }}
            >
              {String(nodes.indexOf(node) + 1).padStart(2, '0')}
            </text>
            {/* Name */}
            <text
              x={node.x + 12} y={node.y + 22}
              fontSize={13} fontWeight={600}
              fill={hasTrace && !traced ? '#cbd5e1' : '#0f172a'}
              style={{ fontFamily: 'system-ui' }}
            >
              {truncateText(node.step.name, 18)}
            </text>
            {/* Todo title */}
            <text
              x={node.x + 12} y={node.y + 40}
              fontSize={11}
              fill={hasTrace && !traced ? '#cbd5e1' : '#64748b'}
            >
              {truncateText(node.step.todo_title || `#${node.step.todo_id}`, 22)}
            </text>
            {/* Executor */}
            <text
              x={node.x + 12} y={node.y + 56}
              fontSize={10}
              fill={hasTrace && !traced ? '#e2e8f0' : '#94a3b8'}
            >
              {node.step.todo_executor || '未指派'}
            </text>
            {/* Gate indicator */}
            {node.step.min_rating != null && (
              <text
                x={node.x + NODE_WIDTH - 8} y={node.y + NODE_HEIGHT - 6}
                textAnchor="end" fontSize={9}
                fill={hasTrace && !traced ? '#e2e8f0' : '#f97316'}
                style={{ fontFamily: 'monospace' }}
              >
                闸门:{node.step.min_rating}
              </text>
            )}
          </g>
          );
        })}
      </svg>

      {/* Add button */}
      <div style={{ display: 'flex', justifyContent: 'center', marginTop: 8 }}>
        <div
          onClick={onAddStep}
          role="button"
          tabIndex={0}
          onKeyDown={(e) => { if (e.key === 'Enter') onAddStep(); }}
          style={{
            display: 'flex', alignItems: 'center', gap: 6, padding: '6px 16px',
            border: '1px dashed var(--color-border, #e2e8f0)',
            borderRadius: 8, cursor: 'pointer',
            color: 'var(--color-text-tertiary, #94a3b8)',
            fontSize: 12,
            transition: 'border-color 200ms, color 200ms',
          }}
          onMouseEnter={(e) => { e.currentTarget.style.borderColor = '#0891b2'; e.currentTarget.style.color = '#0891b2'; }}
          onMouseLeave={(e) => { e.currentTarget.style.borderColor = '#e2e8f0'; e.currentTarget.style.color = '#94a3b8'; }}
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <line x1="12" y1="5" x2="12" y2="19" /><line x1="5" y1="12" x2="19" y2="12" />
          </svg>
          添加环节
        </div>
      </div>
    </div>
  );
}

// ─── Helpers ───

function truncateText(text: string, maxLen: number): string {
  return text.length > maxLen ? text.slice(0, maxLen - 1) + '…' : text;
}

function buildEdgePath(edge: LayoutEdge, nodes: LayoutNode[]): string {
  const from = nodes.find(n => n.id === edge.fromId);
  const to = nodes.find(n => n.id === edge.toId);
  if (!from || !to) return '';

  const x1 = from.x + NODE_WIDTH;
  const y1 = from.y + NODE_HEIGHT / 2;
  const x2 = to.x;
  const y2 = to.y + NODE_HEIGHT / 2;

  const dx = Math.abs(x2 - x1);
  const cx = x1 + dx * 0.4;

  return `M ${x1} ${y1} C ${cx} ${y1}, ${x2 - dx * 0.4} ${y2}, ${x2} ${y2}`;
}

function getEdgeMidX(edge: LayoutEdge, nodes: LayoutNode[]): number {
  const from = nodes.find(n => n.id === edge.fromId);
  const to = nodes.find(n => n.id === edge.toId);
  if (!from || !to) return 0;
  return (from.x + NODE_WIDTH + to.x) / 2;
}

function getEdgeMidY(edge: LayoutEdge, nodes: LayoutNode[]): number {
  const from = nodes.find(n => n.id === edge.fromId);
  const to = nodes.find(n => n.id === edge.toId);
  if (!from || !to) return 0;
  return (from.y + NODE_HEIGHT / 2 + to.y + NODE_HEIGHT / 2) / 2;
}
