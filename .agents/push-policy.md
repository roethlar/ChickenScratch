<!-- push-policy: always -->
Push without asking. After a commit lands with its verification green, push
the branch to every configured remote (currently `github` and `origin`) and
report what went where. Owner directive 2026-10-27 ("set the push policy to
always"), recorded in `.agents/decisions.md`. A refused push is reported,
never forced; history rewrites remain forbidden per `AGENTS.md` Git Safety.
