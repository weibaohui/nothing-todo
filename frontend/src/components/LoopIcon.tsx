// ♾ 无穷环路图标 — 自定义 SVG，替代 antd 中没有的 infinity 符号。
//
// 与 antd <Icon /> 组件兼容：viewBox="0 0 1024 1024"，width/height 由外部控制，
// fill="currentColor" 让颜色继承父元素的 color/style，在暗/亮主题下自动适应。
import React from 'react';

interface LoopIconProps {
  style?: React.CSSProperties;
  className?: string;
}

export function LoopIcon({ style, className }: LoopIconProps) {
  return (
    <svg
      viewBox="0 0 1024 1024"
      width="1em"
      height="1em"
      fill="currentColor"
      style={style}
      className={className}
    >
      <path d="M512 213.3C279.8 213.3 90.7 402.4 90.7 634.7s189.1 421.3 421.3 421.3c104.9 0 200.8-38.3 274.7-101.9l-72.5-72.5c-56.3 48.6-128.8 78.4-202.2 78.4-173.8 0-314.7-140.9-314.7-314.7S338.2 330.7 512 330.7c108.5 0 203.9 55 260.4 138.7H618.7v102.4h256V320h-102.4v108.1C706.5 330.3 614.4 266.7 512 266.7zm0 490.7c84.8 0 153.6-68.8 153.6-153.6S596.8 448 512 448s-153.6 68.8-153.6 153.6S427.2 704 512 704z" />
    </svg>
  );
}
