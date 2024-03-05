import chess
import torch
from agents.transformer.model import Transformer
from agents.transformer.data_utils import Encoding

class TransformerAgent():
    def __init__(self, name):
        self.name = name
        self.enc = Encoding()

        checkpoint = torch.load("agents/transformer/models/glorious-galaxy-62/model-trnwcgr7-48600.pth", map_location=torch.device('cpu'))

        n_vocab = self.enc.n_vocab
        block_size = checkpoint["block_size"]
        n_embd = checkpoint["embedding_size"]
        n_heads = checkpoint["num_heads"]
        n_blocks = checkpoint["num_blocks"]
        dropout = checkpoint["dropout"]

        self.device = 'cuda' if torch.cuda.is_available() else 'cpu'

        self.model = Transformer(block_size, n_vocab, n_embd, n_heads, n_blocks, dropout, self.device)
        self.model.to(self.device)
        self.model.load_state_dict(checkpoint["model_state_dict"])
        self.model.eval()

    def next_move(self, board: chess.Board, last_move: str, next_legal_moves: chess.LegalMoveGenerator):
        if last_move is not None: self.context.append(self.enc.encode(last_move))
        while True:
            move_idx = self.model.generate(torch.tensor([self.context], dtype=torch.long, device=self.device), max_new_tokens=1)[0].tolist()[0]
            move = self.enc.decode(move_idx)
            try:
                if chess.Move.from_uci(move) in board.legal_moves:
                    self.context.append(move_idx)
                    return move
            except Exception as e:
                #print(f"Context: {self.context}")
                print(f"Context length: {len(self.context)}")
                print(f"Invalid move: {move}")
        return None

    def reset(self, self_color):
        self.context = [self.enc.encode(f'<s:{"WHITE" if self_color == chess.WHITE else "BLACK" }>')]

    def terminate(self):
        pass
