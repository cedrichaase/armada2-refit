# Sourced by install and bench-setup.sh: exe_dir <game dir> prints the folder of the
# clone's executable, where the ASI loader looks for plugins and Scene.asi reads and
# writes its files: the game directory for Armada II, Data/ for Fleet Operations: Roots.
# The game is the one A2_PROFILE (or `a2test --game`) names, else whichever descriptor's
# executable the directory holds (games/<id>.toml, exe.data_dir and exe.names).
exe_dir() {
    python3 - "$1" "$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)" <<'EOF'
import os
import sys
sys.path.insert(0, sys.argv[2])
import a2env
game = sys.argv[1]
ids = [os.environ['A2_PROFILE']] if os.environ.get('A2_PROFILE') else []
ids += [i for i in a2env.game_ids() if i not in ids]
for i in ids:
    exe = a2env.load(i).get('exe', {})
    d = os.path.join(game, exe.get('data_dir', ''))
    if any(os.path.isfile(os.path.join(d, n)) for n in exe.get('names', [])):
        print(os.path.normpath(d))
        sys.exit(0)
print(game)
EOF
}
