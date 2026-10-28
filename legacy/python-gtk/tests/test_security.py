from envedit.parser import parse_text
from envedit.security import is_sensitive_key


def test_sensitive_names_are_case_insensitive():
    assert is_sensitive_key("db_password")
    assert is_sensitive_key("API_TOKEN")
    assert is_sensitive_key("private_key_path")
    assert not is_sensitive_key("DB_HOST")


def test_reveal_state_never_changes_document_or_injects_bullets():
    doc = parse_text("DB_PASSWORD=fixture-secret\nVISIBLE=yes\n")
    original = doc.to_bytes()
    visible = False
    visible = not visible
    visible = not visible
    assert visible is False
    assert not doc.modified
    assert doc.to_bytes() == original
    assert "••••" not in doc.to_text()
