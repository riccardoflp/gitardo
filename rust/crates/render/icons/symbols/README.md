# The icons of Symbols

The icons Gitardo Desktop draws beside the names of files and folders (`file-view.file-icons`), from
[Symbols](https://github.com/miguelsolorio/vscode-symbols), the icon theme of VS Code by Miguel Solorio, under the MIT
licence kept in [LICENSE](LICENSE).

Copied from the repository at commit `a405d32cca082d838ad7739b6d0044dea9dbbb79` (4 November 2026):

- `files/` and `folders/`: the SVG files of `src/icons/files/` and `src/icons/folders/` that its theme uses, 261 and
  104, unchanged but for their line endings;
- `theme.txt`: what `src/symbol-icon-theme.json` maps to them, the extensions, the names of files, the names of folders
  and the languages of VS Code, and the icons of a file, a folder and a top folder that nothing else names, written one
  entry a line so that it is read without a parser of JSON. An entry names the file of its icon, where the theme names
  a definition that points to it.

Nothing else of the project is copied. Their colors are not drawn as written: each is taken to the color of its family
in the theme of the window, as the card says.
