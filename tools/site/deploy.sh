#!/bin/bash
# Copies the built site (tools/site/build.sh -> site/) to a server running Caddy and serves it on port 80.
#   tools/site/deploy.sh root@10.27.27.239            the site goes to /srv/rustfoot98; the Caddyfile is replaced (the old one
#                                                     is kept as /etc/caddy/Caddyfile.bak)
# Build first, e.g. GALLERY=1 SKIN=1 bash tools/site/build.sh. ssh asks for the password once per connection (two).
set -e
cd "$(dirname "$0")/../.."
host=${1:?usage: tools/site/deploy.sh user@host}
test -f site/index.html || { echo "site/ missing: run tools/site/build.sh first" >&2; exit 1; }
tar -C site -czf - . | ssh "$host" 'set -e
  rm -rf /srv/rustfoot98.new && mkdir -p /srv/rustfoot98.new && tar -C /srv/rustfoot98.new -xzf -
  rm -rf /srv/rustfoot98 && mv /srv/rustfoot98.new /srv/rustfoot98'
ssh "$host" 'set -e
  test -f /etc/caddy/Caddyfile.bak || cp /etc/caddy/Caddyfile /etc/caddy/Caddyfile.bak
  cat > /etc/caddy/Caddyfile <<EOF
:80 {
	root * /srv/rustfoot98
	encode gzip
	file_server
	# the game asks the browser to check sw.js and the pages on every visit, so an update shows at once
	@fresh path /play/sw.js /play/*.html
	header @fresh Cache-Control "no-cache"
}
EOF
  caddy validate --config /etc/caddy/Caddyfile --adapter caddyfile >/dev/null
  systemctl reload caddy
  echo "deployed: $(du -sh /srv/rustfoot98 | cut -f1) in /srv/rustfoot98"'
echo "docs: http://${host#*@}/   game: http://${host#*@}/play/classic.html   editor: http://${host#*@}/play/editor.html"
