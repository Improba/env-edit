from __future__ import annotations

from dataclasses import dataclass
import hashlib
import os
from pathlib import Path
import stat
import tempfile

from envedit.model import Document
from envedit.parser import parse_bytes


class StorageError(OSError):
    pass


class ConflictError(StorageError):
    pass


def _fingerprint(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


@dataclass
class FileSession:
    path: Path
    document: Document
    original_size: int
    original_mtime_ns: int
    original_hash: str
    mode: int

    def save(self) -> None:
        if self.path.is_symlink():
            raise StorageError("La sauvegarde via un lien symbolique est désactivée par sécurité")
        try:
            current_stat = self.path.stat()
            current = self.path.read_bytes()
        except OSError as exc:
            raise StorageError(f"Impossible de vérifier le fichier avant sauvegarde : {exc}") from exc
        if (current_stat.st_size != self.original_size
                or current_stat.st_mtime_ns != self.original_mtime_ns
                or _fingerprint(current) != self.original_hash):
            raise ConflictError("Le fichier a été modifié sur le disque depuis son ouverture")

        data = self.document.to_bytes()
        directory = self.path.parent
        descriptor = -1
        temporary: str | None = None
        try:
            descriptor, temporary = tempfile.mkstemp(prefix=f".{self.path.name}.", suffix=".tmp", dir=directory)
            os.fchmod(descriptor, self.mode)
            with os.fdopen(descriptor, "wb") as stream:
                descriptor = -1
                stream.write(data)
                stream.flush()
                os.fsync(stream.fileno())
            os.replace(temporary, self.path)
            temporary = None
            directory_fd = os.open(directory, os.O_RDONLY)
            try:
                os.fsync(directory_fd)
            finally:
                os.close(directory_fd)
        except OSError as exc:
            raise StorageError(f"Échec de la sauvegarde atomique : {exc}") from exc
        finally:
            if descriptor >= 0:
                os.close(descriptor)
            if temporary is not None:
                try:
                    os.unlink(temporary)
                except FileNotFoundError:
                    pass

        saved_stat = self.path.stat()
        self.original_size = saved_stat.st_size
        self.original_mtime_ns = saved_stat.st_mtime_ns
        self.original_hash = _fingerprint(data)
        self.document.mark_saved()


def open_file(path: str | Path) -> FileSession:
    resolved = Path(path)
    if resolved.is_symlink():
        raise StorageError("Les liens symboliques sont ouverts en lecture seule par sécurité")
    try:
        info = resolved.stat()
        if not stat.S_ISREG(info.st_mode):
            raise StorageError("Le chemin sélectionné n'est pas un fichier ordinaire")
        data = resolved.read_bytes()
        document = parse_bytes(data)
    except (OSError, UnicodeError) as exc:
        if isinstance(exc, StorageError):
            raise
        raise StorageError(f"Impossible d'ouvrir le fichier : {exc}") from exc
    return FileSession(resolved, document, info.st_size, info.st_mtime_ns,
                       _fingerprint(data), stat.S_IMODE(info.st_mode))
