use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LineKind {
    Blank,
    Comment,
    Assignment,
    Unknown,
}

#[derive(Clone, Debug)]
pub struct EnvLine {
    pub body: String,
    #[allow(dead_code)]
    pub ending: String,
    pub kind: LineKind,
    pub key: Option<String>,
    pub value_start: Option<usize>,
    pub value_end: Option<usize>,
    pub quote: Option<char>,
}

impl EnvLine {
    pub fn value(&self) -> Option<&str> {
        let start = self.value_start?;
        let end = self.value_end?;
        let raw = self.body.get(start..end)?;
        if self.quote.is_some() {
            raw.get(1..raw.len().saturating_sub(1))
        } else {
            Some(raw)
        }
    }

    pub fn set_value(&mut self, value: &str) -> Result<(), String> {
        if value.contains(['\n', '\r']) {
            return Err("Les valeurs multilignes ne sont pas prises en charge".into());
        }
        let start = self
            .value_start
            .ok_or_else(|| "Ligne non éditable".to_string())?;
        let end = self
            .value_end
            .ok_or_else(|| "Ligne non éditable".to_string())?;
        if self.quote.is_some_and(|quote| value.contains(quote)) {
            return Err("La valeur contient le délimiteur de citation existant".into());
        }
        let rendered = self
            .quote
            .map(|quote| format!("{quote}{value}{quote}"))
            .unwrap_or_else(|| value.to_string());
        self.body.replace_range(start..end, &rendered);
        self.value_end = Some(start + rendered.len());
        Ok(())
    }

    pub fn rename(&mut self, key: &str) -> Result<(), String> {
        if !valid_key(key) || self.kind != LineKind::Assignment {
            return Err("Nom de variable invalide".into());
        }
        let old_key = self
            .key
            .as_deref()
            .ok_or_else(|| "Ligne non éditable".to_string())?;
        let value_start = self
            .value_start
            .ok_or_else(|| "Position de valeur introuvable".to_string())?;
        let offset = self.body[..value_start]
            .rfind(old_key)
            .ok_or_else(|| "Position de clé introuvable".to_string())?;
        let end = offset + old_key.len();
        let delta = key.len() as isize - old_key.len() as isize;
        self.body.replace_range(offset..end, key);
        self.key = Some(key.to_string());
        self.value_start = self
            .value_start
            .map(|position| position.saturating_add_signed(delta));
        self.value_end = self
            .value_end
            .map(|position| position.saturating_add_signed(delta));
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct Document {
    pub lines: Vec<EnvLine>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VariableDto {
    pub id: String,
    pub line_index: usize,
    pub key: String,
    pub value: String,
    pub comment: String,
    pub sensitive: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedDocumentDto {
    pub variables: Vec<VariableDto>,
    pub diagnostics: Vec<String>,
    pub unknown_lines: Vec<usize>,
    pub duplicate_keys: Vec<String>,
}

fn is_key_start(character: char) -> bool {
    character == '_' || character.is_ascii_alphabetic()
}

fn is_key_continue(character: char) -> bool {
    character == '_' || character.is_ascii_alphanumeric()
}

fn valid_key(key: &str) -> bool {
    let mut characters = key.chars();
    characters.next().is_some_and(is_key_start) && characters.all(is_key_continue)
}

fn parse_assignment(body: &str, ending: String) -> Option<EnvLine> {
    let bytes = body.as_bytes();
    let mut cursor = 0;
    while bytes.get(cursor).is_some_and(u8::is_ascii_whitespace) {
        cursor += 1;
    }
    if body.get(cursor..)?.starts_with("export") {
        let after = cursor + "export".len();
        if !bytes.get(after).is_some_and(u8::is_ascii_whitespace) {
            return None;
        }
        cursor = after;
        while bytes.get(cursor).is_some_and(u8::is_ascii_whitespace) {
            cursor += 1;
        }
    }
    let key_start = cursor;
    let first = body.get(cursor..)?.chars().next()?;
    if !is_key_start(first) {
        return None;
    }
    cursor += first.len_utf8();
    while let Some(character) = body.get(cursor..).and_then(|rest| rest.chars().next()) {
        if !is_key_continue(character) {
            break;
        }
        cursor += character.len_utf8();
    }
    let key_end = cursor;
    while bytes.get(cursor).is_some_and(u8::is_ascii_whitespace) {
        cursor += 1;
    }
    if bytes.get(cursor) != Some(&b'=') {
        return None;
    }
    cursor += 1;
    while bytes.get(cursor).is_some_and(u8::is_ascii_whitespace) {
        cursor += 1;
    }
    let value_start = cursor;
    let quote = match bytes.get(cursor) {
        Some(b'\'') => Some('\''),
        Some(b'"') => Some('"'),
        _ => None,
    };
    let value_end = if let Some(delimiter) = quote {
        cursor += 1;
        let mut escaped = false;
        let mut closing = None;
        for (offset, character) in body.get(cursor..)?.char_indices() {
            if delimiter == '"' && character == '\\' && !escaped {
                escaped = true;
                continue;
            }
            if character == delimiter && !escaped {
                closing = Some(cursor + offset + character.len_utf8());
                break;
            }
            escaped = false;
        }
        let end = closing?;
        let tail = body.get(end..)?.trim_start();
        if !tail.is_empty() && !tail.starts_with('#') {
            return None;
        }
        end
    } else {
        let rest = body.get(value_start..)?;
        let comment = rest.char_indices().find_map(|(offset, character)| {
            if character == '#'
                && offset > 0
                && rest[..offset]
                    .chars()
                    .last()
                    .is_some_and(char::is_whitespace)
            {
                Some(value_start + offset)
            } else {
                None
            }
        });
        let end = comment.unwrap_or(body.len());
        body[..end].trim_end().len()
    };
    Some(EnvLine {
        body: body.to_string(),
        ending,
        kind: LineKind::Assignment,
        key: Some(body[key_start..key_end].to_string()),
        value_start: Some(value_start),
        value_end: Some(value_end),
        quote,
    })
}

fn split_lines(text: &str) -> Vec<(&str, &str)> {
    let mut result = Vec::new();
    let mut start = 0;
    let bytes = text.as_bytes();
    let mut cursor = 0;
    while cursor < bytes.len() {
        if bytes[cursor] == b'\n' {
            let body_end = if cursor > start && bytes[cursor - 1] == b'\r' {
                cursor - 1
            } else {
                cursor
            };
            let ending = if body_end < cursor { "\r\n" } else { "\n" };
            result.push((&text[start..body_end], ending));
            cursor += 1;
            start = cursor;
        } else if bytes[cursor] == b'\r' {
            if bytes.get(cursor + 1) == Some(&b'\n') {
                cursor += 1;
            } else {
                result.push((&text[start..cursor], "\r"));
                cursor += 1;
                start = cursor;
            }
        } else {
            cursor += 1;
        }
    }
    if start < text.len() {
        result.push((&text[start..], ""));
    }
    result
}

pub fn parse_bytes(data: &[u8]) -> Result<Document, String> {
    let text = std::str::from_utf8(data)
        .map_err(|_| "EnvEdit prend actuellement en charge les fichiers UTF-8 uniquement")?;
    let lines = split_lines(text)
        .into_iter()
        .map(|(body, ending)| {
            if body.trim().is_empty() {
                EnvLine {
                    body: body.to_string(),
                    ending: ending.to_string(),
                    kind: LineKind::Blank,
                    key: None,
                    value_start: None,
                    value_end: None,
                    quote: None,
                }
            } else if body.trim_start().starts_with('#') {
                EnvLine {
                    body: body.to_string(),
                    ending: ending.to_string(),
                    kind: LineKind::Comment,
                    key: None,
                    value_start: None,
                    value_end: None,
                    quote: None,
                }
            } else {
                parse_assignment(body, ending.to_string()).unwrap_or_else(|| EnvLine {
                    body: body.to_string(),
                    ending: ending.to_string(),
                    kind: LineKind::Unknown,
                    key: None,
                    value_start: None,
                    value_end: None,
                    quote: None,
                })
            }
        })
        .collect();
    Ok(Document { lines })
}

impl Document {
    #[allow(dead_code)]
    pub fn to_bytes(&self) -> Vec<u8> {
        self.lines
            .iter()
            .flat_map(|line| [line.body.as_bytes(), line.ending.as_bytes()])
            .flatten()
            .copied()
            .collect()
    }

    pub fn comments_for(&self, line_index: usize) -> String {
        let mut comments = Vec::new();
        let mut cursor = line_index;
        while cursor > 0 && self.lines[cursor - 1].kind == LineKind::Comment {
            cursor -= 1;
            let text = self.lines[cursor].body.trim_start().trim_start_matches('#');
            comments.push(text.strip_prefix(' ').unwrap_or(text));
        }
        comments.reverse();
        comments.join("\n")
    }

    fn preferred_ending(&self) -> &str {
        self.lines
            .iter()
            .find_map(|line| (!line.ending.is_empty()).then_some(line.ending.as_str()))
            .unwrap_or("\n")
    }

    fn comment_start(&self, line_index: usize) -> usize {
        let mut start = line_index;
        while start > 0 && self.lines[start - 1].kind == LineKind::Comment {
            start -= 1;
        }
        start
    }

    pub fn set_comments(&mut self, line_index: usize, text: &str) -> Result<(), String> {
        if text.contains('\r')
            || self.lines.get(line_index).map(|line| line.kind) != Some(LineKind::Assignment)
        {
            return Err("Commentaire invalide ou ligne non éditable".into());
        }
        let start = self.comment_start(line_index);
        let ending = self
            .lines
            .get(line_index)
            .map(|line| line.ending.clone())
            .filter(|ending| !ending.is_empty())
            .unwrap_or_else(|| self.preferred_ending().to_string());
        let replacement = text
            .split('\n')
            .filter(|part| !part.is_empty())
            .map(|part| EnvLine {
                body: format!("# {part}"),
                ending: ending.clone(),
                kind: LineKind::Comment,
                key: None,
                value_start: None,
                value_end: None,
                quote: None,
            })
            .collect::<Vec<_>>();
        self.lines.splice(start..line_index, replacement);
        Ok(())
    }

    pub fn remove_assignment(&mut self, line_index: usize) -> Result<(), String> {
        if self.lines.get(line_index).map(|line| line.kind) != Some(LineKind::Assignment) {
            return Err("Ligne non éditable".into());
        }
        let start = self.comment_start(line_index);
        self.lines.drain(start..=line_index);
        Ok(())
    }

    pub fn add_assignment(&mut self, key: &str, value: &str, comment: &str) -> Result<(), String> {
        if !valid_key(key) || value.contains(['\n', '\r']) || comment.contains('\r') {
            return Err("Nouvelle variable invalide".into());
        }
        let ending = self.preferred_ending().to_string();
        if self.lines.last().is_some_and(|line| line.ending.is_empty()) {
            if let Some(last) = self.lines.last_mut() {
                last.ending = ending.clone();
            }
        }
        for part in comment.split('\n').filter(|part| !part.is_empty()) {
            self.lines.push(EnvLine {
                body: format!("# {part}"),
                ending: ending.clone(),
                kind: LineKind::Comment,
                key: None,
                value_start: None,
                value_end: None,
                quote: None,
            });
        }
        let prefix = format!("{key}=");
        self.lines.push(EnvLine {
            body: format!("{prefix}{value}"),
            ending: String::new(),
            kind: LineKind::Assignment,
            key: Some(key.to_string()),
            value_start: Some(prefix.len()),
            value_end: Some(prefix.len() + value.len()),
            quote: None,
        });
        Ok(())
    }

    pub fn to_dto(&self) -> ParsedDocumentDto {
        let mut counts = BTreeMap::<String, usize>::new();
        let mut unknown_lines = Vec::new();
        let mut variables = Vec::new();
        for (index, line) in self.lines.iter().enumerate() {
            if line.kind == LineKind::Unknown {
                unknown_lines.push(index + 1);
            }
            if line.kind != LineKind::Assignment {
                continue;
            }
            let key = line.key.clone().unwrap_or_default();
            *counts.entry(key.clone()).or_default() += 1;
            variables.push(VariableDto {
                id: format!("line-{index}"),
                line_index: index,
                sensitive: is_sensitive_key(&key),
                key,
                value: line.value().unwrap_or_default().to_string(),
                comment: self.comments_for(index),
            });
        }
        let duplicate_keys = counts
            .into_iter()
            .filter_map(|(key, count)| (count > 1).then_some(key))
            .collect::<Vec<_>>();
        let mut diagnostics = Vec::new();
        if !unknown_lines.is_empty() {
            diagnostics.push(format!(
                "Lignes non prises en charge, conservées sans modification : {}",
                unknown_lines
                    .iter()
                    .map(usize::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if !duplicate_keys.is_empty() {
            diagnostics.push(format!("Clés dupliquées : {}", duplicate_keys.join(", ")));
        }
        ParsedDocumentDto {
            variables,
            diagnostics,
            unknown_lines,
            duplicate_keys,
        }
    }
}

fn is_sensitive_key(key: &str) -> bool {
    let upper = key.to_ascii_uppercase();
    let patterns = [
        "PASSWORD",
        "PASSWD",
        "SECRET",
        "TOKEN",
        "API_KEY",
        "PRIVATE_KEY",
        "CREDENTIAL",
    ];
    patterns.iter().any(|pattern| upper.contains(pattern))
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &[u8] = include_bytes!("../../legacy/python-gtk/tests/fixtures/sample.env");

    #[test]
    fn fixture_round_trip_is_byte_exact() {
        let document = parse_bytes(FIXTURE).unwrap();
        assert_eq!(document.to_bytes(), FIXTURE);
        let dto = document.to_dto();
        assert_eq!(dto.variables.len(), 3);
        assert_eq!(dto.unknown_lines, vec![8]);
    }

    #[test]
    fn targeted_change_preserves_every_other_byte_and_crlf() {
        let original = b"A=one\r\nB='two # keep'\r\n";
        let mut document = parse_bytes(original).unwrap();
        document.lines[0].set_value("changed").unwrap();
        assert_eq!(document.to_bytes(), b"A=changed\r\nB='two # keep'\r\n");
    }

    #[test]
    fn comments_duplicates_and_secrets_are_reported() {
        let document = parse_bytes(b"# one\n# two\nDB_PASSWORD=x\nA=1\nA=2\n").unwrap();
        let dto = document.to_dto();
        assert_eq!(dto.variables[0].comment, "one\ntwo");
        assert!(dto.variables[0].sensitive);
        assert_eq!(dto.duplicate_keys, vec!["A"]);
    }

    #[test]
    fn invalid_utf8_is_refused() {
        assert!(parse_bytes(&[0xff]).is_err());
    }
}
