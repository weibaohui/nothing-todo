// Loop 流程图虚拟节点 (Start / End) 与触发条件徽章。
//
// Start/End 是 dagre 布局时插入的虚拟节点，不渲染真实的环节卡片，但需要
// 视觉提示给用户「这里是一切的起点/终点」。Start 节点旁还会并排显示一组
// 触发条件徽章 (来自该 loop 已启用的 triggers)，让用户一眼看出「哪些方式
// 能让这个 loop 跑起来」。
//
// 之所以独立成文件：让主文件 LoopFlowGraph 保持在 500 行硬限内，
// 虚拟节点视觉与触发条件展示是独立的关注点。

import type { LoopTriggerDto } from '@/types/loop';

export const VIRTUAL_NODE_RADIUS = 20;

export const TRIGGER_SHORT_LABELS: Record<string, string> = {
  manual: '手动触发',
  cron: '定时调度',
  webhook: 'Webhook',
  feishu_message: '飞书消息',
  feishu_command: '飞书指令',
  todo_completed: 'Todo 完成',
  todo_state_changed: 'Todo 状态变更',
};

interface VirtualNodeProps {
  x: number;
  y: number;
  selected?: boolean;
}

// 入口节点：绿色实心圆 + ▶ 符号，下方标 "开始"。
// 选中态改为更深的青色，呼应真实环节的选中样式。
export function StartNode({ x, y, selected = false }: VirtualNodeProps) {
  return (
    <g>
      <circle
        cx={x} cy={y} r={VIRTUAL_NODE_RADIUS}
        fill={selected ? '#0891b2' : '#22c55e'}
        stroke={selected ? '#0e7490' : '#16a34a'}
        strokeWidth={2}
      />
      <text
        x={x} y={y + 5} textAnchor="middle"
        fontSize={15} fontWeight={700} fill="#ffffff"
        style={{ fontFamily: 'system-ui' }}
      >
        ▶
      </text>
      <text
        x={x} y={y + VIRTUAL_NODE_RADIUS + 13} textAnchor="middle"
        fontSize={10} fontWeight={600}
        fill={selected ? '#0e7490' : '#16a34a'}
        style={{ fontFamily: 'system-ui' }}
      >
        开始
      </text>
    </g>
  );
}

// 出口节点：深灰色实心圆 + ■ 符号，下方标 "结束"。
// 故意用低饱和灰色，避免抢走入口节点的视觉重点。
export function EndNode({ x, y }: VirtualNodeProps) {
  return (
    <g>
      <circle
        cx={x} cy={y} r={VIRTUAL_NODE_RADIUS}
        fill="#475569" stroke="#334155" strokeWidth={2}
      />
      <text
        x={x} y={y + 5} textAnchor="middle"
        fontSize={15} fontWeight={700} fill="#ffffff"
        style={{ fontFamily: 'system-ui' }}
      >
        ■
      </text>
      <text
        x={x} y={y + VIRTUAL_NODE_RADIUS + 13} textAnchor="middle"
        fontSize={10} fontWeight={600} fill="#475569"
        style={{ fontFamily: 'system-ui' }}
      >
        结束
      </text>
    </g>
  );
}

interface TriggerBadgesProps {
  triggers: LoopTriggerDto[];
  startX: number;
  startY: number;
  badgeWidth?: number;
  badgeHeight?: number;
  gap?: number;
  maxVisible?: number;
}

// 触发条件徽章：堆叠在 Start 节点左侧，垂直居中对齐 startY。
//
// 全部用纯文本标签（不依赖 emoji 字体），保证 SVG 在不同环境下渲染一致。
// 当启用触发器超过 maxVisible 时折叠成「+N 更多」，避免流程图整体高度被撑高。
export function TriggerBadges({
  triggers, startX, startY,
  badgeWidth = 110, badgeHeight = 22, gap = 4, maxVisible = 4,
}: TriggerBadgesProps) {
  // 只展示当前已启用的触发器，未启用的视觉上等价于「不支持」——它们
  // 不会让 loop 真正运行起来，不应让用户产生「这个 loop 有这条触发路径」的错觉。
  const enabled = triggers.filter(t => t.enabled);
  if (enabled.length === 0) return null;

  const visible = enabled.slice(0, maxVisible);
  const hidden = enabled.length - visible.length;

  // 让徽章组的几何中心落在 startY 上，而不是从 startY 向下铺开——
  // 这样视觉上与 Start 节点是「一组」，不会显得失衡。
  const totalHeight = visible.length * badgeHeight + (visible.length - 1) * gap
    + (hidden > 0 ? badgeHeight + gap : 0);
  const firstY = startY - totalHeight / 2;

  // 徽章右边缘距离 Start 节点左边缘 6px，留一点呼吸空间。
  const groupRight = startX - VIRTUAL_NODE_RADIUS - 6;
  const groupX = groupRight - badgeWidth;

  return (
    <g>
      {visible.map((t, i) => {
        const y = firstY + i * (badgeHeight + gap);
        const label = TRIGGER_SHORT_LABELS[t.trigger_type] || t.trigger_type;
        return (
          <g key={t.id} transform={`translate(${groupX}, ${y})`}>
            <rect
              width={badgeWidth} height={badgeHeight} rx={11}
              fill="#f0f9ff" stroke="#0891b2" strokeWidth={1}
            />
            {/* 在徽章内展示 trigger_type 的简短文字；超过宽度时用 SVG title
                把完整类型暴露给 hover，与主面板的标签保持一致。 */}
            <text
              x={10} y={badgeHeight / 2 + 4}
              fontSize={11} fill="#0f172a"
              style={{ fontFamily: 'system-ui' }}
            >
              {truncateLabel(label, badgeWidth - 20)}
            </text>
            <title>{TRIGGER_SHORT_LABELS[t.trigger_type] || t.trigger_type}</title>
          </g>
        );
      })}
      {hidden > 0 && (
        <g
          transform={`translate(${groupX}, ${firstY + visible.length * (badgeHeight + gap)})`}
        >
          <rect
            width={badgeWidth} height={badgeHeight} rx={11}
            fill="#f1f5f9" stroke="#94a3b8" strokeWidth={1} strokeDasharray="3,2"
          />
          <text
            x={badgeWidth / 2} y={badgeHeight / 2 + 4}
            textAnchor="middle" fontSize={11} fill="#64748b"
            style={{ fontFamily: 'system-ui' }}
          >
            +{hidden} 个未展开
          </text>
        </g>
      )}
    </g>
  );
}

// 按可视宽度粗略截断触发器标签，避免文字溢出徽章。
// 这里用「字符数 / 中文字宽」近似代替精确测量，比 SVG 文本测量 API 更轻。
function truncateLabel(text: string, maxPx: number): string {
  // 中文字符按 11px 算，ASCII 按 6px 算，留 4px 余量。
  let used = 0;
  let result = '';
  for (const ch of text) {
    const w = ch.charCodeAt(0) > 0x7F ? 11 : 6;
    if (used + w > maxPx - 4) {
      return result + '…';
    }
    used += w;
    result += ch;
  }
  return result;
}
