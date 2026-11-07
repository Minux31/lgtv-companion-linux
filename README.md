# LGTV Companion Linux

Portage Linux de [LGTV Companion](https://github.com/JPersson77/LGTVCompanion), avec priorité à Arch Linux et KDE Plasma sous Wayland.

> **État : amorçage du portage.** Ce dépôt contient la structure initiale et les premières briques Rust. Il ne constitue pas encore une version fonctionnelle équivalente au logiciel Windows.

## Objectifs

- Service **systemd système** (pas un service de session utilisateur).
- Démarrage au boot et réveil du téléviseur avant l'écran de connexion.
- Client LG webOS : découverte/configuration, appairage, WebSocket, clé de client persistée, commandes et état d'alimentation.
- Wake-on-LAN en complément du protocole webOS.
- Intégration Linux prudente : distinguer arrêt, redémarrage, veille et reprise. **Un redémarrage ne doit jamais envoyer une commande TV OFF.**
- Interface graphique Tauri 2 prévue dans une étape ultérieure.
- Installation système uniquement : configuration sous `/etc/lgtv-companion/`, données sous `/var/lib/lgtv-companion/`, journaux via journald.

## État des fonctionnalités

| Fonction | État |
|---|---|
| Configuration TOML | Base initiale |
| Paquet Wake-on-LAN | Implémentation initiale |
| Transport WebSocket webOS | Connexion et SSAP registration initiales ; à valider sur téléviseur |
| Appairage et stockage de clé | Échange initial + stockage système 0600 ; parcours TV à valider |
| Commandes et interrogation d'état | À compléter |
| Détection fiable shutdown/reboot/suspend | À concevoir et tester |
| GUI Tauri | Non commencée |
| Packaging Arch Linux | À compléter |

## Développement

Prérequis : Rust stable et Cargo.

```sh
cargo check
cargo test
```

Lancer en mode diagnostic avec un fichier explicite :

```sh
cargo run -- --config ./config.example.toml check-config
```

Le service installé utilise par défaut `/etc/lgtv-companion/config.toml`. Pour tester l'appairage sur une TV, exécuter `sudo lgtv-companion --config /etc/lgtv-companion/config.toml pair` : la clé est enregistrée sous `/var/lib/lgtv-companion/webos-client-key`. Le compte doit avoir accès au répertoire d'état système. Le mode de développement ne doit pas écrire dans les chemins système. Le service systemd fourni reste un squelette : ne pas l'activer comme démon permanent tant que la boucle de service n'est pas implémentée.

## Sécurité et prudence

- Ne jamais journaliser les clés d'appairage ou les messages contenant des secrets.
- Ne jamais envoyer OFF à la suite d'un événement ambigu.
- La logique d'arrêt doit être validée avec systemd/logind avant activation par défaut.
- Le réveil réseau dépend du modèle de téléviseur, de sa configuration et du réseau local.

## Projet amont

Le projet Windows d'origine est développé par JPersson77 : https://github.com/JPersson77/LGTVCompanion. Son code est sous licence MIT. Ce dépôt Linux est indépendant ; consulter `docs/UPSTREAM.md` avant toute reprise de code source afin de préserver les mentions de licence et d'attribution.
