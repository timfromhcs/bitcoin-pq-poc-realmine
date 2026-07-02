# Stratum V1 Mining Protocol - Quick Reference

## Pool: public-pool.io (PPLNS)
| Mode | URL |
|------|-----|
| Stratum V1 | `stratum+tcp://public-pool.io:13333` ✅
| Stratum V1+TLS | `stratum+tls://public-pool.io:14333`
| Stratum V2 | `stratum+tcp://public-pool.io:23331`
| Datum | `datum://public-pool.io:23336`

## Key
- Stratum: `9c4zpyJ2ndm4e8sP2uNc1VNCGxYjqaxWS6wUCjk8zFj6njFquH6`
- Datum: `96c75030cb6efd4f18ad160640a3b52f9a57b9b01f5e6d532459d48cb2d9e71b56571aaf39174e2239ff5c55c1e71a7afbc4311649d67d59fe691fe237db2308`

## Username Format
```
<BTC address>.<worker name>
```

## Message Flow
```
C: {"id":1,"method":"mining.subscribe","params":["HCSminer/2.0"]}
S: {"id":1,"result":[[["mining.set_difficulty","..."]],"extranonce1",2],...}
C: {"id":2,"method":"mining.authorize","params":["btc.worker","x"]}
S: {"id":2,"result":true,...}
S: {"id":null,"method":"mining.set_difficulty","params":[1.0]}
S: {"id":null,"method":"mining.notify","params":[job_id,prevhash,coinb1,coinb2,[merkle],version,nbits,ntime,clean]}
C: {"id":3,"method":"mining.submit","params":["btc.worker",job_id,extranonce2,ntime,nonce]}
```

## Mining Math
1. `coinbase = coinb1 + extranonce1 + extranonce2 + coinb2`
2. `cb_hash = double_sha256(coinbase)`
3. `merkle_root = merkle_reduce(cb_hash, merkle_branches)`
   - For each branch: `combined = root + branch` (concatenate, not reversed)
   - `root = double_sha256(combined)`
4. `header = version(LE) + prevhash(rev) + merkle_root(rev) + ntime(LE) + nbits(LE) + nonce(LE)`
5. `hash = double_sha256(header)`
6. Check: `hash < target_from_nbits`

