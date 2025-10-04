/**
 * Main Application Component
 */

import { useEffect } from 'react';
import { useProjectStore } from './stores/projectStore';
import { Editor } from './components/editor/Editor';
import { DocumentTree } from './components/navigator/DocumentTree';

function App() {
  const {
    currentProject,
    currentDocumentId,
    getCurrentDocument,
    setCurrentDocument,
    updateDocument,
  } = useProjectStore();

  const currentDoc = getCurrentDocument();

  // Auto-save with debouncing (TODO: implement proper debounce)
  const handleContentChange = (content: string) => {
    if (currentDocumentId) {
      updateDocument(currentDocumentId, content);
    }
  };

  // Show welcome screen if no project loaded
  if (!currentProject) {
    return (
      <div className="min-h-screen bg-background flex items-center justify-center">
        <div className="text-center space-y-4">
          <h1 className="text-4xl font-bold text-foreground">
            🐔 Chicken Scratch
          </h1>
          <p className="text-muted-foreground">
            Where messy drafts become masterpieces
          </p>
          <div className="pt-8 space-x-4">
            <button
              className="px-4 py-2 bg-primary text-primary-foreground rounded-md hover:bg-primary/90"
              onClick={() => alert('Create project dialog - TODO')}
            >
              Create Project
            </button>
            <button
              className="px-4 py-2 border rounded-md hover:bg-gray-100"
              onClick={() => alert('Open project dialog - TODO')}
            >
              Open Project
            </button>
          </div>
        </div>
      </div>
    );
  }

  // Main editor view
  return (
    <div className="h-screen flex flex-col">
      {/* Top bar */}
      <div className="h-12 border-b px-4 flex items-center justify-between bg-white">
        <h1 className="font-semibold">{currentProject.name}</h1>
        <div className="text-sm text-gray-500">
          {currentDoc ? currentDoc.name : 'No document selected'}
        </div>
      </div>

      {/* Main content */}
      <div className="flex-1 flex overflow-hidden">
        {/* Sidebar - Document Navigator */}
        <div className="w-64 border-r">
          <DocumentTree
            nodes={currentProject.hierarchy}
            onSelectDocument={setCurrentDocument}
            currentDocumentId={currentDocumentId}
          />
        </div>

        {/* Editor */}
        <div className="flex-1">
          {currentDoc ? (
            <Editor
              content={currentDoc.content}
              onChange={handleContentChange}
              placeholder={`Start writing ${currentDoc.name}...`}
            />
          ) : (
            <div className="h-full flex items-center justify-center text-gray-400">
              Select a document to start writing
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

export default App;
