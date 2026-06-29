import { Card, Button, Typography, Upload, Select, Space, Modal, Tag, Alert, message } from 'antd';
import { DownloadOutlined, InboxOutlined } from '@ant-design/icons';
import { useState, useEffect } from 'react';
import * as db from '@/utils/database';
import { exportLoop, listLoops } from '@/utils/database/loops';

const { Dragger } = Upload;

export function LoopBackupTab() {
  const [selectedLoopId, setSelectedLoopId] = useState<number | null>(null);
  const [exporting, setExporting] = useState(false);
  const [importModalOpen, setImportModalOpen] = useState(false);
  const [yamlPreview, setYamlPreview] = useState<string | null>(null);
  const [previewData, setPreviewData] = useState<any>(null);
  const [importing, setImporting] = useState(false);
  const [selectedWorkspaceId, setSelectedWorkspaceId] = useState<number | null>(null);
  const [workspaces, setWorkspaces] = useState<any[]>([]);
  const [loops, setLoops] = useState<any[]>([]);

  // 加载环路列表
  useEffect(() => {
    listLoops().then(setLoops).catch(() => {});
  }, []);

  // 加载工作空间列表
  const loadWorkspaces = async () => {
    try {
      const ws = await db.getProjectDirectories();
      setWorkspaces(ws);
      if (ws.length > 0 && !selectedWorkspaceId) {
        setSelectedWorkspaceId(ws[0].id);
      }
    } catch (e) {
      console.error('Failed to load workspaces', e);
    }
  };

  // 导出单个环路
  const handleExportLoop = async (loopId: number) => {
    setExporting(true);
    try {
      const yaml = await exportLoop(loopId);
      const blob = new Blob([yaml], { type: 'application/x-yaml' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      const loop = loops.find(l => l.id === loopId);
      const timestamp = new Date().toISOString().replace(/[:.]/g, '-').slice(0, 19);
      a.download = `${loop?.name || 'loop'}-${timestamp}.loop.yaml`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
      message.success('环路导出成功');
    } catch (err: any) {
      message.error(err?.message || '导出失败');
    } finally {
      setExporting(false);
    }
  };

  // 导入文件解析
  const handleImportFile = async (file: File) => {
    try {
      const text = await file.text();
      const preview = await db.previewLoopImport(text);
      if (!preview.valid) {
        message.error('文件格式校验失败');
        return false;
      }
      setYamlPreview(text);
      setPreviewData(preview);
      await loadWorkspaces();
      setImportModalOpen(true);
    } catch (err: any) {
      message.error('解析文件失败: ' + (err?.message || String(err)));
    }
    return false; // 阻止默认上传
  };

  // 执行导入
  const handleConfirmImport = async () => {
    if (!yamlPreview || !selectedWorkspaceId) {
      message.warning('请选择目标工作空间');
      return;
    }
    setImporting(true);
    try {
      const result = await db.importLoops(yamlPreview, selectedWorkspaceId);
      message.success(`导入成功：创建了 ${result.created.loops} 个环路`);
      setImportModalOpen(false);
      setYamlPreview(null);
      setPreviewData(null);
      // 刷新页面
      window.location.reload();
    } catch (err: any) {
      message.error(err?.message || '导入失败');
    } finally {
      setImporting(false);
    }
  };

  const loopOptions = loops.map(l => ({ label: l.name, value: l.id }));

  return (
    <div style={{ maxWidth: 600 }}>
      <Card title="导出环路" size="small" style={{ marginBottom: 24 }}>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
          <Typography.Paragraph type="secondary">
            将环路导出为 .loop.yaml 文件，方便迁移和分享
          </Typography.Paragraph>
          <Select
            placeholder="选择一个环路"
            options={loopOptions}
            value={selectedLoopId}
            onChange={setSelectedLoopId}
            style={{ width: '100%' }}
            allowClear
          />
          <Button
            type="primary"
            icon={<DownloadOutlined />}
            onClick={() => selectedLoopId && handleExportLoop(selectedLoopId)}
            loading={exporting}
            disabled={!selectedLoopId}
            style={{ width: '100%' }}
          >
            导出选中环路
          </Button>
        </div>
      </Card>

      <Card title="导入环路" size="small" style={{ marginBottom: 24 }}>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
          <Typography.Paragraph type="secondary">
            从 .loop.yaml 文件导入环路，支持预览和选择性导入
          </Typography.Paragraph>
          <Dragger
            accept=".yaml,.yml,.loop.yaml"
            beforeUpload={handleImportFile}
            showUploadList={false}
            style={{ borderRadius: 12 }}
          >
            <p className="ant-upload-drag-icon">
              <InboxOutlined style={{ color: '#0891b2' }} />
            </p>
            <p className="ant-upload-text">点击或拖拽 .loop.yaml 文件到此处</p>
            <p className="ant-upload-hint">将解析文件并展示预览，确认后导入到目标工作空间</p>
          </Dragger>
        </div>
      </Card>

      <Modal
        title="导入环路预览"
        open={importModalOpen}
        onCancel={() => setImportModalOpen(false)}
        footer={[
          <Button key="cancel" onClick={() => setImportModalOpen(false)}>取消</Button>,
          <Button
            key="import"
            type="primary"
            loading={importing}
            disabled={!selectedWorkspaceId}
            onClick={handleConfirmImport}
          >
            确认导入
          </Button>,
        ]}
        width={600}
      >
        {previewData && (
          <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
            <Alert
              message="即将导入以下内容"
              description={
                <div>
                  <Space direction="vertical" size="small">
                    <Tag>环路: {previewData.summary.loops} 个</Tag>
                    <Tag>步骤: {previewData.summary.steps} 个</Tag>
                    <Tag>Todo模板: {previewData.summary.todos} 个</Tag>
                    <Tag>评审模板: {previewData.summary.review_templates} 个</Tag>
                    <Tag>标签: {previewData.summary.tags} 个</Tag>
                    <Tag>触发器: {previewData.summary.triggers} 个</Tag>
                  </Space>
                </div>
              }
              type="info"
              showIcon
            />

            {previewData.warnings && previewData.warnings.length > 0 && (
              <Alert
                message="警告"
                description={
                  <ul style={{ margin: 0, paddingLeft: 20 }}>
                    {previewData.warnings.map((w: any, i: number) => (
                      <li key={i}>{w.message}</li>
                    ))}
                  </ul>
                }
                type="warning"
                showIcon
              />
            )}

            <div>
              <Typography.Text strong>目标工作空间</Typography.Text>
              <Select
                placeholder="选择工作空间"
                options={workspaces.map(w => ({ label: w.name, value: w.id }))}
                value={selectedWorkspaceId}
                onChange={setSelectedWorkspaceId}
                style={{ width: '100%', marginTop: 8 }}
              />
            </div>
          </div>
        )}
      </Modal>
    </div>
  );
}
