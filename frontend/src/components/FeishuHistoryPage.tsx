import { useState, useEffect } from 'react';
import {
  Table,
  Button,
  Space,
  message,
  Modal,
  Form,
  Input,
  Select,
  Tag,
  Typography,
} from 'antd';
import {
  ReloadOutlined,
  PlusOutlined,
  HistoryOutlined,
} from '@ant-design/icons';
import type { ColumnsType } from 'antd/es/table';
import * as db from '../utils/database';
import type { FeishuHistoryMessage, FeishuHistoryChat } from '../types';

const { Text } = Typography;

interface FeishuHistoryPageProps {
  onBack?: () => void;
}

export function FeishuHistoryPage(_props: FeishuHistoryPageProps) {
  const [isMobile, setIsMobile] = useState(false);
  const [messages, setMessages] = useState<FeishuHistoryMessage[]>([]);
  const [chats, setChats] = useState<FeishuHistoryChat[]>([]);
  const [loading, setLoading] = useState(false);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(20);
  const [selectedChatId, setSelectedChatId] = useState<string | undefined>(undefined);
  const [addModalOpen, setAddModalOpen] = useState(false);
  const [bots, setBots] = useState<db.AgentBot[]>([]);
  const [form] = Form.useForm();

  useEffect(() => {
    const checkMobile = () => setIsMobile(window.innerWidth < 640);
    checkMobile();
    window.addEventListener('resize', checkMobile);
    return () => window.removeEventListener('resize', checkMobile);
  }, []);

  const loadMessages = async () => {
    setLoading(true);
    try {
      const data = await db.getFeishuHistoryMessages({
        chat_id: selectedChatId,
        page,
        page_size: pageSize,
      });
      setMessages(data.messages);
      setTotal(data.total);
    } catch (e) {
      message.error('加载历史消息失败');
    } finally {
      setLoading(false);
    }
  };

  const loadChats = async () => {
    try {
      const data = await db.getFeishuHistoryChats();
      setChats(data);
    } catch (e) {
      console.error('加载群聊配置失败', e);
    }
  };

  const loadBots = async () => {
    try {
      const data = await db.getAgentBots();
      setBots(data.filter((b) => b.bot_type === 'feishu'));
    } catch (e) {
      console.error('加载机器人列表失败', e);
    }
  };

  useEffect(() => {
    loadChats();
    loadBots();
  }, []);

  useEffect(() => {
    loadMessages();
  }, [page, pageSize, selectedChatId]);

  const handleAddChat = async () => {
    try {
      const values = await form.validateFields();
      await db.createFeishuHistoryChat(values);
      message.success('添加成功');
      setAddModalOpen(false);
      form.resetFields();
      loadChats();
    } catch (e) {
      if (e instanceof Error) {
        message.error(e.message);
      }
    }
  };

  const columns: ColumnsType<FeishuHistoryMessage> = [
    {
      title: '时间',
      dataIndex: 'created_at',
      key: 'created_at',
      width: 150,
      render: (text: string) => {
        if (!text) return '-';
        const d = new Date(text);
        return isNaN(d.getTime()) ? text : d.toLocaleString('zh-CN');
      },
    },
    {
      title: '发送者',
      key: 'sender',
      width: 120,
      render: (_, record) => {
        const isBot = record.sender_type === 'app';
        return (
          <Space>
            <Tag color={isBot ? 'blue' : 'green'}>
              {isBot ? '智能体' : '用户'}
            </Tag>
            <Text type="secondary" style={{ fontSize: 12 }}>
              {record.sender_nickname || record.sender_open_id?.slice(0, 8) || '-'}
            </Text>
          </Space>
        );
      },
    },
    {
      title: '内容',
      dataIndex: 'content',
      key: 'content',
      ellipsis: true,
      render: (content: string, record) => {
        if (record.msg_type === 'text') {
          try {
            const parsed = JSON.parse(content);
            return parsed.text || content;
          } catch {
            return content;
          }
        }
        return <Tag>{record.msg_type}</Tag>;
      },
    },
  ];

  return (
    <div style={{ padding: isMobile ? 8 : 16 }}>
      <div
        style={{
          marginBottom: 16,
          display: 'flex',
          flexWrap: 'wrap',
          gap: 8,
          justifyContent: 'space-between',
          alignItems: 'center',
        }}
      >
        <Space>
          <HistoryOutlined />
          <Text strong style={{ fontSize: isMobile ? 14 : 16 }}>飞书历史消息</Text>
        </Space>
        <Space wrap>
          <Select
            placeholder="筛选群聊"
            allowClear
            style={{ width: isMobile ? 120 : 200 }}
            value={selectedChatId}
            onChange={setSelectedChatId}
            onClear={() => setSelectedChatId(undefined)}
          >
            {chats.map((chat) => (
              <Select.Option key={chat.chat_id} value={chat.chat_id}>
                {chat.chat_name || chat.chat_id}
              </Select.Option>
            ))}
          </Select>
          <Button icon={<ReloadOutlined />} onClick={loadMessages} size={isMobile ? 'small' : 'middle'}>
            刷新
          </Button>
          <Button type="primary" icon={<PlusOutlined />} onClick={() => setAddModalOpen(true)} size={isMobile ? 'small' : 'middle'}>
            添加
          </Button>
        </Space>
      </div>

      <Table
        columns={columns}
        dataSource={messages}
        rowKey="id"
        loading={loading}
        scroll={{ x: 'max-content' }}
        pagination={{
          current: page,
          pageSize,
          total,
          showSizeChanger: !isMobile,
          showQuickJumper: !isMobile,
          showTotal: (t) => `共 ${t} 条`,
          onChange: (p, ps) => {
            setPage(p);
            setPageSize(ps);
          },
        }}
        size={isMobile ? 'small' : 'middle'}
      />

      <Modal
        title="添加监听群聊"
        open={addModalOpen}
        onOk={handleAddChat}
        onCancel={() => {
          setAddModalOpen(false);
          form.resetFields();
        }}
        width={isMobile ? '90%' : 520}
      >
        <Form form={form} layout="vertical">
          <Form.Item
            name="bot_id"
            label="机器人"
            rules={[{ required: true, message: '请选择机器人' }]}
          >
            <Select placeholder="请选择机器人">
              {bots.map((bot) => (
                <Select.Option key={bot.id} value={bot.id}>
                  {bot.bot_name}
                </Select.Option>
              ))}
            </Select>
          </Form.Item>
          <Form.Item
            name="chat_id"
            label="群聊 ID"
            rules={[{ required: true, message: '请输入群聊 ID' }]}
          >
            <Input placeholder="请输入飞书群聊 ID" />
          </Form.Item>
          <Form.Item name="chat_name" label="群聊名称（可选）">
            <Input placeholder="请输入群聊名称，方便识别" />
          </Form.Item>
        </Form>
      </Modal>
    </div>
  );
}
