const axios = require('axios');

const API_BASE = 'https://t-c0263bdb63e24cda.hostc.dev';

async function testApi() {
  console.log('1. 测试 feishuInit...');
  try {
    const res1 = await axios.post(`${API_BASE}/xyz/agent-bots/feishu/init`);
    console.log('feishuInit 响应:', res1.data);
  } catch (err) {
    console.log('feishuInit 错误:', err.message);
  }

  console.log('\n2. 测试 feishuBegin...');
  try {
    const res2 = await axios.post(`${API_BASE}/xyz/agent-bots/feishu/begin`);
    console.log('feishuBegin 响应:', res2.data);
  } catch (err) {
    console.log('feishuBegin 错误:', err.message);
  }

  console.log('\n3. 测试 getAgentBots...');
  try {
    const res3 = await axios.get(`${API_BASE}/xyz/agent-bots`);
    console.log('getAgentBots 响应:', res3.data);
  } catch (err) {
    console.log('getAgentBots 错误:', err.message);
  }
}

testApi();
