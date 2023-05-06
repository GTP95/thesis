# Quick reference - cheatsheet
## Docker containers
- Recreate a container: docker-compose up -d --force-recreate --build container_name  
Note that this doesn't rebuild the image! Which I would have expected, due to the --build flag... I think that
- `docker-compose up -d --force-recreate --build --no-cache container_name` does the trick
### `irmaserver` container
- Issue credentials: `irma session --issue irma-demo.PEP.id=xkjedjejencoemncjedbncienkcneocoecn --server http://127.0.0.1:8088 --disable-schemes-update`
