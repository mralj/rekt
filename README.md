## Info

This repo is for the Liquidity Sniping Bot on BSC. It has achieved some pretty good results, constantly being in the top three bots in the world (usually 1st or 2nd). 
Top 3 means that usually, we were placed +1/2/3 txs away from the Liqudity tx we were targeting.

## Example
Here is an example of sniping Token PepeV2, where the bot made a +70% profit.
1. Target add liquidity tx: [link](https://bscscan.com/tx/0x0cf8d7cf02a0059cc6d4c9ebef0d6c9f60d6c8fd2386d8b0c196ffb7773ecf33)
2. Our buy tx (placement 2nd): [link](https://bscscan.com/tx/0x1c891a2b48e9ecd6fd9a77eb2553533d43d1f503ddd5b9d39d781b3cefef5e06)
3. Our sell txs: [link](https://bscscan.com/token/0x08068904d055d5933036b0c4afba400498c662eb?a=0x44f7f6773b6889c9ac013ad63bf2d84a9346387b)

We send multiple buy transactions to ensure the best possible propagation through the p2p network (multiple transactions from multiple servers; this was variable but usually around 10tx per 20ish servers). 

What happened is that the server that _caught_ liquidity the first was so fast that the fastest transactions accidentally front-ran the liquidity tx. So, while the actual buy was placed second, we were the fastest overall (first bot in the block): [link](https://bscscan.com/txs?block=28635021&p=9). 
Note: the easiest way to spot the bots here is a bunch of failed txs. 

## General architecture overview

The bot is like a super-light BSC/ETH node. It has fully implemented the DEVP2P layer and some parts of the ETH layer, but it doesn't store any blockchain state or has most of the features you would expect from a regular node.

The idea is that when sniping like this, you don't don't need access to the blockchain state. You need to catch the tx you are targeting with the lowest latency possible and send your _buy txs_ with the lowest latency possible. The time scale is microseconds, e.g., if you are ~100 microseconds too slow, you are late, other bots will beat you, and chances for profit are much lower. 


I usually ran ~ about 20 servers on AWS, at least one server per region subnet. This is because you don't know (or at least I didn't) where TX will originate or where the next validator will be. 

Since there was no need to sync blockchain or anything like that, I needed to run the servers only for sniping, vastly decreasing the cost of this whole operation. Also, this enabled me to run the thing on smaller instances, usually `c7g.2xlarge` (I could've used the much cheaper instances to operate this, but I was having latency issues given that I was typically connected to ~500-1k peers, and you need to broadcast txs to all of them as quickly as possible).



### On code:
The code quality is mostly _meh_; some parts are great, and some are bad, but importantly, it is really fast. 
I was working on this alone and was rushing to get it working ASAP. I haven't had time to pay attention to code quality/best practices, etc. 
Additionally, my blockchain knowledge at the time of writing this was 0. 

### Acknowledgements
Small part (especially around RPLX and ECIES) was isnpired by [reth](https://github.com/paradigmxyz/reth). 
open-fast RLP is [gakonst/open-fastrlp](https://github.com/gakonst/open-fastrlp) + optimziations and additions I needed.

 
