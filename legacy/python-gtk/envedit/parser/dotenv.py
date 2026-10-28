from __future__ import annotations

import re

from envedit.model import Document, EnvLine, LineKind


_ASSIGNMENT = re.compile(
    r"^(?P<head>\s*(?:export\s+)?(?P<key>[A-Za-z_][A-Za-z0-9_]*)\s*=\s*)(?P<rest>.*)$"
)


def _split_lines(text: str) -> list[tuple[str, str]]:
    result: list[tuple[str, str]] = []
    for match in re.finditer(r"(.*?)(\r\n|\n|\r|$)", text, re.DOTALL):
        body, ending = match.group(1), match.group(2)
        if body == "" and ending == "" and match.start() == len(text):
            break
        result.append((body, ending))
    return result


def _value_extent(rest: str) -> tuple[int, int, str | None] | None:
    if rest.startswith(("'", '"')):
        quote = rest[0]
        escaped = False
        close = None
        for i, char in enumerate(rest[1:], 1):
            if quote == '"' and char == "\\" and not escaped:
                escaped = True
                continue
            if char == quote and not escaped:
                close = i
                break
            escaped = False
        if close is None:
            return None
        tail = rest[close + 1:]
        if tail.strip() and not tail.lstrip().startswith("#"):
            return None
        return 0, close + 1, quote

    comment = None
    for i, char in enumerate(rest):
        if char == "#" and i > 0 and rest[i - 1].isspace():
            comment = i
            break
    end = len(rest) if comment is None else comment
    while end > 0 and rest[end - 1].isspace():
        end -= 1
    return 0, end, None


def parse_text(text: str) -> Document:
    lines: list[EnvLine] = []
    for body, ending in _split_lines(text):
        if not body.strip():
            lines.append(EnvLine(body, ending, LineKind.BLANK))
            continue
        if body.lstrip().startswith("#"):
            lines.append(EnvLine(body, ending, LineKind.COMMENT))
            continue
        match = _ASSIGNMENT.match(body)
        if match:
            extent = _value_extent(match.group("rest"))
            if extent is not None:
                begin, end, quote = extent
                offset = match.start("rest")
                lines.append(EnvLine(body, ending, LineKind.ASSIGNMENT,
                                     match.group("key"), offset + begin, offset + end, quote))
                continue
        lines.append(EnvLine(body, ending, LineKind.UNKNOWN))
    return Document(lines)


def parse_bytes(data: bytes) -> Document:
    try:
        return parse_text(data.decode("utf-8"))
    except UnicodeDecodeError as exc:
        raise UnicodeError("EnvEdit prend actuellement en charge les fichiers UTF-8 uniquement") from exc
