import os

import pytest

from envedit.storage import ConflictError, StorageError, open_file


def test_save_is_targeted_and_preserves_permissions(tmp_path):
    path = tmp_path / ".env"
    path.write_bytes(b"A=one\r\nB=two\r\n")
    path.chmod(0o640)
    session = open_file(path)
    session.document.assignments[0].set_value("changed")
    session.save()
    assert path.read_bytes() == b"A=changed\r\nB=two\r\n"
    assert path.stat().st_mode & 0o777 == 0o640


def test_external_change_is_never_overwritten(tmp_path):
    path = tmp_path / ".env"
    path.write_text("A=one\n")
    session = open_file(path)
    session.document.assignments[0].set_value("ours")
    path.write_text("A=theirs\n")
    with pytest.raises(ConflictError):
        session.save()
    assert path.read_text() == "A=theirs\n"


def test_symlink_is_refused(tmp_path):
    target = tmp_path / "target"
    target.write_text("A=one\n")
    link = tmp_path / ".env"
    link.symlink_to(target)
    with pytest.raises(StorageError):
        open_file(link)
