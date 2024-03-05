import torch
from model import Transformer
from data_utils import Encoding

enc = Encoding()

n_vocab = enc.n_vocab

# with open("models/model.config", "r") as f:
#     args = json.load(f)
# block_size = args.block_size
# batch_size = args.batch_size
# n_embd = args.embedding_size
# n_heads = args.num_heads
# n_blocks = args.num_blocks
# eval_iters = args.eval_iters
# learning_rate = args.learning_rate
# max_iters = args.max_iters
# eval_interval = args.eval_interval
# dropout = args.dropout

block_size = 256
n_embd = 384
n_heads = 6
n_blocks = 6
dropout = 0.2
device = 'cuda' if torch.cuda.is_available() else 'cpu'

model = Transformer(block_size, n_vocab, n_embd, n_heads, n_blocks, dropout, device)
model.to(device)
model.load_state_dict(torch.load("models/model-1705985287.1557353-1500.pth", map_location=torch.device('cpu')))
model.eval()

context = []
while True:
    try:
        context += [enc.encode(input("> "))]
        x = model.generate(torch.tensor([context], dtype=torch.long, device=device), max_new_tokens=1)[0].tolist()
        context += x
        print([enc.decode(tok) for tok in x])
    except KeyboardInterrupt as e:
        break
