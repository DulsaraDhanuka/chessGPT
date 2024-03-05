vocab = {}
index = 0

itof = {0: 'a', 1: 'b', 2: 'c', 3: 'd', 4: 'e', 5: 'f', 6: 'g', 7: 'h'}
itor = {0: '1', 1: '2', 2: '3', 3: '4', 4: '5', 5: '6', 6: '7', 7: '8'}

def insert_vocab(move):
    global index, vocab
    vocab[move] = index
    vocab[index] = move
    index += 1

insert_vocab("<s:WHITE>")
insert_vocab("<s:BLACK>")
insert_vocab("<s:DRAW>")
insert_vocab("</s>")

for i in range(8):
    for j in range(8):
        for k in range(8):
            for l in range(8):
                insert_vocab(f"{itof[i]}{itor[j]}{itof[k]}{itor[l]}")

for i in range(8):
    for piece in ['b', 'n', 'q', 'r']:
        if i != 0:
            insert_vocab(itof[i] + '7' + itof[i-1] + '8' + piece)
        if i != 7:
            insert_vocab(itof[i] + '7' + itof[i+1] + '8' + piece)
        insert_vocab(itof[i] + '7' + itof[i] + '8' + piece)

for i in range(8):
    for piece in ['b', 'n', 'q', 'r']:
        if i != 0:
            insert_vocab(itof[i] + '2' + itof[i-1] + '1' + piece)
        if i != 7:
            insert_vocab(itof[i] + '2' + itof[i+1] + '1' + piece)
        insert_vocab(itof[i] + '2' + itof[i] + '1' + piece)

import pickle
with open("vocab.pickle", "wb") as f:
    pickle.dump(vocab, f)

