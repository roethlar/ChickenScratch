import { useState } from 'react';

function App() {
  const [greetMsg, setGreetMsg] = useState('');
  const [name, setName] = useState('');

  return (
    <div className="min-h-screen bg-background flex items-center justify-center">
      <div className="text-center space-y-4">
        <h1 className="text-4xl font-bold text-foreground">
          🐔 Chicken Scratch
        </h1>
        <p className="text-muted-foreground">
          Where messy drafts become masterpieces
        </p>

        <div className="pt-8">
          <form
            className="space-y-4"
            onSubmit={(e) => {
              e.preventDefault();
              setGreetMsg(`Hello, ${name}! Welcome to Chicken Scratch.`);
            }}
          >
            <input
              className="px-4 py-2 border rounded-md bg-background text-foreground"
              onChange={(e) => setName(e.currentTarget.value)}
              placeholder="Enter your name..."
              value={name}
            />
            <button
              className="ml-2 px-4 py-2 bg-primary text-primary-foreground rounded-md hover:bg-primary/90"
              type="submit"
            >
              Greet
            </button>
          </form>
          {greetMsg && (
            <p className="mt-4 text-foreground">{greetMsg}</p>
          )}
        </div>

        <div className="pt-8 text-sm text-muted-foreground">
          <p>Phase 1: Foundation - In Development</p>
          <p className="text-xs">Tauri 2.0 + React + TypeScript + Rust</p>
        </div>
      </div>
    </div>
  );
}

export default App;
