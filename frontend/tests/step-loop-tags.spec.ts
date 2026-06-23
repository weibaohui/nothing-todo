/**
 * 环节/环路标签功能测试
 *
 * 验证环节和环路使用标签（Tag）替代原有的 color 字段：
 * 1. 标签 CRUD
 * 2. 环节关联标签
 * 3. 环路关联标签
 */

import { test, expect } from '@playwright/test';

const BACKEND_URL = process.env.E2E_BACKEND_URL || 'http://localhost:18088';

// 生成唯一标签名，避免测试间冲突
const uniqueTagName = (prefix: string) => `${prefix}-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;

test.describe('环节/环路标签功能', () => {
  let tagId: number;
  let stepId: number;
  let loopId: number;

  test('标签 CRUD', async ({ page }) => {
    const tagName = uniqueTagName('crud');

    // 创建标签
    const createRes = await page.request.post(`${BACKEND_URL}/api/tags`, {
      data: { name: tagName, color: '#ff6600' },
    });
    expect(createRes.ok()).toBeTruthy();
    const created = await createRes.json();
    tagId = created.data.id;
    expect(tagId).toBeGreaterThan(0);

    // 列表包含
    const listRes = await page.request.get(`${BACKEND_URL}/api/tags`);
    expect(listRes.ok()).toBeTruthy();
    const tags = await listRes.json();
    const ids = tags.data.map((t: any) => t.id);
    expect(ids).toContain(tagId);

    // 删除
    const delRes = await page.request.delete(`${BACKEND_URL}/api/tags/${tagId}`);
    expect(delRes.ok()).toBeTruthy();
  });

  test('环节关联标签', async ({ page }) => {
    // 创建标签
    const tagRes = await page.request.post(`${BACKEND_URL}/api/tags`, {
      data: { name: uniqueTagName('环节标签'), color: '#1890ff' },
    });
    expect(tagRes.ok()).toBeTruthy();
    const tag = await tagRes.json();
    tagId = tag.data.id;

    // 创建环节（直建）
    const stepRes = await page.request.post(`${BACKEND_URL}/api/steps`, {
      data: { title: uniqueTagName('测试环节'), prompt: 'test prompt' },
    });
    expect(stepRes.ok()).toBeTruthy();
    const step = await stepRes.json();
    stepId = step.data.id;
    expect(step.data.tag_ids).toEqual([]); // 新建环节无标签

    // 更新环节标签
    const updateTagsRes = await page.request.put(`${BACKEND_URL}/api/steps/${stepId}/tags`, {
      data: { tag_ids: [tagId] },
    });
    expect(updateTagsRes.ok()).toBeTruthy();
    const updated = await updateTagsRes.json();
    expect(updated.data.tag_ids).toContain(tagId);

    // 清理
    await page.request.delete(`${BACKEND_URL}/api/tags/${tagId}`);
    await page.request.delete(`${BACKEND_URL}/api/steps/${stepId}`);
  });

  test('环路关联标签', async ({ page }) => {
    // 创建标签
    const tagRes = await page.request.post(`${BACKEND_URL}/api/tags`, {
      data: { name: uniqueTagName('环路标签'), color: '#52c41a' },
    });
    expect(tagRes.ok()).toBeTruthy();
    const tag = await tagRes.json();
    tagId = tag.data.id;

    // 创建环路
    const loopRes = await page.request.post(`${BACKEND_URL}/api/loops`, {
      data: { name: uniqueTagName('测试环路') },
    });
    expect(loopRes.ok()).toBeTruthy();
    const loop = await loopRes.json();
    loopId = loop.data.id;
    expect(loop.data.tag_ids).toEqual([]); // 新建环路无标签

    // 更新环路标签
    const updateTagsRes = await page.request.put(`${BACKEND_URL}/api/loops/${loopId}/tags`, {
      data: { tag_ids: [tagId] },
    });
    expect(updateTagsRes.ok()).toBeTruthy();
    const updated = await updateTagsRes.json();
    expect(updated.data.tag_ids).toContain(tagId);

    // 验证环路详情也包含标签
    const detailRes = await page.request.get(`${BACKEND_URL}/api/loops/${loopId}`);
    expect(detailRes.ok()).toBeTruthy();
    const detail = await detailRes.json();
    expect(detail.data.tag_ids).toContain(tagId);

    // 清理
    await page.request.delete(`${BACKEND_URL}/api/tags/${tagId}`);
    await page.request.delete(`${BACKEND_URL}/api/loops/${loopId}`);
  });

  test('环路列表包含标签', async ({ page }) => {
    // 创建标签
    const tagRes = await page.request.post(`${BACKEND_URL}/api/tags`, {
      data: { name: uniqueTagName('列表标签'), color: '#722ed1' },
    });
    expect(tagRes.ok()).toBeTruthy();
    const tag = await tagRes.json();
    tagId = tag.data.id;

    // 创建环路并设置标签
    const loopRes = await page.request.post(`${BACKEND_URL}/api/loops`, {
      data: { name: uniqueTagName('列表测试环路') },
    });
    expect(loopRes.ok()).toBeTruthy();
    const loop = await loopRes.json();
    loopId = loop.data.id;

    const updateTagsRes = await page.request.put(`${BACKEND_URL}/api/loops/${loopId}/tags`, {
      data: { tag_ids: [tagId] },
    });
    expect(updateTagsRes.ok()).toBeTruthy();

    // 列表接口验证
    const listRes = await page.request.get(`${BACKEND_URL}/api/loops`);
    expect(listRes.ok()).toBeTruthy();
    const list = await listRes.json();
    const target = list.data.find((l: any) => l.id === loopId);
    expect(target).toBeDefined();
    expect(target.tag_ids).toContain(tagId);

    // 清理
    await page.request.delete(`${BACKEND_URL}/api/tags/${tagId}`);
    await page.request.delete(`${BACKEND_URL}/api/loops/${loopId}`);
  });
});
