# Dofus Switcher

Application Linux pour organiser plusieurs fenêtres Dofus, passer de l’une à
l’autre avec des raccourcis et activer le compte dont le nom apparaît au début
du tour.

## Compatibilité

Dofus Switcher utilise les API X11 pour ses opérations sur les fenêtres : il
lit `_NET_CLIENT_LIST`, identifie le jeu avec `WM_CLASS` (ou le PID de la
fenêtre), capture les pixels avec X11 et demande l’activation via
`_NET_ACTIVE_WINDOW`. Sa fenêtre graphique est elle aussi compilée avec le
backend X11.

Cela fonctionne dans une session Wayland lorsque Dofus est exécuté sous
**XWayland**. La session de bureau reste Wayland ; XWayland fournit à Dofus et à
l’application une interface compatible X11. Les fenêtres Wayland natives ne
sont pas visibles par ces API et ne sont donc pas prises en charge actuellement.

Sur la configuration CachyOS vérifiée pour ce projet, la session est Wayland
(`XDG_SESSION_TYPE=wayland`) et XWayland expose les fenêtres du jeu avec
`WM_CLASS=Dofus.x64`. C’est cette classe que le détecteur reconnaît. Pour
confirmer le backend d’une installation, `xprop -root _NET_CLIENT_LIST` doit
inclure les fenêtres Dofus ; `xprop -id <ID> WM_CLASS _NET_WM_PID` permet de
vérifier leur classe et leur processus.

## Compilation

Le projet demande Rust 1.95 ou plus récent. Sur Arch Linux et ses dérivées, les
outils de compilation et les bibliothèques X11 habituelles doivent être
installés. Par exemple :

```sh
sudo pacman -S --needed base-devel pkgconf libx11 libxcb libxkbcommon \
  libxrandr libxinerama libxcursor libxi mesa
cargo build --release
```

Puis lance l’application avec `cargo run --release` ou
`target/release/dofus-switcher`.

## Première utilisation

1. Lance Dofus. Ses fenêtres apparaissent dans la liste de comptes.
2. Associe chaque fenêtre à un nom de personnage. Les noms sont enregistrés et
   l’application tente de rétablir les associations au prochain lancement.
3. Utilise les flèches pour régler l’ordre de tour. Configure les raccourcis
   globaux et, si besoin, un raccourci direct par compte.
4. Dans Dofus, ouvre **Options > Jeu > Général** et active **Afficher l’image
   de la classe comme picto de la fenêtre**. Cette option aide à distinguer les
   fenêtres ; l’OCR lit le nom du personnage affiché dans le jeu, pas cette
   icône.
5. Dans Dofus Switcher, sélectionne un compte puis clique sur **Placer la zone**.
   Déplace la barre du cadre sur le nom qui apparaît au début du tour. Garde le
   nom entier dans le cadre et évite le niveau ou le décor. Redimensionne avec
   la poignée en bas à droite, puis valide avec **Entrée**. **Échap** annule.

La zone OCR est mémorisée. Quand elle reconnaît un nom associé à un compte,
l’application active sa fenêtre. Le bouton **?** dans l’interface ouvre un guide
illustré et les infobulles expliquent les contrôles.

## Modèles OCR

Au premier démarrage du moteur OCR, place ces deux modèles dans le dossier de
cache `ocrs` de l’utilisateur (`~/.cache/ocrs/` par défaut, ou
`$XDG_CACHE_HOME/ocrs/`) :

- `text-detection.onnx`
- `text-recognition.onnx`

Ils sont disponibles dans le dépôt [ocrs-models](https://github.com/robertknight/ocrs-models).

## Configuration

La configuration est enregistrée dans
`$XDG_CONFIG_HOME/dofus-switcher/config.toml` (`~/.config/dofus-switcher/config.toml`
par défaut). Les versions historiques sont migrées à la lecture. Une version
plus récente que celle comprise par l’application est signalée et protégée
contre l’écrasement.
