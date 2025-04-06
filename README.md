## Info

This repo is for Liquidty Sniping Bot on BSC. It has achieved some pretty good results, constantly being in top 3 bots in the world (usually 1st or 2nd). 
Top 3 means that usually, we were placed +1/2/3 txs away from the Liqudity tx we were targeting.

### Example
Here is an example of sniping Token PepeV2 where bot made +70% profit.
1. Target add liqudity tx: [link](https://bscscan.com/tx/0x0cf8d7cf02a0059cc6d4c9ebef0d6c9f60d6c8fd2386d8b0c196ffb7773ecf33)
2. Our buy tx (placement 2nd): [link](https://bscscan.com/tx/0x1c891a2b48e9ecd6fd9a77eb2553533d43d1f503ddd5b9d39d781b3cefef5e06)
3. Our sell txs: [link](https://bscscan.com/token/0x08068904d055d5933036b0c4afba400498c662eb?a=0x44f7f6773b6889c9ac013ad63bf2d84a9346387b)

We send multiple buy transactions to ensure the best possible propagation throuhg p2p network (mutliple transactions from multiple servers, this was variable but usually around 10tx per 20ish servers). 
What happened is that the server which _caught_ liqudity the first was so fast that the fastest transactions accidentaly front-ran the liqudity tx, so while actual buy was placed 2nd, we were fastest overall (first bot in the block): [link](https://bscscan.com/txs?block=28635021&p=9). 
Note: the easiest way to spot the bots here is bunch of falied txs. 
