from rex_agent.modes import tool_names_for_mode


def test_plan_mode_exposes_read_and_list():
    assert tool_names_for_mode("plan") == {"read_file", "list_dir"}


def test_agent_mode_includes_write_and_shell():
    names = tool_names_for_mode("agent")
    assert {"read_file", "list_dir", "write_file", "exec_shell"} <= names


def test_ask_mode_has_no_tools():
    assert tool_names_for_mode("ask") == set()
