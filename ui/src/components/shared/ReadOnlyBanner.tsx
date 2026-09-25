/**
 * Read-only open banner. Shows every plain-English reason the engine gave
 * (`LoadedProject.read_only_reasons`) instead of one fixed sentence — the
 * old banner always blamed "an older version", even for a damaged notes
 * file or a stray document, and hid the real reason in a hover tooltip
 * (GUI validation 2026-09-25, break-it steps 7b and 8).
 */
export function ReadOnlyBanner({ reasons }: { reasons: string[] }) {
  return (
    <div className="readonly-banner" role="status">
      <div>Opened read-only — nothing will be changed.</div>
      {reasons.length > 0 && (
        <ul className="readonly-banner-reasons">
          {reasons.map((r) => (
            <li key={r}>{capitalize(r)}</li>
          ))}
        </ul>
      )}
    </div>
  );
}

function capitalize(s: string): string {
  return s.length > 0 ? s[0].toUpperCase() + s.slice(1) : s;
}
