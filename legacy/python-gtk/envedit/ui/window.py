from __future__ import annotations

from pathlib import Path

import gi

gi.require_version("Gtk", "4.0")
gi.require_version("Adw", "1")
from gi.repository import Adw, Gio, GLib, Gtk  # noqa: E402

from envedit.model import EnvLine, ParseError
from envedit.projects import Project, ProjectCatalog, discover_env_files
from envedit.security import is_sensitive_key
from envedit.storage import ConflictError, FileSession, StorageError, open_file


class VariableRow(Gtk.ListBoxRow):
    def __init__(self, window: "EnvEditWindow", line: EnvLine) -> None:
        super().__init__(selectable=False, activatable=False)
        self.window = window
        self.line = line
        self.sensitive_value = is_sensitive_key(line.key or "")

        box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8,
                      margin_top=6, margin_bottom=6, margin_start=12, margin_end=12)
        self.key_entry = Gtk.Entry(text=line.key or "", width_chars=22,
                                   tooltip_text="Nom de la variable")
        self.key_entry.add_css_class("flat")
        key_focus = Gtk.EventControllerFocus()
        key_focus.connect("leave", self._rename)
        self.key_entry.add_controller(key_focus)
        self.key_entry.connect("activate", self._rename)
        box.append(self.key_entry)

        self.value_entry = Gtk.Entry(text=line.value, hexpand=True,
                                     visibility=not self.sensitive_value,
                                     tooltip_text="Valeur de la variable")
        self.value_entry.connect("changed", self._value_changed)
        box.append(self.value_entry)

        if self.sensitive_value:
            reveal = Gtk.ToggleButton(icon_name="view-reveal-symbolic",
                                      tooltip_text="Afficher ou masquer la valeur")
            reveal.connect("toggled", lambda button: self.value_entry.set_visibility(button.get_active()))
            box.append(reveal)

        comment = window.session.document.comments_for(line) if window.session else ""
        info = Gtk.Button(icon_name="dialog-information-symbolic",
                          tooltip_text=comment or "Ajouter un commentaire")
        info.set_focusable(True)
        info.connect("clicked", self._show_comment_popover)
        box.append(info)
        self.info_button = info

        delete = Gtk.Button(icon_name="user-trash-symbolic", tooltip_text="Supprimer la variable")
        delete.add_css_class("flat")
        delete.connect("clicked", self._delete)
        box.append(delete)
        self.set_child(box)

    def _value_changed(self, entry: Gtk.Entry) -> None:
        try:
            self.line.set_value(entry.get_text())
            self.window.update_modified_state()
        except ParseError as exc:
            self.window.show_error(str(exc))

    def _rename(self, *_args) -> None:
        try:
            self.line.rename(self.key_entry.get_text())
            self.window.update_modified_state()
            self.window.update_diagnostics()
        except ParseError as exc:
            self.key_entry.set_text(self.line.key or "")
            self.window.show_error(str(exc))

    def _delete(self, _button: Gtk.Button) -> None:
        assert self.window.session is not None
        self.window.session.document.remove(self.line)
        self.window.rebuild_variables()
        self.window.update_modified_state()

    def _show_comment_popover(self, button: Gtk.Button) -> None:
        assert self.window.session is not None
        popover = Gtk.Popover()
        content = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8,
                          margin_top=12, margin_bottom=12, margin_start=12, margin_end=12)
        content.append(Gtk.Label(label="Commentaire", xalign=0))
        view = Gtk.TextView(wrap_mode=Gtk.WrapMode.WORD_CHAR, width_request=360, height_request=120)
        view.get_buffer().set_text(self.window.session.document.comments_for(self.line))
        content.append(view)
        apply_button = Gtk.Button(label="Appliquer")
        apply_button.add_css_class("suggested-action")

        def apply_comment(_clicked):
            buffer = view.get_buffer()
            text = buffer.get_text(buffer.get_start_iter(), buffer.get_end_iter(), False)
            self.window.session.document.set_comments(self.line, text)
            self.info_button.set_tooltip_text(text or "Ajouter un commentaire")
            self.window.update_modified_state()
            popover.popdown()

        apply_button.connect("clicked", apply_comment)
        content.append(apply_button)
        popover.set_child(content)
        popover.set_parent(button)
        popover.connect("closed", lambda widget: widget.unparent())
        popover.popup()


class EnvEditWindow(Adw.ApplicationWindow):
    def __init__(self, application: Adw.Application) -> None:
        super().__init__(application=application, title="EnvEdit", default_width=1100, default_height=720)
        self.catalog = ProjectCatalog()
        self.session: FileSession | None = None
        self.current_path: Path | None = None
        self.pending_path: Path | None = None
        try:
            self.catalog.load()
        except ValueError as exc:
            self.catalog.projects = []
            GLib.idle_add(self.show_error, str(exc))
        self._build_ui()
        self.refresh_projects()
        self._install_actions()

    def _build_ui(self) -> None:
        main = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        header = Adw.HeaderBar()
        title = Adw.WindowTitle(title="EnvEdit", subtitle="Éditeur .env local")
        header.set_title_widget(title)
        header.pack_start(Gtk.Button(icon_name="document-open-symbolic", tooltip_text="Ajouter un projet",
                                     action_name="win.add-project"))
        header.pack_end(Gtk.Button(icon_name="view-refresh-symbolic", tooltip_text="Rafraîchir",
                                   action_name="win.refresh"))
        header.pack_end(Gtk.Button(icon_name="document-save-symbolic", tooltip_text="Enregistrer (Ctrl+S)",
                                   action_name="win.save"))
        main.append(header)

        paned = Gtk.Paned(orientation=Gtk.Orientation.HORIZONTAL, wide_handle=True,
                          position=310, vexpand=True)
        sidebar_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=6)
        sidebar_box.add_css_class("sidebar")
        self.sidebar = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=4,
                               margin_top=8, margin_bottom=8, margin_start=8, margin_end=8)
        side_scroll = Gtk.ScrolledWindow(vexpand=True, child=self.sidebar)
        sidebar_box.append(side_scroll)
        paned.set_start_child(sidebar_box)
        paned.set_resize_start_child(False)
        paned.set_shrink_start_child(False)

        editor = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8)
        heading = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8,
                          margin_top=12, margin_start=12, margin_end=12)
        self.file_title = Gtk.Label(label="Sélectionnez un fichier .env", xalign=0, hexpand=True)
        self.file_title.add_css_class("title-2")
        heading.append(self.file_title)
        add_variable = Gtk.Button(label="Ajouter une variable", icon_name="list-add-symbolic")
        add_variable.connect("clicked", self._add_variable)
        heading.append(add_variable)
        editor.append(heading)
        self.search = Gtk.SearchEntry(placeholder_text="Rechercher une clé ou un commentaire",
                                      margin_start=12, margin_end=12)
        self.search.connect("search-changed", lambda _entry: self.rebuild_variables())
        editor.append(self.search)
        self.diagnostics = Gtk.Label(xalign=0, wrap=True, margin_start=12, margin_end=12)
        self.diagnostics.add_css_class("warning")
        editor.append(self.diagnostics)
        self.variable_list = Gtk.ListBox(selection_mode=Gtk.SelectionMode.NONE)
        self.variable_list.add_css_class("boxed-list")
        variable_scroll = Gtk.ScrolledWindow(vexpand=True, child=self.variable_list,
                                             margin_start=12, margin_end=12, margin_bottom=12)
        editor.append(variable_scroll)
        paned.set_end_child(editor)
        main.append(paned)

        self.toast_overlay = Adw.ToastOverlay(child=main)
        self.set_content(self.toast_overlay)

    def _install_actions(self) -> None:
        for name, callback in (("save", self._save), ("add-project", self._choose_project),
                               ("refresh", self._refresh_action)):
            action = Gio.SimpleAction.new(name, None)
            action.connect("activate", callback)
            self.add_action(action)
        app = self.get_application()
        app.set_accels_for_action("win.save", ["<Control>s"])
        app.set_accels_for_action("win.add-project", ["<Control>o"])
        app.set_accels_for_action("win.focus-search", ["<Control>f"])
        focus = Gio.SimpleAction.new("focus-search", None)
        focus.connect("activate", lambda *_args: self.search.grab_focus())
        self.add_action(focus)

    def refresh_projects(self) -> None:
        while child := self.sidebar.get_first_child():
            self.sidebar.remove(child)
        if not self.catalog.projects:
            self.sidebar.append(Gtk.Label(label="Aucun projet\nAjoutez un dossier avec Ctrl+O.",
                                          justify=Gtk.Justification.CENTER, margin_top=24))
            return
        for project in self.catalog.projects:
            file_list = Gtk.ListBox(selection_mode=Gtk.SelectionMode.NONE)
            file_paths: dict[Gtk.ListBoxRow, Path] = {}
            for path in discover_env_files(project.path):
                row = Gtk.ListBoxRow(activatable=True)
                label = Gtk.Label(label=path.name, xalign=0, margin_start=12, margin_top=6, margin_bottom=6)
                row.set_child(label)
                file_paths[row] = path
                file_list.append(row)

            def open_selected(_list: Gtk.ListBox, row: Gtk.ListBoxRow,
                              paths=file_paths) -> None:
                selected = paths.get(row)
                if selected is not None:
                    self.request_open(selected)

            file_list.connect("row-activated", open_selected)
            project_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=4)
            labels = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, hexpand=True)
            labels.append(Gtk.Label(label=project.name, xalign=0, ellipsize=3))
            labels.append(Gtk.Label(label=str(project.path), xalign=0, ellipsize=3, tooltip_text=str(project.path)))
            project_box.append(labels)
            remove = Gtk.Button(icon_name="list-remove-symbolic", tooltip_text="Retirer de la liste")
            remove.add_css_class("flat")
            remove.connect("clicked", lambda _button, selected=project: self._remove_project(selected))
            project_box.append(remove)
            expander = Gtk.Expander(expanded=True, child=file_list)
            expander.set_label_widget(project_box)
            self.sidebar.append(expander)

    def request_open(self, path: Path) -> None:
        if self.session and self.session.document.modified:
            self.pending_path = path
            dialog = Adw.MessageDialog(transient_for=self, heading="Modifications non enregistrées",
                                       body="Enregistrer avant de changer de fichier ?")
            dialog.add_response("cancel", "Annuler")
            dialog.add_response("discard", "Abandonner")
            dialog.add_response("save", "Enregistrer")
            dialog.set_response_appearance("discard", Adw.ResponseAppearance.DESTRUCTIVE)
            dialog.set_response_appearance("save", Adw.ResponseAppearance.SUGGESTED)
            dialog.set_default_response("save")
            dialog.set_close_response("cancel")
            dialog.connect("response", self._navigation_response)
            dialog.present()
            return
        self.open_path(path)

    def _navigation_response(self, _dialog, response: str) -> None:
        path, self.pending_path = self.pending_path, None
        if response == "save":
            if not self.save_current():
                return
        elif response != "discard":
            return
        if path:
            self.open_path(path)

    def open_path(self, path: Path) -> None:
        try:
            self.session = open_file(path)
            self.current_path = path
            self.file_title.set_label(path.name)
            self.file_title.set_tooltip_text(str(path))
            self.rebuild_variables()
            self.update_modified_state()
            self.update_diagnostics()
        except (StorageError, UnicodeError) as exc:
            self.show_error(str(exc))

    def rebuild_variables(self) -> None:
        while row := self.variable_list.get_first_child():
            self.variable_list.remove(row)
        if not self.session:
            return
        query = self.search.get_text().casefold()
        visible_count = 0
        for line in self.session.document.assignments:
            comments = self.session.document.comments_for(line)
            if query and query not in (line.key or "").casefold() and query not in comments.casefold():
                continue
            self.variable_list.append(VariableRow(self, line))
            visible_count += 1
        if visible_count == 0:
            if query:
                message = "Aucune variable ne correspond à la recherche."
            elif self.session.document.unknown_lines:
                message = "Aucune affectation reconnue. Consultez le diagnostic de syntaxe ci-dessus."
            else:
                message = "Ce fichier ne contient aucune variable reconnue."
            empty = Gtk.ListBoxRow(selectable=False, activatable=False)
            empty.set_child(Gtk.Label(label=message, wrap=True, margin_top=24,
                                      margin_bottom=24, margin_start=12, margin_end=12))
            self.variable_list.append(empty)

    def update_modified_state(self) -> None:
        modified = bool(self.session and self.session.document.modified)
        name = self.current_path.name if self.current_path else "Sélectionnez un fichier .env"
        self.file_title.set_label(("● " if modified else "") + name)

    def update_diagnostics(self) -> None:
        if not self.session:
            self.diagnostics.set_label("")
            return
        messages = []
        if self.session.document.unknown_lines:
            numbers = ", ".join(map(str, self.session.document.unknown_lines))
            messages.append(f"Lignes non prises en charge, conservées sans modification : {numbers}")
        if self.session.document.duplicate_keys:
            messages.append("Clés dupliquées : " + ", ".join(sorted(self.session.document.duplicate_keys)))
        messages.append(f"{len(self.session.document.assignments)} variable(s) reconnue(s)")
        self.diagnostics.set_label("\n".join(messages))

    def save_current(self) -> bool:
        if not self.session:
            return True
        try:
            self.session.save()
            self.update_modified_state()
            self.toast_overlay.add_toast(Adw.Toast(title="Fichier enregistré"))
            return True
        except ConflictError as exc:
            self.show_error(f"Conflit : {exc}. Rechargez le fichier avant de réessayer.")
        except StorageError as exc:
            self.show_error(str(exc))
        return False

    def _save(self, *_args) -> None:
        self.save_current()

    def _add_variable(self, _button) -> None:
        if not self.session:
            self.show_error("Sélectionnez d'abord un fichier .env")
            return
        existing = {line.key for line in self.session.document.assignments}
        index = 1
        while f"NOUVELLE_VARIABLE_{index}" in existing:
            index += 1
        self.session.document.add(f"NOUVELLE_VARIABLE_{index}")
        self.rebuild_variables()
        self.update_modified_state()

    def _choose_project(self, *_args) -> None:
        dialog = Gtk.FileDialog(title="Ajouter un dossier projet", modal=True)
        dialog.select_folder(self, None, self._project_chosen)

    def _project_chosen(self, dialog: Gtk.FileDialog, result) -> None:
        try:
            folder = dialog.select_folder_finish(result)
            if folder and folder.get_path():
                self.catalog.add(Path(folder.get_path()))
                self.refresh_projects()
        except GLib.Error as exc:
            if not exc.matches(Gtk.DialogError.quark(), Gtk.DialogError.DISMISSED):
                self.show_error(str(exc))
        except (OSError, ValueError) as exc:
            self.show_error(str(exc))

    def _remove_project(self, project: Project) -> None:
        try:
            self.catalog.remove(project)
            self.refresh_projects()
            self.toast_overlay.add_toast(Adw.Toast(title="Projet retiré de la liste (fichiers conservés)"))
        except (OSError, ValueError) as exc:
            self.show_error(str(exc))

    def _refresh_action(self, *_args) -> None:
        self.refresh_projects()
        self.toast_overlay.add_toast(Adw.Toast(title="Liste des fichiers actualisée"))

    def show_error(self, message: str) -> bool:
        self.toast_overlay.add_toast(Adw.Toast(title=message, timeout=5))
        return False
