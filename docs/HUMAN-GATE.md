# For the repository owner

**You don't load anything.** Grok, Codex, Claude Code, and Antigravity read [`AGENTS.md`](../AGENTS.md) automatically when you're in this repo. Say what you want.

Optional reading: [`START-HERE.md`](START-HERE.md)

---

## Your job

- Say what you want in normal language
- Say yes/no when the agent asks a real decision question
- Try the app when they tell you how

You are **not** expected to read code, diffs, or tell the agent which files to read.

---

## When the agent must ask you

- Two reasonable approaches and it genuinely doesn't know which you prefer
- You'd be throwing away a previous decision (e.g. bringing back the old Windows C# app)
- Something would change how your files are stored on disk

Otherwise it should just do the work and report back simply.

---

## How agents interpret you (agents read this)

**Do not require jargon from the owner.** These plain phrases mean the same as internal governance keywords:

| Owner says | Agent may |
|------------|-----------|
| "I changed my mind — …" | Update invariants / decision notes to match |
| "Stop working on X" / "Forget X" | Update current phase; pause that work |
| "Let's prioritize Y instead" | Reorder work in `CURRENT_PHASE.md` |
| "Do X" | Execute if it doesn't break stored projects |

Never echo acronyms like ADR or invariant to the owner unless they ask.

---

## Handoff format (agents → owner)

1. **What I did** — two or three sentences
2. **Try it** — numbered clicks/steps
3. **Blocked?** — one sentence, plus a yes/no question if needed

No diffs. No file paths unless the owner asks.