import { useTheme } from '../hooks/useTheme';
import MDEditor from '@uiw/react-md-editor';

interface MdEditorProps {
  value: string;
  onChange: (value: string) => void;
  height?: number | string;
}

export function MdEditor({
  value,
  onChange,
  height,
}: MdEditorProps) {
  const { themeMode } = useTheme();

  return (
    <div data-color-mode={themeMode} style={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
      <MDEditor
        value={value}
        onChange={(val) => onChange(val || '')}
        preview="edit"
        style={{ flex: 1, minHeight: typeof height === 'number' ? height : (height || '100%') }}
      />
    </div>
  );
}