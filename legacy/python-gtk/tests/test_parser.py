from pathlib import Path

import pytest

from envedit.model import LineKind, ParseError
from envedit.parser import parse_bytes, parse_text


FIXTURE = Path(__file__).parent / "fixtures" / "sample.env"


def test_identity_is_byte_exact():
    data = FIXTURE.read_bytes()
    assert parse_bytes(data).to_bytes() == data


@pytest.mark.parametrize("ending,final", [("\n", True), ("\r\n", True), ("\n", False), ("\r\n", False)])
def test_line_endings_and_missing_final_newline(ending, final):
    raw = (f"A=one{ending}B='two # x'" + (ending if final else "")).encode()
    doc = parse_bytes(raw)
    doc.assignments[0].set_value("changed")
    expected = (f"A=changed{ending}B='two # x'" + (ending if final else "")).encode()
    assert doc.to_bytes() == expected


def test_only_target_value_changes():
    raw = b"  export A = old  # keep\nB=untouched\n"
    doc = parse_bytes(raw)
    doc.assignments[0].set_value("new")
    assert doc.to_bytes() == b"  export A = new  # keep\nB=untouched\n"


def test_comments_association_and_blank_separator():
    doc = parse_text("# one\n# two\nA=x\n\n# separate\nB=y\n")
    assert doc.comments_for(doc.assignments[0]) == "one\ntwo"
    assert doc.comments_for(doc.assignments[1]) == "separate"
    doc.set_comments(doc.assignments[0], "changed\nagain")
    assert doc.to_text().startswith("# changed\n# again\nA=x\n")


@pytest.mark.parametrize("source,value", [
    ("A=abc\n", "abc"), ("A='a # b'\n", "a # b"), ("A=\"a # b\"\n", "a # b"),
    ("A=a=b#c\n", "a=b#c"), ("A=été\n", "été"), ("A=  spaced  \n", "spaced"),
])
def test_value_forms(source, value):
    doc = parse_text(source)
    assert doc.assignments[0].value == value
    assert doc.to_text() == source


def test_duplicates_are_retained_and_reported():
    doc = parse_text("A=1\nA=2\n")
    assert doc.duplicate_keys == {"A"}
    assert [line.value for line in doc.assignments] == ["1", "2"]
    assert doc.to_text() == "A=1\nA=2\n"


def test_unknown_and_ambiguous_syntax_is_untouched():
    source = "BROKEN LINE\nA=\"unclosed\ncontinuation\nB=ok\n"
    doc = parse_text(source)
    assert doc.unknown_lines == [1, 2, 3]
    doc.assignments[0].set_value("yes")
    assert doc.to_text() == "BROKEN LINE\nA=\"unclosed\ncontinuation\nB=yes\n"


def test_rename_add_remove_and_validation():
    doc = parse_text("# note\nA=1\n")
    line = doc.assignments[0]
    line.rename("RENAMED")
    added = doc.add("B", "2")
    assert doc.to_text() == "# note\nRENAMED=1\nB=2"
    doc.remove(line)
    assert doc.to_text() == "B=2"
    with pytest.raises(ParseError):
        added.rename("bad-key")


def test_rename_exported_key_does_not_change_export_keyword():
    doc = parse_text("  export OLD = value # note\n")
    doc.assignments[0].rename("NEW")
    assert doc.to_text() == "  export NEW = value # note\n"


def test_one_hundred_variables_round_trip_and_targeted_edit():
    source = "".join(f"KEY_{index}=value-{index}\n" for index in range(120))
    doc = parse_text(source)
    assert len(doc.assignments) == 120
    doc.assignments[99].set_value("changed")
    assert doc.to_text() == source.replace("KEY_99=value-99\n", "KEY_99=changed\n")
