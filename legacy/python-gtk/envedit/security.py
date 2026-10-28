import re

_SENSITIVE = re.compile(
    r"(?:PASSWORD|PASSWD|SECRET|TOKEN|API_KEY|PRIVATE_KEY|CREDENTIAL)", re.IGNORECASE
)


def is_sensitive_key(key: str) -> bool:
    """Détermine le masquage depuis le nom, sans jamais inspecter la valeur."""
    return bool(_SENSITIVE.search(key))
