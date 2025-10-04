/**
 * TipTap Editor Component
 *
 * Markdown editor with rich text formatting.
 */

import { useEditor, EditorContent } from '@tiptap/react';
import StarterKit from '@tiptap/starter-kit';
import Placeholder from '@tiptap/extension-placeholder';
import { useEffect } from 'react';

interface EditorProps {
  content: string;
  onChange: (content: string) => void;
  placeholder?: string;
  editable?: boolean;
}

export function Editor({ content, onChange, placeholder, editable = true }: EditorProps) {
  const editor = useEditor({
    extensions: [
      StarterKit,
      Placeholder.configure({
        placeholder: placeholder || 'Start writing...',
      }),
    ],
    content,
    editable,
    onUpdate: ({ editor }) => {
      const markdown = editor.getText(); // TODO: Use proper markdown serializer
      onChange(markdown);
    },
  });

  // Update editor content when prop changes
  useEffect(() => {
    if (editor && content !== editor.getText()) {
      editor.commands.setContent(content);
    }
  }, [content, editor]);

  if (!editor) {
    return <div>Loading editor...</div>;
  }

  return (
    <div className="h-full flex flex-col">
      {editable && (
        <div className="border-b p-2 flex gap-2">
          <button
            onClick={() => editor.chain().focus().toggleBold().run()}
            className={editor.isActive('bold') ? 'font-bold' : ''}
          >
            Bold
          </button>
          <button
            onClick={() => editor.chain().focus().toggleItalic().run()}
            className={editor.isActive('italic') ? 'italic' : ''}
          >
            Italic
          </button>
          <button
            onClick={() => editor.chain().focus().toggleHeading({ level: 1 }).run()}
            className={editor.isActive('heading', { level: 1 }) ? 'font-bold' : ''}
          >
            H1
          </button>
          <button
            onClick={() => editor.chain().focus().toggleHeading({ level: 2 }).run()}
            className={editor.isActive('heading', { level: 2 }) ? 'font-bold' : ''}
          >
            H2
          </button>
        </div>
      )}
      <div className="flex-1 overflow-auto p-4">
        <EditorContent editor={editor} className="prose max-w-none" />
      </div>
    </div>
  );
}
