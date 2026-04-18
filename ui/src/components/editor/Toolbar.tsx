import { useState } from "react";
import type { Editor } from "@tiptap/react";
import {
  Bold,
  Italic,
  Underline,
  Strikethrough,
  Heading1,
  Heading2,
  Heading3,
  List,
  ListOrdered,
  Quote,
  Code,
  Code2,
  Minus,
  Link,
  Unlink,
  Undo,
  Redo,
  Sparkles,
  MessageSquare,
  Asterisk,
} from "lucide-react";
import { useCallback } from "react";
import { dialogPrompt } from "../shared/Dialog";
import { aiTransform, type AiOperation } from "../../commands/ai";
import { toastError, toastSuccess } from "../shared/Toast";
import { useSettingsStore } from "../../stores/settingsStore";
import { useProjectStore } from "../../stores/projectStore";
import * as docCmd from "../../commands/document";
import { getEditorMarkdown } from "./editorRef";

interface ToolbarProps {
  editor: Editor | null;
}

function ToolbarButton({
  onClick,
  active,
  disabled,
  title,
  children,
}: {
  onClick: () => void;
  active?: boolean;
  disabled?: boolean;
  title: string;
  children: React.ReactNode;
}) {
  return (
    <button
      className={`toolbar-btn ${active ? "active" : ""}`}
      onClick={onClick}
      disabled={disabled}
      title={title}
      onMouseDown={(e) => e.preventDefault()} // prevent editor blur
    >
      {children}
    </button>
  );
}

function ToolbarSep() {
  return <div className="toolbar-sep" />;
}

export function Toolbar({ editor }: ToolbarProps) {
  const setLink = useCallback(async () => {
    if (!editor) return;
    const prev = editor.getAttributes("link").href || "";
    const url = await dialogPrompt("URL:", prev);
    if (url === null) return;
    if (url === "") {
      editor.chain().focus().extendMarkRange("link").unsetLink().run();
      return;
    }
    editor.chain().focus().extendMarkRange("link").setLink({ href: url }).run();
  }, [editor]);

  const addFootnote = useCallback(async () => {
    if (!editor) return;
    const body = await dialogPrompt("Footnote text:");
    if (body === null || body.trim() === "") return;
    editor.chain().focus().insertContent({
      type: "footnote",
      attrs: { body: body.trim() },
    }).run();
  }, [editor]);

  const addComment = useCallback(async () => {
    if (!editor) return;
    const { from, to } = editor.state.selection;
    if (from === to) {
      toastError("Select text first to comment on");
      return;
    }
    const body = await dialogPrompt("Comment:");
    if (body === null) return;
    const project = useProjectStore.getState().project;
    const activeDoc = useProjectStore.getState().activeDoc;
    if (!project || !activeDoc) return;
    const commentId = "c_" + Math.random().toString(36).slice(2, 10);
    // Apply the mark locally first
    editor.chain().focus().setMark("comment", { id: commentId }).run();
    try {
      // tiptap-markdown serializes in-process
      const newContent = getEditorMarkdown(editor);
      const updated = await docCmd.addComment(
        project.path, activeDoc.id, commentId, body, newContent
      );
      useProjectStore.setState({ project: updated });
      toastSuccess("Comment added");
    } catch (e) {
      toastError(`Failed: ${e}`);
    }
  }, [editor]);

  const aiEnabled = useSettingsStore((s) => s.appSettings?.ai.enabled) ?? false;

  if (!editor) return null;

  const s = 15; // icon size

  return (
    <div className="editor-toolbar">
      <ToolbarButton
        onClick={() => editor.chain().focus().undo().run()}
        disabled={!editor.can().undo()}
        title="Undo (Ctrl+Z)"
      >
        <Undo size={s} />
      </ToolbarButton>
      <ToolbarButton
        onClick={() => editor.chain().focus().redo().run()}
        disabled={!editor.can().redo()}
        title="Redo (Ctrl+Shift+Z)"
      >
        <Redo size={s} />
      </ToolbarButton>

      <ToolbarSep />

      <ToolbarButton
        onClick={() => editor.chain().focus().toggleBold().run()}
        active={editor.isActive("bold")}
        title="Bold (Ctrl+B)"
      >
        <Bold size={s} />
      </ToolbarButton>
      <ToolbarButton
        onClick={() => editor.chain().focus().toggleItalic().run()}
        active={editor.isActive("italic")}
        title="Italic (Ctrl+I)"
      >
        <Italic size={s} />
      </ToolbarButton>
      <ToolbarButton
        onClick={() => editor.chain().focus().toggleUnderline().run()}
        active={editor.isActive("underline")}
        title="Underline (Ctrl+U)"
      >
        <Underline size={s} />
      </ToolbarButton>
      <ToolbarButton
        onClick={() => editor.chain().focus().toggleStrike().run()}
        active={editor.isActive("strike")}
        title="Strikethrough"
      >
        <Strikethrough size={s} />
      </ToolbarButton>

      <ToolbarSep />

      <ToolbarButton
        onClick={() => editor.chain().focus().toggleHeading({ level: 1 }).run()}
        active={editor.isActive("heading", { level: 1 })}
        title="Heading 1"
      >
        <Heading1 size={s} />
      </ToolbarButton>
      <ToolbarButton
        onClick={() => editor.chain().focus().toggleHeading({ level: 2 }).run()}
        active={editor.isActive("heading", { level: 2 })}
        title="Heading 2"
      >
        <Heading2 size={s} />
      </ToolbarButton>
      <ToolbarButton
        onClick={() => editor.chain().focus().toggleHeading({ level: 3 }).run()}
        active={editor.isActive("heading", { level: 3 })}
        title="Heading 3"
      >
        <Heading3 size={s} />
      </ToolbarButton>

      <ToolbarSep />

      <ToolbarButton
        onClick={() => editor.chain().focus().toggleBulletList().run()}
        active={editor.isActive("bulletList")}
        title="Bullet List"
      >
        <List size={s} />
      </ToolbarButton>
      <ToolbarButton
        onClick={() => editor.chain().focus().toggleOrderedList().run()}
        active={editor.isActive("orderedList")}
        title="Numbered List"
      >
        <ListOrdered size={s} />
      </ToolbarButton>
      <ToolbarButton
        onClick={() => editor.chain().focus().toggleBlockquote().run()}
        active={editor.isActive("blockquote")}
        title="Blockquote"
      >
        <Quote size={s} />
      </ToolbarButton>

      <ToolbarSep />

      <ToolbarButton
        onClick={() => editor.chain().focus().toggleCode().run()}
        active={editor.isActive("code")}
        title="Inline Code"
      >
        <Code size={s} />
      </ToolbarButton>
      <ToolbarButton
        onClick={() => editor.chain().focus().toggleCodeBlock().run()}
        active={editor.isActive("codeBlock")}
        title="Code Block"
      >
        <Code2 size={s} />
      </ToolbarButton>
      <ToolbarButton
        onClick={() => editor.chain().focus().setHorizontalRule().run()}
        title="Horizontal Rule"
      >
        <Minus size={s} />
      </ToolbarButton>

      <ToolbarSep />

      <ToolbarButton
        onClick={setLink}
        active={editor.isActive("link")}
        title="Insert Link"
      >
        <Link size={s} />
      </ToolbarButton>
      {editor.isActive("link") && (
        <ToolbarButton
          onClick={() => editor.chain().focus().unsetLink().run()}
          title="Remove Link"
        >
          <Unlink size={s} />
        </ToolbarButton>
      )}

      <ToolbarSep />

      <ToolbarButton
        onClick={addComment}
        active={editor.isActive("comment")}
        title="Add Comment"
      >
        <MessageSquare size={s} />
      </ToolbarButton>
      <ToolbarButton
        onClick={addFootnote}
        title="Insert Footnote"
      >
        <Asterisk size={s} />
      </ToolbarButton>

      {aiEnabled && (
        <>
          <ToolbarSep />
          <AiMenu editor={editor} />
        </>
      )}
    </div>
  );
}

function AiMenu({ editor }: { editor: Editor }) {
  const [open, setOpen] = useState(false);
  const [working, setWorking] = useState(false);

  const handleOp = useCallback(async (op: AiOperation) => {
    const { from, to } = editor.state.selection;
    if (from === to) {
      toastError("Select some text first");
      setOpen(false);
      return;
    }
    const selectedText = editor.state.doc.textBetween(from, to, " ");
    setWorking(true);
    setOpen(false);
    try {
      const result = await aiTransform(selectedText, op);
      if (result) {
        if (op === "brainstorm") {
          // Insert brainstorm results after selection
          editor.chain().focus().setTextSelection(to).insertContent(
            `<p></p><blockquote><p>${result.replace(/\n/g, "</p><p>")}</p></blockquote>`
          ).run();
        } else {
          // Replace selection
          editor.chain().focus().setTextSelection({ from, to }).deleteSelection().insertContent(result).run();
        }
      }
    } catch (e) {
      toastError(`AI failed: ${e}`);
    }
    setWorking(false);
  }, [editor]);

  return (
    <div className="ai-menu-wrapper">
      <ToolbarButton
        onClick={() => setOpen(!open)}
        active={open}
        disabled={working}
        title="AI writing tools (select text first)"
      >
        {working ? <span className="ai-spinner">...</span> : <Sparkles size={15} />}
      </ToolbarButton>
      {open && (
        <div className="ai-menu-dropdown">
          <button onClick={() => handleOp("polish")}>Polish</button>
          <button onClick={() => handleOp("expand")}>Expand</button>
          <button onClick={() => handleOp("simplify")}>Simplify</button>
          <button onClick={() => handleOp("brainstorm")}>Brainstorm</button>
        </div>
      )}
    </div>
  );
}
