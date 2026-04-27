import { useEffect, useRef, useId } from 'react';
import Cherry from 'cherry-markdown';
import 'cherry-markdown/dist/cherry-markdown.css';

interface CherryMarkdownEditorProps {
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  height?: string;
}

export function CherryMarkdownEditor({
  value,
  onChange,
  placeholder,
  height = '300px',
}: CherryMarkdownEditorProps) {
  const reactId = useId();
  const editorId = 'cherry-' + reactId.replace(/:/g, '');
  const cherryRef = useRef<Cherry | null>(null);
  const onChangeRef = useRef(onChange);
  const isInternalUpdate = useRef(false);

  onChangeRef.current = onChange;

  useEffect(() => {
    const el = document.getElementById(editorId);
    if (!el) return;

    cherryRef.current = new Cherry({
      id: editorId,
      value: value || '',
      isPreviewOnly: false,
      editor: {
        defaultModel: 'edit&preview',
        height,
        codemirror: placeholder ? { placeholder } : {},
      },
      toolbars: {
        toolbar: [
          'bold', 'italic', 'strikethrough', '|',
          'header', 'list', '|',
          'code', 'inlineCode', '|',
          'table', 'link', '|',
          'togglePreview',
        ],
        toolbarRight: [],
        bubble: [],
        float: [],
      },
      callback: {
        afterChange: (newVal: string) => {
          if (!isInternalUpdate.current) {
            onChangeRef.current(newVal);
          }
        },
      },
    });

    return () => {
      cherryRef.current?.destroy();
      cherryRef.current = null;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    const cherry = cherryRef.current;
    if (!cherry) return;

    const currentVal = cherry.getMarkdown();
    if (currentVal !== value) {
      isInternalUpdate.current = true;
      cherry.setMarkdown(value || '');
      isInternalUpdate.current = false;
    }
  }, [value]);

  return (
    <div
      id={editorId}
      style={{ marginBottom: 12 }}
    />
  );
}
