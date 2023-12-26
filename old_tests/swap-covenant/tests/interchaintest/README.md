# interchaintest setup

Prior to running the interchaintests, a modification of the stride image is needed.
We are using the [v9.2.1 tagged version](https://github.com/Stride-Labs/stride/tree/v9.2.1) image.

In there, we alter the `utils/admins.go` as follows to allow minting tokens from our address in the tests:
```go
var Admins = map[string]bool{  
-       "stride1k8c2m5cn322akk5wy8lpt87dd2f4yh9azg7jlh": true, // F5
+       "stride1u20df3trc2c2zdhm8qvh2hdjx9ewh00sv6eyy8": true, // F5
        "stride10d07y265gmmuvt4z0w9aw880jnsr700jefnezl": true, // gov module
}
```

Then we use heighliner by strangelove to build a local docker image, [as described in their documentation](https://github.com/strangelove-ventures/heighliner#example-cosmos-sdk-chain-development-cycle-build-a-local-repository):
```bash
# in the stride directory
heighliner build -c stride --local
```

With stride image present in our local docker, we are ready to run the interchaintests. To do that, we navigate to the `stride-covenant` directory and run `just simtest`.
