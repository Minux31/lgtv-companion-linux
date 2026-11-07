# Projet amont et attribution

Le projet d'origine est [LGTV Companion](https://github.com/JPersson77/LGTVCompanion), développé par JPersson77 et distribué sous licence MIT.

Ce dépôt Linux est un portage indépendant. Les premières briques présentes ici ont été écrites séparément et ne recopient pas de fichiers source du projet Windows.

Avant de porter une portion de code upstream :
1. vérifier la licence et les avis de copyright du fichier concerné ;
2. conserver les mentions exigées par la licence MIT ;
3. documenter les adaptations Linux/Rust ;
4. ne pas importer de secrets, certificats privés ou données d'utilisateur.

L'implémentation du protocole webOS doit être comparée au comportement du client amont (notamment appairage, clé de session, messages, keepalive, reconnexion et interrogation d'état), sans présumer que le simple envoi d'un message `register` suffit.
