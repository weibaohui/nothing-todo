import { useState } from 'react';
import { Modal, Button, message, Space, Typography, Upload } from 'antd';
import {
  DownloadOutlined,
  UploadOutlined,
  WarningOutlined,
  InboxOutlined,
} from '@ant-design/icons';
import { importBackup } from '../utils/database';

const { Text, Paragraph } = Typography;
const { Dragger } = Upload;

interface BackupModalProps {
  open: boolean;
  onClose: () => void;
}

export function BackupModal({ open, onClose }: BackupModalProps) {
  const [exporting, setExporting] = useState(false);
  const [importing, setImporting] = useState(false);
  const [importConfirm, setImportConfirm] = useState(false);

  const handleExport = async () => {
    setExporting(true);
    try {
      const response = await fetch('/xyz/backup/export', {
        headers: { 'Accept': 'application/x-yaml' },
      });
      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }
      const yamlText = await response.text();
      const blob = new Blob([yamlText], { type: 'application/x-yaml' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      const timestamp = new Date().toISOString().replace(/[:.]/g, '-').slice(0, 19);
      a.download = `aietodo-backup-${timestamp}.yaml`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
      message.success('备份导出成功');
      onClose();
    } catch (err: any) {
      message.error(err?.message || '导出失败');
    } finally {
      setExporting(false);
    }
  };

  const handleFileSelect = async (file: File) => {
    const text = await file.text();
    setImporting(true);
    try {
      const msg = await importBackup(text);
      message.success(msg);
      setImportConfirm(false);
      onClose();
      // 触发页面刷新
      window.location.reload();
    } catch (err: any) {
      message.error(err?.message || '导入失败');
    } finally {
      setImporting(false);
    }
    return false; // 阻止默认上传
  };

  return (
    <Modal
      title={
        <Space>
          <DownloadOutlined />
          <span>备份与恢复</span>
        </Space>
      }
      open={open}
      onCancel={onClose}
      footer={null}
      width={480}
      destroyOnClose
    >
      <div style={{ display: 'flex', flexDirection: 'column', gap: 24 }}>

        {/* 导出区域 */}
        <div
          style={{
            padding: 20,
            borderRadius: 12,
            border: '1px solid var(--color-border, #e2e8f0)',
            background: 'var(--color-bg-container, #fff)',
          }}
        >
          <Space direction="vertical" style={{ width: '100%' }} size={12}>
            <div>
              <Text strong style={{ fontSize: 15 }}>导出备份</Text>
              <Paragraph type="secondary" style={{ margin: '4px 0 0 0', fontSize: 13 }}>
                将所有 Todo 和标签导出为 YAML 文件，方便迁移和存档
              </Paragraph>
            </div>
            <Button
              type="primary"
              icon={<DownloadOutlined />}
              onClick={handleExport}
              loading={exporting}
              block
              size="large"
            >
              导出为 YAML 文件
            </Button>
          </Space>
        </div>

        {/* 导入区域 */}
        <div
          style={{
            padding: 20,
            borderRadius: 12,
            border: '1px solid var(--color-border, #e2e8f0)',
            background: 'var(--color-bg-container, #fff)',
          }}
        >
          <Space direction="vertical" style={{ width: '100%' }} size={12}>
            <div>
              <Text strong style={{ fontSize: 15 }}>导入备份</Text>
              <Paragraph type="secondary" style={{ margin: '4px 0 0 0', fontSize: 13 }}>
                从 YAML 文件恢复数据
                <Text type="danger" style={{ display: 'block', fontSize: 12, marginTop: 4 }}>
                  <WarningOutlined /> 导入将清空当前所有数据，操作不可逆！
                </Text>
              </Paragraph>
            </div>

            {!importConfirm ? (
              <Button
                danger
                icon={<UploadOutlined />}
                onClick={() => setImportConfirm(true)}
                block
                size="large"
              >
                选择备份文件导入
              </Button>
            ) : (
              <div>
                <Dragger
                  accept=".yaml,.yml"
                  beforeUpload={(file) => {
                    handleFileSelect(file);
                    return false;
                  }}
                  showUploadList={false}
                  disabled={importing}
                  style={{
                    borderColor: '#ff4d4f',
                    borderRadius: 12,
                  }}
                >
                  <p className="ant-upload-drag-icon">
                    <InboxOutlined style={{ color: '#ff4d4f' }} />
                  </p>
                  <p className="ant-upload-text">点击或拖拽 YAML 文件到此处</p>
                  <p className="ant-upload-hint" style={{ color: '#ff4d4f' }}>
                    导入将清空当前所有数据
                  </p>
                </Dragger>
                <div style={{ marginTop: 8, textAlign: 'right' }}>
                  <Button
                    size="small"
                    onClick={() => setImportConfirm(false)}
                    disabled={importing}
                  >
                    取消
                  </Button>
                </div>
              </div>
            )}
          </Space>
        </div>

      </div>
    </Modal>
  );
}