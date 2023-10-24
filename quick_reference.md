# Quick reference - cheatsheet
## Docker containers
- Recreate a container: `docker-compose up -d --force-recreate --build container_name`
- Rebuild image ignoring cache (clean build): `docker-compose build --no-cache`
### `irmaserver` container
- Issue credentials: `irma session --issue irma-demo.PEP.id=xkjedjejencoemncjedbncienkcneocoecn --server http://127.0.0.1:8088 --disable-schemes-update`


## composer & PHP projects
When facing a composer configuration problem, remeber to try using `composer config` instead of directly editing the `composer.json` file. It does magic.
