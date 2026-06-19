// 环节详情面板：右侧展示环节的完整信息。

import { useEffect, useState } from 'react';
import { Skeleton, Empty, Tag, Descriptions } from 'antd';
import { ApartmentOutlined, ThunderboltOutlined } from '@ant-design/icons';
import * as dbSteps from '@/utils/database/steps';
import type { StepSummary } from '@/types';
import { formatRelativeTime } from '@/utils/datetime';

interface StepDetailPanelProps {
  stepId: number;
}

export function StepDetailPanel({ stepId }: StepDetailPanelProps) {
  const [step, setStep] = useState<StepSummary | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    setLoading(true);
    dbSteps
      .getStep(stepId)
      .then(setStep)
      .catch(() => setStep(null))
      .finally(() => setLoading(false));
  }, [stepId]);

  if (loading) {
    return <Skeleton active style={{ padding: 24 }} />;
  }
  if (!step) {
    return <Empty description="无法加载该环节" style={{ marginTop: 64 }} />;
  }

  return (
    <div style={{ padding: '20px 24px' }}>
      {/* Header */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 12, marginBottom: 20 }}>
        <h2 style={{ margin: 0, fontSize: 18, flex: 1, minWidth: 0, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', color: 'var(--color-text, #0f172a)' }}>
          {step.title}
        </h2>
        <span style={{ color: 'var(--color-text-tertiary, #94a3b8)', fontSize: 12, fontFamily: 'monospace' }}>#{step.id}</span>
      </div>

      {/* 基本信息 */}
      <section style={{
        background: 'var(--color-bg-elevated, #ffffff)',
        border: '1px solid var(--color-border, #e2e8f0)',
        borderRadius: 8,
        padding: 16,
        marginBottom: 12,
      }}>
        <div style={{ fontSize: 14, fontWeight: 600, color: 'var(--color-text, #0f172a)', marginBottom: 12 }}>基本信息</div>
        <Descriptions column={2} size="small" bordered={false}>
          <Descriptions.Item label="执行器">
            {step.executor ? (
              <span><ThunderboltOutlined style={{ color: '#fa8c16', marginRight: 4 }} />{step.executor}</span>
            ) : (
              <span style={{ color: 'var(--color-text-tertiary, #94a3b8)' }}>未指派</span>
            )}
          </Descriptions.Item>
          <Descriptions.Item label="复用次数">
            <Tag icon={<ApartmentOutlined />} color={step.used_by_loop_stage_count > 0 ? 'purple' : 'default'}>
              {step.used_by_loop_stage_count}
            </Tag>
          </Descriptions.Item>
          <Descriptions.Item label="来源事项">
            {step.source_todo_id ? (
              <span>#<code>{step.source_todo_id}</code></span>
            ) : (
              <span style={{ color: 'var(--color-text-tertiary, #94a3b8)' }}>—</span>
            )}
          </Descriptions.Item>
          <Descriptions.Item label="更新于">
            {step.updated_at ? formatRelativeTime(step.updated_at) : '—'}
          </Descriptions.Item>
        </Descriptions>
      </section>

      {/* 验收标准 */}
      {step.acceptance_criteria && (
        <section style={{
          background: 'var(--color-bg-elevated, #ffffff)',
          border: '1px solid var(--color-border, #e2e8f0)',
          borderRadius: 8,
          padding: 16,
          marginBottom: 12,
        }}>
          <div style={{ fontSize: 14, fontWeight: 600, color: 'var(--color-text, #0f172a)', marginBottom: 8 }}>验收标准</div>
          <div style={{ fontSize: 13, color: 'var(--color-text-secondary, #475569)', whiteSpace: 'pre-wrap' }}>
            {step.acceptance_criteria}
          </div>
        </section>
      )}

      {/* Prompt */}
      <section style={{
        background: 'var(--color-bg-elevated, #ffffff)',
        border: '1px solid var(--color-border, #e2e8f0)',
        borderRadius: 8,
        padding: 16,
      }}>
        <div style={{ fontSize: 14, fontWeight: 600, color: 'var(--color-text, #0f172a)', marginBottom: 8 }}>提示词 (Prompt)</div>
        <div style={{
          fontSize: 13, color: 'var(--color-text-secondary, #475569)',
          background: 'var(--color-bg-secondary, #f8fafc)',
          padding: 12, borderRadius: 6, whiteSpace: 'pre-wrap',
          lineHeight: 1.6,
        }}>
          {step.prompt || <span style={{ color: 'var(--color-text-tertiary, #94a3b8)' }}>无提示词</span>}
        </div>
      </section>
    </div>
  );
}
