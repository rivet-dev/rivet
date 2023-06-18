# Generate SSH Key Pair

## Generate Keys

```
rm -rf /tmp/keys
mkdir /tmp/keys
for key in nebula_lighthouse salt_master salt_minion; do
	ssh-keygen -f /tmp/keys/$key -t ecdsa -b 521
done
```

