import { useProjectStore } from "./stores/projectStore";
import { Welcome } from "./components/welcome/Welcome";
import { Binder } from "./components/binder/Binder";
import { Editor } from "./components/editor/Editor";

export default function App() {
  const project = useProjectStore((s) => s.project);

  if (!project) {
    return <Welcome />;
  }

  return (
    <div className="app">
      <Binder />
      <Editor />
    </div>
  );
}
