import { useState } from 'react';
import ReactMde from 'react-mde';
import { marked } from 'marked';
import DOMPurify from 'dompurify';
import 'react-mde/lib/styles/css/react-mde-editor.css';
import 'react-mde/lib/styles/css/react-mde-preview.css';
import 'react-mde/lib/styles/css/react-mde-toolbar.css';
import 'react-mde/lib/styles/css/react-mde-suggestions.css';
import 'react-mde/lib/styles/css/variables.css';

interface ReactMdeEditorProps {
  value: string;
  onChange: (value: string) => void;
  height?: number;
}

export function ReactMdeEditor({
  value,
  onChange,
  height = 300,
}: ReactMdeEditorProps) {
  const [selectedTab, setSelectedTab] = useState<'write' | 'preview'>('write');

  return (
    <div style={{ marginBottom: 12, height }}>
      <ReactMde
        value={value}
        onChange={onChange}
        selectedTab={selectedTab}
        onTabChange={setSelectedTab}
        generateMarkdownPreview={(markdown: string) => {
          const html = marked.parse(markdown) as string;
          return Promise.resolve(
            <div dangerouslySetInnerHTML={{ __html: DOMPurify.sanitize(html) }} />
          );
        }}
      />
    </div>
  );
}
