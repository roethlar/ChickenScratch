import { useState, type FormEvent } from "react";
import { KeyRound } from "lucide-react";
import { setWebToken } from "../../commands/transport";

interface WebTokenGateProps {
  onDone: () => void;
}

/**
 * Shown in the browser shell when no access token is stored (first visit
 * without the `?token=` link, or the server rejected the stored one).
 */
export function WebTokenGate({ onDone }: WebTokenGateProps) {
  const [value, setValue] = useState("");

  const submit = (event: FormEvent) => {
    event.preventDefault();
    const token = value.trim();
    if (!token) return;
    setWebToken(token);
    onDone();
  };

  return (
    <form className="welcome-token" onSubmit={submit}>
      <p className="welcome-token-text">
        <KeyRound size={16} /> This server asks for an access token. It was printed when
        chikn-web started, and the link it printed carries it too.
      </p>
      <label className="welcome-token-field">
        <span>Access token</span>
        <input
          type="password"
          value={value}
          onChange={(e) => setValue(e.target.value)}
          autoComplete="off"
          spellCheck={false}
        />
      </label>
      <button type="submit" className="welcome-btn" disabled={!value.trim()}>
        <span>Continue</span>
      </button>
    </form>
  );
}
