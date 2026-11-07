# Notes de protocole webOS SSAP

Cette implémentation s'appuie sur l'analyse du comportement du client upstream MIT, en particulier :
- `LGTV Companion Service/web_os_client.cpp`
- `Common/lg_api.h`

## Connexion

- WebSocket non chiffré : TCP/3000.
- WebSocket TLS : TCP/3001.
- Chemin WebSocket : `/`.
- Identifiant de la trame d'enregistrement : `register_0`.

## Appairage

Le client envoie une trame `type=register`, avec `pairingType=PROMPT`, `forcePairing=false` et le manifeste de permissions. Une clé existante est transmise dans `payload.client-key`.

Une réponse réussie utilise `type=registered`. Lors du premier appairage, la clé renvoyée se trouve dans `payload.client-key`. Une réponse `type=error` indique un échec ou un refus d'appairage. L'implémentation actuelle retente une seule fois sans la clé précédente si celle-ci est rejetée.

La clé est écrite sous `/var/lib/lgtv-companion/webos-client-key`, avec permissions de fichier 0600. Elle ne doit jamais être journalisée ni copiée dans le TOML de configuration.

## États d'alimentation

Le client upstream interroge `ssap://com.webos.service.tvpower/power/getPowerState`. Les états observés comprennent :

- `Active` : téléviseur actif.
- `Screen Off` : téléviseur actif, écran éteint.
- `Active Standby` : veille active.

Aucune commande OFF ne doit être déduite d'une requête échouée, d'un état inconnu, d'une socket déconnectée ou d'un événement hôte ambigu.

## Limites actuelles

La branche actuelle réalise la connexion, l'enregistrement SSAP, l'analyse de la réponse et la persistance de la clé. Elle ne réalise pas encore :
- le traitement complet des réponses et la corrélation des requêtes ;
- le polling d'état, le keepalive et la reconnexion bornée ;
- les commandes d'allumage/extinction ;
- la boucle de service permanente ;
- les tests sur matériel LG réel.

Le comportement TLS doit être validé sur les modèles visés. La validation de certificat ne doit pas être désactivée silencieusement.

## Attribution

Les comportements de protocole ont été étudiés dans le projet MIT de JPersson77 : https://github.com/JPersson77/LGTVCompanion. Aucun fichier source upstream n'est copié dans ce dépôt.
