# AnyRoblox (Any Games To Roblox)

Convertit le LOD visuel principal des modèles Arma 3 en Wavefront OBJ, avec matériaux MTL et textures diffuses PNG. Le mode local fonctionne hors ligne. Le mode Workshop utilise exclusivement les métadonnées et le lien de fichier rendus publics par Steam.

## Compiler et lancer

### Application graphique Windows x64

L’interface est en anglais. La vue 3D occupe la partie gauche de la fenêtre. À droite, **Outliner** permet de sélectionner un modèle et **Conversion Properties** regroupe les sources et les options d’export. Faites glisser les séparateurs pour redimensionner les panneaux. **Grid** affiche la grille au sol et **Solid** permet d’inspecter la géométrie avec un matériau neutre.

Télécharger [AnyRoblox.exe pour Windows x64](https://github.com/niskil377-dotcom/AnyRoblox/releases/latest/download/AnyRoblox.exe), puis le lancer. L’accueil propose **FiveM → OBJ** et **Arma 3 → OBJ**. Le bouton **Switch workspace** ramène à cet écran.

**FiveM.** Choisir un fichier YDR, YDD ou YFT de GTA V Legacy, ou sélectionner le dossier `stream` d’une ressource locale. Cliquer sur **Convert to OBJ** et choisir un emplacement de sortie. Studio ouvre les OBJ produits dans la visionneuse et conserve le rapport du lot dans le dossier de sortie.

Le parcours FiveM lit le premier LOD visuel disponible, les triangles, les normales, les UV et le matériau de chaque géométrie. Un YDD peut produire plusieurs OBJ. Pour un YFT, l’export couvre le drawable principal. Les squelettes animés, fragments de dégâts, simulations de tissu et collisions restent hors export. Les textures diffuses du paramètre DiffuseSampler peuvent être exportées en PNG depuis le modèle ou un YTD homonyme voisin. Les dépendances de textures partagées entre ressources demandent une résolution supplémentaire. Les fichiers protégés et les formats Enhanced ne sont pas pris en charge.

La compilation graphique récupère les paquets NuGet CodeWalker.Core 1.0.3 et SharpDX 4.2.0 dans le cache et vérifie leurs empreintes SHA-256. Les bibliothèques sont intégrées à l’EXE. Le parcours a été testé sur les fichiers binaires Legacy YDR et YFT des tests Sollumz et sur un YDD synthétique construit à partir du YDR. La lecture des OBJ résultants dans Studio a donné respectivement 12, 24 et 12 triangles. [CodeWalker](https://github.com/dexyfex/CodeWalker) · [Fichiers de vérification Sollumz](https://github.com/Sollumz/Sollumz/tree/main/tests/assets/gen8)

**Arma 3**

1. S’abonner au mod dans le Workshop Steam d’Arma 3 et attendre la fin du téléchargement dans le client Steam.
2. Ouvrir Studio, ou cliquer sur **Refresh** si le téléchargement vient de finir. Sélectionner le mod dans **Installed mods**.
3. Cliquer sur **Convert to OBJ** et choisir un emplacement de sortie extérieur au mod. Les modèles s’affichent dans la vue 3D. **Open output folder** donne accès aux exports et aux rapports.

Studio examine les bibliothèques Steam déclarées dans `libraryfolders.vdf`, notamment `steamapps/workshop/content/107410`, les dossiers `!Workshop` et `mods` du jeu ainsi que le dossier `mods` à côté de l’exécutable. **Library…** ajoute un dossier de mods et mémorise ce choix. **Folder…** et **PBO / ZIP…** permettent de sélectionner directement une source avant de cliquer sur **Convert to OBJ**.

Pour inspecter un export existant, choisir **Open OBJ…**. Dans la vue 3D, tourner la molette pour zoomer, maintenir le bouton central et glisser pour orbiter, ou ajouter Maj pour déplacer le point de vue. La touche F recadre le modèle.

L’exécutable contient la visionneuse WPF, la CLI Rust, P3DDebin et HEMTT. Il utilise .NET Framework 4.8 sous Windows x64. Pour le reconstruire, placer les outils de conversion dans `cache/outils` puis lancer `./desktop/build.ps1` sous PowerShell.

### Ligne de commande

Avec Rust stable, Cargo et l’éditeur de liens de la cible installés, depuis la racine du projet :

```sh
cargo test
cargo build --release
cargo install --path . --locked
arma2obj --workshop "https://steamcommunity.com/sharedfiles/filedetails/?id=3842854567" --output ./sortie/workshop
arma2obj --workshop 3842854567 --output ./sortie/par-id
arma2obj --input ./mon-mod --output ./sortie/local
```

L’installation par Cargo est facultative. Le binaire se trouve aussi dans `target/release`. Chaque exécution accepte un dossier de sortie vide ou à créer, extérieur à l’entrée. `--input` accepte un dossier, un ZIP ou un PBO. Les ZIP et les PBO sont extraits dans un dossier temporaire. Les outils externes reçoivent une copie de travail.

**Rejouer le cas réel sur ce poste**, depuis PowerShell, avec les outils et le PBO de test placés dans le cache local :

```powershell
.\target\release\arma2obj.exe --input .\cache\validation\ace3-3.21.3\ace_dogtags.pbo --debinarizer .\cache\outils\P3DDebin.exe --paa-tool .\cache\outils\hemtt.exe --output .\sortie\essai
```

Pour réutiliser les mêmes outils avec la commande courte, définir `ARMA2OBJ_DEBINARIZER` et `ARMA2OBJ_PAA_TOOL` avec leurs chemins. Les options CLI prennent la priorité. Le cache, les sorties de validation et les exécutables externes sont exclus de Git. Pour reconstruire sur le poste de validation avec la chaîne Windows GNU installée pendant le test, ajouter Cargo et dlltool au PATH de la session :

```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;$(Resolve-Path .\cache\outils\rust-gnu);$env:Path"
cargo build --release
```

## Workshop

Le programme normalise l’ID, effectue le POST anonyme, vérifie l’ID retourné, le résultat, la visibilité publique, le statut de bannissement et l’application Arma 3 (107410). Le rapport conserve notamment le titre et le SteamID de l’auteur. Un téléchargement abouti est conservé sous `./cache/<id>/`, configurable par `--cache`. Si Steam donne une date de mise à jour, le cache distingue les révisions. L’accès public est revérifié avant réutilisation. Le format est déterminé par les premiers octets du ZIP ou la validation de la table PBO. Les signatures 7z, RAR et gzip sont identifiées pour demander une extraction locale. [API Steam](https://partner.steamgames.com/doc/webapi/ISteamRemoteStorage#GetPublishedFileDetails)

**Essai du 24 novembre 2026.** Pour `3842854567`, Steam a répondu HTTP 200, titre « RZ - M60 REVIVAL », créateur `76561198416561244`, visibilité publique et `file_url=""`. Aucun fichier reçu. Le format, les modèles et les textures de cet article restent indéterminés. Son traitement demande une archive ou un dossier fourni localement. Un refus HTTP, une fiche privée et une panne réseau sont également remontés. Si le réseau bloque Steam, joindre l’archive manuellement. [Fiche testée](https://steamcommunity.com/sharedfiles/filedetails/?id=3842854567)

**Reforger.** La page « Sample Mod - New Car » présente l’auteur, la description, la licence, la version, la taille et l’identifiant. Les liens publics visibles lors de l’examen menaient notamment aux images et au journal des changements. Aucun lien direct vers l’archive du mod n’a été identifié. Pour une URL Reforger, la CLI lit le titre de la fiche puis signale la limite. `--input` convient aux ressources Arma 3 compatibles, les formats natifs Enfusion sortent du périmètre. [Fiche examinée](https://reforger.armaplatform.com/workshop/5614E482BF83E310-SampleMod-NewCar)

## Formats et outils

| Étape | Prise en charge et conventions |
|---|---|
| Archives | ZIP stocké/Deflate, PBO à entrées contiguës, propriétés Vers, préfixe virtuel et entrées brutes ou Cprs/LZSS. Vérification de la somme additive Cprs. Le pied optionnel SHA de 21 octets est reconnu structurellement. La signature cryptographique et les fichiers bisign ne sont pas vérifiés. |
| Limites de lecture | 512 Mio par archive ou modèle lu en mémoire, 100 000 entrées par archive, 2 Gio de fichiers décompressés par ZIP et 2 Gio d’extraction PBO par lot. Noms UTF-8 ou ASCII. Les formats de paquet propriétaires ou obfusqués peuvent être refusés. |
| ODOL | Lecture de la signature et de la version little-endian. La CLI accepte la v75 par défaut. `--odol-version 73,74,75` remplace cette liste pour un autre outil dont on connaît les capacités. Toute sortie du débinariseur repasse par le lecteur MLOD. |
| MLOD | Version 257, blocs P3DM 28/256, points, normales, triangles/quads, quatre emplacements de coin par face, UV de base, texture et RVMAT par face, TAGG et résolution. Sélection de la plus petite résolution dans [0, 1000[. L’absence de LOD visuel produit un échec explicite. |
| OBJ et MTL | Indices OBJ à partir de 1, normales et UV par coin, groupes par couple texture/RVMAT. Axe vertical OBJ Y, troisième coordonnée inversée, signe des normales adapté et V remplacé par 1−V. Les PNG transparents alimentent aussi la carte d’opacité. |
| Textures | Résolution des références dans tous les PBO du lot, par préfixe, puis relativement au modèle. Recherche insensible à la casse ASCII. Pour un dossier extrait, les marqueurs `$PBOPREFIX$` et `$PREFIX$` rétablissent le chemin virtuel. Un PNG homonyme peut fournir la texture directement. Si la conversion est indisponible, le matériau garde sa couleur diffuse et le rapport indique « partiel ». |
| Éléments inventoriés | Résolutions, LOD HitPoints, proxies, sélections nommées et temps des tags Animation. Pour les model.cfg voisins ou parents, noms des classes de squelettes et d’animations déclarées en texte. Ces déclarations peuvent concerner plusieurs modèles. Les faces des proxies identifiés sont écartées. |
| Hors export | LOD techniques, collisions, poids et hiérarchie d’os, animations, modèles référencés par proxy, shaders et passes RVMAT, textures procédurales, UV secondaires. Pas de prétraitement ni de résolution d’héritage des model.cfg. Les listes vides indiquent ce que le lecteur a observé. |

Le parseur PBO suit les champs documentés et la variante LZSS de Bohemia. La disposition MLOD a été croisée avec le lecteur Arma 3 Object Builder puis vérifiée sur un fichier MLOD réel. [Format PBO](https://community.bistudio.com/wiki/PBO) · [LZSS](https://community.bistudio.com/wiki/Compressed_LZSS_File_Format) · [Format MLOD](https://community.bistudio.com/wiki/P3D_File_Format_-_MLOD) · [Lecteur de référence](https://github.com/MrClock8163/Arma3ObjectBuilder/blob/master/Arma3ObjectBuilder/io/data_p3d.py)

**Contrats des exécutables**. Les arguments sont transmis séparément via `std::process::Command` :

- `P3DDebin.exe debin input.p3d -o <dossier>` produit `input_debin.p3d`. Version 1.2.1 testée sur ODOL 75. [Distribution testée](https://github.com/Wesley-TB/P3DDebinarizer/releases/tag/v1.2.1)
- `P3DDebinarizer.exe input.p3d` attend `input_mlod.p3d`. La distribution archivée de ScripyZz a signalé « Unknown ODOL version » sur le même modèle en console interactive. Elle a également échoué dans une console redirigée. [Dépôt archivé](https://github.com/ScripyZz/P3D-Debinarizer-Arma-3)
- `hemtt utils paa convert input.paa output.png` convertit le premier mipmap. HEMTT 1.23.1 a été testé. [Commande HEMTT](https://hemtt.dev/utilities/paa/convert.html)
- Un autre nom d’exécutable sélectionne le contrat générique `outil entrée sortie`, avec deux chemins absolus. Un adaptateur local peut ainsi appeler un logiciel ayant une autre interface.

## Rapports et codes de sortie

Le `report.json` à la racine contient les métadonnées Workshop, le chemin en cache, le format reçu, le préfixe et la liste des fichiers de chaque PBO, les dossiers de modèles avec leur statut, ainsi que les erreurs de lot. Chaque dossier de modèle contient son propre `report.json`. Un export réussi contient aussi `model.obj`, `model.mtl` et les PNG disponibles. Le rapport associe les noms de matériaux OBJ aux références Arma.

Les statuts sont `ok`, `partiel` et `échec`. Un fichier corrompu ou une version inconnue laisse les modèles suivants se convertir. Le code de sortie vaut 0 pour un lot entièrement réussi et 2 pour un lot incomplet ou refusé. Une erreur de préparation ou d’écriture empêchant la création du rapport remonte directement sur stderr.

## Vérifications effectuées

| Étape | Résultat observé | Limite de la vérification |
|---|---|---|
| Compilation et tests | Rust stable 1.100, édition 2021, Windows x64 GNU. `cargo build --release` et six tests d’intégration réussis. | Linux et macOS non exécutés. |
| Steam | ID numérique et URL testés par Cargo. Appel réel de la CLI sur le lien fourni, métadonnées enregistrées et arrêt sur file_url vide. | Téléchargement Steam réussi et réutilisation d’une révision du cache non exercés sur un mod réel. |
| Extraction | PBO réel `ace_dogtags.pbo` de la distribution ACE3 3.21.3, 48 fichiers inventoriés, préfixe virtuel résolu. SHA-256 de l’entrée identique avant et après. ZIP contenant un PBO, Cprs, somme incorrecte, troncature et chemins invalides testés sur des fixtures synthétiques. | La compression Cprs est vérifiée sur fixture synthétique. |
| Modèle | MLOD source ACE3 `ace_dogtag.p3d` lu directement, puis ODOL v75 de la distribution débinarisé indépendamment. Les deux donnent le LOD 1, 28 sommets et 26 faces. | Les autres versions ODOL et la fidélité géométrique de modèles complexes restent à vérifier. |
| Matériaux et Blender | PAA réel converti en PNG 256 × 256. Import OBJ dans Blender 4.4.3, contrôle des 28 sommets et 26 faces, chargement du PNG et inspection du rendu de la plaque, trou transparent visible. | Le rendu valide cet exemple de plaque plane. L’apparence des shaders en jeu et l’orientation sur un modèle asymétrique complexe restent à vérifier. |
| Reprise | Même lot synthétique avec ODOL inconnu, MLOD tronqué et modèle valide à texture absente. Les deux premiers échouent, le troisième produit OBJ/MTL et un rapport partiel. Autre test avec repli PNG, sélection multirésolution, groupes de matériaux et indices UV. | Proxies, animations et exclusion du LOD HitPoints vérifiés sur données synthétiques. |

Les hypothèses de sélection du LOD et d’identification des proxies reposent sur les conventions P3DM. Les sélections nommées peuvent représenter autre chose que des os. Les classes trouvées dans un model.cfg décrivent les déclarations disponibles dans son dossier, avec les limites d’héritage indiquées plus haut. Le rendu et les rapports de la validation locale sont dans `sortie/validation-ace3/`. [Distribution ACE3 testée](https://github.com/acemod/ACE3/releases/tag/v3.21.3) · [MLOD source utilisé](https://github.com/acemod/ACE3/blob/a6c876718e5e86016a41ffd050d584a305633e5b/addons/dogtags/data/ace_dogtag.p3d)

## Dépendances directes

Clap gère les arguments et les variables d’environnement. Reqwest bloquant utilise TLS natif. Serde et serde_json lisent la réponse Steam et écrivent les rapports. Zip gère le conteneur ZIP, image valide et réencode les PNG, anyhow propage le contexte d’erreur. La dépendance supplémentaire tempfile fournit les répertoires de travail isolés et la publication de fichiers sans écrasement. La lecture binaire utilise des tranches contrôlées et `from_le_bytes` de la bibliothèque standard. Le projet interdit `unsafe` dans son code.

