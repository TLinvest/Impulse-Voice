# Architecture

```text
Hyprland shortcut
       |
       v
Quickshell capsule <---- NDJSON / Unix socket ----> Rust daemon
                                                    |
                                           CPAL microphone
                                                    |
                                     downmix mono + rubato 16 kHz
                                                    |
                                        energy-based silence trim
                                                    |
                                   Parakeet TDT 0.6B v3 INT8
                                                    |
                                        transcript normalization
                                                    |
                                      wl-copy + wtype/ydotool
```

## Cycle de dictée

Le microphone est ouvert uniquement lors de `start`. `stop` détruit le flux
CPAL, récupère le tampon, le resample puis envoie le PCM à Parakeet. Cette
stratégie évite de laisser l'indicateur microphone actif au repos.

Parakeet est chargé paresseusement lors de la première transcription et reste
en mémoire dans le service. Les transcriptions suivantes évitent donc le coût
de chargement du modèle.

L'inférence et l'insertion sont exécutées hors du runtime asynchrone. Le socket
reste réactif pendant les opérations CPU bloquantes.

## Interface

La fenêtre Quickshell utilise la couche overlay mais reste non focalisable avec
une zone d'exclusion nulle. L'application qui possédait le focus avant la
dictée le conserve, ce qui permet à `wtype` d'y envoyer `Ctrl+V`.

## Modèle

Le dossier Parakeet doit contenir :

```text
parakeet-tdt-0.6b-v3-int8/
├── encoder-model.int8.onnx
├── decoder_joint-model.int8.onnx
├── nemo128.onnx
└── vocab.txt
```

L'installateur utilise l'archive publiée par Handy et vérifie le SHA-256 avant
extraction. Le code d'inférence dépend directement de `transcribe-rs`.

## Limites actuelles

Le filtre de silence repose sur l'énergie RMS, adapté au push-to-talk. Silero
VAD pourra le remplacer si un mode mains libres ou streaming est ajouté.

