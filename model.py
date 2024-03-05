import torch
import torch.nn as nn
from torch.nn import functional as F

class Head(nn.Module):
    def __init__(self, block_size, n_embd, head_size, dropout):
        super().__init__()
        self.query = nn.Linear(n_embd, head_size, bias=False)
        self.key = nn.Linear(n_embd, head_size, bias=False)
        self.value = nn.Linear(n_embd, head_size, bias=False)
        self.register_buffer('tril', torch.tril(torch.ones((block_size, block_size))))
        self.dropout = nn.Dropout(dropout)
    def forward(self, x):
        _, T, C = x.shape
        q = self.query(x)
        k = self.key(x)
        wei = q @ k.mT * C**-0.5
        
        wei = wei.masked_fill(self.tril[:T, :T] == 0, float('-inf'))
        wei = F.softmax(wei, dim=-1)
        wei = self.dropout(wei)

        v = self.value(x)
        out = wei @ v
        return out

class MultiHeadAttention(nn.Module):
    def __init__(self, block_size, n_embd, n_heads, dropout):
        super().__init__()
        self.block_size = block_size
        self.n_embd = n_embd
        self.n_heads = n_heads
        assert (self.n_embd % self.n_heads) == 0
        self.head_size = n_embd // self.n_heads
        self.heads = nn.ModuleList([Head(self.block_size, self.n_embd, self.head_size, dropout) for _ in range(self.n_heads)])
        self.projection = nn.Linear(n_embd, n_embd)
        self.dropout = nn.Dropout(dropout)
    def forward(self, x):
        x = torch.cat([head(x) for head in self.heads], dim=-1)
        x = self.dropout(self.projection(x))
        return x

class FeedForward(nn.Module):
    def __init__(self, n_embd, dropout):
        super().__init__()
        self.n_embd = n_embd
        self.net = nn.Sequential(
            nn.Linear(n_embd, n_embd * 4),
            nn.ReLU(),
            nn.Linear(n_embd * 4, n_embd),
            nn.Dropout(dropout),
        )
        
    def forward(self, x): return self.net(x)

class TransformerBlock(nn.Module):
    def __init__(self, block_size, n_embd, n_heads, dropout):
        super().__init__()
        self.block_size = block_size
        self.n_embd = n_embd
        self.n_heads = n_heads
        self.dropout = dropout

        self.self_attention = MultiHeadAttention(self.block_size, self.n_embd, self.n_heads, self.dropout)
        self.feedforward = FeedForward(self.n_embd, self.dropout)
        self.ln1 = nn.LayerNorm(self.n_embd)
        self.ln2 = nn.LayerNorm(self.n_embd)
    def forward(self, x):
        x = x + self.self_attention(self.ln1(x))
        x = x + self.feedforward(self.ln2(x))
        return x

class Transformer(nn.Module):
    def __init__(self, block_size, n_vocab, n_embd, n_heads, n_blocks, dropout, device):
        super().__init__()
        self.block_size = block_size
        self.n_vocab = n_vocab
        self.n_embd = n_embd
        self.n_heads = n_heads
        self.n_blocks = n_blocks
        self.dropout = dropout
        self.device = device
        
        self.embedding = nn.Embedding(self.n_vocab, self.n_embd)
        self.positional_embedding = nn.Embedding(self.block_size, self.n_embd)
        self.blocks = nn.Sequential(*[TransformerBlock(self.block_size, self.n_embd, self.n_heads, self.dropout) for _ in range(self.n_blocks)])
        self.ln = nn.LayerNorm(self.n_embd)
        self.lm_head = nn.Linear(self.n_embd, self.n_vocab)
    def forward(self, idx, targets=None):
        _, T = idx.shape
        tok_embd = self.embedding(idx)
        pos_embd = self.positional_embedding(torch.arange(0, T, device=self.device))
        x = tok_embd + pos_embd
        x = self.blocks(x)
        x = self.ln(x)
        logits = self.lm_head(x)
        
        if targets is None:
            return logits, None
        
        B, T, C = logits.shape
        logits = logits.view(B*T, C)
        targets = targets.view(B*T)
        loss = F.cross_entropy(logits, targets)
        return logits, loss

    def generate(self, idx, max_new_tokens):
        new_toks = torch.tensor([[]], dtype=torch.long, device=self.device)
        for _ in range(max_new_tokens):
            idx_cond = idx[:, -self.block_size:]
            logits, _ = self(idx_cond)
            logits = logits[:, -1, :]
            probs = F.softmax(logits, dim=1)
            idx_next = torch.multinomial(probs, num_samples=1)
            idx = torch.cat((idx, idx_next), dim=1)
            new_toks = torch.cat((new_toks, idx_next), dim=1)
        return new_toks
