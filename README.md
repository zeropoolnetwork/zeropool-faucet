# zeropool-faucet

A simple faucet service for various testnets. Only supports the NEAR testnet for now.

## Endpoints

`POST /:chain/:token` - transfer configured amount of tokens to the specified address.
### Request format:
```json
{
    "amount": "1000000000000000000000000",
    "to": "address"
}
```

`GET /info` - get information about the faucet.