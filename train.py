import os
import time
import torch
import wandb
import argparse
import torch.nn as nn
from model import Transformer
from torch.nn import functional as F
from data_utils import Encoding
import numpy as np

parser = argparse.ArgumentParser(
                    prog='Chess Transformer Trainer',
                    description='Train the transformer on a dataset')

parser.add_argument('-i', '--input', help='Input data .npy', default='data.npy')
parser.add_argument('-o', '--output', help='Output directory', required=True)
parser.add_argument('--block_size', help='Block (context) size', type=int)
parser.add_argument('--batch_size', help='Batch size', type=int, required=True)
parser.add_argument('--embedding_size', help='Embedding dimensions', type=int, )
parser.add_argument('--num_heads', help='Number of heads in the multi-head attention layer', type=int,) 
parser.add_argument('--num_blocks', help='Number of transformer blocks', type=int,)
parser.add_argument('--eval_iters', help='Evaluation iterations', type=int, required=True)
parser.add_argument('--eval_interval', help='Evaluation interval', type=int, required=True)
parser.add_argument('--learning_rate', help='Learning rate', type=float, )
parser.add_argument('--max_iters', help='Max Iterations', type=int, required=True)
parser.add_argument('--dropout', help='Dropout rate', type=float, )
parser.add_argument('--checkpoint', help='Saved checkpoint of a previous model iteration', type=str, default=None)
parser.add_argument('--keep_saves', help='Number of checkpoints to keep all previous checkpoints will be delete. All checkpoints be kept if no argument is provided', type=int, default=None)
args = parser.parse_args()

try:
    import torch_xla
    import torch_xla.core.xla_model as xm

    device = xm.xla_device()
except Exception as e:
    device = 'cuda' if torch.cuda.is_available() else 'cpu'

print(f"Running on {device}")

batch_size = args.batch_size
eval_iters = args.eval_iters
max_iters = args.max_iters
eval_interval = args.eval_interval

if "seed" in args: torch.manual_seed(args.seed)

start_iter = 0
if "checkpoint" in args and args.checkpoint is not None:
    checkpoint = torch.load(args.checkpoint)
    start_iter = checkpoint["current_iter"]
    run_id = checkpoint["model_id"]
    block_size = checkpoint["block_size"]
    n_embd = checkpoint["embedding_size"]
    n_heads = checkpoint["num_heads"]
    n_blocks = checkpoint["num_blocks"]
    dropout = checkpoint["dropout"]
    learning_rate = checkpoint["learning_rate"] 
else:
    run_id = wandb.util.generate_id()
    block_size = args.block_size
    n_embd = args.embedding_size
    n_heads = args.num_heads
    n_blocks = args.num_blocks
    dropout = args.dropout
    learning_rate = args.learning_rate

wandb.init(
    project="chess_transformer",
    id=run_id,
    config={
        "block_size": block_size,
        "batch_size": batch_size,
        "embedding_size": n_embd,
        "num_heads": n_heads,
        "num_blocks": n_blocks,
        "eval_iters": eval_iters,
        "learning_rate": learning_rate,
        "max_iters": max_iters,
        "eval_interval": eval_interval,
        "dropout": dropout,
    },
    resume='allow'
)


if "checkpoint" in args and args.checkpoint is not None:
    wandb.restore(args.checkpoint)

with open(args.input, 'rb') as f:
    tokens = np.load(f)

data = torch.from_numpy(tokens)
data = data.type(torch.LongTensor)
n = int(0.9*data.shape[0])
train_data = data[:n]
val_data = data[n:]

def get_batch(split):
    data = train_data if split == "train" else val_data
    ix = torch.randint(len(data) - block_size, (batch_size,))
    x = torch.stack([data[i:i+block_size] for i in ix])
    y = torch.stack([data[i+1:i+block_size+1] for i in ix])
    x, y = x.to(device), y.to(device)
    return x, y

@torch.no_grad()
def estimate_loss():
    out = {}
    model.eval()
    for split in ["train", "val"]:
        losses = torch.zeros(eval_iters)
        for k in range(eval_iters):
            X, Y = get_batch(split)
            _, loss = model(X, Y)
            losses[k] = loss.item()
        out[split] = losses.mean()
    model.train()
    return out

def save_checkpoint(save_path, step, model, optimizer, run_id):
    torch.save({
        "current_iter": step,
        "model_state_dict": model.state_dict(),
        "optimizer_state_dict": optimizer.state_dict(),
        "model_id": run_id,
        "block_size": block_size,
        "embedding_size": n_embd,
        "num_heads": n_heads,
        "num_blocks": n_blocks,
        "dropout": dropout,
        "learning_rate": learning_rate,
    }, save_path)
    #wandb.save(os.path.join(args.output, f"model-{run_id}-{step}.pth"))

def clear_past(keep_saves, saved_checkpoints):
    if keep_saves is None: return
    while len(saved_checkpoints) >= keep_saves:
        os.remove(saved_checkpoints.pop(0))

model = Transformer(block_size, Encoding().n_vocab, n_embd, n_heads, n_blocks, dropout, device)
model.train()
model.to(device)
optimizer = torch.optim.AdamW(model.parameters(), lr=learning_rate)

if "checkpoint" in args and args.checkpoint is not None:
    model.load_state_dict(checkpoint["model_state_dict"])
    optimizer.load_state_dict(checkpoint["optimizer_state_dict"])

training_epoch_loss = []
validation_epoch_loss = []
saved_checkpoints = []
for step in range(start_iter, max_iters):
    try:
        if step % eval_interval == 0:
            losses = estimate_loss()
            training_epoch_loss.append(losses['train'])
            validation_epoch_loss.append(losses['val'])
            clear_past(args.keep_saves, saved_checkpoints)
            save_checkpoint(os.path.join(args.output, f"model-{run_id}-{step}.pth"), step, model, optimizer, run_id)
            saved_checkpoints.append(os.path.join(args.output, f"model-{run_id}-{step}.pth"))
            print(f"step: {step}, training loss: {losses['train']}, validation loss: {losses['val']}")
            wandb.log({"training_loss": losses['train'], "validation_loss": losses['val']})
        logits, loss = model(*get_batch("train"))
        optimizer.zero_grad(set_to_none=True)
        loss.backward()
        optimizer.step()
    except KeyboardInterrupt as e:
        break

losses = estimate_loss()
training_epoch_loss.append(losses['train'])
validation_epoch_loss.append(losses['val'])
clear_past(args.keep_saves, saved_checkpoints)
save_checkpoint(os.path.join(args.output, f"model-{run_id}-{step}.pth"), step, model, optimizer, run_id)
saved_checkpoints.append(os.path.join(args.output, f"model-{run_id}-{step}.pth"))
print(f"step: {step}, training loss: {losses['train']}, validation loss: {losses['val']}")
wandb.log({"training_loss": losses['train'], "validation_loss": losses['val']})

wandb.finish()
