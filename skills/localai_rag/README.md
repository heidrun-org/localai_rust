# Local AI RAG

Install [SKILL.md](SKILL.md) in your agent's skill directory. The skill searches saved collections of local Markdown and plain-text files by meaning and cites matching passages by file and line number.

The skill reuses the executable and model when available. If setup is needed, the agent explains what needs to be installed, built, or downloaded and asks for approval before starting.

## Example request

> Search my saved `project-notes` collection for why review servers use a separate port. Quote the sentence that explains the reason and cite its file and line number.

See the [Local AI RAG command reference](../../packages/localai_rag/README.md) for how to create, refresh, and search a collection.
