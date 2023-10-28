source .env

docker stop hardware-monitoring-container
docker rm hardware-monitoring-container

docker run \
-p 8080:80 \
-v $(pwd)/../frontend:/usr/share/nginx/html:ro \
-v $(pwd)/default.conf:/etc/nginx/conf.d/default.conf:ro \
--name hardware-monitoring-container \
nginx
