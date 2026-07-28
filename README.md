# Impulse Voice

Dictée locale et privée pour CachyOS, Hyprland, Quickshell et Illogical
Impulse. Maintiens un raccourci, parle, relâche : le texte est transcrit par
Parakeet V3 sur le CPU puis inséré dans l'application active.

## Fonctionnalités

- capture du microphone par CPAL via la pile PipeWire/ALSA
- conversion multicanal vers mono et resampling 16 kHz avec rubato
- suppression légère du silence avant et après la parole
- Parakeet TDT 0.6B v3 INT8 local via ONNX Runtime
- modèle conservé en mémoire après la première dictée
- insertion Wayland par `wl-copy` et `wtype`, avec restauration du presse-papiers
- capsule Quickshell non focalisable
- daemon systemd utilisateur et protocole JSON sur socket Unix
- diagnostic matériel et logiciel intégré

L'audio ne quitte jamais la machine.

## Prérequis CachyOS/Arch

```bash
sudo pacman -S --needed \
  base-devel alsa-lib pipewire pipewire-alsa wireplumber \
  wl-clipboard wtype curl
```

Rust doit être disponible via `rustup` ou les paquets Arch.

## Installation

Depuis la racine du dépôt :

```bash
./scripts/install.sh
```

Le script :

1. télécharge Parakeet V3 INT8 et vérifie son SHA-256 ;
2. compile le daemon en mode release ;
3. installe le binaire et le service systemd utilisateur ;
4. installe les deux fichiers QML ;
5. ajoute idempotemment le loader Illogical Impulse ;
6. ajoute les raccourcis Hyprland ;
7. démarre le service et exécute le diagnostic.

Options :

```bash
./scripts/install.sh --no-model
./scripts/install.sh --no-quickshell
./scripts/install.sh --no-start
```

## Utilisation

- maintenir `Super+Alt+V` : enregistrer
- relâcher `Super+Alt+V` : transcrire et coller
- `Super+Alt+Shift+V` : mode démarrer/arrêter
- `Super+Alt+Échap` : annuler

Commandes Quickshell :

```bash
qs -c ii ipc call impulseVoice start
qs -c ii ipc call impulseVoice stop
qs -c ii ipc call impulseVoice toggle
qs -c ii ipc call impulseVoice cancel
```

## Diagnostic

```bash
impulse-voice-daemon --doctor
impulse-voice-daemon --list-input-devices
impulse-voice-daemon --warmup
systemctl --user status impulse-voice.service
journalctl --user -u impulse-voice.service -f
```

Le diagnostic vérifie le microphone par défaut, le modèle et les outils
d'insertion Wayland.

Un fichier WAV mono 16 kHz peut tester l'inférence sans ouvrir le microphone :

```bash
impulse-voice-daemon --transcribe-wav /chemin/vers/test.wav
```

## Test sans collage

Arrête d'abord le service puis lance :

```bash
systemctl --user stop impulse-voice.service
impulse-voice-daemon --no-paste
```

Dans un autre terminal :

```bash
printf '{"id":1,"command":"start"}\n' |
  ncat -U "$XDG_RUNTIME_DIR/impulse-voice.sock"

# Parler, puis :
printf '{"id":2,"command":"stop","paste":false}\n' |
  ncat -U "$XDG_RUNTIME_DIR/impulse-voice.sock"
```

Pour un client persistant, garder une seule connexion `ncat -U` ouverte et
envoyer successivement les deux lignes JSON.

## Emplacements

```text
~/.local/bin/impulse-voice-daemon
~/.local/share/impulse-voice/models/parakeet-tdt-0.6b-v3-int8/
~/.config/systemd/user/impulse-voice.service
~/.config/quickshell/ii/services/ImpulseVoiceService.qml
~/.config/quickshell/ii/modules/ii/impulseVoice/ImpulseVoice.qml
```

Le modèle peut être déplacé avec `IMPULSE_VOICE_MODEL=/chemin/du/modèle`.
Le microphone peut être sélectionné avec
`IMPULSE_VOICE_INPUT_DEVICE="nom CPAL exact"`.

## Développement

```bash
cargo fmt --check
cargo test
cargo run -- --doctor
```

Voir [l'architecture](docs/architecture.md) et le
[protocole IPC](docs/protocol.md).

## Désinstallation

```bash
./scripts/uninstall.sh
```

Le modèle est volontairement conservé.

## Crédits et licence

La direction produit et certaines décisions d'architecture sont inspirées de
[Handy](https://github.com/cjpais/Handy), sous licence MIT. Le nom, le logo et
les éléments de marque de Handy ne sont pas utilisés.

Impulse Voice est distribué sous [licence MIT](LICENSE).
