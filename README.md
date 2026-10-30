# TrayReveal

[English](#english) · [Deutsch](#deutsch)

## English

A tiny tray app for Windows 11 that keeps your notification area icons visible.
Windows 11 hides every new tray icon behind the `^` overflow menu and no longer offers the old "Always show all icons" checkbox. TrayReveal brings that convenience back: it watches the icon list of the current user and shows new icons right away – or all of them, if you prefer.

> **Status:** early development; versions `0.x` are pre-releases for testing.

### Installation
1. Download `TrayReveal-<version>-windows-x64.zip` from [Releases](../../releases).
2. Extract it, e.g. to `C:\Program Files\TrayReveal` (needs administrator rights once) or to any folder of your choice. Keep the `lang` folder next to `TrayReveal.exe`.
3. Start `TrayReveal.exe`. The program is not signed, so SmartScreen may warn on the first start: "More info" → "Run anyway".

Each archive has a `.sha256` file to verify the download, and the release notes link a VirusTotal scan.

### Usage
Start `TrayReveal.exe`. On the first start it only remembers the icons that already exist and changes nothing. Right-click (or left-click) its tray icon for the menu:

- **Off** – watch only, change nothing
- **Show new icons** (default) – every new icon is shown once; if you hide it later in the Windows Settings, it stays hidden
- **Always show all icons** – keeps every icon visible, except the exceptions
- **Show all icons now / Hide all icons now** – one-time actions; hiding is not available while all icons are always shown
- **Exceptions** – checked apps always stay hidden
- **Notify about new icons**, **Language**, **Start with Windows**, **Quit**

When an app updates into a new versioned folder (for example `app-1.2.3` or a Microsoft Store package), Windows creates a new hidden entry for it. TrayReveal recognizes the same app and keeps your previous choice. Exceptions apply to all versions of an app.

`TrayReveal.exe --once` applies the current settings once without a tray icon and exits, e.g. for the Task Scheduler.

### Translations
The texts are plain text files named after the language code, e.g. `de.lang` or `pt-BR.lang`, with one `key = text` per line. Copy [`lang/en.lang`](lang/en.lang), translate the texts after the `=` and keep the keys and placeholders such as `{name}`. Missing texts are shown in English.

Language files are read from the `lang` folder next to the program and from `%APPDATA%\TrayReveal\lang`. The second folder needs no administrator rights and its texts take priority, so you can add a language or change single texts there. Translations for this repository are welcome as a pull request.

### How it works
Windows stores the visibility of each tray icon per user under `HKCU\Control Panel\NotifyIconSettings`, one subkey per icon. The value `IsPromoted` decides whether the icon is shown on the taskbar (`1`) or hidden in the overflow menu (`0`); for icons that were never shown, the value is missing.
TrayReveal waits for changes to this key (no polling), sets or creates `IsPromoted` according to the chosen mode and leaves everything else untouched. It is the same value the Windows Settings toggle writes, so you can still change single icons there.

- Runs as a standard user, no administrator rights, writes only to `HKCU`
- No window, only a tray icon with a context menu
- No network access, no telemetry
- Settings are stored in the registry under `HKCU\Software\TrayReveal`

The registry key is not documented by Microsoft and may change with future Windows updates. If its layout changes, TrayReveal does nothing instead of guessing.

### Building
TrayReveal is written in Rust and only depends on the official Windows API bindings.

```
cargo build --release
```

A release is created automatically when a new version in `Cargo.toml` reaches `main`.

### License
[MIT](LICENSE) – not affiliated with or endorsed by Microsoft. Windows is a trademark of the Microsoft group of companies.

## Deutsch

Eine kleine Tray-App für Windows 11, die die Symbole im Infobereich sichtbar hält.
Windows 11 versteckt jedes neue Tray-Icon hinter dem `^`-Überlaufmenü und bietet das frühere Häkchen „Immer alle Symbole anzeigen“ nicht mehr an. TrayReveal bringt diesen Komfort zurück: Es beobachtet die Icon-Liste des aktuellen Benutzers und blendet neue Icons sofort ein – oder auf Wunsch alle.

> **Stand:** frühe Entwicklung; Versionen `0.x` sind Vorabversionen zum Testen.

### Installation
1. `TrayReveal-<Version>-windows-x64.zip` unter [Releases](../../releases) herunterladen.
2. Entpacken, z. B. nach `C:\Program Files\TrayReveal` (braucht einmalig Administratorrechte) oder in einen beliebigen Ordner. Der Ordner `lang` bleibt neben `TrayReveal.exe`.
3. `TrayReveal.exe` starten. Das Programm ist nicht signiert, deshalb warnt SmartScreen beim ersten Start eventuell: „Weitere Informationen“ → „Trotzdem ausführen“.

Zu jedem Archiv gibt es eine `.sha256`-Datei zum Prüfen des Downloads, und die Release-Notizen verlinken einen VirusTotal-Scan.

### Bedienung
`TrayReveal.exe` starten. Beim ersten Start merkt es sich nur die bereits vorhandenen Icons und ändert nichts. Ein Rechtsklick (oder Linksklick) auf das Tray-Icon öffnet das Menü:

- **Aus** – nur beobachten, nichts ändern
- **Neue Icons einblenden** (Standard) – jedes neue Icon wird einmal eingeblendet; blendest du es später in den Windows-Einstellungen aus, bleibt es ausgeblendet
- **Immer alle Icons einblenden** – hält alle Icons sichtbar, außer den Ausnahmen
- **Jetzt alle Icons einblenden / ausblenden** – einmalige Aktionen; Ausblenden ist nicht verfügbar, solange immer alle Icons eingeblendet werden
- **Ausnahmen** – angehakte Apps bleiben immer ausgeblendet
- **Hinweis bei neuen Icons**, **Sprache**, **Mit Windows starten**, **Beenden**

Wenn sich eine App in einen neuen Ordner mit Versionsnummer aktualisiert (zum Beispiel `app-1.2.3` oder ein Microsoft-Store-Paket), legt Windows dafür einen neuen, ausgeblendeten Eintrag an. TrayReveal erkennt die gleiche App und übernimmt deine bisherige Wahl. Ausnahmen gelten für alle Versionen einer App.

`TrayReveal.exe --once` wendet die aktuellen Einstellungen einmal ohne Tray-Icon an und beendet sich, etwa für die Aufgabenplanung.

### Übersetzungen
Die Texte liegen in einfachen Textdateien, benannt nach dem Sprachcode, z. B. `de.lang` oder `pt-BR.lang`, mit einem `Schlüssel = Text` pro Zeile. Kopiere [`lang/en.lang`](lang/en.lang), übersetze die Texte nach dem `=` und lass die Schlüssel und Platzhalter wie `{name}` unverändert. Fehlende Texte werden auf Englisch angezeigt.

Sprachdateien werden aus dem Ordner `lang` neben dem Programm und aus `%APPDATA%\TrayReveal\lang` gelesen. Der zweite Ordner braucht keine Administratorrechte und seine Texte haben Vorrang, dort kannst du also eine Sprache ergänzen oder einzelne Texte ändern. Übersetzungen für dieses Repository sind als Pull Request willkommen.

### Funktionsweise
Windows speichert die Sichtbarkeit jedes Tray-Icons pro Benutzer unter `HKCU\Control Panel\NotifyIconSettings`, ein Unterschlüssel pro Icon. Der Wert `IsPromoted` legt fest, ob das Icon in der Taskleiste angezeigt (`1`) oder im Überlaufmenü versteckt wird (`0`); bei Icons, die noch nie angezeigt wurden, fehlt der Wert.
TrayReveal wartet auf Änderungen an diesem Schlüssel (ohne regelmäßiges Abfragen), setzt oder erzeugt `IsPromoted` je nach gewähltem Modus und lässt alles andere unverändert. Es ist derselbe Wert, den der Schalter in den Windows-Einstellungen schreibt, einzelne Icons lassen sich dort also weiterhin ändern.

- Läuft als Standardbenutzer, ohne Administratorrechte, schreibt nur in `HKCU`
- Kein Fenster, nur ein Tray-Icon mit Kontextmenü
- Kein Netzwerkzugriff, keine Telemetrie
- Einstellungen liegen in der Registry unter `HKCU\Software\TrayReveal`

Der Registry-Schlüssel ist von Microsoft nicht dokumentiert und kann sich mit künftigen Windows-Updates ändern. Ändert sich sein Aufbau, tut TrayReveal nichts, statt zu raten.

### Bauen
TrayReveal ist in Rust geschrieben und hängt nur von den offiziellen Windows-API-Bindings ab.

```
cargo build --release
```

Ein Release entsteht automatisch, sobald eine neue Version in `Cargo.toml` auf `main` landet.

### Lizenz
[MIT](LICENSE) – keine Verbindung zu Microsoft und nicht von Microsoft unterstützt. Windows ist eine Marke der Microsoft-Unternehmensgruppe.
