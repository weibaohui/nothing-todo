import { useTheme } from '../hooks/useTheme';
import MDEditor from '@uiw/react-md-editor';

interface MdEditorProps {
  value: string;
  onChange: (value: string) => void;
  height?: number;
}

export function MdEditor({
  value,
  onChange,
  height = 400,
}: MdEditorProps) {
  const { themeMode } = useTheme();

  return (
    <div data-color-mode={themeMode}>
      <MDEditor
        value={value}
        onChange={(val) => onChange(val || '')}
        height={height}
        preview="edit"
        style={{ height, minHeight: height }}
      />
    </div>
  );
}