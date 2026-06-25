import { useState, useEffect, useRef } from 'react';
import { Card, Button, Switch, Input, Select, Tag, message, Tabs, List, Space, Spin, Modal, Typography, AutoComplete } from 'antd';
import { LeftOutlined, QrcodeOutlined, CopyOutlined } from '@ant-design/icons';
import QRCode from 'qrcode';
import * as db from '@/utils/database';
import type { AgentBot, FeishuPushStatus, WhitelistEntry, FeishuSenderItem } from '@/utils/database';
import type { FeishuHistoryMessage, FeishuHistoryChat } from '@/types';
import { copyToClipboard } from '@/utils/clipboard';

const { Paragraph } = Typography;

interface BotDetailPageProps {
  bot: AgentBot;
  onBack: () => void;
  onRefresh: () => void;
}

/**
 * Bot 详情页：显示单个 bot 的配置、聊天绑定和消息记录
 * 包含三个 Tab：基本设置 / 聊天绑定 / 消息记录
 */
export function BotDetailPage({ bot, onBack, onRefresh }: BotDetailPageProps) {
  // 基本设置状态
  const [botConfig, setBotConfig] = useState<Record<string, boolean>>({ dm_enabled: true, group_enabled: true, group_require_mention: true, echo_reply: true });
  const [pushStatus, setPushStatus] = useState<FeishuPushStatus | null>(null);
  const [groupWhitelist, setGroupWhitelist] = useState<WhitelistEntry[]>([]);
  const [whitelistOpenId, setWhitelistOpenId] = useState('');
  const [whitelistName, setWhitelistName] = useState('');
  const [historySenders, setHistorySenders] = useState<FeishuSenderItem[]>([]);

  // 聊天绑定状态
  const [binding, setBinding] = useState(false);
  const [bindModalOpen, setBindModalOpen] = useState(false);
  const [qrCodeUrl, setQrCodeUrl] = useState('');
  const [pollError, setPollError] = useState('');
  const [bindSuccess, setBindSuccess] = useState(false);
  const [feishuEventSource, setFeishuEventSource] = useState<EventSource | null>(null);
  const successTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // 消息记录状态
  const [historyMessages, setHistoryMessages] = useState<FeishuHistoryMessage[]>([]);
  const [historyChats, setHistoryChats] = useState<FeishuHistoryChat[]>([]);
  const [historyLoading, setHistoryLoading] = useState(false);
  const [historyTotal, setHistoryTotal] = useState(0);
  const [historyPage, setHistoryPage] = useState(1);
  const [historyPageSize] = useState(20);
  const [historySelectedChatId, setHistorySelectedChatId] = useState<string | undefined>(undefined);
  const [historyIsHistory, setHistoryIsHistory] = useState<boolean | undefined>(undefined);
  const [historySelectedSenderId] = useState<string | undefined>(undefined);
  const [historyViewMsg, setHistoryViewMsg] = useState<string | null>(null);

  // 加载 bot 配置
  useEffect(() => {
    try {
      const parsed = JSON.parse(bot.config || '{}');
      setBotConfig({ dm_enabled: true, group_enabled: true, group_require_mention: true, echo_reply: true, ...parsed });
    } catch {}
  }, [bot]);

  // 加载推送状态
  useEffect(() => {
    db.getFeishuPush().then(status => {
      const botStatus = status.find(s => s.bot_id === bot.id);
      setPushStatus(botStatus || null);
    }).catch(() => {});
  }, [bot.id]);

  // 加载白名单
  useEffect(() => {
    if (pushStatus) {
      db.getGroupWhitelist(bot.id).then(setGroupWhitelist).catch(() => setGroupWhitelist([]));
    }
  }, [bot.id, pushStatus]);

  // 加载历史发送者
  useEffect(() => {
    db.getFeishuSenders().then(setHistorySenders).catch(() => {});
    db.getFeishuHistoryChats().then(setHistoryChats).catch(() => {});
  }, []);

  // 加载历史消息
  useEffect(() => {
    loadHistoryMessages();
  }, [historyPage, historyPageSize, historySelectedChatId, historyIsHistory, historySelectedSenderId]);

  const loadHistoryMessages = async () => {
    setHistoryLoading(true);
    try {
      const data = await db.getFeishuHistoryMessages({
        chat_id: historySelectedChatId,
        is_history: historyIsHistory,
        sender_open_id: historySelectedSenderId,
        page: historyPage,
        page_size: historyPageSize,
      });
      setHistoryMessages(data.messages);
      setHistoryTotal(data.total);
    } catch {
      message.error('加载历史消息失败');
    } finally {
      setHistoryLoading(false);
    }
  };

  // 处理配置变更
  const handleConfigChange = async (key: string, val: boolean) => {
    const newConfig = { ...botConfig, [key]: val };
    try {
      await db.updateAgentBotConfig(bot.id, JSON.stringify(newConfig));
      setBotConfig(newConfig);
      onRefresh();
    } catch (e: any) {
      message.error('保存配置失败: ' + (e.message || '未知错误'));
    }
  };

  // 处理推送级别变更
  const handlePushLevelChange = async (level: db.FeishuPushLevel) => {
    try {
      await db.updateFeishuPush({ botId: bot.id, pushLevel: level });
      onRefresh();
    } catch (e: any) {
      message.error('设置推送失败: ' + (e.message || '未知错误'));
    }
  };

  // 处理响应开关变更
  const handleResponseEnabledChange = async (targetType: 'p2p' | 'group', enabled: boolean) => {
    try {
      if (targetType === 'p2p') {
        await db.updateFeishuPush({ botId: bot.id, p2pResponseEnabled: enabled });
      } else {
        await db.updateFeishuPush({ botId: bot.id, groupResponseEnabled: enabled });
      }
      onRefresh();
    } catch (e: any) {
      message.error('更新响应开关失败: ' + (e.message || '未知错误'));
    }
  };

  // 添加白名单
  const handleAddWhitelist = async () => {
    if (!whitelistOpenId.trim()) return;
    try {
      await db.addGroupWhitelist(bot.id, whitelistOpenId.trim(), whitelistName.trim() || undefined);
      setWhitelistOpenId('');
      setWhitelistName('');
      db.getGroupWhitelist(bot.id).then(setGroupWhitelist).catch(() => {});
    } catch (e: any) {
      message.error('添加白名单失败: ' + (e.message || '未知错误'));
    }
  };

  // 删除白名单
  const handleDeleteWhitelist = async (id: number) => {
    try {
      await db.deleteGroupWhitelist(id);
      db.getGroupWhitelist(bot.id).then(setGroupWhitelist).catch(() => {});
    } catch (e: any) {
      message.error('删除白名单失败: ' + (e.message || '未知错误'));
    }
  };

  // 复制文本
  const doCopyText = async (text: string, label: string) => {
    const ok = await copyToClipboard(text);
    if (ok) message.success(`${label} 已复制`);
    else message.error('复制失败');
  };

  // 开始绑定
  const handleStartBind = async () => {
    if (successTimerRef.current) clearTimeout(successTimerRef.current);
    if (feishuEventSource) feishuEventSource.close();

    setBinding(true);
    setBindSuccess(false);
    setPollError('');
    setQrCodeUrl('');
    setBindModalOpen(true);

    try {
      const initRes = await db.feishuInit();
      if (!initRes.supported) {
        setPollError('当前环境不支持 client_secret 认证');
        setBinding(false);
        return;
      }

      const beginRes = await db.feishuBegin();
      const qrDataUrl = await QRCode.toDataURL(beginRes.qr_url, { width: 256, margin: 2 });
      setQrCodeUrl(qrDataUrl);

      const eventSource = db.feishuPollSSE(
        beginRes.device_code,
        beginRes.interval,
        beginRes.expire_in,
        (pollRes) => {
          if (pollRes.success) {
            setBindSuccess(true);
            message.success(`绑定成功！Bot: ${pollRes.bot_name || 'Feishu Bot'}`);
            onRefresh();
            successTimerRef.current = setTimeout(() => {
              setBindModalOpen(false);
              setQrCodeUrl('');
            }, 2000);
          } else {
            const errMsg = pollRes.error === 'access_denied' ? '用户拒绝了绑定请求'
              : pollRes.error === 'expired_token' ? '二维码已过期，请重新绑定'
              : '绑定超时，请重试';
            setPollError(errMsg);
          }
          setBinding(false);
        },
        (error) => {
          setPollError(error || 'SSE 连接失败');
          setBinding(false);
        }
      );
      setFeishuEventSource(eventSource);
    } catch (err: any) {
      setPollError(err?.message || '启动绑定失败');
      setBinding(false);
    }
  };

  // 关闭绑定弹窗
  useEffect(() => {
    return () => {
      feishuEventSource?.close();
      if (successTimerRef.current) clearTimeout(successTimerRef.current);
    };
  }, [feishuEventSource]);

  const isFeishu = bot.bot_type === 'feishu';

  // Tab 内容：基本设置
  const basicSettingsTab = (
    <div style={{ maxWidth: 700 }}>
      <Card title="基本配置" size="small" style={{ marginBottom: 16 }}>
        <div style={{ display: 'flex', flexWrap: 'wrap', gap: '8px 16px' }}>
          <span style={{ display: 'flex', alignItems: 'center', gap: 4, fontSize: 13 }}>
            <Switch size="small" checked={botConfig.dm_enabled !== false} onChange={v => handleConfigChange('dm_enabled', v)} />接收单聊消息
          </span>
          <span style={{ display: 'flex', alignItems: 'center', gap: 4, fontSize: 13 }}>
            <Switch size="small" checked={botConfig.group_enabled !== false} onChange={v => handleConfigChange('group_enabled', v)} />接收群聊消息
          </span>
          <span style={{ display: 'flex', alignItems: 'center', gap: 4, fontSize: 13 }}>
            <Switch size="small" checked={botConfig.group_require_mention !== false} onChange={v => handleConfigChange('group_require_mention', v)} />群聊仅处理@
          </span>
          <span style={{ display: 'flex', alignItems: 'center', gap: 4, fontSize: 13 }}>
            <Switch size="small" checked={botConfig.echo_reply !== false} onChange={v => handleConfigChange('echo_reply', v)} />Echo 回复
          </span>
        </div>
      </Card>

      {isFeishu && pushStatus && (
        <Card title="推送配置" size="small" style={{ marginBottom: 16 }}>
          <div style={{ marginBottom: 12 }}>
            <span style={{ fontSize: 13, marginRight: 8 }}>推送目标</span>
            <Select size="small" value={pushStatus.push_level} onChange={handlePushLevelChange} style={{ width: 90 }}
              options={[
                { value: 'disabled', label: '关闭' },
                { value: 'result_only', label: '仅结论' },
                { value: 'all', label: '全部' },
              ]}
            />
          </div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 4, marginBottom: 12 }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
              <span style={{ fontSize: 12, width: 60, color: 'var(--color-text-tertiary)' }}>单聊ID:</span>
              <Input size="small" value={pushStatus.p2p_receive_id} style={{ flex: 1, fontSize: 12 }} />
              <Button size="small" icon={<CopyOutlined />} onClick={() => doCopyText(pushStatus.p2p_receive_id, 'p2p_receive_id')} />
            </div>
            <div style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
              <span style={{ fontSize: 12, width: 60, color: 'var(--color-text-tertiary)' }}>群ID:</span>
              <Input size="small" value={pushStatus.group_chat_id || ''} style={{ flex: 1, fontSize: 12 }} />
              <Button size="small" icon={<CopyOutlined />} onClick={() => doCopyText(pushStatus.group_chat_id || '', 'group_chat_id')} />
            </div>
          </div>
          <div style={{ display: 'flex', gap: 16, fontSize: 13 }}>
            <span style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
              <Switch size="small" checked={pushStatus.p2p_response_enabled} onChange={v => handleResponseEnabledChange('p2p', v)} />单聊响应
            </span>
            <span style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
              <Switch size="small" checked={pushStatus.group_response_enabled} onChange={v => handleResponseEnabledChange('group', v)} />群聊响应
            </span>
          </div>
        </Card>
      )}

      {isFeishu && (
        <Card title="群聊响应白名单" size="small">
          <Paragraph type="secondary" style={{ fontSize: 13, marginBottom: 12 }}>
            白名单为空时不限制，仅白名单内的用户消息会触发响应
          </Paragraph>
          <div style={{ display: 'flex', gap: 8, marginBottom: 8 }}>
            <AutoComplete
              size="small" placeholder="搜索或粘贴 Open ID" style={{ flex: 1 }}
              value={whitelistOpenId}
              onChange={setWhitelistOpenId}
              options={historySenders.filter(s => s.sender_open_id).map(s => ({
                value: s.sender_open_id,
                label: `${s.sender_nickname || s.sender_open_id} (${s.count}条)`
              }))}
            />
            <Input size="small" placeholder="备注名" value={whitelistName} onChange={e => setWhitelistName(e.target.value)} style={{ width: 100 }} />
            <Button size="small" onClick={handleAddWhitelist}>添加</Button>
          </div>
          {groupWhitelist.map(w => (
            <div key={w.id} style={{ display: 'flex', alignItems: 'center', gap: 4, fontSize: 12, marginBottom: 4 }}>
              <span style={{ flex: 1 }}>{w.sender_name || w.sender_open_id}</span>
              <span style={{ color: 'var(--color-text-tertiary)', fontSize: 11 }}>{w.sender_open_id.slice(0, 12)}...</span>
              <Button size="small" danger type="link" style={{ fontSize: 11, padding: 0 }} onClick={() => handleDeleteWhitelist(w.id)}>删除</Button>
            </div>
          ))}
          {groupWhitelist.length === 0 && (
            <div style={{ fontSize: 12, color: 'var(--color-text-tertiary)' }}>暂无白名单，所有用户均可触发响应</div>
          )}
        </Card>
      )}
    </div>
  );

  // Tab 内容：聊天绑定
  const chatBindingsTab = (
    <div style={{ maxWidth: 700 }}>
      <Card title="绑定信息" size="small" style={{ marginBottom: 16 }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: 12 }}>
          <div style={{ width: 36, height: 36, borderRadius: 8, background: isFeishu ? '#1976D2' : '#888', display: 'flex', alignItems: 'center', justifyContent: 'center', color: '#fff', fontWeight: 700, fontSize: 14 }}>
            {isFeishu ? '飞' : '其他'}
          </div>
          <div>
            <div style={{ fontWeight: 600, fontSize: 14 }}>{bot.bot_name}</div>
            <div style={{ fontSize: 12, color: 'var(--color-text-secondary)' }}>App ID: {bot.app_id}</div>
          </div>
          <Tag color={bot.enabled ? 'green' : 'default'} style={{ marginLeft: 'auto' }}>
            {bot.enabled ? '已启用' : '已禁用'}
          </Tag>
        </div>
        <Button type="primary" icon={<QrcodeOutlined />} onClick={handleStartBind} loading={binding} size="small">
          绑定飞书
        </Button>
      </Card>

      <Modal
        title={<Space><QrcodeOutlined />绑定飞书智能体</Space>}
        open={bindModalOpen}
        onCancel={() => { setBindModalOpen(false); setQrCodeUrl(''); setPollError(''); setBindSuccess(false); }}
        footer={null} width={400} centered
        afterClose={() => { onRefresh(); }}
      >
        <div style={{ textAlign: 'center', padding: '16px 0' }}>
          {pollError && <div style={{ marginBottom: 16, color: '#ff4d4f', fontSize: 13 }}>{pollError}</div>}
          {bindSuccess ? (
            <div style={{ color: '#52c41a', fontSize: 48, marginBottom: 16 }}>✓</div>
          ) : qrCodeUrl ? (
            <div style={{ marginBottom: 16 }}>
              <img src={qrCodeUrl} alt="QR Code" style={{ width: '100%', maxWidth: 200, height: 'auto' }} />
              <div style={{ marginTop: 12, color: 'var(--color-text-secondary)', fontSize: 13 }}>请使用飞书 App 扫描二维码绑定</div>
              <div style={{ marginTop: 6, fontSize: 12, color: 'var(--color-text-tertiary)' }}>二维码有效期 10 分钟，请尽快完成</div>
            </div>
          ) : (
            <Spin size="large" />
          )}
          {binding && !qrCodeUrl && <div style={{ marginTop: 16, color: 'var(--color-text-secondary)', fontSize: 13 }}>正在生成二维码...</div>}
        </div>
      </Modal>
    </div>
  );

  // Tab 内容：消息记录
  const messageRecordsTab = (
    <div style={{ maxWidth: 900 }}>
      <Card title="历史消息" size="small">
        <div style={{ display: 'flex', gap: 12, marginBottom: 12, flexWrap: 'wrap' }}>
          <Select size="small" placeholder="筛选群聊" allowClear style={{ width: 150 }}
            value={historySelectedChatId}
            onChange={v => { setHistorySelectedChatId(v); setHistoryPage(1); }}
            options={historyChats.map(c => ({ value: c.chat_id, label: c.chat_name || c.chat_id }))}
          />
          <Select size="small" placeholder="消息类型" allowClear style={{ width: 120 }}
            value={historyIsHistory}
            onChange={v => { setHistoryIsHistory(v); setHistoryPage(1); }}
            options={[{ value: true, label: '历史消息' }, { value: false, label: '实时消息' }]}
          />
        </div>

        <List
          loading={historyLoading}
          dataSource={historyMessages}
          locale={{ emptyText: '暂无消息记录' }}
          renderItem={msg => (
            <List.Item key={msg.id} style={{ padding: '8px 0', borderBottom: '1px solid var(--color-border-light)' }}>
              <div style={{ flex: 1 }}>
                <div style={{ fontSize: 12, color: 'var(--color-text-secondary)', marginBottom: 4 }}>
                  <span style={{ fontWeight: 500 }}>{msg.sender_nickname || msg.sender_open_id}</span>
                  <span style={{ marginLeft: 8 }}>{msg.created_at ? new Date(msg.created_at).toLocaleString() : ''}</span>
                  {msg.is_history && <Tag style={{ marginLeft: 8 }}>历史</Tag>}
                </div>
                <div style={{ fontSize: 13 }}>{msg.content}</div>
              </div>
              <Button size="small" type="link" onClick={() => setHistoryViewMsg(msg.content || '')}>详情</Button>
            </List.Item>
          )}
        />

        <div style={{ marginTop: 12, display: 'flex', justifyContent: 'flex-end', gap: 8 }}>
          <Button size="small" disabled={historyPage <= 1} onClick={() => setHistoryPage(p => p - 1)}>上一页</Button>
          <span style={{ fontSize: 12, lineHeight: '24px' }}>第 {historyPage} 页，共 {historyTotal} 条</span>
          <Button size="small" disabled={historyPage * historyPageSize >= historyTotal} onClick={() => setHistoryPage(p => p + 1)}>下一页</Button>
        </div>
      </Card>

      <Modal open={!!historyViewMsg} onCancel={() => setHistoryViewMsg(null)} footer={null} width={560} title="消息详情">
        <div style={{ fontSize: 13, lineHeight: 1.8, whiteSpace: 'pre-wrap', wordBreak: 'break-all', maxHeight: 400, overflowY: 'auto' }}>
          {historyViewMsg}
        </div>
      </Modal>
    </div>
  );

  return (
    <div className="bot-detail-page">
      <div className="detail-header" style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 16 }}>
        <Button type="text" size="small" icon={<LeftOutlined />} onClick={onBack} className="back-btn" />
        <h3 className="card-title" style={{ margin: 0 }}>{bot.bot_name}</h3>
        <Tag color={bot.enabled ? 'green' : 'default'} style={{ marginLeft: 8 }}>
          {bot.enabled ? '已启用' : '已禁用'}
        </Tag>
      </div>

      <Tabs
        size="small"
        items={[
          { key: 'basic', label: '基本设置', children: basicSettingsTab },
          { key: 'binding', label: '聊天绑定', children: chatBindingsTab },
          { key: 'records', label: '消息记录', children: messageRecordsTab },
        ]}
      />
    </div>
  );
}
