from rex_agent.streaming import chunk_text, run_turn_chunks


def test_chunk_text_splits_by_max_chars() -> None:
    assert chunk_text("abcdef", max_chars=2) == ["ab", "cd", "ef"]


def test_run_turn_chunks_terminal_done() -> None:
    chunks = run_turn_chunks("hi", max_chars=8)
    assert len(chunks) >= 2
    assert not chunks[0].done
    assert chunks[-1].done
    assert chunks[-1].text == ""
    assert "".join(c.text for c in chunks if not c.done) == "hi"


def test_empty_text_yields_only_terminal_chunk() -> None:
    chunks = run_turn_chunks("")
    assert len(chunks) == 1
    assert chunks[0].done
