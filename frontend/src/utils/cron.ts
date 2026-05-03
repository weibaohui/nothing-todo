import type { Locale } from 'react-js-cron';

/** Chinese locale for react-js-cron */
export const CRON_ZH_LOCALE: Locale = {
  everyText: '每',
  emptyMonths: '每月',
  emptyMonthDays: '每日',
  emptyMonthDaysShort: '每日',
  emptyWeekDays: '每周几',
  emptyWeekDaysShort: '每天',
  emptyHours: '每小时',
  emptyMinutes: '每分钟',
  emptyMinutesForHourPeriod: '每分钟',
  yearOption: '年',
  monthOption: '月',
  weekOption: '周',
  dayOption: '天',
  hourOption: '小时',
  minuteOption: '分钟',
  rebootOption: '重启',
  prefixPeriod: '每',
  prefixMonths: '的',
  prefixMonthDays: '的',
  prefixWeekDays: '的',
  prefixWeekDaysForMonthAndYearPeriod: '和',
  prefixHours: '的',
  prefixMinutes: '的',
  prefixMinutesForHourPeriod: '的',
  suffixMinutesForHourPeriod: '',
  errorInvalidCron: '无效的 Cron 表达式',
  clearButtonText: '清空',
  weekDays: ['周日', '周一', '周二', '周三', '周四', '周五', '周六'],
  months: ['一月', '二月', '三月', '四月', '五月', '六月', '七月', '八月', '九月', '十月', '十一月', '十二月'],
  altWeekDays: ['日', '一', '二', '三', '四', '五', '六'],
  altMonths: ['1月', '2月', '3月', '4月', '5月', '6月', '7月', '8月', '9月', '10月', '11月', '12月'],
};

/** 6-field cron (with seconds) → 5-field (drop seconds) */
export function cronTo5(value: string): string {
  const parts = value.trim().split(/\s+/);
  return parts.length >= 6 ? parts.slice(1).join(' ') : value;
}

/** 5-field cron → 6-field (prepend seconds=0) */
export function cronTo6(value: string): string {
  return `0 ${value}`;
}
