"""Mode → tool name mapping (no LangChain dependency)."""


def tool_names_for_mode(mode: str) -> set[str]:
    normalized = (mode or "ask").strip().lower() or "ask"
    if normalized == "ask":
        return set()
    if normalized == "plan":
        return {"read_file", "list_dir"}
    return {"read_file", "list_dir", "write_file", "exec_shell"}
