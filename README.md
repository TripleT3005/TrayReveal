# TrayReveal

[English](#english) · [Deutsch](#deutsch)

## English

A tiny tray app for Windows 11 that keeps your notification area icons visible.
Windows 11 hides every new tray icon behind the `^` overflow menu and no longer offers the old "Always show all icons" checkbox. TrayReveal brings that convenience back: it watches the icon list of the current user and shows new icons right away – or all of them, if you prefer.

> **Status:** early development, there is no release yet.

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

### License
[MIT](LICENSE) – not affiliated with or endorsed by Microsoft. Windows is a trademark of the Microsoft group of companies.

## Deutsch

Eine kleine Tray-App für Windows 11, die die Symbole im Infobereich sichtbar hält.
Windows 11 versteckt jedes neue Tray-Icon hinter dem `^`-Überlaufmenü und bietet das frühere Häkchen „Immer alle Symbole anzeigen“ nicht mehr an. TrayReveal bringt diesen Komfort zurück: Es beobachtet die Icon-Liste des aktuellen Benutzers und blendet neue Icons sofort ein – oder auf Wunsch alle.

> **Stand:** frühe Entwicklung, es gibt noch keine Veröffentlichung.

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

### Lizenz
[MIT](LICENSE) – keine Verbindung zu Microsoft und nicht von Microsoft unterstützt. Windows ist eine Marke der Microsoft-Unternehmensgruppe.
