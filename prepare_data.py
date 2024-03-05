import os
import chess
import chess.pgn
from data_utils import Encoding
import numpy as np

dir_path = os.path.dirname(os.path.realpath(__file__))
data_path = os.path.join(dir_path, "data_pgn")

pgn_files = [f for f in os.listdir(data_path) if os.path.isfile(os.path.join(data_path, f)) and os.path.splitext(f)[1] == ".pgn"]
enc = Encoding()
white_games = 0
black_games = 0
draw_games = 0
tokens = []
for pgn_name in pgn_files:
    with open(os.path.join(data_path, pgn_name), 'r', encoding='utf-8') as pgn:
        while True:
            try:
                game = chess.pgn.read_game(pgn)
                if game is None: break
                game_status = "DRAW"
                if game.headers["Result"].split("-")[0] == "1":
                    game_status = "WHITE"
                    white_games += 1
                elif game.headers["Result"].split("-")[1] == "1":
                    game_status = "BLACK"
                    black_games += 1
                else:
                    draw_games += 1
                tokens.append(enc.encode(f"<s:{game_status}>"))
                for move in game.mainline_moves():
                    tokens.append(enc.encode(move.uci()))
                tokens.append(enc.encode("</s>"))
            except Exception as e:
                print(e)
                continue
n_vocab = enc.n_vocab
print()
print("White games: ", white_games)
print("Black games: ", black_games)
print("Draw games: ", draw_games)
print(f"Training data loaded - {len(tokens)} tokens")

with open("data.npy", "wb") as f:
    np.save(f, np.array(tokens, dtype=np.int32))

