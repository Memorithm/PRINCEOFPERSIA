#!/usr/bin/env python3
"""Génère et valide les 15 mondes du jeu top-down, puis émet world_data.rs.

Quête : le Vizir Ganar a brisé le Sceau de Lumière en 4 fragments.
Le Prince doit réunir les 4 fragments (un par boss de donjon), trouver
le Miroir d'argent, ouvrir le Sanctuaire du Miroir puis affronter Ganar.
"""
import random
from collections import deque

W, H = 48, 36          # vallée / ombre
DW, DH = 36, 28        # geôles
PW, PH = 40, 30        # palais
FW, FH = 44, 32        # forêt
CW, CH = 36, 30        # cavernes de cristal
SW, SH = 48, 32        # désert
NW, NH = 40, 32        # nécropole
MW, MH = 44, 30        # marais
GW, GH = 40, 28        # col des vautours
KW, KH = 36, 28        # mines
OW, OH = 26, 22        # oasis
SW2, SH2 = 30, 26      # sanctuaire du miroir
TW, TH = 30, 34        # tour
RW, RH = 24, 22        # trône des ténèbres

PASSABLE = set('.,%MCX"PhLgkieHVYombzn12394F')   # sans 'D' (porte verrouillée)

def grid(w, h, fill=','):
    return [[fill for _ in range(w)] for _ in range(h)]

def fill(g, x0, y0, x1, y1, ch):
    for y in range(y0, y1 + 1):
        for x in range(x0, x1 + 1):
            g[y][x] = ch

def put(g, x, y, ch):
    g[y][x] = ch

def border_trees(g, ch='T'):
    w, h = len(g[0]), len(g)
    for x in range(w):
        g[0][x] = ch; g[h-1][x] = ch
        g[1][x] = ch if g[1][x] == ',' else g[1][x]
        g[h-2][x] = ch if g[h-2][x] == ',' else g[h-2][x]
    for y in range(h):
        g[y][0] = ch; g[y][1] = ch
        g[y][w-1] = ch; g[y][w-2] = ch

def render(g):
    return [''.join(row) for row in g]

# ------------------------------------------------------------------ helpers

def room(g, x0, y0, x1, y1, deco=True, floor='.'):
    fill(g, x0, y0, x1, y1, floor)
    if deco:
        put(g, x0, y0, 't'); put(g, x1, y0, 't')
        put(g, x0, y1, 't'); put(g, x1, y1, 't')

def link(g, x0, y0, x1, y1):
    fill(g, min(x0,x1), min(y0,y1), max(x0,x1), max(y0,y1), '.')

def hut(g, x0, y0):
    fill(g, x0, y0, x0+6, y0+3, '#')
    fill(g, x0+1, y0+1, x0+5, y0+2, '.')
    put(g, x0+3, y0+3, '.')

# =====================================================================
# Monde 0 — La Vallée d'Ispahan (hub de la Lumière)
# =====================================================================
def build_valley():
    g = grid(W, H, ',')
    fill(g, 0, 0, W-1, 0, '#')
    border_trees(g)

    # --- porte du Palais (nord-est) ------------------------------------
    fill(g, 38, 1, 44, 7, ',')
    fill(g, 38, 1, 44, 1, '#')
    fill(g, 38, 2, 39, 4, '#')
    fill(g, 43, 2, 44, 4, '#')
    put(g, 41, 3, 'X')                    # -> Palais
    fill(g, 40, 5, 42, 7, '.')
    put(g, 38, 5, '#'); put(g, 44, 5, '#')
    fill(g, 34, 8, 41, 8, '.')

    # --- cascade + grotte secrète (nord-ouest) --------------------------
    fill(g, 3, 2, 13, 8, '#')
    fill(g, 7, 2, 8, 7, '~')
    put(g, 8, 8, 'C')                     # -> Ombre (secret)
    fill(g, 6, 9, 9, 10, '.')
    fill(g, 6, 11, 20, 11, '.')
    put(g, 5, 9, 'b'); put(g, 10, 9, 'b')

    # --- sentier nord vers la forêt --------------------------------------
    fill(g, 17, 2, 18, 2, '.')
    fill(g, 17, 2, 18, 8, '.')

    # --- porte est vers le désert ----------------------------------------
    fill(g, 45, 12, 45, 14, ',')
    fill(g, 38, 13, 44, 13, '.')

    # --- sentier nord-est vers le col -------------------------------------
    fill(g, 43, 10, 44, 10, '.')
    fill(g, 40, 11, 43, 11, '.')

    # --- le village -------------------------------------------------------
    hut(g, 14, 13); put(g, 17, 14, 'V'); put(g, 15, 14, 'g')
    hut(g, 22, 8);  put(g, 25, 9, 'g')
    hut(g, 30, 13); put(g, 33, 14, '"')
    fill(g, 22, 17, 30, 20, '.')
    put(g, 26, 18, 'X')                   # LE PUITS -> Geôles
    put(g, 24, 18, 'R'); put(g, 28, 18, 'R')
    fill(g, 25, 21, 27, 26, '.')
    fill(g, 31, 18, 38, 18, '.')
    fill(g, 14, 18, 21, 18, '.')
    fill(g, 17, 19, 17, 22, '.')

    # --- lac (sud-est) -----------------------------------------------------
    for y in range(H):
        for x in range(W):
            dx, dy = (x - 37) / 9.0, (y - 28) / 5.5
            if dx*dx + dy*dy < 1.0:
                g[y][x] = '~'
    fill(g, 38, 23, 38, 33, '%')
    put(g, 34, 27, 'z'); put(g, 41, 30, 'z')
    put(g, 44, 24, 'g')

    # --- cercle des dalles-miroir (sud-ouest) -------------------------------
    fill(g, 5, 24, 13, 30, '.')
    for (sx, sy) in [(6,25),(12,25),(6,29),(12,29)]:
        put(g, sx, sy, 'S')
    put(g, 8, 27, 'M'); put(g, 9, 27, 'M'); put(g, 10, 27, 'M')
    put(g, 9, 26, 'M'); put(g, 9, 28, 'M')
    put(g, 9, 25, 't'); put(g, 9, 30, 't')

    # --- fougères du sud-ouest : passage secret vers l'oasis -----------------
    fill(g, 3, 31, 4, 33, '.')
    fill(g, 4, 29, 4, 31, '.')            # raccord au cercle des dalles
    put(g, 3, 33, 'C')                    # -> Oasis (secret)

    # --- forêt de l'ouest -----------------------------------------------------
    rng = random.Random(7)
    for _ in range(90):
        x, y = rng.randint(2, 16), rng.randint(11, 33)
        if g[y][x] == ',':
            g[y][x] = rng.choice('TTTBRR')
    fill(g, 5, 9, 11, 16, '.')             # zones sensibles dégagées
    fill(g, 39, 6, 43, 8, '.')
    fill(g, 16, 1, 19, 9, '.')
    fill(g, 38, 12, 45, 14, '.')
    fill(g, 39, 10, 44, 12, '.')

    for _ in range(70):
        x, y = rng.randint(2, W-3), rng.randint(2, H-3)
        if g[y][x] == ',':
            g[y][x] = rng.choice('",,B,')

    # portails, posés après tous les déblais
    put(g, 18, 2, 'X')                    # -> Forêt
    put(g, 44, 13, 'X')                   # -> Désert
    put(g, 44, 10, 'R')
    put(g, 43, 11, 'X')                   # -> Col des Vautours (détour)

    # --- ennemis ---------------------------------------------------------------
    for (x, y) in [(20,24),(33,22),(43,16),(24,28),(12,22),(36,12),(29,24)]:
        put(g, x, y, 'o')
    for (x, y) in [(19,26),(31,20),(9,15)]:
        put(g, x, y, 'm')
    for (x, y) in [(36,10),(38,10)]:
        put(g, x, y, 'n')

    # --- objets ------------------------------------------------------------------
    for (x, y) in [(21,10),(45,10),(13,26),(43,21),(3,12),(35,20)]:
        put(g, x, y, 'g')
    put(g, 4, 20, 'h')

    put(g, 26, 20, 'P')
    return g

# =====================================================================
# Monde 3 — L'Ombre de la Vallée
# =====================================================================
def build_shadow():
    g = build_valley()
    for y in range(H):
        for x in range(W):
            if g[y][x] == '~':
                g[y][x] = ':'
            elif g[y][x] == '"':
                g[y][x] = '.'
    fill(g, 3, 2, 13, 8, '#')             # la cascade s'est pétrifiée…
    put(g, 8, 8, 'C')                     # …mais la grotte demeure
    # les sorties du monde lumineux sont murées dans l'ombre
    put(g, 18, 2, 'B'); put(g, 43, 11, 'R'); put(g, 44, 13, 'B')
    put(g, 17, 14, '.')                   # pas d'Erfan dans l'ombre
    fill(g, 5, 9, 11, 16, '.')
    fill(g, 4, 4, 6, 6, '.')
    fill(g, 4, 7, 6, 10, '.')
    put(g, 5, 5, 'h')
    fill(g, 40, 2, 42, 4, '.')            # parvis de la porte scellée
    put(g, 41, 3, 'X')                    # -> Sanctuaire du Miroir
    put(g, 41, 5, 'L')                    # porte scellée (4 fragments)
    # passage vers les marais (est) — corniche le long du lac noir
    fill(g, 44, 14, 45, 31, '.')
    put(g, 44, 30, 'X')                   # -> Marais
    rng = random.Random(21)
    for y in range(H):
        for x in range(W):
            if g[y][x] in 'ombzn':
                g[y][x] = '.' if g[y][x] != 'z' else ':'
    for (x, y) in [(20,24),(33,22),(43,16),(24,28),(12,22),(36,12),(29,24),(16,20),(26,22)]:
        put(g, x, y, rng.choice('mn'))
    for (x, y) in [(19,26),(31,20),(9,15),(36,21),(40,20)]:
        put(g, x, y, 'b')
    for (x, y) in [(34,27),(41,30)]:
        put(g, x, y, 'z')
    put(g, 44, 21, 'g'); put(g, 3, 12, 'g')
    return g

# =====================================================================
# Monde 1 — Les Geôles Oubliées (boss : la Brute)
# =====================================================================
def build_dungeon():
    g = grid(DW, DH, '#')
    room(g, 2, 2, 9, 9)
    put(g, 3, 3, 'X')
    put(g, 4, 4, 'P')
    fill(g, 5, 10, 6, 14, '.')
    room(g, 2, 15, 12, 22)
    put(g, 7, 18, 'S'); put(g, 8, 18, 'S')
    put(g, 4, 20, 'g'); put(g, 10, 20, 'h')
    fill(g, 10, 5, 16, 6, '.')
    fill(g, 17, 2, 26, 9, '.')
    fill(g, 18, 4, 19, 8, '#')
    fill(g, 22, 4, 23, 8, '#')
    put(g, 25, 3, 'k'); put(g, 20, 3, 'g')
    put(g, 21, 6, 'o'); put(g, 24, 7, 'b')
    put(g, 17, 5, 'D')
    room(g, 24, 12, 33, 19)
    fill(g, 13, 18, 24, 19, '.')
    put(g, 30, 14, 'k'); put(g, 27, 17, 'g'); put(g, 31, 17, 'h')
    put(g, 28, 13, 'b'); put(g, 31, 15, 'o'); put(g, 26, 18, 'm')
    room(g, 14, 22, 21, 26)
    fill(g, 7, 23, 8, 25, '.')
    fill(g, 8, 25, 14, 25, '.')
    put(g, 16, 24, 'g')
    room(g, 23, 22, 33, 26)
    fill(g, 22, 23, 22, 25, '.')
    put(g, 28, 24, '1')                   # LA BRUTE
    put(g, 26, 23, 'o'); put(g, 31, 25, 'o')
    return g

# =====================================================================
# Monde 2 — Le Palais de Sable (boss : le Garde royal, + le Miroir)
# =====================================================================
def build_palace():
    g = grid(PW, PH, '#')
    room(g, 13, 20, 26, 27)
    put(g, 20, 26, 'X')
    put(g, 19, 25, 'P')
    fill(g, 16, 21, 16, 26, '"'); fill(g, 23, 21, 23, 26, '"')
    fill(g, 18, 14, 21, 19, '.')
    put(g, 18, 14, 'S'); put(g, 21, 14, 'S')
    room(g, 3, 18, 11, 23)
    put(g, 11, 20, 'D')
    fill(g, 12, 20, 13, 21, '.')
    put(g, 6, 20, 'g'); put(g, 5, 21, 'h')
    room(g, 28, 18, 36, 23)
    put(g, 28, 20, 'D')
    fill(g, 26, 20, 27, 21, '.')
    put(g, 33, 20, 'i')                   # MIROIR D'ARGENT
    fill(g, 3, 10, 36, 12, '.')
    room(g, 5, 6, 8, 9)
    room(g, 31, 6, 34, 9)
    put(g, 6, 7, 'k'); put(g, 33, 7, 'k')
    put(g, 7, 9, 'b'); put(g, 32, 9, 'n')
    fill(g, 15, 11, 16, 11, '~'); fill(g, 23, 11, 24, 11, '~')
    put(g, 15, 10, 'z'); put(g, 24, 10, 'z')
    put(g, 12, 11, 'n'); put(g, 27, 11, 'o')
    room(g, 14, 2, 25, 8)
    put(g, 19, 5, '2')                    # LE GARDE NOIR ROYAL
    put(g, 16, 4, 'S'); put(g, 23, 4, 'S')
    put(g, 17, 7, 'o'); put(g, 22, 7, 'b')
    put(g, 20, 3, 'g')
    fill(g, 19, 9, 20, 13, '.')
    fill(g, 19, 13, 20, 19, '.')
    return g

# =====================================================================
# Monde 4 — La Forêt aux Mille Susurrations (labyrinthe)
# =====================================================================
def build_forest():
    g = grid(FW, FH, ',')
    border_trees(g)
    rng = random.Random(42)
    # labyrinthe d'arbres : couloirs de 2 de large
    for gy in range(3, FH-4, 5):
        for gx in range(3, FW-4, 6):
            if rng.random() < 0.85:
                fill(g, gx, gy, gx+2, gy+1, 'T')
            if rng.random() < 0.85:
                fill(g, gx+3, gy+2, gx+4, gy+3, 'T')
    # clairières
    fill(g, 18, 24, 28, 29, '.')
    put(g, 22, 29, 'X')                   # entrée -> vallée
    put(g, 21, 28, 'P')
    fill(g, 4, 4, 10, 9, '.')
    put(g, 6, 6, 'C')                     # -> Cavernes de cristal (secret)
    put(g, 5, 7, 'b'); put(g, 9, 5, 'b')
    fill(g, 32, 4, 40, 9, '.')
    put(g, 38, 6, 'h'); put(g, 34, 8, 'g')
    fill(g, 33, 22, 40, 28, '.')
    put(g, 38, 26, 'k'); put(g, 35, 24, 'g')
    # sentiers garanties
    fill(g, 20, 20, 26, 24, '.')
    fill(g, 8, 9, 12, 20, '.')
    fill(g, 12, 20, 20, 24, '.')
    fill(g, 26, 9, 34, 12, '.')
    fill(g, 34, 9, 36, 22, '.')
    fill(g, 12, 12, 34, 14, '.')
    # faune de la forêt
    for (x, y) in [(10,16),(24,10),(30,20),(16,26),(36,18),(28,16),(20,18)]:
        put(g, x, y, rng.choice('omb'))
    for (x, y) in [(14,22),(30,26)]:
        put(g, x, y, 'm')
    # trésors
    for (x, y) in [(6,26),(40,12),(18,8),(26,26),(4,14)]:
        put(g, x, y, 'g')
    put(g, 39, 4, 'h')
    return g

# =====================================================================
# Monde 5 — Les Cavernes de Cristal (boss : le Golem de cristal)
# =====================================================================
def build_crystal():
    g = grid(CW, CH, '#')
    room(g, 2, 24, 10, 27)
    put(g, 4, 26, 'X')
    put(g, 3, 25, 'P')
    fill(g, 5, 20, 6, 23, '.')
    room(g, 2, 12, 12, 19)
    # lacs de cristal (eau) + passerelles
    fill(g, 14, 12, 20, 19, '~')
    fill(g, 17, 12, 17, 19, '%')
    fill(g, 13, 15, 21, 16, '%')          # travée traversant tout le lac
    put(g, 16, 15, 'z'); put(g, 18, 17, 'z')
    room(g, 22, 12, 30, 19)
    put(g, 26, 15, 'k'); put(g, 28, 17, 'g')
    put(g, 24, 13, 'o'); put(g, 29, 18, 'b')
    fill(g, 11, 15, 13, 16, '.')
    fill(g, 21, 15, 21, 16, '.')
    room(g, 8, 4, 16, 9)
    fill(g, 11, 10, 12, 11, '.')
    put(g, 10, 6, 'h'); put(g, 14, 8, 'g')
    put(g, 12, 5, 'b')
    # antichambre + salle du boss
    room(g, 20, 22, 26, 26)
    fill(g, 25, 20, 26, 21, '.')
    room(g, 28, 21, 33, 26)
    fill(g, 27, 23, 27, 24, '.')
    put(g, 31, 24, '3')                   # LE GOLEM DE CRISTAL
    put(g, 29, 22, 'o'); put(g, 32, 25, 'b')
    put(g, 33, 21, 't'); put(g, 28, 26, 't')
    # cristaux décoratifs (statues)
    for (x, y) in [(3,13),(11,18),(23,13),(29,13),(9,25)]:
        put(g, x, y, 'S')
    for (x, y) in [(4,17),(24,18),(30,12)]:
        put(g, x, y, 'g')
    return g

# =====================================================================
# Monde 6 — Le Désert des Sables Mouvants
# =====================================================================
def build_desert():
    g = grid(SW, SH, '.')
    border_trees(g, 'R')                  # rochers au lieu d'arbres
    rng = random.Random(13)
    # dunes de rochers et gouffres
    for _ in range(26):
        x, y = rng.randint(3, SW-4), rng.randint(3, SH-4)
        if rng.random() < 0.5:
            fill(g, x, y, x+rng.randint(1,3), y+rng.randint(0,2), 'R')
        else:
            fill(g, x, y, x+rng.randint(1,2), y+rng.randint(1,2), ':')
    # oasis central
    for y in range(SH):
        for x in range(SW):
            dx, dy = (x-24)/5.0, (y-16)/4.0
            if dx*dx+dy*dy < 1.0:
                g[y][x] = '~'
    fill(g, 22, 14, 26, 18, '.')
    fill(g, 23, 13, 25, 13, 'B')
    put(g, 24, 15, 'h')
    # routes caravanières garanties
    fill(g, 4, 15, 22, 17, '.')
    fill(g, 26, 15, 42, 17, '.')
    fill(g, 40, 4, 43, 17, '.')
    fill(g, 8, 17, 8, 26, '.')
    fill(g, 8, 26, 30, 27, '.')
    # squelettes du désert (crabs) + sbires
    for (x, y) in [(12,12),(18,22),(30,10),(36,20),(14,26),(34,25),(44,12),(26,8)]:
        put(g, x, y, rng.choice('oo'))
    for (x, y) in [(20,18),(32,22)]:
        put(g, x, y, 'm')
    put(g, 42, 16, 'n')
    # la pyramide (nord-est)
    fill(g, 38, 2, 46, 8, '#')
    fill(g, 40, 4, 44, 7, '.')
    fill(g, 42, 8, 42, 9, '.')            # porte d'entrée de la pyramide
    put(g, 42, 5, 'X')                    # -> Nécropole
    put(g, 40, 7, 't'); put(g, 44, 7, 't')
    put(g, 40, 4, 't'); put(g, 44, 4, 't')
    # trésors
    fill(g, 43, 26, 46, 28, '.')          # éperon surplombant la route sud
    for (x, y) in [(6,6),(4,28),(20,28),(45,27),(30,4)]:
        put(g, x, y, 'g')
    put(g, 24, 16, 'P')
    # zones d'arrivée
    fill(g, 3, 14, 6, 18, '.')
    return g

# =====================================================================
# Monde 7 — La Nécropole des Rois (boss : le Pharaon momifié)
# =====================================================================
def build_necro():
    g = grid(NW, NH, '#')
    room(g, 17, 26, 23, 29)
    put(g, 20, 28, 'X')
    put(g, 19, 27, 'P')
    fill(g, 19, 22, 21, 25, '.')
    room(g, 6, 18, 34, 21)
    # piliers
    for x in range(8, 34, 5):
        fill(g, x, 19, x+1, 20, '#')
    put(g, 8, 18, 't'); put(g, 32, 18, 't')
    put(g, 12, 20, 'o'); put(g, 28, 19, 'b')
    fill(g, 8, 14, 9, 17, '.')
    fill(g, 31, 14, 32, 17, '.')
    room(g, 4, 8, 13, 13)
    put(g, 6, 10, 'k'); put(g, 11, 12, 'g')
    put(g, 8, 9, 'b'); put(g, 11, 9, 'o')
    room(g, 26, 8, 35, 13)
    put(g, 33, 10, 'k'); put(g, 28, 12, 'h')
    put(g, 30, 9, 'm'); put(g, 34, 12, 'b')
    fill(g, 18, 14, 22, 17, '.')
    room(g, 14, 3, 26, 8)
    put(g, 19, 5, '4')                    # LE PHARAON MOMIFIÉ
    put(g, 21, 5, 'b')
    put(g, 15, 3, 'S'); put(g, 25, 3, 'S')
    put(g, 20, 3, 'g')
    fill(g, 20, 8, 20, 13, '.')           # couloir étroit vers le sarcophage
    put(g, 20, 11, 'D')                   # porte verrouillée dans l'antichambre
    # sarcophages (statues)
    for (x, y) in [(5,19),(35,19),(5,9),(35,9)]:
        put(g, x, y, 'S')
    return g

# =====================================================================
# Monde 8 — Le Marais des Sangsues
# =====================================================================
def build_swamp():
    g = grid(MW, MH, ',')
    border_trees(g)
    rng = random.Random(99)
    # flaques d'eau partout
    for _ in range(40):
        x, y = rng.randint(3, MW-6), rng.randint(3, MH-6)
        for k in range(rng.randint(2, 5)):
            put(g, x+rng.randint(0,3), y+rng.randint(0,2), '~')
    # chemins de planches garantis
    fill(g, 3, 14, 8, 16, '%')
    fill(g, 8, 8, 10, 15, '%')
    fill(g, 10, 8, 24, 9, '%')
    fill(g, 24, 9, 26, 20, '%')
    fill(g, 26, 18, 38, 20, '%')
    fill(g, 38, 12, 40, 19, '%')
    fill(g, 8, 15, 12, 24, '%')
    fill(g, 12, 24, 30, 25, '%')
    fill(g, 30, 20, 31, 24, '%')
    # clairières (posées AVANT objets et portail)
    fill(g, 18, 3, 24, 6, '.')
    fill(g, 17, 6, 18, 8, '%')
    fill(g, 3, 14, 5, 16, '.')
    # cabane de l'ermite : coffres
    put(g, 20, 4, 'h'); put(g, 22, 5, 'g')
    # bestioles du marais (sur les planches)
    for (x, y) in [(9,10),(12,9),(20,9),(25,12),(30,19),(39,15),(10,20),(20,25),(31,22),(6,15)]:
        put(g, x, y, rng.choice('bbz'))
    for (x, y) in [(17,7),(25,15)]:
        put(g, x, y, 'm')
    # entrée / sortie
    put(g, 3, 15, 'X')                    # -> Ombre
    put(g, 4, 14, 'P')
    for (x, y) in [(40,13),(24,4),(10,24)]:
        put(g, x, y, 'g')
    put(g, 38, 19, 'h')
    return g

# =====================================================================
# Monde 9 — Le Col des Vautours
# =====================================================================
def build_pass():
    g = grid(GW, GH, '.')
    border_trees(g, 'R')
    rng = random.Random(55)
    # parois rocheuses formant un col en S
    fill(g, 8, 2, 12, 12, 'R')
    fill(g, 16, 8, 22, 20, 'R')
    fill(g, 26, 2, 32, 10, 'R')
    fill(g, 26, 16, 32, 25, 'R')
    # gouffres au bord des chemins
    for _ in range(10):
        x, y = rng.randint(3, GW-4), rng.randint(3, GH-4)
        if g[y][x] == '.':
            put(g, x, y, ':')
    # chemin garanti en S
    fill(g, 3, 13, 8, 15, '.')
    fill(g, 8, 13, 15, 15, '.')
    fill(g, 13, 15, 15, 22, '.')
    fill(g, 13, 21, 26, 23, '.')
    fill(g, 24, 11, 26, 22, '.')
    fill(g, 24, 11, 36, 13, '.')
    fill(g, 34, 13, 36, 15, '.')
    # rapaces (bats rapides) et gardes
    for (x, y) in [(10,14),(18,22),(28,12),(34,12),(25,12)]:
        put(g, x, y, 'b')
    put(g, 25, 22, 'n'); put(g, 26, 19, 'o')
    # entrée / sortie
    put(g, 3, 14, 'X')                    # -> Vallée
    put(g, 4, 13, 'P')
    put(g, 35, 14, 'X')                   # -> Mines
    for (x, y) in [(6,22),(24,13),(35,22)]:
        put(g, x, y, 'g')
    put(g, 14, 14, 'h')
    return g

# =====================================================================
# Monde 10 — Les Mines de Cuivre (l'Épée d'argent)
# =====================================================================
def build_mines():
    g = grid(KW, KH, '#')
    room(g, 2, 22, 9, 25)
    put(g, 4, 24, 'X')
    put(g, 3, 23, 'P')
    fill(g, 5, 18, 6, 21, '.')
    room(g, 2, 10, 12, 17)
    # piliers de mine
    for x in range(4, 12, 3):
        put(g, x, 12, '#'); put(g, x, 15, '#')
    put(g, 8, 13, 'o'); put(g, 4, 16, 'b')
    put(g, 10, 11, 'g')
    fill(g, 13, 13, 17, 14, '.')
    room(g, 18, 8, 27, 15)
    put(g, 22, 11, 'n'); put(g, 25, 14, 'o'); put(g, 19, 14, 'b')
    put(g, 26, 9, 'k')
    fill(g, 28, 11, 30, 12, '.')
    room(g, 31, 6, 33, 17)
    # la forge : voûte de l'épée d'argent
    room(g, 20, 19, 27, 24)
    fill(g, 22, 16, 23, 18, '.')
    put(g, 23, 21, 'e')                   # ÉPÉE D'ARGENT
    put(g, 20, 19, 't'); put(g, 27, 19, 't')
    put(g, 21, 23, 'n'); put(g, 26, 23, 'o')
    put(g, 32, 8, 'h'); put(g, 32, 15, 'g')
    for (x, y) in [(6,24),(18,9),(28,14)]:
        put(g, x, y, 'g')
    return g

# =====================================================================
# Monde 11 — L'Oasis du Palmier Bleu (refuge, fée, conteneur de cœur)
# =====================================================================
def build_oasis():
    g = grid(OW, OH, ',')
    border_trees(g, 'T')
    # lac sacré
    for y in range(OH):
        for x in range(OW):
            dx, dy = (x-13)/6.0, (y-11)/4.5
            if dx*dx+dy*dy < 1.0:
                g[y][x] = '~'
    fill(g, 11, 9, 15, 13, '.')
    put(g, 13, 11, 'F')                   # LA FÉE DU LAC (soigne)
    put(g, 12, 8, 'S'); put(g, 14, 8, 'S')
    put(g, 13, 6, 'H')                    # CONTENEUR DE CŒUR
    # entrée secrète
    fill(g, 3, 16, 6, 18, '.')
    put(g, 4, 17, 'C')                    # -> Vallée
    fill(g, 6, 12, 11, 17, '.')
    put(g, 6, 17, 'P')
    # palmiers décoratifs
    for (x, y) in [(4,4),(20,4),(4,18),(21,17),(8,19),(18,19)]:
        put(g, x, y, 'T')
    for (x, y) in [(7,7),(19,8),(6,14)]:
        put(g, x, y, '"')
    put(g, 20, 13, 'g')
    return g

# =====================================================================
# Monde 12 — Le Sanctuaire du Miroir (puzzle, accès à la Tour)
# =====================================================================
def build_sanctuary():
    g = grid(SW2, SH2, '#')
    room(g, 10, 17, 19, 23)
    put(g, 14, 21, 'X')                   # -> Ombre
    put(g, 13, 20, 'P')
    fill(g, 13, 12, 16, 16, '.')
    # allée de dalles-miroir décoratives
    put(g, 12, 14, 'M'); put(g, 14, 14, 'M'); put(g, 16, 14, 'M')
    put(g, 13, 12, 'S'); put(g, 15, 12, 'S')
    room(g, 8, 3, 21, 9)
    put(g, 13, 10, 'L')                   # porte scellée : 4 fragments
    fill(g, 13, 9, 15, 11, '.')
    put(g, 14, 5, 'X')                    # -> La Tour
    put(g, 10, 3, 't'); put(g, 19, 3, 't'); put(g, 10, 9, 't'); put(g, 19, 9, 't')
    put(g, 12, 7, 'S'); put(g, 16, 7, 'S')
    put(g, 14, 8, 'g')
    return g

# =====================================================================
# Monde 13 — La Tour du Vizir (donjon final)
# =====================================================================
def build_tower():
    g = grid(TW, TH, '#')
    room(g, 10, 27, 19, 32)
    put(g, 14, 30, 'X')                   # -> Sanctuaire
    put(g, 13, 29, 'P')
    room(g, 3, 20, 9, 26)
    put(g, 5, 23, 'o'); put(g, 7, 25, 'b'); put(g, 4, 25, 'g')
    room(g, 20, 20, 26, 26)
    put(g, 23, 23, 'm'); put(g, 25, 25, 'o'); put(g, 21, 25, 'h')
    room(g, 22, 12, 26, 18)
    put(g, 24, 15, 'n'); put(g, 25, 17, 'b')
    room(g, 16, 8, 21, 13)
    put(g, 19, 9, 'k'); put(g, 18, 12, 'g')
    room(g, 7, 8, 14, 13, deco=False)
    put(g, 15, 10, 'D')
    put(g, 10, 11, 'm'); put(g, 12, 12, 'h')
    room(g, 3, 12, 6, 19, deco=False)
    put(g, 5, 15, 'b'); put(g, 4, 18, 'o'); put(g, 6, 17, 'g')
    room(g, 3, 3, 9, 8)
    put(g, 6, 5, 'n'); put(g, 4, 7, 'b')
    # salle du portail supérieur, gardée
    room(g, 10, 2, 22, 7)
    put(g, 15, 4, '2')                    # un Garde royal elite devant le portail
    put(g, 19, 4, 'n')
    put(g, 11, 3, 'S'); put(g, 20, 3, 'S')
    put(g, 14, 3, 'X')                    # -> Trône des Ténèbres
    put(g, 14, 6, 'k')
    # liaisons
    fill(g, 8, 25, 13, 26, '.')
    fill(g, 16, 25, 21, 26, '.')
    fill(g, 23, 19, 24, 20, '.')
    fill(g, 21, 12, 22, 13, '.')
    fill(g, 4, 9, 5, 11, '.')
    return g

# =====================================================================
# Monde 14 — Le Trône des Ténèbres (arène finale : Ganar + Zahra)
# =====================================================================
def build_throne():
    g = grid(RW, RH, '#')
    room(g, 9, 15, 14, 19)
    put(g, 11, 18, 'X')                   # -> Tour
    put(g, 10, 17, 'P')
    fill(g, 10, 10, 13, 14, '.')
    # grande arène
    room(g, 4, 2, 19, 9)
    put(g, 11, 5, '9')                    # LE VIZIR GANAR
    put(g, 12, 4, 'Y')                    # la Princesse Zahra
    put(g, 6, 3, 'S'); put(g, 16, 3, 'S')
    put(g, 8, 8, 'b'); put(g, 14, 8, 'b')
    put(g, 5, 8, 't'); put(g, 18, 8, 't')
    return g

# =====================================================================
# Validation
# =====================================================================
def bfs(rows, extra=''):
    w = len(rows[0]); h = len(rows)
    sx = sy = None
    for y in range(h):
        for x in range(w):
            if rows[y][x] == 'P':
                sx, sy = x, y
    assert sx is not None, "pas de départ"
    passable = PASSABLE | set(extra)
    seen = {(sx, sy)}
    q = deque([(sx, sy)])
    while q:
        x, y = q.popleft()
        for dx, dy in ((1,0),(-1,0),(0,1),(0,-1)):
            nx, ny = x+dx, y+dy
            if 0 <= nx < w and 0 <= ny < h and (nx,ny) not in seen:
                if rows[ny][nx] in passable:
                    seen.add((nx, ny)); q.append((nx, ny))
    return seen

def validate(name, g):
    rows = render(g)
    w = len(rows[0])
    assert all(len(r) == w for r in rows), f"{name}: largeurs inégales"
    strict = bfs(rows)               # sans ouvrir les portes D
    lenient = bfs(rows, 'D')         # portes ouvertes (clé en main)
    n_k = sum(r.count('k') for r in rows)
    n_d = sum(r.count('D') for r in rows)
    assert n_k >= n_d, f"{name}: {n_d} portes pour {n_k} clés"
    missing = []
    for y in range(len(rows)):
        for x in range(w):
            c = rows[y][x]
            if c == 'k' and (x, y) not in strict:
                missing.append((c, x, y))
            elif c != 'k' and c in 'XCMhgkieHVYombn12349F' and (x, y) not in lenient:
                missing.append((c, x, y))
            elif c == 'D' and not any((x+dx, y+dy) in strict
                                      for dx, dy in ((1,0),(-1,0),(0,1),(0,-1))):
                missing.append((c, x, y))
    assert not missing, f"{name}: inaccessibles {missing}"
    return rows

builders = [
    ("La Vallée d'Ispahan", build_valley),
    ("Les Geôles Oubliées", build_dungeon),
    ("Le Palais de Sable", build_palace),
    ("L'Ombre de la Vallée", build_shadow),
    ("La Forêt aux Mille Susurrations", build_forest),
    ("Les Cavernes de Cristal", build_crystal),
    ("Le Désert des Sables Mouvants", build_desert),
    ("La Nécropole des Rois", build_necro),
    ("Le Marais des Sangsues", build_swamp),
    ("Le Col des Vautours", build_pass),
    ("Les Mines de Cuivre", build_mines),
    ("L'Oasis du Palmier Bleu", build_oasis),
    ("Le Sanctuaire du Miroir", build_sanctuary),
    ("La Tour du Vizir", build_tower),
    ("Le Trône des Ténèbres", build_throne),
]

rows_all = []
for name, fn in builders:
    rows_all.append((name, validate(name, fn())))

# invariant miroir : W0 et W3 partagent leurs dalles M
r0 = rows_all[0][1]
r3 = rows_all[3][1]
m0 = {(x, y) for y in range(len(r0)) for x in range(len(r0[0])) if r0[y][x] == 'M'}
m3 = {(x, y) for y in range(len(r3)) for x in range(len(r3[0])) if r3[y][x] == 'M'}
assert m0 == m3 and len(m0) >= 3, f"dalles-miroir désalignées {m0} vs {m3}"

print(f"OK — {len(rows_all)} mondes valides. M={sorted(m0)}")

for name, rows in rows_all:
    marks = []
    for y, r in enumerate(rows):
        for x, c in enumerate(r):
            if c in 'XCPiFeH12349VY':
                marks.append(f"{c}({x},{y})")
    print(f"{name}: {' '.join(marks)}")

# =====================================================================
# Émission du fichier Rust
# =====================================================================
def emit(var, rows, comment=""):
    out = [f"/// {comment}" if comment else ""]
    out.append(f"pub const {var}: &[&str] = &[")
    for r in rows:
        out.append(f'    "{r.replace(chr(34), chr(92)+chr(34))}",')
    out.append("];")
    return "\n".join(out)

names = [
    "MAP_VALLEY", "MAP_DUNGEON", "MAP_PALACE", "MAP_SHADOW", "MAP_FOREST",
    "MAP_CRYSTAL", "MAP_DESERT", "MAP_NECRO", "MAP_SWAMP", "MAP_PASS",
    "MAP_MINES", "MAP_OASIS", "MAP_SANCTUARY", "MAP_TOWER", "MAP_THRONE",
]
with open("/root/PrinceOfPersia/src/adventure/world_data.rs", "w") as f:
    f.write("//! Cartes des mondes — générées et validées par tools/genmaps.py.\n")
    f.write("//! Ne pas éditer à la main : modifier le générateur puis relancer.\n\n")
    for (name, rows), var in zip(rows_all, names):
        f.write(emit(var, rows, name) + "\n\n")
print("world_data.rs écrit (15 mondes).")
