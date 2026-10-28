from __future__ import annotations

import sys

import gi

gi.require_version("Adw", "1")
from gi.repository import Adw, Gio  # noqa: E402

from envedit.ui import EnvEditWindow


class EnvEditApplication(Adw.Application):
    def __init__(self) -> None:
        super().__init__(application_id="io.github.envedit.EnvEdit",
                         flags=Gio.ApplicationFlags.DEFAULT_FLAGS)

    def do_activate(self) -> None:
        window = self.get_active_window()
        if window is None:
            window = EnvEditWindow(self)
        window.present()


def main() -> int:
    app = EnvEditApplication()
    return app.run(sys.argv)


if __name__ == "__main__":
    raise SystemExit(main())
