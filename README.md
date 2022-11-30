# zeropool-faucet

A simple faucet service for various testnets. Only supports the NEAR testnet for now.

## Usage
This service requires some environment variables to be set. They are listed in the `.env.example` file.

For testing, you can run it locally:
`cargo run`

For a production setup, take a look at the `docker-compose.prod.yml` file.


## Endpoints
`POST /near/:address` - transfer configured amount of tokens to the specified address.
`GET /info` - get information about the faucet.