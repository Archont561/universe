"""Tests for shared filesystem helpers."""

from pathlib import Path

from scripts.lib.files import list_files, path_exists


def test_lists_nested_files_in_stable_order_and_checks_existence(tmp_path: Path) -> None:
    nested = tmp_path / "b"
    nested.mkdir()
    (nested / "z.txt").write_text("z", encoding="utf-8")
    (tmp_path / "a.txt").write_text("a", encoding="utf-8")

    listed = [path.relative_to(tmp_path).as_posix() for path in list_files(tmp_path)]

    assert listed == ["a.txt", "b/z.txt"]
    assert path_exists(tmp_path / "a.txt")
    assert not path_exists(tmp_path / "missing.txt")
