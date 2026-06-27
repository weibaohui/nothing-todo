import { Pagination } from 'antd';
import { ForumPostCard } from './ForumPostCard';
import type { SessionGroup } from './helpers';

/**
 * 论坛帖子列表 —— 只显示主帖（每个 session 的第一条记录）。
 * 追问数量以 badge 展示，点击主帖进入详情页查看完整帖子流。
 */
export function ForumPostList({
  sessionGroups,
  selectedRecordId,
  onSelectRecord,
  historyTotal,
  historyLimit,
  historyPage,
  onPageChange,
}: {
  sessionGroups: SessionGroup[];
  selectedRecordId: number | null;
  onSelectRecord: (id: number) => void;
  historyTotal: number;
  historyLimit: number;
  historyPage: number;
  onPageChange: (page: number, pageSize: number) => void;
}) {
  return (
    <div style={{ flex: 1, overflow: 'auto', minHeight: 0 }}>
      {sessionGroups.map(group => {
        const mainRecord = group.records[0];
        const replyCount = group.records.length > 1 ? group.records.length - 1 : 0;

        return (
          <div key={group.sessionId}>
            <ForumPostCard
              record={mainRecord}
              isSelected={selectedRecordId === mainRecord.id}
              onSelect={() => onSelectRecord(mainRecord.id)}
              replyCount={replyCount}
            />
          </div>
        );
      })}

      {/* 分页 */}
      {historyTotal > historyLimit && (
        <div style={{ display: 'flex', justifyContent: 'center', padding: '12px 0' }}>
          <Pagination
            simple
            current={historyPage}
            pageSize={historyLimit}
            total={historyTotal}
            onChange={onPageChange}
            size="small"
          />
        </div>
      )}
    </div>
  );
}
