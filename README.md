# <img src="assets/icons/icon.svg" alt="" width="40" height="40" align="center"> TrayReveal

<!-- web:skip -->
[English](#english) · [Deutsch](#deutsch)
<!-- /web:skip -->

## English

A tiny tray app for Windows 11 that keeps your notification area icons visible.
Windows 11 hides every new tray icon behind the `^` overflow menu and no longer offers the old "Always show all icons" checkbox. TrayReveal brings that convenience back: it watches the icon list of the current user and shows new icons right away – or all of them, if you prefer.

> Feedback and bug reports are welcome as [issues](../../issues). Not affiliated with or endorsed by Microsoft. Windows is a trademark of the Microsoft group of companies.

### Features
- Shows every new tray icon right away, or keeps all icons visible
- Exceptions for apps whose icons should always stay hidden
- Keeps your choice when an app updates into a new versioned folder
- Runs as a standard user, no administrator rights, writes only to `HKCU`
- No window, only a tray icon with a context menu
- No telemetry; the only network access is an optional update check (see [Privacy](#privacy))

### Installation
Download one of the files from [Releases](../../releases):

- **Installer** `TrayReveal-<version>-windows-x64-setup.exe` – installs for the current user (default, no administrator rights) or for all users, adds a Start menu entry and optionally starts TrayReveal with Windows. It can be removed under Settings → Apps.
- **Portable** `TrayReveal-<version>-windows-x64.zip` – extract it to any folder and start `TrayReveal.exe`. Keep the `lang` folder next to the program.

The program is not signed, so SmartScreen may warn on the first start: "More info" → "Run anyway". Each file has a `.sha256` file to verify the download, and the release notes link a VirusTotal scan. Changes are listed in the [changelog](CHANGELOG.md).

#### Installer parameters
The installer accepts the usual Inno Setup parameters, e.g. for a silent installation:

| Parameter | Effect |
|---|---|
| `/CURRENTUSER` | Install for the current user only (default) |
| `/ALLUSERS` | Install for all users (asks for administrator rights) |
| `/DIR="C:\Path"` | Installation folder |
| `/SILENT`, `/VERYSILENT` | Without wizard (with or without progress window) |
| `/SUPPRESSMSGBOXES` | No message boxes in silent mode |
| `/TASKS="autostart,desktopicon"` | Selected options; `autostart` starts TrayReveal with Windows for the installing user, `desktopicon` adds a desktop shortcut. Without the parameter, autostart is on and the desktop shortcut off |
| `/NOICONS` | No Start menu entry |
| `/LOG="C:\Path\setup.log"` | Write a log file |

Example: `TrayReveal-1.1.0-windows-x64-setup.exe /VERYSILENT /SUPPRESSMSGBOXES /ALLUSERS /TASKS="autostart"`

Settings such as the mode or the language are stored per user and are not set by the installer. Uninstalling removes the program and the autostart entry but keeps the settings in `HKCU\Software\TrayReveal`.

### Usage
Start `TrayReveal.exe`. On the first start it only remembers the icons that already exist and changes nothing. Right-click (or left-click) its tray icon for the menu:

![The TrayReveal context menu](docs/menu.png)

- **Off** – watch only, change nothing
- **Show new icons** (default) – every new icon is shown once; if you hide it later in the Windows Settings, it stays hidden
- **Always show all icons** – keeps every icon visible, except the exceptions
- **Show all icons now / Hide all icons now** – one-time actions; hiding is not available while all icons are always shown
- **Exceptions** – lists the apps with their icons; apps marked with a red cross always stay hidden, clicking them again shows them again (except in Off mode)
- **Undo last change** – reverts the last action from the menu (mode, show/hide all, exceptions) while TrayReveal is running
- **Check for updates** – checks once a day whether a new version exists and announces it with a notification and the menu item **Download version …**
- **Notify about new icons**, **Language**, **Start with Windows**, **Quit**

The menu follows the light or dark mode of the taskbar.

When an app updates into a new versioned folder (for example `app-1.2.3` or a Microsoft Store package), Windows creates a new hidden entry for it. TrayReveal recognizes the same app and keeps your previous choice. Exceptions apply to all versions of an app. If several older versions exist, the one changed last counts.

Entries whose program has been missing for three days in a row are removed from the Windows tray settings, so old versions do not pile up. In Off mode nothing is removed.

`TrayReveal.exe --once` applies the current settings once without a tray icon and exits, e.g. for the Task Scheduler.

#### Languages
TrayReveal comes in 51 languages, including German dialects and a few fun languages; [LANGUAGES.md](LANGUAGES.md) lists all codes. With **Automatic**, it follows the Windows display language and falls back to English.

The translations were generated by Claude (AI). Most of them have not been checked by a native speaker yet; [LANGUAGES.md](LANGUAGES.md) and the header of each file show which ones have. If you notice a mistake or know a better wording, please open an [issue](../../issues) or a pull request. Reviewers are credited with their GitHub name in the language file and in [LANGUAGES.md](LANGUAGES.md), if they agree.

The texts are plain text files named after the language code, e.g. `de.lang` or `pt-BR.lang`, with one `key = text` per line. To add a language, copy [`lang/en.lang`](lang/en.lang), translate the texts after the `=` and keep the keys and placeholders such as `{name}`. Missing texts are taken from the fallback language (`language.fallback`), otherwise from English.

Language files are read from the `lang` folder next to the program and from `%APPDATA%\TrayReveal\lang`. The second folder needs no administrator rights and its texts take priority, so you can add a language or change single texts there.

#### Updates
With **Check for updates** turned on, TrayReveal checks shortly after the start and then once a day whether a new version exists and announces it with a notification and the menu item **Download version …**. Updates are never downloaded or installed automatically, the menu only links to the release page.

### How it works
Windows stores the visibility of each tray icon per user under `HKCU\Control Panel\NotifyIconSettings`, one subkey per icon. The value `IsPromoted` decides whether the icon is shown on the taskbar (`1`) or hidden in the overflow menu (`0`); for icons that were never shown, the value is missing.
TrayReveal waits for changes to this key (no polling), sets or creates `IsPromoted` according to the chosen mode and leaves everything else untouched. It is the same value the Windows Settings toggle writes, so you can still change single icons there.

The registry key is not documented by Microsoft and may change with future Windows updates. If its layout changes, TrayReveal does nothing instead of guessing.

### Privacy
- The only network access is the optional update check: shortly after the start and then once a day, TrayReveal requests the information about the latest release from `api.github.com`. It only downloads this information; nothing about your system or your icons is sent or uploaded, and GitHub only sees the usual request data such as the IP address and the program name with version (`TrayReveal/<version>`). It can be turned off with **Check for updates** in the menu. Updates are never downloaded or installed automatically, the menu only links to the release page.
- No telemetry
- Settings are stored in the registry under `HKCU\Software\TrayReveal`

<!-- web:skip -->

### Building
TrayReveal is written in Rust and only depends on the official Windows API bindings.

```
cargo build --release
```

For tests, `cargo run --example tray_probe -- "Probe" 60` shows a test tray icon for 60 seconds. Copied into folders such as `app-1.0.0` and `app-1.0.1`, it simulates app updates.

The icons in [`assets/icons`](assets/icons) (ICO and PNG in all common sizes) are generated from `icon.svg` with [`render-icons.mjs`](https://github.com/TripleT3005/TripleT3005/blob/main/scripts/render-icons.mjs), which needs Node.js and Edge or Chrome: `node render-icons.mjs assets/icons`. The program, the setup and the release announcement take their icon from there.

The license list for releases is created with Python 3 (standard library only): `python scripts/third_party_licenses.py --rust-std --output THIRD-PARTY-LICENSES.txt`. The installer needs this file. `python -m unittest discover -s scripts` tests the script.

A release is created automatically when a new version in `Cargo.toml` reaches `main`. Each version needs a section with English and German notes in [CHANGELOG.md](CHANGELOG.md), which becomes the release notes; tests and pipeline fail without it.

<!-- /web:skip -->

### License
[MIT](LICENSE)

Release packages and the installer include `THIRD-PARTY-LICENSES.txt` with the licenses of all third-party components built into TrayReveal. The CI fails if a dependency uses a license that is not on the allow list in [`scripts/third_party_licenses.py`](scripts/third_party_licenses.py).

## Deutsch

Eine kleine Tray-App für Windows 11, die die Symbole im Infobereich sichtbar hält.
Windows 11 versteckt jedes neue Tray-Icon hinter dem `^`-Überlaufmenü und bietet das frühere Häkchen „Immer alle Symbole anzeigen“ nicht mehr an. TrayReveal bringt diesen Komfort zurück: Es beobachtet die Icon-Liste des aktuellen Benutzers und blendet neue Icons sofort ein – oder auf Wunsch alle.

> Rückmeldungen und Fehlerberichte gern als [Issue](../../issues). Keine Verbindung zu Microsoft und nicht von Microsoft unterstützt. Windows ist eine Marke der Microsoft-Unternehmensgruppe.

### Funktionen
- Blendet jedes neue Tray-Icon sofort ein oder hält alle Icons sichtbar
- Ausnahmen für Apps, deren Icons immer ausgeblendet bleiben sollen
- Übernimmt deine Wahl, wenn sich eine App in einen neuen Ordner mit Versionsnummer aktualisiert
- Läuft als Standardbenutzer, ohne Administratorrechte, schreibt nur in `HKCU`
- Kein Fenster, nur ein Tray-Icon mit Kontextmenü
- Keine Telemetrie; einziger Netzwerkzugriff ist eine abschaltbare Update-Prüfung (siehe [Datenschutz](#datenschutz))

### Installation
Eine der Dateien unter [Releases](../../releases) herunterladen:

- **Installer** `TrayReveal-<Version>-windows-x64-setup.exe` – installiert für den aktuellen Benutzer (Standard, ohne Administratorrechte) oder für alle Benutzer, legt einen Startmenü-Eintrag an und startet TrayReveal auf Wunsch mit Windows. Entfernen lässt es sich unter Einstellungen → Apps.
- **Portabel** `TrayReveal-<Version>-windows-x64.zip` – in einen beliebigen Ordner entpacken und `TrayReveal.exe` starten. Der Ordner `lang` bleibt neben dem Programm.

Das Programm ist nicht signiert, deshalb warnt SmartScreen beim ersten Start eventuell: „Weitere Informationen“ → „Trotzdem ausführen“. Zu jeder Datei gibt es eine `.sha256`-Datei zum Prüfen des Downloads, und die Release-Notizen verlinken einen VirusTotal-Scan. Änderungen stehen im [Changelog](CHANGELOG.md).

#### Installer-Parameter
Der Installer versteht die üblichen Inno-Setup-Parameter, z. B. für eine stille Installation:

| Parameter | Wirkung |
|---|---|
| `/CURRENTUSER` | Nur für den aktuellen Benutzer installieren (Standard) |
| `/ALLUSERS` | Für alle Benutzer installieren (fragt nach Administratorrechten) |
| `/DIR="C:\Pfad"` | Installationsordner |
| `/SILENT`, `/VERYSILENT` | Ohne Assistent (mit oder ohne Fortschrittsfenster) |
| `/SUPPRESSMSGBOXES` | Keine Meldungsfenster bei stiller Installation |
| `/TASKS="autostart,desktopicon"` | Gewählte Optionen; `autostart` startet TrayReveal für den installierenden Benutzer mit Windows, `desktopicon` legt eine Desktop-Verknüpfung an. Ohne den Parameter ist der Autostart an und die Desktop-Verknüpfung aus |
| `/NOICONS` | Kein Startmenü-Eintrag |
| `/LOG="C:\Pfad\setup.log"` | Protokolldatei schreiben |

Beispiel: `TrayReveal-1.1.0-windows-x64-setup.exe /VERYSILENT /SUPPRESSMSGBOXES /ALLUSERS /TASKS="autostart"`

Einstellungen wie Modus oder Sprache gelten pro Benutzer und werden vom Installer nicht gesetzt. Die Deinstallation entfernt das Programm und den Autostart-Eintrag, lässt die Einstellungen in `HKCU\Software\TrayReveal` aber stehen.

### Bedienung
`TrayReveal.exe` starten. Beim ersten Start merkt es sich nur die bereits vorhandenen Icons und ändert nichts. Ein Rechtsklick (oder Linksklick) auf das Tray-Icon öffnet das Menü:

![Das Kontextmenü von TrayReveal](docs/menu-de.png)

- **Aus** – nur beobachten, nichts ändern
- **Neue Icons einblenden** (Standard) – jedes neue Icon wird einmal eingeblendet; blendest du es später in den Windows-Einstellungen aus, bleibt es ausgeblendet
- **Immer alle Icons einblenden** – hält alle Icons sichtbar, außer den Ausnahmen
- **Jetzt alle Icons einblenden / ausblenden** – einmalige Aktionen; Ausblenden ist nicht verfügbar, solange immer alle Icons eingeblendet werden
- **Ausnahmen** – listet die Apps mit ihren Icons; mit rotem X markierte Apps bleiben immer ausgeblendet, ein erneuter Klick blendet sie wieder ein (außer im Modus „Aus“)
- **Letzte Änderung rückgängig machen** – nimmt die letzte Aktion aus dem Menü zurück (Modus, Alle ein-/ausblenden, Ausnahmen), solange TrayReveal läuft
- **Nach Updates suchen** – prüft einmal täglich, ob es eine neue Version gibt, und meldet sie mit einem Hinweis und dem Menüpunkt **Version … herunterladen**
- **Hinweis bei neuen Icons**, **Sprache**, **Mit Windows starten**, **Beenden**

Das Menü folgt dem hellen oder dunklen Modus der Taskleiste.

Wenn sich eine App in einen neuen Ordner mit Versionsnummer aktualisiert (zum Beispiel `app-1.2.3` oder ein Microsoft-Store-Paket), legt Windows dafür einen neuen, ausgeblendeten Eintrag an. TrayReveal erkennt die gleiche App und übernimmt deine bisherige Wahl. Ausnahmen gelten für alle Versionen einer App. Gibt es mehrere ältere Versionen, zählt die zuletzt geänderte.

Einträge, deren Programm drei Tage am Stück fehlt, werden aus den Tray-Einstellungen von Windows entfernt, damit sich alte Versionen nicht ansammeln. Im Modus „Aus“ wird nichts entfernt.

`TrayReveal.exe --once` wendet die aktuellen Einstellungen einmal ohne Tray-Icon an und beendet sich, etwa für die Aufgabenplanung.

#### Sprachen
TrayReveal gibt es in 51 Sprachen, darunter deutsche Dialekte und einige Spaßsprachen; [LANGUAGES.md](LANGUAGES.md) listet alle Codes. Mit **Automatisch** folgt es der Anzeigesprache von Windows und fällt sonst auf Englisch zurück.

Die Übersetzungen wurden von Claude (KI) erzeugt. Die meisten hat noch keine Muttersprachlerin und kein Muttersprachler geprüft; welche geprüft sind, zeigen [LANGUAGES.md](LANGUAGES.md) und der Kopf jeder Datei. Wenn dir ein Fehler auffällt oder du eine bessere Formulierung kennst, eröffne gern ein [Issue](../../issues) oder einen Pull Request. Mit Einverständnis wird die prüfende Person mit ihrem GitHub-Namen in der Sprachdatei und in [LANGUAGES.md](LANGUAGES.md) genannt.

Die Texte liegen in einfachen Textdateien, benannt nach dem Sprachcode, z. B. `de.lang` oder `pt-BR.lang`, mit einem `Schlüssel = Text` pro Zeile. Für eine neue Sprache kopierst du [`lang/en.lang`](lang/en.lang), übersetzt die Texte nach dem `=` und lässt die Schlüssel und Platzhalter wie `{name}` unverändert. Fehlende Texte kommen aus der Rückfallsprache (`language.fallback`), sonst aus Englisch.

Sprachdateien werden aus dem Ordner `lang` neben dem Programm und aus `%APPDATA%\TrayReveal\lang` gelesen. Der zweite Ordner braucht keine Administratorrechte und seine Texte haben Vorrang, dort kannst du also eine Sprache ergänzen oder einzelne Texte ändern.

#### Updates
Ist **Nach Updates suchen** eingeschaltet, prüft TrayReveal kurz nach dem Start und danach einmal täglich, ob es eine neue Version gibt, und meldet sie mit einem Hinweis und dem Menüpunkt **Version … herunterladen**. Updates werden nie automatisch heruntergeladen oder installiert, das Menü verlinkt nur auf die Release-Seite.

### Funktionsweise
Windows speichert die Sichtbarkeit jedes Tray-Icons pro Benutzer unter `HKCU\Control Panel\NotifyIconSettings`, ein Unterschlüssel pro Icon. Der Wert `IsPromoted` legt fest, ob das Icon in der Taskleiste angezeigt (`1`) oder im Überlaufmenü versteckt wird (`0`); bei Icons, die noch nie angezeigt wurden, fehlt der Wert.
TrayReveal wartet auf Änderungen an diesem Schlüssel (ohne regelmäßiges Abfragen), setzt oder erzeugt `IsPromoted` je nach gewähltem Modus und lässt alles andere unverändert. Es ist derselbe Wert, den der Schalter in den Windows-Einstellungen schreibt, einzelne Icons lassen sich dort also weiterhin ändern.

Der Registry-Schlüssel ist von Microsoft nicht dokumentiert und kann sich mit künftigen Windows-Updates ändern. Ändert sich sein Aufbau, tut TrayReveal nichts, statt zu raten.

### Datenschutz
- Netzwerkzugriff gibt es nur für die abschaltbare Update-Prüfung: Kurz nach dem Start und danach einmal täglich ruft TrayReveal die Angaben zum neuesten Release von `api.github.com` ab. Dabei wird nur abgerufen, nichts über dein System oder deine Icons gesendet oder hochgeladen; GitHub sieht nur die üblichen Anfragedaten wie die IP-Adresse und den Programmnamen mit Version (`TrayReveal/<version>`). Abschalten lässt sie sich im Menü mit **Nach Updates suchen**. Updates werden nie automatisch heruntergeladen oder installiert, das Menü verlinkt nur auf die Release-Seite.
- Keine Telemetrie
- Einstellungen liegen in der Registry unter `HKCU\Software\TrayReveal`

<!-- web:skip -->

### Bauen
TrayReveal ist in Rust geschrieben und hängt nur von den offiziellen Windows-API-Bindings ab.

```
cargo build --release
```

Für Tests zeigt `cargo run --example tray_probe -- "Probe" 60` für 60 Sekunden ein Test-Tray-Icon an. In Ordner wie `app-1.0.0` und `app-1.0.1` kopiert, simuliert es App-Updates.

Die Icons in [`assets/icons`](assets/icons) (ICO und PNG in allen gängigen Größen) entstehen aus `icon.svg` mit [`render-icons.mjs`](https://github.com/TripleT3005/TripleT3005/blob/main/scripts/render-icons.mjs), das Node.js und Edge oder Chrome braucht: `node render-icons.mjs assets/icons`. Programm, Setup und Release-Ankündigung nehmen ihr Icon von dort.

Die Lizenzliste für Releases entsteht mit Python 3 (nur Standardbibliothek): `python scripts/third_party_licenses.py --rust-std --output THIRD-PARTY-LICENSES.txt`. Der Installer braucht diese Datei. `python -m unittest discover -s scripts` testet das Skript.

Ein Release entsteht automatisch, sobald eine neue Version in `Cargo.toml` auf `main` landet. Jede Version braucht einen Abschnitt mit englischen und deutschen Notizen in [CHANGELOG.md](CHANGELOG.md), der zu den Release-Notizen wird; ohne ihn schlagen Tests und Pipeline fehl.

<!-- /web:skip -->

### Lizenz
[MIT](LICENSE)

Release-Pakete und Installer enthalten `THIRD-PARTY-LICENSES.txt` mit den Lizenzen aller Fremdkomponenten, die in TrayReveal eingebaut sind. Die CI schlägt fehl, wenn eine Abhängigkeit eine Lizenz verwendet, die nicht auf der Liste erlaubter Lizenzen in [`scripts/third_party_licenses.py`](scripts/third_party_licenses.py) steht.
