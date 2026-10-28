from __future__ import annotations

from dataclasses import dataclass
from enum import Enum, auto
import re


class ParseError(ValueError):
    """Raised when an edit cannot be represented without risking data loss."""


class LineKind(Enum):
    BLANK = auto()
    COMMENT = auto()
    ASSIGNMENT = auto()
    UNKNOWN = auto()


@dataclass
class EnvLine:
    body: str
    ending: str
    kind: LineKind
    key: str | None = None
    value_start: int | None = None
    value_end: int | None = None
    quote: str | None = None

    @property
    def value(self) -> str:
        if self.kind is not LineKind.ASSIGNMENT:
            raise ParseError("Cette ligne n'est pas une affectation éditable")
        assert self.value_start is not None and self.value_end is not None
        value = self.body[self.value_start:self.value_end]
        if self.quote:
            return value[1:-1]
        return value

    def set_value(self, value: str) -> None:
        if self.kind is not LineKind.ASSIGNMENT:
            raise ParseError("Cette ligne n'est pas une affectation éditable")
        if "\n" in value or "\r" in value:
            raise ParseError("Les valeurs multilignes ne sont pas prises en charge")
        assert self.value_start is not None and self.value_end is not None
        if self.quote and self.quote in value:
            raise ParseError("La valeur contient le délimiteur de citation existant")
        rendered = f"{self.quote}{value}{self.quote}" if self.quote else value
        self.body = self.body[:self.value_start] + rendered + self.body[self.value_end:]
        self.value_end = self.value_start + len(rendered)

    def rename(self, key: str) -> None:
        if self.kind is not LineKind.ASSIGNMENT or self.key is None:
            raise ParseError("Cette ligne n'est pas une affectation éditable")
        if not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", key):
            raise ParseError("Nom de variable invalide")
        match = re.match(r"^\s*(?:export\s+)?(?P<key>[A-Za-z_][A-Za-z0-9_]*)", self.body)
        assert match is not None
        start, end = match.span("key")
        delta = len(key) - (end - start)
        self.body = self.body[:start] + key + self.body[end:]
        self.key = key
        assert self.value_start is not None and self.value_end is not None
        self.value_start += delta
        self.value_end += delta


class Document:
    def __init__(self, lines: list[EnvLine], encoding: str = "utf-8") -> None:
        self.lines = lines
        self.encoding = encoding
        self._original = self.to_bytes()

    def to_text(self) -> str:
        return "".join(line.body + line.ending for line in self.lines)

    def to_bytes(self) -> bytes:
        return self.to_text().encode(self.encoding)

    @property
    def modified(self) -> bool:
        return self.to_bytes() != self._original

    def mark_saved(self) -> None:
        self._original = self.to_bytes()

    @property
    def assignments(self) -> list[EnvLine]:
        return [line for line in self.lines if line.kind is LineKind.ASSIGNMENT]

    @property
    def unknown_lines(self) -> list[int]:
        return [i + 1 for i, line in enumerate(self.lines) if line.kind is LineKind.UNKNOWN]

    @property
    def duplicate_keys(self) -> set[str]:
        seen: set[str] = set()
        duplicates: set[str] = set()
        for line in self.assignments:
            assert line.key is not None
            if line.key in seen:
                duplicates.add(line.key)
            seen.add(line.key)
        return duplicates

    def comments_for(self, target: EnvLine) -> str:
        index = self.lines.index(target)
        comments: list[str] = []
        i = index - 1
        while i >= 0 and self.lines[i].kind is LineKind.COMMENT:
            text = self.lines[i].body.lstrip()[1:]
            comments.append(text[1:] if text.startswith(" ") else text)
            i -= 1
        return "\n".join(reversed(comments))

    def set_comments(self, target: EnvLine, text: str) -> None:
        if "\r" in text:
            raise ParseError("Retour chariot invalide dans le commentaire")
        index = self.lines.index(target)
        start = index
        while start > 0 and self.lines[start - 1].kind is LineKind.COMMENT:
            start -= 1
        ending = target.ending or self.preferred_ending
        replacement = [EnvLine("# " + part, ending, LineKind.COMMENT)
                       for part in text.split("\n") if part != ""]
        self.lines[start:index] = replacement

    @property
    def preferred_ending(self) -> str:
        return next((line.ending for line in self.lines if line.ending), "\n")

    def add(self, key: str, value: str = "") -> EnvLine:
        if not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", key):
            raise ParseError("Nom de variable invalide")
        if "\n" in value or "\r" in value:
            raise ParseError("Les valeurs multilignes ne sont pas prises en charge")
        if self.lines and not self.lines[-1].ending:
            self.lines[-1].ending = self.preferred_ending
        line = EnvLine(f"{key}={value}", "", LineKind.ASSIGNMENT, key, len(key) + 1,
                       len(key) + 1 + len(value), None)
        self.lines.append(line)
        return line

    def remove(self, target: EnvLine) -> None:
        index = self.lines.index(target)
        start = index
        while start > 0 and self.lines[start - 1].kind is LineKind.COMMENT:
            start -= 1
        del self.lines[start:index + 1]
